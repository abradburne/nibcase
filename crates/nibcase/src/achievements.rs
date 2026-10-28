//! RetroAchievements in play, through nibkit's `nib-cheevos`: the sign-in DSlot passes in the
//! environment, the game identified by rcheevos' rhash as DSlot identifies it, the set loaded and
//! evaluated after every frame the core runs, fast-forward's too, against the console's memory as
//! rcheevos maps it over the core's (`nib_cheevos::libretro`: the core's memory map, or its
//! memory ids), what happens said on stderr, where DSlot's bug report picks it up, and shown over
//! the game as nibkit's toasts and indicators, as every nibOS emulator shows them. Casual
//! (softcore) only; nothing here changes how the game runs.
//!
//! DSlot starts a game with `RA_TOKEN_FILE` (the token file: username, then token, one a line),
//! `RA_USERNAME` and `RA_HARDCORE`, and only for the systems a core's description declares
//! (`[achievements]`). Without `RA_TOKEN_FILE` there is no session and nothing is logged. The
//! password never reaches nibcase, and the token is never written or logged.
//!
//! rcheevos' rules for libretro cores hold: a core it rules out for a console earns nothing
//! there, and while one of the core's options has a value rcheevos rules out for it (a PAL
//! region, a graphics layer turned off, an underclocked CPU) the set is not evaluated; when the
//! option is put back, its progress starts over, so the change can't unlock anything.

use nib_cheevos::badges::{self, Badges};
use nib_cheevos::launch::Launch;
use nib_cheevos::libretro::{self, Block, CoreMemory, Disallowed, MemoryDescriptor, Regions};
use nib_cheevos::{Achievement, Client, Event, Settings, Summary};
use nibcase_libretro::ffi::{
    RETRO_MEMORY_SAVE_RAM, RETRO_MEMORY_SYSTEM_RAM, RETRO_MEMORY_VIDEO_RAM,
};
use nibcase_libretro::{systems::System, Core};
use nibkit::menu::indicators::Indicators;
use nibkit::menu::toast::{self, Kind, Notice};
use std::collections::HashMap;
use std::path::Path;

/// A badge decoded: RGBA, width, height.
type Picture = (Vec<u8>, usize, usize);

/// What the console's memory regions were built over: the core's map (how many it has given)
/// and where its memories were. When any of it changes, they are built again.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Over {
    maps: u32,
    memory: [Option<(*mut u8, usize)>; 3],
}

/// The session, and what it shows over the game.
pub struct Session {
    client: Client,
    /// rcheevos' id for the console.
    console: u32,
    /// The core's options rcheevos rules out, and the one in effect now, if any (its label and
    /// value's), while the set is not evaluated.
    disallowed: Disallowed,
    blocked: Option<String>,
    /// The console's memory over the core's, and what it was built over.
    regions: Option<(Over, Regions)>,
    badges: Option<Badges>,
    /// Badges decoded, by name.
    pictures: HashMap<String, Picture>,
    /// The notices over the game, one after another, and the indicators (nibkit's).
    pub toasts: toast::Queue,
    pub indicators: Indicators,
    /// `NIBCASE_PEEK=<hex address>`: the service's address whose 4 bytes are logged once a
    /// second (60 frames), to check a core's memory against a game's known RAM map. With the
    /// frames counted.
    peek: Option<u32>,
    frames: u64,
}

/// The core's segment of the User-Agent: its file's name without `_libretro`, and the first
/// word of its version ("snes9x/1.62.3").
pub fn core_segment(core_file: &Path, version: &str) -> String {
    let name = core_file
        .file_stem()
        .map(|s| s.to_string_lossy().trim_end_matches("_libretro").to_owned())
        .unwrap_or_default();
    let version = version.split_whitespace().next().unwrap_or("0");
    format!("{name}/{version}")
}

