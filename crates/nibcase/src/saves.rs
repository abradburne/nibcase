//! Battery saves and versioned states, written atomically on a dedicated non-real-time thread.
use nibcase_libretro::{
    ffi::{RETRO_MEMORY_RTC, RETRO_MEMORY_SAVE_RAM},
    Core, Frame,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC: &[u8; 8] = b"NIBSTATE";
const LIMIT: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub core: String,
    pub version: String,
    #[serde(with = "hex")]
    pub game_hash: u64,
    pub created: u64,
    pub play_seconds: u64,
    #[serde(with = "hex")]
    pub state_hash: u64,
}
#[derive(Clone)]
pub struct State {
    pub metadata: Metadata,
    pub picture: Frame,
    pub data: Vec<u8>,
}
pub fn hash(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
fn png(frame: &Frame) -> Result<Vec<u8>, String> {
    if frame.width == 0 || frame.height == 0 {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, frame.width, frame.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&frame.rgba)
            .map_err(|e| e.to_string())?;
    }
    Ok(out)
}
fn thumbnail(frame: &Frame) -> Frame {
    if frame.width == 0 || frame.height == 0 {
        return Frame::default();
    }
    let scale = (256.0 / f64::from(frame.width))
        .min(240.0 / f64::from(frame.height))
        .min(1.0);
    let (w, h) = (
        (f64::from(frame.width) * scale).round().max(1.0) as u32,
        (f64::from(frame.height) * scale).round().max(1.0) as u32,
    );
    let mut rgba = vec![0; w as usize * h as usize * 4];
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = (x * frame.width / w, y * frame.height / h);
            rgba[((y * w + x) * 4) as usize..][..4]
                .copy_from_slice(&frame.rgba[((sy * frame.width + sx) * 4) as usize..][..4]);
        }
    }
    Frame {
        width: w,
        height: h,
        rgba,
    }
}
impl State {
    pub fn capture(core: &mut Core, game_hash: u64, seconds: u64) -> Result<Self, String> {
        let data = core.serialize()?;
        Ok(Self {
            metadata: Metadata {
                core: core.info().name.clone(),
                version: core.info().version.clone(),
                game_hash,
                created: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                play_seconds: seconds,
                state_hash: hash(&data),
            },
            picture: thumbnail(core.frame()),
            data,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let meta = toml::to_string(&self.metadata)
            .map_err(|e| e.to_string())?
            .into_bytes();
        let picture = png(&self.picture)?;
        if meta.len() > 65536 || picture.len() > 16 * 1024 * 1024 || self.data.len() > LIMIT {
            return Err("save state exceeds size limit".into());
        }
        let mut out = MAGIC.to_vec();
        out.extend(1u32.to_le_bytes());
        for len in [meta.len(), picture.len(), self.data.len()] {
            out.extend((len as u32).to_le_bytes());
        }
        out.extend(meta);
        out.extend(picture);
        out.extend(&self.data);
        Ok(out)
    }
    pub fn read(path: &Path) -> Result<Self, String> {
        let f = File::open(path).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        f.take((LIMIT + 16 * 1024 * 1024 + 65536 + 25) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Self::decode(&bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 24
            || &bytes[..8] != MAGIC
            || u32::from_le_bytes(bytes[8..12].try_into().unwrap()) != 1
        {
            return Err("not a nibcase version 1 state".into());
        }
        let mut lengths = [0usize; 3];
        for (i, len) in lengths.iter_mut().enumerate() {
            *len = u32::from_le_bytes(bytes[12 + i * 4..16 + i * 4].try_into().unwrap()) as usize;
        }
        let [meta_len, picture_len, data_len] = lengths;
        if meta_len > 65536
            || picture_len > 16 * 1024 * 1024
            || data_len > LIMIT
            || data_len == 0
            || 24 + meta_len + picture_len + data_len != bytes.len()
        {
            return Err("truncated or oversized state".into());
        }
        let metadata: Metadata = toml::from_str(
            std::str::from_utf8(&bytes[24..24 + meta_len]).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let data = bytes[24 + meta_len + picture_len..].to_vec();
        if hash(&data) != metadata.state_hash {
            return Err("state checksum does not match".into());
        }
        let picture = if picture_len == 0 {
            Frame::default()
        } else {
            let decoder = png::Decoder::new(&bytes[24 + meta_len..24 + meta_len + picture_len]);
            let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
            if reader.info().width > 256
                || reader.info().height > 240
                || reader.info().color_type != png::ColorType::Rgba
                || reader.info().bit_depth != png::BitDepth::Eight
            {
                return Err("invalid state thumbnail".into());
            }
            let mut rgba = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut rgba).map_err(|e| e.to_string())?;
            rgba.truncate(info.buffer_size());
            Frame {
                width: info.width,
                height: info.height,
                rgba,
            }
        };
        Ok(Self {
            metadata,
            picture,
            data,
        })
    }
    pub fn restore(&self, core: &mut Core, game_hash: u64) -> Result<(), String> {
        if self.metadata.game_hash != game_hash {
            return Err("state belongs to another ROM".into());
        }
        if self.metadata.core != core.info().name || self.metadata.version != core.info().version {
            return Err(format!(
                "state was made with {} {}",
                self.metadata.core, self.metadata.version
            ));
        }
        core.unserialize(&self.data)
    }
}

enum Job {
    Write(PathBuf, Vec<u8>),
    Barrier(mpsc::Sender<Result<(), String>>),
}
pub struct Writer {
    sender: Option<SyncSender<Job>>,
    errors: Receiver<String>,
    handle: Option<JoinHandle<()>>,
}
impl Writer {
    pub fn new() -> Self {
        let (sender, jobs) = mpsc::sync_channel(4);
        let (report, errors) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut failed = None;
            let mut counter = 0u64;
            while let Ok(job) = jobs.recv() {
                match job {
                    Job::Write(path, data) => {
                        counter += 1;
                        if let Err(error) = atomic_write(&path, &data, counter) {
                            let error = format!("{}: {error}", path.display());
                            let _ = report.send(error.clone());
                            failed = Some(error);
                        }
                    }
                    Job::Barrier(reply) => {
                        let _ = reply.send(failed.take().map_or(Ok(()), Err));
                    }
                }
            }
        });
        Self {
            sender: Some(sender),
            errors,
            handle: Some(handle),
        }
    }
    pub fn write(&self, path: PathBuf, data: Vec<u8>) -> Result<(), String> {
        self.sender
            .as_ref()
            .unwrap()
            .send(Job::Write(path, data))
            .map_err(|_| "save writer stopped".into())
    }
    pub fn try_write(&self, path: PathBuf, data: Vec<u8>) -> Result<bool, String> {
        match self
            .sender
            .as_ref()
            .unwrap()
            .try_send(Job::Write(path, data))
        {
            Ok(()) => Ok(true),
            Err(TrySendError::Full(_)) => Ok(false),
            Err(_) => Err("save writer stopped".into()),
        }
    }
    pub fn errors(&self) -> Vec<String> {
        self.errors.try_iter().collect()
    }
    pub fn flush(&self) -> Result<(), String> {
        let (reply, done) = mpsc::channel();
        self.sender
            .as_ref()
            .unwrap()
            .send(Job::Barrier(reply))
            .map_err(|_| "save writer stopped")?;
        done.recv().map_err(|_| "save writer stopped")?
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(t) = self.handle.take() {
            let _ = t.join();
        }
    }
}
fn atomic_write(path: &Path, data: &[u8], counter: u64) -> std::io::Result<()> {
    let dir = path.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(dir)?;
    let name = path
        .file_name()
        .ok_or(std::io::ErrorKind::InvalidInput)?
        .to_string_lossy();
    let temp = dir.join(format!(".{name}.{}.{}.tmp", std::process::id(), counter));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        f.write_all(data)?;
        f.sync_all()?;
        fs::rename(&temp, path)?;
        if let Ok(dir) = File::open(dir) {
            let _ = dir.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

pub struct Saves {
    pub root: PathBuf,
    pub stem: String,
    pub game_hash: u64,
    pub play_seconds: u64,
    writer: Writer,
    last_ram: Vec<u8>,
    last_rtc: Vec<u8>,
    /// What the player should be told about the saves found (shown in the menu, and logged).
    pub notices: Vec<String>,
    /// Battery files read at start for memory the core didn't show yet: some cores
    /// expose their battery RAM only once a frame has run. Until it is loaded, that memory is
    /// never written, so the file on the card can't be replaced by a blank one.
    pending: Vec<(u32, PathBuf, Vec<u8>)>,
    /// Frames run while battery files waited; after [`PATIENCE`] the core is taken not to use them.
    waited: u32,
}

/// Frames to wait for a core to show memory a battery file is for.
const PATIENCE: u32 = 60;

/// The battery RAM to give a core whose memory is `current` from a file of another core:
/// the file's bytes as far as both reach, the core's own beyond that.
pub fn fit_battery(mut current: Vec<u8>, file: &[u8]) -> Vec<u8> {
    let n = current.len().min(file.len());
    current[..n].copy_from_slice(&file[..n]);
    current
}

/// Where a battery save of `len` bytes is kept when a core of another size loads it:
/// `<stem>.srm.<len>.bak`, one per size, never overwritten.
pub fn battery_backup(path: &Path, len: usize) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{len}.bak"));
    path.with_file_name(name)
}

impl Saves {
    pub fn new(root: PathBuf, stem: String, game_hash: u64) -> Self {
        Self {
            root,
            stem,
            game_hash,
            play_seconds: 0,
            writer: Writer::new(),
            last_ram: Vec::new(),
            last_rtc: Vec::new(),
            notices: Vec::new(),
            pending: Vec::new(),
            waited: 0,
        }
    }
    pub fn path(&self, slot: usize) -> PathBuf {
        self.root.join(if slot == 0 {
            format!("{}.auto.state", self.stem)
        } else {
            format!("{}.state{slot}", self.stem)
        })
    }
    /// Loads `<stem>.srm` and `<stem>.rtc` into the core. Both cores of a system share the game's
    /// saves folder, and they need not agree on the size of its battery RAM (the libretro API
    /// leaves it to each core). A file of another size is loaded as far as both reach, and kept as
    /// it was beside the save first ([`battery_backup`]), since the next write replaces it with
    /// this core's size. What happened is in [`Saves::notices`]. Memory the core doesn't show yet
    /// is loaded by [`Saves::restore_pending`] once it does.
    pub fn restore_battery(&mut self, core: &mut Core) -> Result<(), String> {
        for (id, ext) in [(RETRO_MEMORY_SAVE_RAM, "srm"), (RETRO_MEMORY_RTC, "rtc")] {
            let path = self.root.join(format!("{}.{ext}", self.stem));
            if path.exists() {
                let f = File::open(&path).map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                f.take((LIMIT + 1) as u64)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                if bytes.len() > LIMIT {
                    return Err("battery save exceeds size limit".into());
                }
                self.pending.push((id, path, bytes));
            }
        }
        self.restore_pending(core)
    }
    /// Whether a battery file still waits for the core to show its memory.
    pub fn pending(&self) -> bool {
        !self.pending.is_empty()
    }
    /// Loads the battery files whose memory the core now shows; called at start and after each
    /// frame while [`Saves::pending`].
    pub fn restore_pending(&mut self, core: &mut Core) -> Result<(), String> {
        let mut waiting = Vec::new();
        for (id, path, bytes) in std::mem::take(&mut self.pending) {
            let current = core.memory(id);
            if current.is_empty() {
                if self.waited < PATIENCE {
                    waiting.push((id, path, bytes));
                } else {
                    // Nothing will write it either: the file stays as it is.
                    self.notices.push(format!(
                        "{} isn't used by {}; left as it is",
                        path.display(),
                        core.info().name
                    ));
                }
                continue;
            }
            if current.len() != bytes.len() {
                let backup = battery_backup(&path, bytes.len());
                if !backup.exists() {
                    fs::copy(&path, &backup).map_err(|e| format!("{}: {e}", backup.display()))?;
                }
                self.notices.push(format!(
                    "Battery save is {} bytes, {} uses {}: loaded what fits, original kept as {}",
                    bytes.len(),
                    core.info().name,
                    current.len(),
                    backup.file_name().unwrap_or_default().to_string_lossy()
                ));
            }
            core.restore_memory(id, &fit_battery(current, &bytes))
                .map_err(|e| format!("{}: {e}", path.display()))?;
            // What was loaded is what the card holds: nothing to write until the game changes it.
            if id == RETRO_MEMORY_SAVE_RAM {
                self.last_ram = core.memory(id);
            } else {
                self.last_rtc = core.memory(id);
            }
        }
        self.pending = waiting;
        self.waited += 1;
        Ok(())
    }
    /// Makes `<stem>.auto.state` this core's resume state, before it is read. Another core's is
    /// moved aside to `<stem>.auto-<its core>.state` rather than replaced when this one quits,
    /// and this core's own, if it was set aside earlier, comes back: switching a game between
    /// cores and back resumes each where it was.
    pub fn settle_resume(&mut self, core_name: &str) -> Result<(), String> {
        let auto = self.path(0);
        let mine = self.aside(core_name);
        if auto.exists() {
            let made_by = State::read(&auto).map(|s| s.metadata.core);
            match made_by {
                Ok(other) if other != core_name => {
                    let aside = self.aside(&other);
                    fs::rename(&auto, &aside).map_err(|e| format!("{}: {e}", aside.display()))?;
                }
                // Ours, or unreadable: left for the resume to use or report.
                _ => return Ok(()),
            }
        }
        if mine.exists() {
            fs::rename(&mine, &auto).map_err(|e| format!("{}: {e}", auto.display()))?;
        }
        Ok(())
    }
    fn aside(&self, core_name: &str) -> PathBuf {
        let key: String = core_name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        self.root.join(format!("{}.auto-{key}.state", self.stem))
    }
    pub fn battery(&mut self, core: &mut Core, force: bool) -> Result<(), String> {
        for (id, ext, last) in [
            (RETRO_MEMORY_SAVE_RAM, "srm", &mut self.last_ram),
            (RETRO_MEMORY_RTC, "rtc", &mut self.last_rtc),
        ] {
            let bytes = core.memory(id);
            let waiting = self.pending.iter().any(|(p, _, _)| *p == id);
            if waiting || bytes.is_empty() || (!force && bytes == *last) {
                continue;
            }
            let path = self.root.join(format!("{}.{ext}", self.stem));
            if force {
                self.writer.write(path, bytes.clone())?;
                *last = bytes;
            } else if self.writer.try_write(path, bytes.clone())? {
                *last = bytes;
            }
        }
        let errors = self.writer.errors();
        if !errors.is_empty() {
            self.last_ram.clear();
            self.last_rtc.clear();
            return Err(errors.join("; "));
        }
        Ok(())
    }
    pub fn save(&self, core: &mut Core, slot: usize) -> Result<State, String> {
        let state = State::capture(core, self.game_hash, self.play_seconds)?;
        self.writer.write(self.path(slot), state.encode()?)?;
        self.writer.flush()?;
        Ok(state)
    }
    pub fn load(&mut self, core: &mut Core, slot: usize) -> Result<(), String> {
        let state = State::read(&self.path(slot))?;
        state.restore(core, self.game_hash)?;
        self.play_seconds = state.metadata.play_seconds;
        Ok(())
    }
    pub fn flush(&self) -> Result<(), String> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn example() -> State {
        let data = vec![3, 4, 5, 6];
        State {
            metadata: Metadata {
                core: "test".into(),
                version: "1".into(),
                game_hash: 3,
                created: 7,
                play_seconds: 42,
                state_hash: hash(&data),
            },
            picture: Frame {
                width: 2,
                height: 1,
                rgba: vec![255, 0, 0, 255, 0, 255, 0, 255],
            },
            data,
        }
    }
    #[test]
    fn container_round_trip_and_damage() {
        let state = example();
        let bytes = state.encode().unwrap();
        let decoded = State::decode(&bytes).unwrap();
        assert_eq!(decoded.picture, state.picture);
        assert_eq!(decoded.data, state.data);
        assert_eq!(decoded.metadata.play_seconds, 42);
        for n in [0, 8, 23, 24, bytes.len() - 1] {
            assert!(State::decode(&bytes[..n]).is_err());
        }
        let mut bad = bytes;
        *bad.last_mut().unwrap() ^= 1;
        assert!(State::decode(&bad).is_err());
    }
    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nibcase-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn state_by(core: &str) -> Vec<u8> {
        let mut state = example();
        state.metadata.core = core.into();
        state.encode().unwrap()
    }
    #[test]
    fn battery_of_another_size_loads_what_fits() {
        assert_eq!(fit_battery(vec![9; 4], &[1, 2]), [1, 2, 9, 9]);
        assert_eq!(fit_battery(vec![9; 2], &[1, 2, 3, 4]), [1, 2]);
        assert_eq!(
            battery_backup(Path::new("/s/DKC.srm"), 2048),
            Path::new("/s/DKC.srm.2048.bak")
        );
    }
    #[test]
    fn another_cores_resume_state_is_kept_aside_and_comes_back() {
        let dir = temp("resume");
        let mut saves = Saves::new(dir.clone(), "DKC".into(), 3);
        fs::write(saves.path(0), state_by("Snes9x")).unwrap();
        // Another core starts: Snes9x's resume state goes aside, there is none for it.
        saves.settle_resume("Other").unwrap();
        assert!(!saves.path(0).exists());
        assert!(dir.join("DKC.auto-snes9x.state").exists());
        // It quits, writing its own; Snes9x starts again and gets its own back.
        fs::write(saves.path(0), state_by("Other")).unwrap();
        saves.settle_resume("Snes9x").unwrap();
        assert_eq!(State::read(&saves.path(0)).unwrap().metadata.core, "Snes9x");
        assert!(dir.join("DKC.auto-other.state").exists());
        assert!(!dir.join("DKC.auto-snes9x.state").exists());
        // The same core again changes nothing.
        saves.settle_resume("Snes9x").unwrap();
        assert_eq!(State::read(&saves.path(0)).unwrap().metadata.core, "Snes9x");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn a_battery_save_waits_for_a_core_that_shows_its_memory_late() {
        use nibcase_libretro::testing::{fixture, game};
        use nibcase_libretro::Config;
        let dir = temp("late");
        let mut core = Core::open(Config {
            core: fixture().into(),
            system_dir: dir.clone(),
            save_dir: dir.clone(),
            options: Default::default(),
        })
        .unwrap();
        core.load_game(&game(6)).unwrap();
        // The card holds a save the game made earlier, in another core of a larger size.
        let srm = dir.join("late.srm");
        fs::write(&srm, [0xAA; 32]).unwrap();
        let mut saves = Saves::new(dir.clone(), "late".into(), core.game_hash());
        saves.restore_battery(&mut core).unwrap();
        assert!(saves.pending());
        // Nothing is written while the core shows no memory.
        saves.battery(&mut core, true).unwrap();
        saves.flush().unwrap();
        assert_eq!(fs::read(&srm).unwrap(), [0xAA; 32]);
        core.run(0, [0; 2], false, 50).unwrap();
        saves.restore_pending(&mut core).unwrap();
        assert!(!saves.pending());
        assert_eq!(core.memory(RETRO_MEMORY_SAVE_RAM)[4..], [0xAA; 12]);
        assert_eq!(fs::read(dir.join("late.srm.32.bak")).unwrap(), [0xAA; 32]);
        drop(core);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn atomic_writer_finishes_and_reports_errors() {
        let dir = std::env::temp_dir().join(format!("nibcase-writer-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let w = Writer::new();
        w.write(dir.join("test.srm"), vec![7; 16]).unwrap();
        w.flush().unwrap();
        assert_eq!(fs::read(dir.join("test.srm")).unwrap(), [7; 16]);
        w.write(dir.join("test.srm/cannot-write"), vec![1]).unwrap();
        assert!(w.flush().is_err());
        assert!(!w.errors().is_empty());
        drop(w);
        fs::remove_dir_all(dir).unwrap();
    }
}

mod hex {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{value:016x}"))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let value = String::deserialize(d)?;
        u64::from_str_radix(&value, 16).map_err(serde::de::Error::custom)
    }
}
