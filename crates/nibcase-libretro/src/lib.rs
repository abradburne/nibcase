//! A single-threaded libretro software-core host. The pinned header defines the FFI contract.
//! Cores are trusted native libraries: these checks do not sandbox a malicious core.
mod environment;
pub mod systems;
mod video;
pub use video::Frame;
#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    dead_code,
    clippy::all
)]
pub mod ffi;
#[cfg(feature = "testing")]
pub mod testing;

use ffi::*;
use libloading::Library;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{c_void, CStr, CString};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

static ACTIVE: AtomicBool = AtomicBool::new(false);
thread_local! { static CONTEXT: Cell<*mut Context> = const { Cell::new(std::ptr::null_mut()) }; }

#[derive(Clone, Debug, PartialEq)]
pub struct OptionDef {
    pub key: String,
    pub label: String,
    pub info: String,
    pub category: String,
    pub values: Vec<(String, String)>,
    pub default: String,
    pub visible: bool,
}

#[derive(Clone, Debug)]
pub struct Info {
    pub name: String,
    pub version: String,
    pub extensions: Vec<String>,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

pub struct Config {
    pub core: PathBuf,
    pub system_dir: PathBuf,
    pub save_dir: PathBuf,
    pub options: BTreeMap<String, String>,
}

struct Context {
    core_path: CString,
    system_dir: CString,
    save_dir: CString,
    assets_dir: CString,
    options: BTreeMap<String, OptionDef>,
    values: BTreeMap<String, CString>,
    retired_values: Vec<CString>,
    overrides: BTreeMap<String, String>,
    categories: BTreeMap<String, String>,
    variables_changed: bool,
    update_display: Option<unsafe extern "C" fn() -> bool>,
    format: u32,
    av: retro_system_av_info,
    frame: Frame,
    audio: Vec<i16>,
    buttons: u16,
    analog: [i16; 2],
    shutdown: bool,
    error: Option<String>,
    messages: Vec<String>,
    unknown: BTreeSet<u32>,
    quirks: u64,
    rotation: u32,
    fast_forward: bool,
    frame_time: retro_frame_time_callback,
    audio_status: retro_audio_buffer_status_callback,
    occupancy: u32,
    frames: u64,
    /// What each button of the first joypad does, as the core described it.
    inputs: Vec<(u32, String)>,
    /// The touch panel as libretro's pointer: the last position (each axis -0x7fff..=0x7fff
    /// over the frame) and whether a finger is down.
    pointer: (i16, i16),
    pressed: bool,
    /// A game of several discs: the core's disk control (the basic interface's, without labels,
    /// held in the extended one's shape).
    disk: Option<retro_disk_control_ext_callback>,
    /// The pads the core offers for the first port (description, device id).
    controllers: Vec<(String, u32)>,
    /// With `NIBCASE_DEBUG=input`, each (port, device, index, id) the core reads, logged once.
    asked: Option<BTreeSet<(u32, u32, u32, u32)>>,
}

impl Context {
    fn set_value(&mut self, key: String, value: CString) {
        if self.values.get(&key) == Some(&value) {
            return;
        }
        // GET_VARIABLE strings must remain usable by the core through subsequent calls.
        if let Some(old) = self.values.insert(key, value) {
            self.retired_values.push(old);
        }
    }
}
/// A game's bytes before a core loads it: the file, or in a `.zip` its first member with one of
/// `extensions` (lower case). For what a game says about itself that the core needs to know at
/// load (a WonderSwan game's orientation). At most 64 MiB, as a load.
pub fn peek(path: &Path, extensions: &[&str]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    const LIMIT: u64 = 64 * 1024 * 1024;
    let mut data = Vec::new();
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        for index in 0..zip.len() {
            let entry = zip.by_index(index).map_err(|e| e.to_string())?;
            let ext = Path::new(entry.name())
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !entry.is_dir() && extensions.contains(&ext.as_str()) {
                entry
                    .take(LIMIT)
                    .read_to_end(&mut data)
                    .map_err(|e| e.to_string())?;
                return Ok(data);
            }
        }
        return Err("ZIP contains no game".into());
    }
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(LIMIT)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    Ok(data)
}
fn content_hash(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

struct Api {
    init: unsafe extern "C" fn(),
    deinit: unsafe extern "C" fn(),
    get_system_info: unsafe extern "C" fn(*mut retro_system_info),
    get_system_av_info: unsafe extern "C" fn(*mut retro_system_av_info),
    set_controller_port_device: unsafe extern "C" fn(u32, u32),
    set_environment: unsafe extern "C" fn(retro_environment_t),
    set_video_refresh: unsafe extern "C" fn(retro_video_refresh_t),
    set_audio_sample: unsafe extern "C" fn(retro_audio_sample_t),
    set_audio_sample_batch: unsafe extern "C" fn(retro_audio_sample_batch_t),
    set_input_poll: unsafe extern "C" fn(retro_input_poll_t),
    set_input_state: unsafe extern "C" fn(retro_input_state_t),
    run: unsafe extern "C" fn(),
    reset: unsafe extern "C" fn(),
    serialize_size: unsafe extern "C" fn() -> usize,
    serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    unserialize: unsafe extern "C" fn(*const c_void, usize) -> bool,
    load_game: unsafe extern "C" fn(*const retro_game_info) -> bool,
    unload_game: unsafe extern "C" fn(),
    get_memory_data: unsafe extern "C" fn(u32) -> *mut c_void,
    get_memory_size: unsafe extern "C" fn(u32) -> usize,
}

impl Api {
    unsafe fn load(lib: &Library) -> Result<Self, String> {
        macro_rules! symbol {
            ($name:ident) => {{
                // SAFETY: each field's signature is the corresponding declaration in libretro.h;
                // Library stays alive until after unload/deinit and no function escapes Core.
                *unsafe { lib.get(concat!("retro_", stringify!($name), "\0").as_bytes()) }
                    .map_err(|e| format!("missing retro_{}: {e}", stringify!($name)))?
            }};
        }
        Ok(Self {
            init: symbol!(init),
            deinit: symbol!(deinit),
            get_system_info: symbol!(get_system_info),
            get_system_av_info: symbol!(get_system_av_info),
            set_controller_port_device: symbol!(set_controller_port_device),
            set_environment: symbol!(set_environment),
            set_video_refresh: symbol!(set_video_refresh),
            set_audio_sample: symbol!(set_audio_sample),
            set_audio_sample_batch: symbol!(set_audio_sample_batch),
            set_input_poll: symbol!(set_input_poll),
            set_input_state: symbol!(set_input_state),
            run: symbol!(run),
            reset: symbol!(reset),
            serialize_size: symbol!(serialize_size),
            serialize: symbol!(serialize),
            unserialize: symbol!(unserialize),
            load_game: symbol!(load_game),
            unload_game: symbol!(unload_game),
            get_memory_data: symbol!(get_memory_data),
            get_memory_size: symbol!(get_memory_size),
        })
    }
}

/// Not Send or Sync: libretro callbacks and all calls into a core stay on this thread.
pub struct Core {
    api: Api,
    ctx: Box<Context>,
    info: Info,
    initialized: bool,
    loaded: bool,
    content: Vec<u8>,
    game_hash: u64,
    content_path: Option<CString>,
    extracted: Option<PathBuf>,
    _library: Library,
    _thread: PhantomData<Rc<()>>,
}

struct Lease;
impl Drop for Lease {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
    }
}
struct CallScope(*mut Context);
impl Drop for CallScope {
    fn drop(&mut self) {
        CONTEXT.with(|c| c.set(self.0));
    }
}