/// The session, when the launcher passed a sign-in: signed in and the game loading, both on their
/// way; the outcome arrives as events ([`Session::show`]). `rom` is the file DSlot named (a zip is
/// its first entry with one of the console's extensions, as DSlot hashes it).
pub fn start(
    core: &Core,
    core_file: &Path,
    system: System,
    rom: &Path,
    verbose: bool,
) -> Option<Session> {
    let launch = match Launch::from_env() {
        Ok(Some(launch)) => launch,
        Ok(None) => return None,
        Err(e) => {
            eprintln!("cheevos: {e}");
            return None;
        }
    };
    let Some(console) = nib_cheevos::hash::console_id(system.id()) else {
        eprintln!("cheevos: {} has no RetroAchievements console", system.id());
        return None;
    };
    let library = core.info().name.clone();
    if !libretro::system_allowed(&library, console) {
        eprintln!(
            "cheevos: rcheevos doesn't allow {library} to earn achievements for {}",
            system.label()
        );
        return None;
    }
    if launch.hardcore {
        eprintln!("cheevos: hardcore was asked for (RA_HARDCORE=1); nibcase plays Casual, hardcore is not supported yet");
    }
    libretro::verbose(verbose);
    let user_agent = nib_cheevos::ua::user_agent(
        "nibcase",
        env!("CARGO_PKG_VERSION"),
        Some(&core_segment(core_file, &core.info().version)),
    );
    let mut client = match Client::start(&user_agent, Settings::default(), verbose) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cheevos: achievements unavailable: {e}");
            return None;
        }
    };
    eprintln!(
        "cheevos: using the token from {} (the launcher's sign-in)",
        launch.token_file.display()
    );
    client.sign_in_with_token(&launch.credentials);
    let mut session = Session {
        client,
        console,
        disallowed: libretro::disallowed_settings(&library),
        blocked: None,
        regions: None,
        badges: badges::dir().and_then(|dir| Badges::start(dir, &user_agent)),
        pictures: HashMap::new(),
        toasts: toast::Queue::new(),
        indicators: Indicators::new(),
        peek: std::env::var("NIBCASE_PEEK")
            .ok()
            .and_then(|a| u32::from_str_radix(a.trim_start_matches("0x"), 16).ok()),
        frames: 0,
    };
    if session.badges.is_none() {
        eprintln!("cheevos: no badges: no cache folder or no curl");
    }
    match nib_cheevos::hash::file(system.id(), rom) {
        Ok(hash) => {
            if verbose {
                eprintln!("cheevos: {} hashes to {hash}", rom.display());
            }
            session.client.load_game(&hash);
        }
        Err(e) => {
            eprintln!("cheevos: cannot identify this game: {e}");
            session.toasts.push(
                Notice::new(Kind::Problem, "No achievements for this game")
                    .header(RA)
                    .detail(format!("it can't be identified: {e}")),
            );
        }
    }
    Some(session)
}

const RA: &str = "RetroAchievements";

/// One of nibcase-libretro's descriptors as nib-cheevos takes it.
fn descriptor(d: &nibcase_libretro::ffi::retro_memory_descriptor) -> MemoryDescriptor {
    MemoryDescriptor {
        flags: d.flags,
        ptr: d.ptr.cast(),
        offset: d.offset,
        start: d.start,
        select: d.select,
        disconnect: d.disconnect,
        len: d.len,
        addrspace: std::ptr::null(),
    }
}

/// What the core's memory is now, to compare with what the regions were built over.
fn over(core: &mut Core) -> Over {
    Over {
        maps: core.memory_maps(),
        memory: [
            RETRO_MEMORY_SYSTEM_RAM,
            RETRO_MEMORY_SAVE_RAM,
            RETRO_MEMORY_VIDEO_RAM,
        ]
        .map(|id| core.memory_in_place(id)),
    }
}

