//! A small dual-screen libretro player. See docs/provenance.md for the API sources.
mod audio;
mod connections;
mod input;
mod menu;
mod saves;
mod screens;
mod settings;
mod unit;

use input::Action;
use menu::Command;
use nibcase_libretro::{systems::System, Config, Core};
use nibkit::input::Button;
use settings::Preferences;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

const USAGE: &str =
    "nibcase [--core FILE.so] [--system SFC|FC|MD|GG|LYNX] [--saves DIR] [--bios DIR]
        [--option key=value]... [--fresh] [ROM]

Without a ROM, choose a game from either card. Menu saves and quits; Back pauses.
Menu+R saves a state, Menu+L loads it, Menu+Select holds fast-forward.
Diagnostics: --headless --frames N | --screenshot FILE.png [--pause] [--frames N]";
#[derive(Default)]
struct Args {
    core: PathBuf,
    system: String,
    saves: Option<PathBuf>,
    bios: Option<PathBuf>,
    options: BTreeMap<String, String>,
    rom: Option<PathBuf>,
    fresh: bool,
    headless: bool,
    frames: Option<u64>,
    shot: Option<PathBuf>,
    pause: bool,
    /// With --pause, the menu page to open (0 Play .. 5 Connections), for screenshots.
    page: usize,
    /// With --pause and --screenshot: how long the menu is open before the shot, for a page
    /// that fills in as it reads (Connections reads the radios every 2 s).
    shot_delay: Duration,
}
impl Args {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Self>, String> {
        let mut result = Self {
            system: String::new(),
            ..Self::default()
        };
        let mut args = args.into_iter();
        let mut positional = false;
        while let Some(arg) = args.next() {
            if positional {
                if result.rom.replace(arg.into()).is_some() {
                    return Err("only one ROM may be opened".into());
                }
                continue;
            }
            let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--" => positional = true,
                "-h" | "--help" => {
                    println!("{USAGE}");
                    return Ok(None);
                }
                "--core" => result.core = value()?.into(),
                "--system" => result.system = value()?,
                "--saves" => result.saves = Some(value()?.into()),
                "--bios" => result.bios = Some(value()?.into()),
                "--option" => {
                    let pair = value()?;
                    let (k, v) = pair.split_once('=').ok_or("--option needs key=value")?;
                    if k.is_empty() || v.is_empty() {
                        return Err("--option needs a nonempty key and value".into());
                    }
                    result.options.insert(k.into(), v.into());
                }
                "--fresh" | "--no-resume" => result.fresh = true,
                "--headless" => result.headless = true,
                "--pause" => result.pause = true,
                "--shot-delay" => {
                    let s: f64 = value()?.parse().map_err(|_| "--shot-delay needs seconds")?;
                    result.shot_delay = Duration::from_secs_f64(s.clamp(0.0, 30.0));
                }
                "--page" => {
                    result.page = value()?.parse().map_err(|_| "--page needs a number")?;
                }
                "--frames" => {
                    let n = value()?
                        .parse::<u64>()
                        .map_err(|_| "--frames needs a positive integer")?;
                    if n == 0 {
                        return Err("--frames must be positive".into());
                    }
                    result.frames = Some(n);
                }
                "--screenshot" => result.shot = Some(value()?.into()),
                _ if arg.starts_with('-') => return Err(format!("unknown option {arg}")),
                _ => {
                    if result.rom.replace(arg.into()).is_some() {
                        return Err("only one ROM may be opened".into());
                    }
                }
            }
        }
        let system = if result.system.is_empty() {
            result
                .rom
                .as_deref()
                .and_then(System::from_rom)
                .or_else(|| System::from_core(&result.core))
                .unwrap_or(System::Snes)
        } else {
            System::from_id(&result.system)
                .ok_or_else(|| format!("unknown system {}", result.system))?
        };
        result.system = system.id().into();
        if result.headless && (result.shot.is_some() || result.pause) {
            return Err("screenshots and menus require video".into());
        }
        if result.core.as_os_str().is_empty() {
            result.core = Some(PathBuf::from(format!(
                "/usr/lib/libretro/{}_libretro.so",
                system.default_core()
            )))
            .filter(|p| p.is_file())
            .ok_or_else(|| {
                format!(
                    "no {} core installed; install nibcase-{} or pass --core FILE.so",
                    system.label(),
                    system.default_core()
                )
            })?;
        }
        Ok(Some(result))
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("nibcase: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let Some(mut args) = Args::parse(std::env::args().skip(1))? else {
        return Ok(());
    };
    let stop = Arc::new(AtomicBool::new(false));
    for signal in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT] {
        signal_hook::flag::register(signal, stop.clone()).map_err(|e| e.to_string())?;
    }
    let mut screens = if args.headless {
        None
    } else {
        Some(screens::Screens::open()?)
    };
    if args.rom.is_none() {
        if let Some(s) = &mut screens {
            args.rom = pick(s, &stop, System::from_id(&args.system).unwrap())?;
        } else {
            return Err("headless mode needs a ROM".into());
        }
    }
    let Some(rom) = args.rom.as_ref() else {
        return Ok(());
    };
    let name = settings::stem(rom);
    let root = args
        .saves
        .clone()
        .unwrap_or_else(|| default_saves(rom, &args.system, &name));
    let bios = bios_dir(
        args.bios.as_deref(),
        &nibkit::paths::rom_roots(),
        &args.system,
    )?;
    let mut prefs = Preferences::new(&args.core, &args.system, &name, args.options.clone());
    let mut core = Core::open(Config {
        core: args.core.clone(),
        system_dir: bios.clone(),
        save_dir: root.clone(),
        options: args.options.clone(),
    })?;
    // What the game says about itself, before the core loads it: cores read some options only
    // at load (a WonderSwan game's orientation).
    if let Some(system) = System::from_id(&args.system).filter(|s| s.has_rom_defaults()) {
        let rom_bytes = nibcase_libretro::peek(rom, system.extensions()).unwrap_or_default();
        prefs.rom_defaults = nibcase_libretro::systems::rom_defaults(system, &rom_bytes)
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
    }
    prefs.apply(&mut core, true)?;
    core.load_game(rom)?;
    prefs.apply(&mut core, false)?;
    plug_pad(&mut core, &prefs);
    eprintln!(
        "nibcase: {} {} · {:.5} Hz · {:.1} Hz audio",
        core.info().name,
        core.info().version,
        core.av().timing.fps,
        core.av().timing.sample_rate
    );
    let mut saves = saves::Saves::new(root, name.clone(), core.game_hash());
    saves.restore_battery(&mut core)?;
    // A PlayStation game without the console's BIOS runs on the core's own (HLE): most do; some
    // won't, and the player should know why and what to add.
    if let Some(system) = System::from_id(&args.system) {
        let optional = system.optional_bios();
        if !optional.is_empty() && !optional.iter().any(|f| bios.join(f).is_file()) {
            saves.notices.push(format!(
                "No {} BIOS in {}: playing with the core's own. Add {} for the best results",
                system.label(),
                bios.display(),
                optional[0]
            ));
        }
    }
    saves.settle_resume(&core.info().name.clone())?;
    for notice in &saves.notices {
        eprintln!("nibcase: {notice}");
    }
    if !args.fresh && prefs.enabled("resume") && core.has_states() && saves.path(0).exists() {
        if let Err(e) = saves.load(&mut core, 0) {
            eprintln!("nibcase: resume skipped: {e}");
        }
    }
    // Always finish through this path: a rendering/audio error must still preserve battery RAM.
    let result = if let Some(s) = screens.as_mut() {
        player(s, &mut core, &mut prefs, &mut saves, &args, &stop)
    } else {
        headless(&mut core, &mut saves, args.frames.unwrap_or(120), &stop)
    };
    let ram = saves.battery(&mut core, true);
    let state = if !core.has_states() {
        Ok(())
    } else {
        saves.save(&mut core, 0).map(|_| ())
    };
    let flush = saves.flush();
    for error in [&ram, &state, &flush]
        .into_iter()
        .filter_map(|r| r.as_ref().err())
    {
        eprintln!("nibcase: saving: {error}");
    }
    result.and(ram).and(state).and(flush)
}
/// BIOS files may live on either card. An explicit directory always wins. A system that needs
/// one (`System::bios`: Lynx, PC Engine CD, Disk System) uses the card that has it, and doesn't
/// start without it.
fn bios_dir(explicit: Option<&Path>, roots: &[PathBuf], system: &str) -> Result<PathBuf, String> {
    let candidates: Vec<_> = roots
        .iter()
        .filter_map(|p| p.parent())
        .map(|p| p.join("BIOS"))
        .collect();
    let needed = System::from_id(system).and_then(System::bios);
    // The Lynx's boot ROM is exactly 512 bytes; the others only need to be there.
    let usable = |dir: &Path, file: &str| {
        std::fs::metadata(dir.join(file))
            .is_ok_and(|m| m.is_file() && m.len() > 0 && (file != "lynxboot.img" || m.len() == 512))
    };
    let optional = System::from_id(system).map_or(&[][..], System::optional_bios);
    let bios = explicit
        .map(Path::to_owned)
        .or_else(|| {
            let file = needed?;
            candidates.iter().find(|p| usable(p, file)).cloned()
        })
        // A BIOS the system plays better with: the card that has one.
        .or_else(|| {
            candidates
                .iter()
                .find(|p| optional.iter().any(|f| usable(p, f)))
                .cloned()
        })
        .or_else(|| candidates.first().cloned())
        .unwrap_or_else(|| nibkit::paths::AppDirs::new("nibcase").data.join("bios"));
    if let Some(file) = needed {
        if !usable(&bios, file) {
            let label = System::from_id(system).map_or(system.to_owned(), |s| s.label().into());
            return Err(format!(
                "{label} needs {file} in {} (or use --bios DIR)",
                bios.display()
            ));
        }
    }
    Ok(bios)
}