fn path_string(path: &Path) -> Result<CString, String> {
    CString::new(path.as_os_str().as_encoded_bytes()).map_err(|_| "path contains NUL".into())
}

// SAFETY: the caller supplies a libretro-owned NUL-terminated string, or null, as the API says.
unsafe fn text(ptr: *const std::ffi::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        // SAFETY: see this function's contract.
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

impl Core {
    pub fn open(config: Config) -> Result<Self, String> {
        if ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("only one libretro core may be active in a process".into());
        }
        let lease = Lease;
        let core_path = path_string(&config.core)?;
        let system_dir = path_string(&config.system_dir)?;
        let save_dir = path_string(&config.save_dir)?;
        std::fs::create_dir_all(&config.save_dir).map_err(|e| e.to_string())?;
        // SAFETY: loading trusted native code is the purpose of this API. Symbols are validated
        // below and the Library is owned for the full lifetime of every function pointer.
        let library = unsafe { Library::new(&config.core) }
            .map_err(|e| format!("{}: {e}", config.core.display()))?;
        // SAFETY: the API version function has the signature required by the pinned header.
        let version =
            unsafe { library.get::<unsafe extern "C" fn() -> u32>(b"retro_api_version\0") }
                .map_err(|e| e.to_string())?;
        // SAFETY: symbol is valid while library is alive.
        if unsafe { version() } != RETRO_API_VERSION {
            return Err("unsupported libretro API version".into());
        }
        // SAFETY: Api resolves precisely the pinned header's signatures and keeps library alive.
        let api = unsafe { Api::load(&library) }?;
        let ctx = Box::new(Context {
            core_path,
            system_dir,
            save_dir,
            assets_dir: CString::new("/usr/share/nibcase").unwrap(),
            options: BTreeMap::new(),
            values: BTreeMap::new(),
            retired_values: Vec::new(),
            overrides: config.options,
            categories: BTreeMap::new(),
            variables_changed: false,
            update_display: None,
            format: 0,
            av: retro_system_av_info::default(),
            frame: Frame::default(),
            audio: Vec::new(),
            buttons: 0,
            analog: [0; 2],
            shutdown: false,
            error: None,
            messages: Vec::new(),
            unknown: BTreeSet::new(),
            quirks: 0,
            rotation: 0,
            fast_forward: false,
            frame_time: retro_frame_time_callback::default(),
            audio_status: retro_audio_buffer_status_callback::default(),
            occupancy: 0,
            frames: 0,
            inputs: Vec::new(),
            pointer: (0, 0),
            pressed: false,
            disk: None,
            controllers: Vec::new(),
            asked: std::env::var("NIBCASE_DEBUG")
                .is_ok_and(|d| d.split(',').any(|w| w == "input"))
                .then(BTreeSet::new),
        });
        let mut core = Self {
            api,
            ctx,
            info: Info {
                name: String::new(),
                version: String::new(),
                extensions: Vec::new(),
                need_fullpath: false,
                block_extract: false,
            },
            initialized: false,
            loaded: false,
            content: Vec::new(),
            game_hash: 0,
            content_path: None,
            extracted: None,
            _library: library,
            _thread: PhantomData,
        };
        // Core now owns the process lease, including on any subsequent error.
        std::mem::forget(lease);
        core.call(|a| {
            // SAFETY: callbacks have the pinned API signatures; call scope sets their thread-local
            // context. The core holds these static function pointers until deinit.
            unsafe {
                (a.set_environment)(Some(environment::environment));
                (a.set_video_refresh)(Some(video_callback));
                (a.set_audio_sample)(Some(audio_sample));
                (a.set_audio_sample_batch)(Some(audio_batch));
                (a.set_input_poll)(Some(input_poll));
                (a.set_input_state)(Some(input_state));
            }
        });
        let mut info = retro_system_info::default();
        core.call(|a| {
            // SAFETY: writable API struct, core is live and callbacks are installed.
            unsafe {
                (a.get_system_info)(&mut info);
                (a.init)();
            }
        });
        core.initialized = true;
        // SAFETY: core info's strings remain owned by the live library.
        core.info = unsafe {
            Info {
                name: text(info.library_name),
                version: text(info.library_version),
                extensions: text(info.valid_extensions)
                    .split('|')
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect(),
                need_fullpath: info.need_fullpath,
                block_extract: info.block_extract,
            }
        };
        core.check_error()?;
        Ok(core)
    }

    fn call<T>(&mut self, f: impl FnOnce(&Api) -> T) -> T {
        let pointer = &mut *self.ctx as *mut Context;
        let previous = CONTEXT.with(|c| c.replace(pointer));
        let _scope = CallScope(previous);
        f(&self.api)
    }

    pub fn info(&self) -> &Info {
        &self.info
    }
    pub fn game_hash(&self) -> u64 {
        self.game_hash
    }
    /// The loaded game's bytes, as the core was given them; empty for a core that loads the file
    /// itself (`need_fullpath`).
    pub fn content(&self) -> &[u8] {
        &self.content
    }
    pub fn options(&self) -> impl Iterator<Item = &OptionDef> {
        self.ctx.options.values()
    }
    pub fn value(&self, key: &str) -> Option<&str> {
        self.ctx.values.get(key)?.to_str().ok()
    }
    pub fn frame(&self) -> &Frame {
        &self.ctx.frame
    }
    /// What each button of the first joypad does in this game, as the core described it
    /// (`RETRO_DEVICE_ID_JOYPAD_*`, description), in the core's order.
    pub fn input_descriptors(&self) -> &[(u32, String)] {
        &self.ctx.inputs
    }
    /// The pads the core offers for the first player (description, device id), as it declared
    /// them; empty when it declared none.
    pub fn controllers(&self) -> &[(String, u32)] {
        &self.ctx.controllers
    }
    /// Plugs pad `device` (one of [`Core::controllers`]) into the first port.
    pub fn set_controller(&mut self, device: u32) -> Result<(), String> {
        if !self.ctx.controllers.iter().any(|(_, id)| *id == device) {
            return Err(format!("the core offers no controller {device}"));
        }
        // SAFETY: the live core's own entry point, on its thread.
        self.call(|a| unsafe { (a.set_controller_port_device)(0, device) });
        Ok(())
    }
    /// The disc in the drive and how many the game has, for a game of more than one.
    pub fn discs(&mut self) -> Option<(u32, u32)> {
        let disk = self.ctx.disk?;
        let (index, count) = (disk.get_image_index?, disk.get_num_images?);
        // SAFETY: callbacks the live core registered, called on its thread.
        let (i, n) = self.call(|_| unsafe { (index(), count()) });
        (n > 1).then_some((i, n))
    }
    /// The core's name for disc `index` ("Disc 2"), where it gives one.
    pub fn disc_label(&mut self, index: u32) -> Option<String> {
        let label = self.ctx.disk?.get_image_label?;
        let mut text = [0 as std::ffi::c_char; 256];
        // SAFETY: the core writes at most `len` bytes, a terminated string, into our buffer.
        let ok = self.call(|_| unsafe { label(index, text.as_mut_ptr(), text.len()) });
        // SAFETY: on success the buffer holds a NUL-terminated string within its length.
        (ok && text.contains(&0)).then(|| unsafe { self::text(text.as_ptr()) })
    }
    /// Changes the disc as a player would: open the tray, swap, close it.
    pub fn insert_disc(&mut self, index: u32) -> Result<(), String> {
        let disk = self.ctx.disk.ok_or("this game has one disc")?;
        // An index of the disc count would mean an empty drive (libretro.h): not a disc.
        let (_, count) = self.discs().ok_or("this game has one disc")?;
        if index >= count {
            return Err(format!("no disc {}", index + 1));
        }
        let (eject, set) = (
            disk.set_eject_state.ok_or("the core can't change discs")?,
            disk.set_image_index.ok_or("the core can't change discs")?,
        );
        // SAFETY: callbacks the live core registered, called on its thread, in the order the
        // header gives (the tray open while the index changes).
        let ok = self.call(|_| unsafe { eject(true) && set(index) && eject(false) });
        if ok {
            Ok(())
        } else {
            // Leave the tray closed on whatever disc is in.
            // SAFETY: as above.
            self.call(|_| unsafe { eject(false) });
            Err(format!("no disc {}", index + 1))
        }
    }
    /// A finger on the frame, in libretro's pointer coordinates, or `None` when lifted. The
    /// core reads it in the next frame; a lifted pointer keeps its last position.
    pub fn set_pointer(&mut self, at: Option<(i16, i16)>) {
        self.ctx.pressed = at.is_some();
        if let Some(at) = at {
            self.ctx.pointer = at;
        }
    }
    pub fn av(&self) -> retro_system_av_info {
        let mut av = self.ctx.av;
        if self.ctx.rotation % 2 == 1 {
            std::mem::swap(&mut av.geometry.base_width, &mut av.geometry.base_height);
            std::mem::swap(&mut av.geometry.max_width, &mut av.geometry.max_height);
            if av.geometry.aspect_ratio > 0.0 {
                av.geometry.aspect_ratio = av.geometry.aspect_ratio.recip();
            }
        }
        av
    }
    pub fn rotation(&self) -> u32 {
        self.ctx.rotation
    }
    pub fn wants_shutdown(&self) -> bool {
        self.ctx.shutdown
    }
    pub fn single_session_states(&self) -> bool {
        self.ctx.quirks & u64::from(RETRO_SERIALIZATION_QUIRK_SINGLE_SESSION) != 0
    }
    /// Whether states can be kept across runs: the core serializes at all (gw-libretro doesn't:
    /// its size is 0) and its states aren't single-session ones.
    pub fn has_states(&mut self) -> bool {
        if !self.loaded || self.single_session_states() {
            return false;
        }
        let size = self.call(|a| {
            // SAFETY: initialized, loaded core on its owning thread.
            unsafe { (a.serialize_size)() }
        });
        size > 0 && size <= 64 * 1024 * 1024
    }
    pub fn messages(&mut self) -> Vec<String> {
        std::mem::take(&mut self.ctx.messages)
    }

    pub fn set_option(&mut self, key: &str, value: &str) -> Result<(), String> {
        let def = self
            .ctx
            .options
            .get(key)
            .ok_or_else(|| format!("unknown core option {key}"))?;
        if !def.values.iter().any(|v| v.0 == value) {
            return Err(format!("invalid value {value} for {key}"));
        }
        self.ctx
            .set_value(key.into(), CString::new(value).map_err(|e| e.to_string())?);
        // A core may publish its definitions again after a runtime change. Preserve the
        // current choice instead of reinstating the startup command-line override.
        self.ctx.overrides.insert(key.into(), value.into());
        self.ctx.variables_changed = true;
        if let Some(update) = self.ctx.update_display {
            self.call(|_| {
                // SAFETY: callback registered by the live core, called in its scoped thread.
                unsafe { update() };
            });
        }
        Ok(())
    }

    pub fn load_game(&mut self, path: &Path) -> Result<(), String> {
        if self.loaded {
            return Err("a game is already loaded".into());
        }
        let mut actual_path = path.to_owned();
        let zipped = path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("zip"));
        if zipped && !self.info.block_extract {
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            let mut found = false;
            for index in 0..zip.len() {
                let entry = zip.by_index(index).map_err(|e| e.to_string())?;
                let ext = Path::new(entry.name())
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if entry.is_dir() || !self.info.extensions.iter().any(|e| e == &ext) {
                    continue;
                }
                if entry.size() > 64 * 1024 * 1024 {
                    return Err("cartridge exceeds the 64 MiB content limit".into());
                }
                use std::io::Read;
                entry
                    .take(64 * 1024 * 1024 + 1)
                    .read_to_end(&mut self.content)
                    .map_err(|e| e.to_string())?;
                if self.content.len() > 64 * 1024 * 1024 {
                    return Err("ZIP content exceeds limit".into());
                }
                if self.info.need_fullpath {
                    let dir = std::env::var_os("XDG_RUNTIME_DIR")
                        .map(PathBuf::from)
                        .unwrap_or_else(std::env::temp_dir);
                    // A private directory prevents collisions and symlink attacks during extraction.
                    let dir = dir.join(format!("nibcase-{}", std::process::id()));
                    std::fs::create_dir(&dir).map_err(|e| format!("create runtime folder: {e}"))?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                            .map_err(|e| e.to_string())?;
                    }
                    self.extracted = Some(dir.clone());
                    actual_path = dir.join(format!("content.{ext}"));
                    std::fs::write(&actual_path, &self.content).map_err(|e| e.to_string())?;
                    self.game_hash = content_hash(&self.content);
                    self.content.clear();
                }
                found = true;
                break;
            }
            if !found {
                return Err("ZIP contains no ROM supported by this core".into());
            }
        } else if !self.info.need_fullpath {
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            use std::io::Read;
            file.take(64 * 1024 * 1024 + 1)
                .read_to_end(&mut self.content)
                .map_err(|e| e.to_string())?;
            if self.content.len() > 64 * 1024 * 1024 {
                return Err("cartridge exceeds the 64 MiB content limit".into());
            }
        }
        if !self.content.is_empty() {
            self.game_hash = content_hash(&self.content);
        } else if self.game_hash == 0 {
            // A core that reads the file itself may be handed a disc of hundreds of MB: the
            // game is known by its first 64 MiB and its size, which only identifies it.
            use std::io::Read;
            let file = std::fs::File::open(&actual_path).map_err(|e| e.to_string())?;
            let size = file.metadata().map_err(|e| e.to_string())?.len();
            let mut data = Vec::new();
            file.take(64 * 1024 * 1024)
                .read_to_end(&mut data)
                .map_err(|e| e.to_string())?;
            data.extend_from_slice(&size.to_le_bytes());
            self.game_hash = content_hash(&data);
        }
        self.content_path = Some(path_string(&actual_path)?);
        let game = retro_game_info {
            path: self.content_path.as_ref().unwrap().as_ptr(),
            data: if self.info.need_fullpath {
                std::ptr::null()
            } else {
                self.content.as_ptr().cast()
            },
            size: if self.info.need_fullpath {
                0
            } else {
                self.content.len()
            },
            meta: std::ptr::null(),
        };
        let loaded = self.call(|a| {
            // SAFETY: path and content are owned by Core until unload_game; game points at them.
            unsafe { (a.load_game)(&game) }
        });
        if !loaded {
            return Err("core rejected the game".into());
        }
        self.loaded = true;
        let mut av = retro_system_av_info::default();
        self.call(|a| {
            // SAFETY: writable AV info; core has successfully loaded content.
            unsafe {
                (a.get_system_av_info)(&mut av);
                (a.set_controller_port_device)(0, RETRO_DEVICE_JOYPAD);
            }
        });
        self.ctx.av = av;
        self.validate_av()?;
        for key in self.ctx.overrides.keys() {
            if !self.ctx.options.contains_key(key) {
                return Err(format!("unknown core option {key}"));
            }
        }
        self.check_error()
    }

    fn validate_av(&self) -> Result<(), String> {
        let a = self.ctx.av;
        if !(1.0..=240.0).contains(&a.timing.fps)
            || !(8000.0..=192000.0).contains(&a.timing.sample_rate)
            || a.geometry.base_width == 0
            || a.geometry.base_height == 0
            || a.geometry.base_width > 8192
            || a.geometry.base_height > 8192
        {
            return Err("core returned invalid AV information".into());
        }
        Ok(())
    }

    pub fn run(
        &mut self,
        buttons: u16,
        analog: [i16; 2],
        fast_forward: bool,
        occupancy: u32,
    ) -> Result<Vec<i16>, String> {
        if !self.loaded {
            return Err("no game loaded".into());
        }
        self.ctx.buttons = buttons;
        self.ctx.analog = analog;
        self.ctx.fast_forward = fast_forward;
        self.ctx.occupancy = occupancy.min(100);
        self.ctx.audio.clear();
        let frame_time = self.ctx.frame_time.callback;
        let status = self.ctx.audio_status.callback;
        let micros = (1_000_000.0 / self.ctx.av.timing.fps) as i64;
        self.call(|a| {
            // SAFETY: callbacks registered by the live core; invoked synchronously with context.
            unsafe {
                if let Some(f) = frame_time {
                    f(micros);
                }
                if let Some(f) = status {
                    f(true, occupancy.min(100), occupancy < 25);
                }
                (a.run)();
            }
        });
        self.ctx.frames += 1;
        self.check_error()?;
        self.validate_av()?;
        Ok(std::mem::take(&mut self.ctx.audio))
    }

    pub fn reset(&mut self) {
        self.call(|a| {
            // SAFETY: the core is initialized and run on its owning thread.
            unsafe { (a.reset)() };
        });
    }

    pub fn serialize(&mut self) -> Result<Vec<u8>, String> {
        if !self.loaded {
            return Err("no game loaded".into());
        }
        if self.ctx.quirks & u64::from(RETRO_SERIALIZATION_QUIRK_MUST_INITIALIZE) != 0
            && self.ctx.frames == 0
        {
            self.run(0, [0; 2], false, 50)?;
        }
        let size = self.call(|a| {
            // SAFETY: initialized, loaded core on its owning thread.
            unsafe { (a.serialize_size)() }
        });
        if size == 0 || size > 64 * 1024 * 1024 {
            return Err("core has no supported save states".into());
        }
        let mut data = vec![0; size];
        let saved = self.call(|a| {
            // SAFETY: writable buffer of precisely the core's advertised serialization size.
            unsafe { (a.serialize)(data.as_mut_ptr().cast(), data.len()) }
        });
        if !saved {
            return Err("core could not serialize state".into());
        }
        Ok(data)
    }

    pub fn unserialize(&mut self, data: &[u8]) -> Result<(), String> {
        if !self.loaded || data.is_empty() || data.len() > 64 * 1024 * 1024 {
            return Err("invalid save state".into());
        }
        if self.ctx.quirks & u64::from(RETRO_SERIALIZATION_QUIRK_MUST_INITIALIZE) != 0
            && self.ctx.frames == 0
        {
            self.run(0, [0; 2], false, 50)?;
        }
        let loaded = self.call(|a| {
            // SAFETY: read-only serialized bytes live throughout the synchronous call.
            unsafe { (a.unserialize)(data.as_ptr().cast(), data.len()) }
        });
        if loaded {
            Ok(())
        } else {
            Err("core rejected the save state".into())
        }
    }

    pub fn memory(&mut self, id: u32) -> Vec<u8> {
        self.call(|a| {
            // SAFETY: libretro owns this memory until the next core call; it is copied here.
            unsafe {
                let n = (a.get_memory_size)(id);
                let ptr = (a.get_memory_data)(id);
                if ptr.is_null() || n == 0 || n > 64 * 1024 * 1024 {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(ptr.cast::<u8>(), n).to_vec()
                }
            }
        })
    }

    pub fn restore_memory(&mut self, id: u32, data: &[u8]) -> Result<(), String> {
        self.call(|a| {
            // SAFETY: core owns writable memory of its advertised size; copy only on exact match.
            unsafe {
                let n = (a.get_memory_size)(id);
                let ptr = (a.get_memory_data)(id);
                if ptr.is_null() || n != data.len() {
                    return Err("battery save size does not match this core".into());
                }
                std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.cast(), n);
                Ok(())
            }
        })
    }

    fn check_error(&mut self) -> Result<(), String> {
        match self.ctx.error.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        let (loaded, initialized) = (self.loaded, self.initialized);
        self.call(|a| {
            // SAFETY: library and context remain live during final unload/deinit.
            unsafe {
                if loaded {
                    (a.unload_game)();
                }
                if initialized {
                    (a.deinit)();
                }
            }
        });
        if let Some(path) = self.extracted.take() {
            let _ = std::fs::remove_dir_all(path);
        }
        ACTIVE.store(false, Ordering::Release);
    }
}