/// The memory of the console with rcheevos id `console` over the core's as it is now (`over`):
/// its memory map, else its memory ids, as rcheevos lays them out.
///
/// # Safety
///
/// The regions are read only while `core` still has the game loaded and reports the same map
/// and memories (`over` again equal), on this thread, between its frames.
unsafe fn build(console: u32, core: &Core, over: Over) -> Regions {
    let block = |m: Option<(*mut u8, usize)>| {
        m.map_or(Block::default(), |(data, size)| Block { data, size })
    };
    let memory = CoreMemory {
        system_ram: block(over.memory[0]),
        save_ram: block(over.memory[1]),
        video_ram: block(over.memory[2]),
    };
    let map: Vec<MemoryDescriptor> = core.memory_map().iter().map(descriptor).collect();
    // SAFETY: the map's and the memories' pointers are the loaded core's, as it gave them,
    // and read only as this function's caller promises.
    unsafe { Regions::new(console, &map, memory) }
}

impl Session {
    /// The console's memory over the core's as it is now: built again when the core gave a new
    /// map or its memories moved or changed size (a core may show some only after a frame).
    fn regions(&mut self, core: &mut Core) -> &Regions {
        let now = over(core);
        if self.regions.as_ref().is_none_or(|(o, _)| *o != now) {
            // SAFETY: read only in frame() and idle(), right after this check, while `core` is
            // borrowed, on this thread, between the core's frames.
            let regions = unsafe { build(self.console, core, now) };
            if !regions.backed() {
                eprintln!("cheevos: the core shows none of the console's memory (yet)");
            }
            self.regions = Some((now, regions));
        }
        &self.regions.as_ref().unwrap().1
    }

    /// After every frame the core ran, fast or not: the set checked against the machine (unless
    /// an option rcheevos rules out is set, when the session only keeps alive).
    pub fn frame(&mut self, core: &mut Core) {
        let blocked = self.blocked.is_some();
        self.regions(core);
        let (client, regions) = (&mut self.client, &self.regions.as_ref().unwrap().1);
        if blocked {
            client.idle(regions);
        } else {
            client.frame(regions);
        }
        self.frames += 1;
        if let Some(at) = self.peek.filter(|_| self.frames.is_multiple_of(60)) {
            let mut b = [0u8; 4];
            let backed = regions.read(at, &mut b);
            eprintln!("cheevos: peek ${at:06X}: {b:02X?} ({backed} backed)");
        }
    }

    /// While the game is paused: the session kept alive, the network's answers taken.
    pub fn idle(&mut self, core: &mut Core) {
        self.regions(core);
        self.client.idle(&self.regions.as_ref().unwrap().1);
    }

    /// The game started over: so does the set's progress.
    pub fn reset(&mut self) {
        self.client.reset();
        self.indicators.clear();
    }

    /// The set's progress, to keep in a save state; `None` without a set.
    pub fn progress(&mut self) -> Option<Vec<u8>> {
        self.client.progress()
    }

    /// A save state was loaded: the set's progress becomes the state's, or starts over when it
    /// has none, so the jump can't unlock anything.
    pub fn jumped(&mut self, progress: Option<&[u8]>) {
        self.client.restore_progress(progress);
    }

    /// Checks the core's options against rcheevos' rules for it: called at the start and when
    /// play resumes from the menu, where they are changed.
    pub fn check_options(&mut self, core: &Core) {
        if self.disallowed.is_empty() {
            return;
        }
        let ruled_out = core.options().find_map(|d| {
            let value = core.value(&d.key)?;
            (!self.disallowed.allows(&d.key, value)).then(|| {
                let label = d
                    .values
                    .iter()
                    .find(|v| v.0 == value)
                    .map_or(value, |v| v.1.as_str());
                format!("{}: {label}", d.label)
            })
        });
        match (&self.blocked, ruled_out) {
            (None, Some(option)) => {
                eprintln!("cheevos: Achievements paused -- {option} is not allowed");
                self.toasts.push(
                    Notice::new(Kind::Problem, "Achievements paused")
                        .header(RA)
                        .detail(format!("{option} is not allowed")),
                );
                self.indicators.clear();
                self.blocked = Some(option);
            }
            (Some(_), None) => {
                eprintln!("cheevos: Achievements resumed");
                self.toasts
                    .push(Notice::new(Kind::Info, "Achievements resumed").header(RA));
                self.client.reset();
                self.blocked = None;
            }
            (Some(_), Some(option)) => self.blocked = Some(option),
            (None, None) => {}
        }
    }