/// For a game of several discs: the one in the drive, and each disc's name (the core's, else
/// "Disc n").
fn discs(core: &mut Core) -> Option<(u32, Vec<String>)> {
    let (at, count) = core.discs()?;
    let names = (0..count)
        .map(|i| {
            core.disc_label(i)
                .unwrap_or_else(|| format!("Disc {}", i + 1))
        })
        .collect();
    Some((at, names))
}
/// The PlayStation pad the player chose, among those the core offers: a DualShock, or the
/// digital pad (the core's first).
fn plug_pad(core: &mut Core, prefs: &Preferences) {
    if prefs.system != "PS" {
        return;
    }
    let want = prefs.text("pad");
    let pick = core
        .controllers()
        .iter()
        .find(|(name, _)| (want == "dualshock") == name.to_lowercase().contains("dualshock"))
        .or_else(|| core.controllers().first())
        .map(|&(_, id)| id);
    if let Some(id) = pick {
        if let Err(e) = core.set_controller(id) {
            eprintln!("nibcase: controller: {e}");
        }
    }
}
fn default_saves(rom: &Path, system: &str, name: &str) -> PathBuf {
    let base = nibkit::paths::rom_roots()
        .into_iter()
        .find(|p| rom.starts_with(p))
        .and_then(|p| p.parent().map(|p| p.join("Saves")))
        .unwrap_or_else(nibkit::paths::saves_root);
    base.join(system).join(name)
}
fn headless(
    core: &mut Core,
    saves: &mut saves::Saves,
    frames: u64,
    stop: &AtomicBool,
) -> Result<(), String> {
    let began = Instant::now();
    let mut ran = 0;
    for i in 0..frames {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        core.run(0, [0; 2], false, 50)?;
        if saves.pending() {
            saves.restore_pending(core)?;
        }
        ran += 1;
        if i % 60 == 0 {
            saves.battery(core, false)?;
        }
    }
    saves.play_seconds += (ran as f64 / core.av().timing.fps) as u64;
    eprintln!(
        "nibcase: {ran} headless frames in {:.3}s; {}x{}",
        began.elapsed().as_secs_f64(),
        core.frame().width,
        core.frame().height
    );
    Ok(())
}
fn pick(
    s: &mut screens::Screens,
    stop: &AtomicBool,
    system: System,
) -> Result<Option<PathBuf>, String> {
    let mut games = Vec::new();
    for root in nibkit::paths::rom_roots() {
        if let Ok(entries) = std::fs::read_dir(root.join(system.id())) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && system.takes(&p) {
                    games.push(p);
                }
            }
        }
    }
    games.sort_by_key(|p| settings::stem(p).to_ascii_lowercase());
    let rows = if games.is_empty() {
        vec![nibkit::menu::Row::heading(format!(
            "Put cartridge ROMs or .zip games in Roms/{}",
            system.id()
        ))]
    } else {
        games
            .iter()
            .map(|p| nibkit::menu::Row::new(settings::stem(p), nibkit::menu::Value::None))
            .collect()
    };
    let mut model = nibkit::menu::Menu {
        title: format!("Choose a {} game", system.label()),
        rows,
        hint: "A  play   ·   B  quit".into(),
        ..Default::default()
    };
    let controllers = s.sdl.game_controller()?;
    for mapping in nibkit::input::CONTROLLER_MAPPINGS {
        let _ = controllers.add_mapping(mapping);
    }
    let _pads: Vec<_> = (0..controllers.num_joysticks()?)
        .filter_map(|i| controllers.open(i).ok())
        .collect();
    let mut pump = s.sdl.event_pump()?;
    let mut input = input::Input::default();
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(None);
        }
        s.picker(&model, &menu::status())?;
        let event = pump.wait_event_timeout(100);
        for event in event.into_iter().chain(pump.poll_iter()) {
            let effect = match input.event(event) {
                Some(Action::Quit) | Some(Action::Press(Button::Menu | Button::Back)) => {
                    return Ok(None)
                }
                Some(Action::Press(key)) => input::menu(key).and_then(|key| model.key(key)),
                Some(Action::Touch {
                    window,
                    x,
                    y,
                    normalized,
                }) => s
                    .touch(window, x, y, normalized)
                    .and_then(|(x, y)| model.touch(x, y)),
                _ => None,
            };
            match effect {
                Some(nibkit::menu::Effect::Back) => return Ok(None),
                Some(nibkit::menu::Effect::Choose(index)) => return Ok(games.get(index).cloned()),
                _ => {}
            }
        }
    }
}
fn command(
    command: Command,
    core: &mut Core,
    saves: &mut saves::Saves,
    menu: &mut menu::Menu,
    paused: &mut bool,
    fast: &mut bool,
) -> Result<bool, String> {
    match command {
        Command::None => {}
        Command::Resume => {
            menu.settled();
            *paused = false;
        }
        Command::Quit => return Ok(true),
        Command::Reset => {
            core.reset();
            *paused = false;
        }
        Command::Fast => {
            *fast = !*fast;
            *paused = false;
        }
        Command::Disc(n) => {
            core.insert_disc(n)?;
            menu.discs = discs(core);
            menu.notice = format!("Disc {} is in the drive", n + 1);
        }
        Command::Save(slot) => {
            if !core.has_states() {
                return Err("This core does not support persistent states".into());
            }
            let state = saves.save(core, slot)?;
            menu.states[slot] = Some(state);
            menu.notice = format!("Saved slot {slot}");
        }
        Command::Load(slot) => {
            if !core.has_states() {
                return Err("This core does not support persistent states".into());
            }
            saves.load(core, slot)?;
            menu.notice = format!("Loaded slot {slot}");
            *paused = false;
        }
    }
    Ok(false)
}
fn player(
    s: &mut screens::Screens,
    core: &mut Core,
    prefs: &mut Preferences,
    saves: &mut saves::Saves,
    args: &Args,
    stop: &AtomicBool,
) -> Result<(), String> {
    let controllers = s.sdl.game_controller()?;
    for mapping in nibkit::input::CONTROLLER_MAPPINGS {
        let _ = controllers.add_mapping(mapping);
    }
    let mut pads: Vec<_> = (0..controllers.num_joysticks()?)
        .filter_map(|i| controllers.open(i).ok())
        .collect();
    let audio_system = s.sdl.audio()?;
    let queue = audio_system.open_queue::<f32, _>(
        None,
        &sdl2::audio::AudioSpecDesired {
            freq: Some(48000),
            channels: Some(2),
            samples: Some(512),
        },
    )?;
    if queue.spec().freq != 48000 || queue.spec().channels != 2 {
        return Err("audio output must support 48 kHz stereo".into());
    }
    let mut resampler = audio::Resampler::new(core.av().timing.sample_rate);
    let mut pump = s.sdl.event_pump()?;
    let mut input = input::Input::default();
    let mut model = menu::Menu::new(saves);
    if let Some(notice) = saves.notices.first() {
        model.notice = notice.clone();
    }
    let mut paused = args.pause;
    model.page = args.page;
    let mut fast = false;
    let mut repeat =
        nibkit::input::Repeat::new(Duration::from_millis(340), Duration::from_millis(85));
    let mut brightness = nibkit::backlight::watch();
    let mut volume = nibkit::device::watch_volume();
    let mut next = Instant::now();
    let mut last = next;
    let mut ram_at = next;
    let mut elapsed = Duration::ZERO;
    let mut frames = 0u64;
    let mut mode = None;
    let mut running_audio = false;
    // Produce a real preview before opening a diagnostic pause menu.
    if paused {
        core.run(0, [0; 2], false, 50)?;
        saves.restore_pending(core)?;
    }
    let mut dirty = true;
    let mut menu_at = Instant::now();
    let opened = Instant::now();
    // Game & Watch: the unit's photograph scaled smoothly, cut at its hinge or with touch buttons
    // under it (unit.rs); the core's Select / Start hand followed through every press.
    let watch = prefs.system == "GW";
    // Photographs (Game & Watch) and PlayStation's changing resolutions are scaled smoothly.
    s.smooth = watch || prefs.system == "PS";
    let mut keys = unit::Keys::new(&prefs.name);
    let mut card: Option<Vec<bool>> = None;
    // Screen only: the Selects to the core's Zoom when it is zoomed (pressed again to unzoom).
    let mut zoomed: Option<u32> = None;
    // Screen only, brighter: the frame's copy with its LCDs brightened, and each LCD's gain,
    // measured once for a frame of that size.
    let mut bright = nibcase_libretro::Frame::default();
    let mut gains: Option<((u32, u32), Vec<f32>)> = None;
    let mut pad = prefs.text("pad");
    let unit_name = prefs.name.clone();
    let fold = |frame: &nibcase_libretro::Frame| {
        watch
            .then(|| unit::fold(&unit_name, (frame.width, frame.height)))
            .flatten()
    };
    loop {
        if stop.load(Ordering::Relaxed) || core.wants_shutdown() {
            break;
        }
        let now = Instant::now();
        let delta = now.saturating_duration_since(last);
        last = now;
        if !paused && delta < Duration::from_secs(1) {
            elapsed += delta;
            if elapsed >= Duration::from_secs(1) {
                saves.play_seconds += elapsed.as_secs();
                elapsed -= Duration::from_secs(elapsed.as_secs());
            }
        }
        let mut events = Vec::new();
        if paused {
            if let Some(e) = pump.wait_event_timeout(25) {
                events.push(e);
            }
        }
        events.extend(pump.poll_iter());
        for event in events {
            if let sdl2::event::Event::ControllerDeviceAdded { which, .. } = event {
                if let Ok(p) = controllers.open(which) {
                    pads.push(p);
                }
            }
            if let sdl2::event::Event::ControllerDeviceRemoved { which, .. } = event {
                pads.retain(|p| p.instance_id() != which);
                input.clear();
            }
            let Some(action) = input.event(event) else {
                continue;
            };
            let mut cmd = Command::None;
            match action {
                Action::Quit => return Ok(()),
                Action::Press(Button::Menu) => {
                    input.guide = true;
                    input.chord = false;
                }
                Action::Release(Button::Menu) => {
                    input.guide = false;
                    input.fast = false;
                    if !input.chord {
                        return Ok(());
                    }
                }
                Action::Press(key) if input.guide => {
                    input.chord = true;
                    cmd = match key {
                        Button::R => Command::Save(model.last_slot),
                        Button::L => Command::Load(model.last_slot),
                        Button::Select => {
                            input.fast = true;
                            Command::None
                        }
                        _ => Command::None,
                    };
                }
                Action::Release(Button::Select) => {
                    input.fast = false;
                }
                Action::Press(Button::Back) => {
                    paused = !paused;
                    input.clear();
                    repeat.stop();
                    model.settled();
                }
                Action::Press(key) if paused => {
                    if let Some(key) = input::menu(key) {
                        if matches!(
                            key,
                            nibkit::menu::Key::Up
                                | nibkit::menu::Key::Down
                                | nibkit::menu::Key::Left
                                | nibkit::menu::Key::Right
                        ) {
                            repeat.press(key, now);
                        }
                        match model.key(key, prefs, core) {
                            Ok(c) => cmd = c,
                            Err(e) => model.notice = e,
                        }
                    }
                }
                Action::Release(key) => {
                    if let Some(key) = input::menu(key) {
                        repeat.release(key);
                    }
                    model.settled();
                }
                Action::Touch {
                    window,
                    x,
                    y,
                    normalized,
                } => {
                    if let Some((x, y)) = s.touch(window, x, y, normalized) {
                        let key = (watch && !paused && fold(core.frame()).is_none())
                            .then(|| unit::hit(keys.ordered(), x, y))
                            .flatten();
                        if paused {
                            match model.touch(x, y, prefs, core) {
                                Ok(c) => cmd = c,
                                Err(e) => model.notice = e,
                            }
                        } else if let Some(key) = key {
                            keys.tap(key);
                        } else {
                            paused = true;
                            input.clear();
                        }
                    }
                }
                _ => {}
            }
            match cmd {
                // Where the hand was in the state isn't known; a restarted unit has it away.
                Command::Load(_) => keys.forget(),
                Command::Reset => {
                    keys = unit::Keys::new(&prefs.name);
                    zoomed = None;
                }
                _ => {}
            }
            if matches!(cmd, Command::Load(_) | Command::Reset) {
                queue.pause();
                queue.clear();
                resampler.clear();
                running_audio = false;
                next = Instant::now();
            }
            match command(cmd, core, saves, &mut model, &mut paused, &mut fast) {
                Ok(true) => return Ok(()),
                Err(e) => {
                    paused = true;
                    model.notice = e;
                }
                _ => {}
            }
            dirty = true;
        }
        if paused {
            if let Some(key) = repeat.due(now) {
                let cmd = model.key(key, prefs, core).unwrap_or_else(|e| {
                    model.notice = e;
                    Command::None
                });
                if command(cmd, core, saves, &mut model, &mut paused, &mut fast)? {
                    return Ok(());
                }
                dirty = true;
            }
        }
        let av = core.av();
        let smooth = prefs.text("timing") == "smooth" && (av.timing.fps / 59.94 - 1.0).abs() < 0.02;
        let rate = if smooth { 59.94 } else { av.timing.fps };
        let factor = if fast || input.fast { 2 } else { 1 };
        let new_mode = (paused, factor, smooth, av.timing.sample_rate.to_bits());
        if mode != Some(new_mode) {
            queue.pause();
            queue.clear();
            running_audio = false;
            resampler = audio::Resampler::new(av.timing.sample_rate * f64::from(factor));
            s.set_sync(smooth || paused);
            // A Game & Watch unit uses the bottom panel in play: its body, or touch buttons.
            s.bottom_power(!paused && !watch && prefs.text("bottom") == "off");
            input.clear();
            repeat.stop();
            next = Instant::now();
            mode = Some(new_mode);
            dirty = true;
        }
        // The Connections page lives while the menu is open (its worker talks to iwd and BlueZ).
        model.keep_connections(paused);
        if paused {
            if brightness.as_mut().is_some_and(|w| w.changed())
                || volume.as_mut().is_some_and(|w| w.changed())
                || menu_at.elapsed() > Duration::from_secs(1)
            {
                dirty = true;
            }
            if dirty {
                // The pad, plugged again when the player changes it in the menu.
                let wanted = prefs.text("pad");
                if wanted != pad {
                    plug_pad(core, prefs);
                    pad = wanted;
                }
                model.discs = discs(core);
                model.rebuild(prefs, core, fast);
                let shoot = args.shot.is_some() && opened.elapsed() >= args.shot_delay;
                if let (Some(path), true) = (&args.shot, shoot) {
                    s.capture_next(path.clone());
                }
                s.pause(&model, &menu::status(), core.frame(), &prefs.name)?;
                menu_at = Instant::now();
                dirty = false;
                if shoot {
                    return Ok(());
                }
            }
            continue;
        }
        if delta > Duration::from_millis(250) {
            queue.clear();
            resampler.clear();
            next = Instant::now();
        }
        let occupancy = (queue.size() / 8 * 100 / 4096).min(100);
        let mut samples = Vec::new();
        for _ in 0..factor {
            let pad = input.buttons(prefs.enabled("stick"));
            samples.extend(core.run(
                if watch { keys.frame(pad) } else { pad },
                input.analog,
                factor > 1,
                occupancy,
            )?);
            frames += 1;
            if saves.pending() {
                saves.restore_pending(core)?;
            }
        }
        let queued = f64::from(queue.size() / 8);
        if queued > 8192.0 {
            queue.clear();
        }
        let step = audio::step(
            av.timing.sample_rate * f64::from(factor),
            av.timing.fps,
            rate,
            queued,
            1024.0,
        );
        queue.queue_audio(resampler.process(&samples, step))?;
        if !running_audio && queue.size() >= 8192 {
            queue.resume();
            running_audio = true;
        }
        let done = args.frames.is_some_and(|limit| frames >= limit)
            || (args.shot.is_some() && frames >= args.frames.unwrap_or(120));
        if done {
            if let Some(path) = &args.shot {
                s.capture_next(path.clone());
            }
        }
        let screen_only = watch && prefs.text("show") == "screen";
        if watch && keys.idle() {
            // Zoom or unzoom when the setting and the core disagree: the frame is the whole
            // unit's while it isn't zoomed, so its size finds the unit in the table.
            let f = core.frame();
            match (screen_only, zoomed) {
                (true, None) => {
                    if let Some(k) = unit::zoom(&prefs.name, (f.width, f.height)) {
                        keys.zoom(k);
                        zoomed = Some(k);
                    }
                }
                (false, Some(k)) => {
                    keys.zoom(k);
                    zoomed = None;
                }
                _ => {}
            }
        }
        let mut frame = core.frame();
        let two = screen_only
            .then(|| unit::screens(&prefs.name, (frame.width, frame.height)))
            .flatten();
        // The LCDs to brighten: the two cut out, or the whole frame once the core has zoomed
        // (its frame then no longer has the whole unit's size).
        let size = (frame.width, frame.height);
        let lcds: Vec<unit::Rect> = match two {
            Some((a, b)) => vec![a.src, b.src],
            None if zoomed.is_some() && unit::zoom(&prefs.name, size).is_none() => {
                vec![(0, 0, frame.width, frame.height)]
            }
            None => Vec::new(),
        };
        if screen_only && prefs.text("lcd") == "bright" && !lcds.is_empty() && frame.width > 0 {
            if gains.as_ref().is_none_or(|(at, _)| *at != size) {
                let g = lcds
                    .iter()
                    .map(|r| unit::lcd_gain(&frame.rgba, frame.width, *r))
                    .collect();
                gains = Some((size, g));
            }
            bright.width = frame.width;
            bright.height = frame.height;
            bright.rgba.clear();
            bright.rgba.extend_from_slice(&frame.rgba);
            for (r, g) in lcds.iter().zip(&gains.as_ref().unwrap().1) {
                unit::brighten(&mut bright.rgba, bright.width, *r, *g);
            }
            frame = &bright;
        }
        if let Some((top, bottom)) = two {
            s.play_split(frame, top, bottom)?;
        } else if let Some(f) = fold(frame) {
            let (lid, body) = unit::split(frame.width, frame.height, f);
            s.play_split(frame, lid, body)?;
        } else {
            if watch {
                // Redrawn only when a button's state changes.
                let ready = unit::buttons(keys.ordered())
                    .iter()
                    .map(|(k, _, _)| keys.ready(*k))
                    .collect();
                if card.as_ref() != Some(&ready) {
                    s.set_card(Some(&unit::card(&prefs.name, &keys)))?;
                    card = Some(ready);
                }
            }
            s.play(
                frame,
                av.geometry.aspect_ratio,
                prefs.text("scale") == "fit",
                prefs.text("aspect") == "tv",
            )?;
        }
        for message in core.messages() {
            eprintln!("nibcase: {message}");
        }
        if ram_at.elapsed() >= Duration::from_secs(1) {
            if let Err(e) = saves.battery(core, false) {
                paused = true;
                model.notice = e;
            }
            ram_at = Instant::now();
        }
        if done {
            break;
        }
        let period = Duration::from_secs_f64(1.0 / rate);
        next += period;
        let after = Instant::now();
        if next > after {
            std::thread::sleep(next - after);
        } else if after.duration_since(next) > period * 2 {
            next = after;
        }
    }
    model.settled();
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lynx_bios_on_either_card_and_explicit_override() {
        let temp = std::env::temp_dir().join(format!("nibcase-bios-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let roots = [temp.join("one/Roms"), temp.join("two/Roms")];
        let second = temp.join("two/BIOS");
        std::fs::create_dir_all(&second).unwrap();
        assert!(bios_dir(None, &roots, "LYNX").is_err());
        let first = temp.join("one/BIOS");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::write(first.join("lynxboot.img"), b"short").unwrap();
        std::fs::write(second.join("lynxboot.img"), [0; 512]).unwrap();
        assert_eq!(bios_dir(None, &roots, "LYNX").unwrap(), second);
        assert!(bios_dir(Some(&temp.join("missing")), &roots, "LYNX").is_err());
        assert_eq!(bios_dir(None, &roots, "GG").unwrap(), temp.join("one/BIOS"));
        // The Disk System's BIOS, on the first card only.
        assert!(bios_dir(None, &roots, "FDS")
            .unwrap_err()
            .contains("disksys.rom"));
        std::fs::write(temp.join("one/BIOS/disksys.rom"), [1; 8192]).unwrap();
        assert_eq!(
            bios_dir(None, &roots, "FDS").unwrap(),
            temp.join("one/BIOS")
        );
        // A PlayStation BIOS is optional: the card that has one, else the first.
        assert_eq!(bios_dir(None, &roots, "PS").unwrap(), temp.join("one/BIOS"));
        std::fs::write(second.join("scph5501.bin"), [1; 16]).unwrap();
        assert_eq!(bios_dir(None, &roots, "PS").unwrap(), second);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn cli_validation() {
        let parse = |v: &[&str]| Args::parse(v.iter().map(|v| v.to_string()));
        assert!(parse(&["--core", "test.so", "--frames", "0"]).is_err());
        assert!(parse(&["--core", "test.so", "--system", "../SFC"]).is_err());
        assert!(parse(&["--core", "test.so", "--option", "bad"]).is_err());
        let nes = parse(&["--core", "test.so", "Game.nes.zip"])
            .unwrap()
            .unwrap();
        assert_eq!(nes.system, "FC");
        let nes = parse(&["--core", "test.so", "--system", "FC"])
            .unwrap()
            .unwrap();
        assert_eq!(nes.system, "FC");
        let a = parse(&["--core", "test.so", "--option", "a=b", "--", "-game.sfc"])
            .unwrap()
            .unwrap();
        assert_eq!(a.options["a"], "b");
        assert_eq!(a.rom.unwrap(), Path::new("-game.sfc"));
    }
}