fn callback<T>(fallback: T, f: impl FnOnce(&mut Context) -> T) -> T {
    CONTEXT.with(|c| {
        let ptr = c.get();
        if ptr.is_null() {
            return fallback;
        }
        // SAFETY: only the thread that owns Core sets this pointer, and only around calls into
        // libretro. No Rust reference to Context is held over such a call. It remains boxed.
        let ctx = unsafe { &mut *ptr };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(ctx))) {
            Ok(value) => value,
            Err(_) => {
                ctx.error = Some("panic contained at libretro callback boundary".into());
                fallback
            }
        }
    })
}

unsafe extern "C" fn video_callback(data: *const c_void, width: u32, height: u32, pitch: usize) {
    callback((), |c| {
        if data.is_null() {
            return;
        } // A duplicate frame retains the previous texture.
        if data as usize == usize::MAX {
            c.error = Some("hardware-rendered cores are not supported yet".into());
            return;
        }
        let bpp = if c.format == 1 { 4 } else { 2 };
        let row = (width as usize).saturating_mul(bpp);
        if width > 8192
            || height > 8192
            || width == 0
            || height == 0
            || pitch < row
            || pitch > 128 * 1024
        {
            c.error = Some("invalid frame geometry or pitch".into());
            return;
        }
        let size = pitch * (height as usize - 1) + row;
        // SAFETY: the core guarantees a buffer of pitch*height bytes until this callback returns.
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
        match Frame::decode(bytes, width, height, pitch, c.format) {
            Ok(frame) => c.frame = frame.rotated(c.rotation),
            Err(error) => c.error = Some(error),
        }
    });
}
unsafe extern "C" fn audio_sample(left: i16, right: i16) {
    callback((), |c| {
        if c.audio.len() < 2 * 192000 {
            c.audio.extend([left, right]);
        }
    });
}
unsafe extern "C" fn audio_batch(data: *const i16, frames: usize) -> usize {
    callback(0, |c| {
        if data.is_null() || frames > 192000 || c.audio.len().saturating_add(frames * 2) > 384000 {
            return 0;
        }
        // SAFETY: libretro supplies interleaved stereo samples valid for this callback.
        c.audio
            .extend_from_slice(unsafe { std::slice::from_raw_parts(data, frames * 2) });
        frames
    })
}
unsafe extern "C" fn input_poll() {}
unsafe extern "C" fn input_state(port: u32, device: u32, index: u32, id: u32) -> i16 {
    callback(0, |c| {
        if let Some(asked) = &mut c.asked {
            if asked.insert((port, device, index, id)) {
                eprintln!("nibcase: input read: port {port} device {device} index {index} id {id}");
            }
        }
        // There is one touch panel: the pointer answers whichever port a core reads it on
        // (gw-libretro reads port 2). Joypad and stick are the first player's only.
        if device & RETRO_DEVICE_MASK == RETRO_DEVICE_POINTER && index == 0 {
            return match id {
                RETRO_DEVICE_ID_POINTER_X => c.pointer.0,
                RETRO_DEVICE_ID_POINTER_Y => c.pointer.1,
                RETRO_DEVICE_ID_POINTER_PRESSED | RETRO_DEVICE_ID_POINTER_COUNT => {
                    i16::from(c.pressed)
                }
                _ => 0,
            };
        }
        if port != 0 {
            return 0;
        }
        if device & RETRO_DEVICE_MASK == RETRO_DEVICE_JOYPAD {
            if id == RETRO_DEVICE_ID_JOYPAD_MASK {
                c.buttons as i16
            } else if id < 16 {
                i16::from(c.buttons & (1 << id) != 0)
            } else {
                0
            }
        } else if device & RETRO_DEVICE_MASK == RETRO_DEVICE_ANALOG
            && index == RETRO_DEVICE_INDEX_ANALOG_LEFT
            && id < 2
        {
            c.analog[id as usize]
        } else {
            0
        }
    })
}