    /// The events since the last call, said: logged, and shown over the game.
    pub fn show(&mut self) {
        let fetched = self
            .badges
            .as_mut()
            .map(Badges::fetched)
            .unwrap_or_default();
        for (name, file) in fetched {
            let Some((rgba, w, h)) = decode(&file) else {
                continue;
            };
            self.pictures.insert(name.clone(), (rgba.clone(), w, h));
            self.indicators.badge(&name, rgba.clone(), w, h);
            self.toasts.badge(&name, rgba, w, h);
        }
        for event in self.client.take_events() {
            if let Some(line) = line(&event) {
                eprintln!("cheevos: {line}");
            }
            match event {
                Event::Unlocked(a) => {
                    let n = Notice::unlock(a.title.as_str(), a.description.as_str(), a.points);
                    let n = self.with_badge(n, &a);
                    self.toasts.push(n);
                }
                Event::ChallengeShown(a) => {
                    let n = self.with_badge(Notice::challenge(a.title.as_str()), &a);
                    self.indicators.challenge(a.id, n);
                }
                Event::ChallengeHidden { id } => self.indicators.end_challenge(id),
                Event::Progress(a) => {
                    let n = Notice::progress(a.title.as_str(), a.measured_progress.as_str());
                    let n = self.with_badge(n, &a);
                    self.indicators.progress(a.id, n);
                }
                Event::ProgressHidden => self.indicators.hide_progress(),
                e => {
                    if let Some(n) = notice(&e) {
                        self.toasts.push(n);
                    }
                }
            }
        }
    }

    /// `n` with achievement `a`'s badge when it is decoded or in the cache; else awaiting it
    /// while it is fetched (nibkit holds a notice or an indicator a moment for it).
    fn with_badge(&mut self, n: Notice, a: &Achievement) -> Notice {
        let name = a.badge_name.as_str();
        if name.is_empty() {
            return n;
        }
        if let Some((rgba, w, h)) = self.pictures.get(name) {
            return n.badge(rgba.clone(), *w, *h);
        }
        let Some(badges) = self.badges.as_mut() else {
            return n;
        };
        match badges.want(name, &a.badge_url).and_then(|f| decode(&f)) {
            Some((rgba, w, h)) => {
                self.pictures.insert(name.to_owned(), (rgba.clone(), w, h));
                n.badge(rgba, w, h)
            }
            None => n.awaits_badge(name),
        }
    }
}

/// A badge's PNG as RGBA.
fn decode(file: &Path) -> Option<Picture> {
    let data = std::fs::read(file).ok()?;
    let mut decoder = png::Decoder::new(&data[..]);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => px
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    (rgba.len() == w * h * 4).then_some((rgba, w, h))
}

/// "Casual: 12 of 40 unlocked, 3 unsupported", for the placard.
pub fn standing(summary: &Summary, hardcore: bool) -> String {
    let mode = if hardcore { "Hardcore" } else { "Casual" };
    let mut s = format!("{mode}: {} of {} unlocked", summary.unlocked, summary.total);
    if summary.unsupported > 0 {
        s += &format!(", {} unsupported", summary.unsupported);
    }
    s
}

fn points(n: u32) -> String {
    if n == 1 {
        "1 point".to_owned()
    } else {
        format!("{n} points")
    }
}

