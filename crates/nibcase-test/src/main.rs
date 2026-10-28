//! Deterministic headless core validation and DSlot description generation.
use nibcase_libretro::{
    systems::{System, CORES},
    Config, Core,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    if let Err(error) = run() {
        eprintln!("nibcase-test: {error}");
        std::process::exit(1);
    }
}
fn hash(bytes: impl IntoIterator<Item = u8>, seed: u64) -> u64 {
    bytes
        .into_iter()
        .fold(seed, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
}
/// Some cores print directly through C stdio. Keep --describe stdout valid TOML;
/// leave their diagnostics on stderr, including buffered output during core teardown.
fn description_output() -> Result<Box<dyn Write>, String> {
    #[cfg(unix)]
    {
        use std::os::fd::FromRawFd;
        // SAFETY: fflush accepts NULL to flush all C streams. dup gives us a new owned
        // descriptor; dup2 redirects only this single-threaded diagnostic process's stdout.
        unsafe {
            libc::fflush(std::ptr::null_mut());
            let fd = libc::dup(libc::STDOUT_FILENO);
            if fd < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let output = std::fs::File::from_raw_fd(fd);
            if libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO) < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(Box::new(output))
        }
    }
    #[cfg(not(unix))]
    {
        Ok(Box::new(std::io::stdout()))
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--sync") {
        return sync(&args[1..]);
    }
    if args.len() < 2 {
        return Err("usage: nibcase-test --describe CORE | --bench CORE ROM [--bios DIR] [--frames N] [--option key=value] [--state-check] [--frame PNG] [--press BUTTON@N] [--tap X,Y@N]".into());
    }
    let describe = args[0] == "--describe";
    if !describe && args[0] != "--bench" {
        return Err("choose --describe or --bench".into());
    }
    let path = PathBuf::from(&args[1]);
    let mut rom = None;
    let mut frames = 3600usize;
    let mut options = BTreeMap::new();
    let mut state_check = false;
    let mut picture = None;
    let mut presses: Vec<(u16, usize)> = Vec::new();
    let mut taps: Vec<((u32, u32), usize)> = Vec::new();
    let mut installed = path.clone();
    let mut system = None;
    let mut bios = None;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--system" => {
                index += 1;
                system = Some(
                    System::from_id(
                        args.get(index)
                            .ok_or("--system needs a system folder name (SFC, FC, MD, ...)")?,
                    )
                    .ok_or("unsupported system")?,
                );
            }
            "--bios" => {
                index += 1;
                bios = Some(PathBuf::from(
                    args.get(index).ok_or("--bios needs a directory")?,
                ));
            }
            "--installed-core" => {
                index += 1;
                installed = args
                    .get(index)
                    .ok_or("--installed-core needs a path")?
                    .into();
            }
            "--frames" => {
                index += 1;
                frames = args
                    .get(index)
                    .ok_or("--frames needs a number")?
                    .parse()
                    .map_err(|_| "invalid frame count")?;
            }
            "--option" => {
                index += 1;
                let (k, v) = args
                    .get(index)
                    .ok_or("--option needs key=value")?
                    .split_once('=')
                    .ok_or("--option needs key=value")?;
                options.insert(k.into(), v.into());
            }
            "--state-check" => state_check = true,
            "--press" => {
                index += 1;
                let (name, at) = args
                    .get(index)
                    .and_then(|a| a.split_once('@'))
                    .ok_or("--press needs BUTTON@FRAME")?;
                let id = (0..16)
                    .find(|&i| button_name(i).eq_ignore_ascii_case(name))
                    .ok_or_else(|| format!("unknown button {name}"))?;
                presses.push((1 << id, at.parse().map_err(|_| "invalid --press frame")?));
            }
            "--tap" => {
                index += 1;
                let (xy, at) = args
                    .get(index)
                    .and_then(|a| a.split_once('@'))
                    .ok_or("--tap needs X,Y@FRAME")?;
                let (x, y) = xy.split_once(',').ok_or("--tap needs X,Y@FRAME")?;
                let n = |v: &str| v.parse::<u32>().map_err(|_| "invalid --tap position");
                taps.push((
                    (n(x)?, n(y)?),
                    at.parse().map_err(|_| "invalid --tap frame")?,
                ));
            }
            "--frame" => {
                index += 1;
                picture = Some(PathBuf::from(
                    args.get(index).ok_or("--frame needs a file")?,
                ));
            }
            a if !a.starts_with('-') && rom.is_none() => rom = Some(PathBuf::from(a)),
            a => return Err(format!("unexpected argument {a}")),
        }
        index += 1;
    }
    if frames == 0 || frames > 10_000_000 {
        return Err("frame count must be 1..10000000".into());
    }
    let temp = std::env::temp_dir().join(format!("nibcase-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
    let mut output = if describe {
        Some(description_output()?)
    } else {
        None
    };
    let result = (|| {
        let mut core = Core::open(Config {
            core: path.clone(),
            system_dir: bios.unwrap_or_else(|| temp.join("BIOS")),
            save_dir: temp.join("Saves"),
            options,
        })?;
        if describe {
            // --system names one; else every system the core is registered for.
            let one;
            let systems = match system {
                Some(s) => {
                    one = [s];
                    &one[..]
                }
                None if !System::of_core(&installed).is_empty() => System::of_core(&installed),
                None => &[System::Snes],
            };
            let text = description(&core, &installed, systems)?;
            output
                .as_mut()
                .unwrap()
                .write_all(text.as_bytes())
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        core.load_game(&rom.ok_or("--bench needs a ROM")?)?;
        for _ in 0..120 {
            core.run(0, [0; 2], false, 50)?;
        }
        let mut costs = Vec::with_capacity(frames);
        let mut audio_hash = 0xcbf29ce484222325;
        let started = Instant::now();
        // Scripted input: each press and tap lasts six frames, counted from the first measured.
        let held = |at: usize, n: usize| (at..at + 6).contains(&n);
        for n in 0..frames {
            let buttons = presses
                .iter()
                .filter(|(_, at)| held(*at, n))
                .fold(0, |b, (bit, _)| b | bit);
            let frame = core.frame();
            let (w, h) = (frame.width.max(1), frame.height.max(1));
            core.set_pointer(taps.iter().find(|(_, at)| held(*at, n)).map(|((x, y), _)| {
                let axis = |v: u32, size: u32| ((v * 0xfffe) / size) as i32 - 0x7fff;
                (axis(*x, w) as i16, axis(*y, h) as i16)
            }));
            let begin = Instant::now();
            let sound = core.run(buttons, [0; 2], false, 50)?;
            costs.push(begin.elapsed().as_secs_f64() * 1000.0);
            audio_hash = hash(sound.into_iter().flat_map(i16::to_le_bytes), audio_hash);
        }
        let seconds = started.elapsed().as_secs_f64();
        let frame_hash = hash(core.frame().rgba.iter().copied(), 0xcbf29ce484222325);
        costs.sort_by(f64::total_cmp);
        println!("core={} version={} frames={} fps={:.5} size={}x{} video={frame_hash:016x} audio={audio_hash:016x} p50_ms={:.4} p99_ms={:.4} elapsed_s={seconds:.3}",
            core.info().name, core.info().version, frames, core.av().timing.fps, core.frame().width, core.frame().height,
            costs[frames/2], costs[(frames-1)*99/100]);
        // What the game's buttons do, as the core describes them.
        for (id, what) in core.input_descriptors() {
            println!("button {}: {what}", button_name(*id));
        }
        if let Some(path) = &picture {
            write_png(path, core.frame())?;
        }
        if state_check {
            let state = core.serialize()?;
            let mut first = Vec::new();
            for _ in 0..10 {
                first.extend(core.run(0, [0; 2], false, 50)?);
            }
            let picture = core.frame().clone();
            core.unserialize(&state)?;
            let mut second = Vec::new();
            for _ in 0..10 {
                second.extend(core.run(0, [0; 2], false, 50)?);
            }
            if picture != *core.frame() || first != second {
                return Err(format!("save-state continuation differs: video_match={}, audio_match={}, audio_samples={}/{}, first_audio_difference={:?}",picture==*core.frame(),first==second,first.len(),second.len(),first.iter().zip(&second).position(|(a,b)|a!=b)));
            }
            println!("state-check: video and audio continuation match");
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(temp);
    result
}
/// The RetroPad button `RETRO_DEVICE_ID_JOYPAD_*` is, by the SNES-style name libretro gives it.
fn button_name(id: u32) -> &'static str {
    const NAMES: [&str; 16] = [
        "B", "Y", "Select", "Start", "Up", "Down", "Left", "Right", "A", "X", "L", "R", "L2", "R2",
        "L3", "R3",
    ];
    NAMES.get(id as usize).copied().unwrap_or("?")
}
/// The core's last frame, as it drew it, for looking at its layout.
fn write_png(path: &std::path::Path, frame: &nibcase_libretro::Frame) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), frame.width, frame.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(&frame.rgba))
        .map_err(|e| e.to_string())
}
/// DSlot's description of the core at `path` (where it is installed), playing `systems`.
fn description(
    core: &Core,
    path: &std::path::Path,
    systems_played: &[System],
) -> Result<String, String> {
    use toml::Value as V;
    let array = |values: Vec<String>| V::Array(values.into_iter().map(V::String).collect());
    let mut doc = toml::Table::new();
    doc.insert(
        "name".into(),
        V::String(format!("{} (nibcase)", core.info().name)),
    );
    doc.insert("priority".into(), V::Integer(100));
    doc.insert(
        "requires".into(),
        array(vec![path.to_string_lossy().into_owned()]),
    );
    let mut systems = toml::Table::new();
    for system in systems_played {
        systems.insert(
            system.id().into(),
            array(vec![
                "/usr/bin/nibcase".into(),
                "--core".into(),
                path.to_string_lossy().into_owned(),
                "--system".into(),
                system.id().into(),
                "{options}".into(),
                "--saves".into(),
                "{saves}".into(),
                "{rom}".into(),
            ]),
        );
    }
    doc.insert("systems".into(), V::Table(systems));
    // Where it earns RetroAchievements: the systems it plays that were checked (Casual only).
    let earning: Vec<String> = systems_played
        .iter()
        .filter(|s| s.achievements())
        .map(|s| s.id().to_owned())
        .collect();
    if !earning.is_empty() {
        let mut achievements = toml::Table::new();
        achievements.insert("systems".into(), array(earning));
        doc.insert("achievements".into(), V::Table(achievements));
    }
    let options = core
        .options()
        .filter(|d| {
            d.key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
        .map(|d| {
            let mut t = toml::Table::new();
            t.insert("key".into(), V::String(d.key.clone()));
            t.insert("label".into(), V::String(d.label.clone()));
            t.insert(
                "values".into(),
                array(d.values.iter().map(|v| v.0.clone()).collect()),
            );
            t.insert(
                "labels".into(),
                array(d.values.iter().map(|v| v.1.clone()).collect()),
            );
            let id = format!(
                "nibcase-{}",
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .trim_end_matches("_libretro")
            );
            t.insert(
                "default".into(),
                V::String(nibcase_libretro::systems::option_default(&id, d).into()),
            );
            t.insert(
                "args".into(),
                array(vec!["--option".into(), format!("{}={{value}}", d.key)]),
            );
            V::Table(t)
        })
        .collect();
    doc.insert("options".into(), V::Array(options));
    Ok(format!(
        "# Generated from {} {} by nibcase-test --describe.\n{}",
        core.info().name,
        core.info().version,
        toml::to_string_pretty(&doc).map_err(|e| e.to_string())?
    ))
}

/// Pacman invokes this after installing/removing/updating the player or a supported core.
/// Only these registered generated IDs belong to nibcase; user descriptors are never removed.
fn sync(args: &[String]) -> Result<(), String> {
    let directory = PathBuf::from(
        args.first()
            .ok_or("--sync needs the DSlot descriptor directory")?,
    );
    let mut core_dir = PathBuf::from("/usr/lib/libretro");
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--core-dir" => {
                index += 1;
                core_dir = args
                    .get(index)
                    .ok_or("--core-dir needs a directory")?
                    .into();
            }
            _ => return Err("unknown --sync argument".into()),
        }
        index += 1;
    }
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let temp = std::env::temp_dir().join(format!("nibcase-sync-{}", std::process::id()));
    std::fs::create_dir(&temp).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut errors = Vec::new();
        for spec in CORES {
            let name = spec.name;
            let path = core_dir.join(format!("{name}_libretro.so"));
            let target = directory.join(format!("nibcase-{name}.toml"));
            if !path.is_file() {
                if target.exists() {
                    std::fs::remove_file(&target).map_err(|e| e.to_string())?;
                }
                continue;
            }
            let result = (|| {
                let core = Core::open(Config {
                    core: path.clone(),
                    system_dir: temp.join("bios"),
                    save_dir: temp.join("saves"),
                    options: BTreeMap::new(),
                })?;
                let text = description(&core, &path, spec.systems)?;
                // Skip identical descriptions so DSlot does not rebuild needlessly.
                if std::fs::read_to_string(&target).ok().as_deref() == Some(&text) {
                    return Ok(());
                }
                let temporary =
                    directory.join(format!(".nibcase-{name}.{}.tmp", std::process::id()));
                std::fs::write(&temporary, text).map_err(|e| e.to_string())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o644))
                        .map_err(|e| e.to_string())?;
                }
                std::fs::rename(&temporary, &target).map_err(|e| e.to_string())?;
                eprintln!("nibcase: registered {}", target.display());
                Ok::<_, String>(())
            })();
            if let Err(e) = result {
                errors.push(format!("{name}: {e}"));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    })();
    let _ = std::fs::remove_dir_all(temp);
    result
}