/// A notice for an event that is one (not an unlock or an indicator, which [`Session::show`]
/// makes with their badges); `None` for one only logged.
pub fn notice(e: &Event) -> Option<Notice> {
    let says =
        |kind, title: &str, detail: &str| Some(Notice::new(kind, title).header(RA).detail(detail));
    match e {
        // DSlot signed in already; the placard that follows says the rest.
        Event::SignedIn { .. } | Event::NoAchievements { .. } | Event::ResetAsked => None,
        Event::SignInFailed {
            token_refused: true,
            ..
        } => says(
            Kind::Problem,
            "Sign-in refused",
            "sign in again in DSlot's settings",
        ),
        Event::SignInFailed { message, .. } => says(Kind::Problem, "Sign-in failed", message),
        Event::GameLoaded {
            title,
            summary,
            hardcore,
        } => says(Kind::Info, title, &standing(summary, *hardcore)),
        Event::UnknownGame { .. } => says(
            Kind::Problem,
            "No achievements for this game",
            "RetroAchievements does not recognise this dump",
        ),
        Event::LoadFailed { message } => {
            says(Kind::Problem, "Could not load achievements", message)
        }
        Event::GameCompleted {
            title,
            summary,
            hardcore,
        } => Some(
            Notice::new(Kind::Unlock, title.as_str())
                .header(if *hardcore { "Mastered" } else { "Completed" })
                .detail(format!(
                    "all {} achievements, {}",
                    summary.total,
                    points(summary.points)
                )),
        ),
        Event::SubsetCompleted { title } => {
            Some(Notice::new(Kind::Unlock, title.as_str()).header("Subset completed"))
        }
        Event::ServerError { message, .. } => says(Kind::Problem, "Server error", message),
        Event::Disconnected => says(
            Kind::Problem,
            "Offline",
            "unlocks will be sent when the connection returns",
        ),
        Event::Reconnected => says(Kind::Info, "Reconnected", "pending unlocks sent"),
        // Shown by Session::show; leaderboards run in hardcore only, which nibcase doesn't
        // play: logged.
        Event::Unlocked(_)
        | Event::ChallengeShown(_)
        | Event::ChallengeHidden { .. }
        | Event::Progress(_)
        | Event::ProgressHidden
        | Event::LeaderboardStarted(_)
        | Event::LeaderboardFailed(_)
        | Event::LeaderboardSubmitted(_)
        | Event::Tracker { .. }
        | Event::TrackerHidden { .. } => None,
    }
}

/// An event as a line for the log, `<what> -- <detail>` as DSperate and inkdot log them (so
/// DSlot's bug report reads the same for all); `None` for the ones that come every frame or so.
pub fn line(e: &Event) -> Option<String> {
    let with = |head: String, detail: &str| {
        Some(if detail.is_empty() {
            head
        } else {
            format!("{head} -- {detail}")
        })
    };
    match e {
        Event::SignedIn { display_name } => with("Signed in".into(), display_name),
        Event::SignInFailed { message, .. } => with("Sign-in failed".into(), message),
        Event::GameLoaded {
            title,
            summary,
            hardcore,
        } => with(title.clone(), &standing(summary, *hardcore)),
        Event::NoAchievements { title } => with(title.clone(), "no published achievements"),
        Event::UnknownGame { hash } => with(
            "No achievements for this game".into(),
            &format!("RetroAchievements does not recognise this dump (hash {hash})"),
        ),
        Event::LoadFailed { message } => with("Could not load achievements".into(), message),
        Event::Unlocked(a) => with(
            format!("Achievement unlocked: {} ({})", a.title, points(a.points)),
            &a.description,
        ),
        Event::ChallengeShown(a) => with("Challenge".into(), &a.title),
        Event::ChallengeHidden { .. }
        | Event::Progress(_)
        | Event::ProgressHidden
        | Event::Tracker { .. }
        | Event::TrackerHidden { .. } => None,
        Event::LeaderboardStarted(l) => with("Leaderboard started".into(), &l.title),
        Event::LeaderboardFailed(l) => with("Leaderboard failed".into(), &l.title),
        Event::LeaderboardSubmitted(l) => with(
            "Leaderboard submitted".into(),
            &format!("{}: {}", l.title, l.value),
        ),
        Event::GameCompleted { title, .. } => with("Game completed".into(), title),
        Event::SubsetCompleted { title } => with("Subset completed".into(), title),
        Event::ServerError { api, message } => with(format!("Server error ({api})"), message),
        Event::Disconnected => with(
            "RetroAchievements offline".into(),
            "unlocks will be sent when the connection returns",
        ),
        Event::Reconnected => with(
            "RetroAchievements reconnected".into(),
            "pending unlocks sent",
        ),
        Event::ResetAsked => with("Reset asked for".into(), "ignored: hardcore is not played"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(total: u32, unlocked: u32, unsupported: u32) -> Summary {
        Summary {
            total,
            unlocked,
            unsupported,
            points: 400,
            ..Summary::default()
        }
    }

    #[test]
    fn the_placard_says_casual_and_the_standing() {
        let loaded = |unsupported| Event::GameLoaded {
            title: "Donkey Kong Country".into(),
            summary: summary(40, 12, unsupported),
            hardcore: false,
        };
        let n = notice(&loaded(0)).unwrap();
        assert_eq!(n.said().title, "Donkey Kong Country");
        assert_eq!(n.said().detail, "Casual: 12 of 40 unlocked");
        assert_eq!(
            line(&loaded(3)).unwrap(),
            "Donkey Kong Country -- Casual: 12 of 40 unlocked, 3 unsupported"
        );
    }

    #[test]
    fn lines_read_as_the_other_emulators_log_them() {
        let unlocked = Event::Unlocked(Achievement {
            title: "Barrel Blast".into(),
            description: "Clear the first level".into(),
            points: 5,
            ..Achievement::default()
        });
        assert_eq!(
            line(&unlocked).unwrap(),
            "Achievement unlocked: Barrel Blast (5 points) -- Clear the first level"
        );
        assert!(notice(&unlocked).is_none(), "shown with its badge instead");
        assert_eq!(line(&Event::ProgressHidden), None);
        let refused = notice(&Event::SignInFailed {
            token_refused: true,
            message: "Invalid token".into(),
        })
        .unwrap();
        assert!(refused.said().detail.contains("DSlot"));
        assert_eq!(refused.said().kind, Kind::Problem);
    }

    /// The test core's work RAM read as a Super Nintendo's: through its memory map (game 11,
    /// at $7E0000), and through its memory ids without one (game 0); the frame it ran is at the
    /// service's $0000, the A button at $0001.
    #[test]
    fn the_consoles_memory_is_read_over_the_cores() {
        use nibcase_libretro::testing::{fixture, game};
        use nibcase_libretro::Config;
        let _one = crate::tests::CORE.lock().unwrap_or_else(|e| e.into_inner());
        let dir = fixture().parent().unwrap().join("cheevos-memory");
        let snes = nib_cheevos::hash::console_id("SFC").unwrap();
        for mode in [11, 0] {
            let mut core = Core::open(Config {
                core: fixture().into(),
                system_dir: dir.clone(),
                save_dir: dir.clone(),
                options: Default::default(),
            })
            .unwrap();
            core.load_game(&game(mode)).unwrap();
            assert_eq!(core.memory_map().is_empty(), mode == 0);
            for _ in 0..3 {
                core.run(1 << 8, [0; 2], false, 50).unwrap();
            }
            let now = over(&mut core);
            // SAFETY: read here, the core loaded and not running.
            let regions = unsafe { build(snes, &core, now) };
            assert!(regions.backed(), "game {mode}");
            let mut b = [0u8; 2];
            assert_eq!(regions.read(0, &mut b), 2, "game {mode}");
            assert_eq!(b, [2, 1], "game {mode}");
            // The cartridge's RAM: the test core's battery RAM, when it has no map.
            let mut c = [0u8; 1];
            let got = regions.read(0x020000, &mut c);
            assert_eq!(got, u32::from(mode == 0), "game {mode}");
            assert!(over(&mut core) == now, "nothing moved");
        }
    }

    #[test]
    fn the_core_is_named_in_the_user_agent_by_its_file_and_version() {
        assert_eq!(
            core_segment(
                Path::new("/usr/lib/libretro/snes9x_libretro.so"),
                "1.62.3 6f0d4e1"
            ),
            "snes9x/1.62.3"
        );
        assert_eq!(
            core_segment(Path::new("mednafen_pce_fast_libretro.so"), "v0.9.38.7"),
            "mednafen_pce_fast/v0.9.38.7"
        );
        assert_eq!(core_segment(Path::new("gw_libretro.so"), ""), "gw/0");
    }
}
