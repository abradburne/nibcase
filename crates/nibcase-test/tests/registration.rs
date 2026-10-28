//! Exercise the actual package-hook command against an original fixture shared library.
use std::path::Path;
use std::process::{Command, Output};
fn tool(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nibcase-test"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn generate_validate_update_and_remove_only_owned_descriptions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = std::env::temp_dir().join(format!("nibcase-registration-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    let cores = temp.join("cores");
    let descriptions = temp.join("descriptions");
    std::fs::create_dir_all(&cores).unwrap();
    std::fs::create_dir(&descriptions).unwrap();
    let library = cores.join("snes9x_libretro.so");
    let cc = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()))
        .args([
            if cfg!(target_os = "macos") {
                "-dynamiclib"
            } else {
                "-shared"
            },
            "-fPIC",
            "-std=c99",
            "-DNIBCASE_TEST_STDOUT",
        ])
        .arg("-I")
        .arg(root.join("third_party/libretro"))
        .arg(root.join("crates/testcore/testcore.c"))
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        cc.status.success(),
        "{}",
        String::from_utf8_lossy(&cc.stderr)
    );
    let sync = || {
        let output = tool(&[
            Path::new("--sync"),
            &descriptions,
            Path::new("--core-dir"),
            &cores,
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    sync();
    let target = descriptions.join("nibcase-snes9x.toml");
    let text = std::fs::read_to_string(&target).unwrap();
    let emulator = nibkit::dslot::check("nibcase-snes9x", &text).unwrap();
    assert!(emulator.plays("SFC"));
    assert!(emulator.earns("SFC"), "SNES earns RetroAchievements");
    assert!(!emulator.achievements.hardcore, "Casual only");
    assert_eq!(emulator.priority, 100);
    let option = emulator
        .options_for("SFC")
        .find(|o| o.def.key == "nibcase-snes9x.fixture_format")
        .unwrap();
    assert_eq!(option.def.label, "Pixel format");
    let stamp = std::fs::metadata(&target).unwrap().modified().unwrap();
    sync();
    assert_eq!(
        std::fs::metadata(&target).unwrap().modified().unwrap(),
        stamp
    );
    let output = tool(&[
        Path::new("--describe"),
        &library,
        Path::new("--installed-core"),
        Path::new("/usr/lib/libretro/snes9x_libretro.so"),
    ]);
    assert!(output.status.success());
    let canonical = String::from_utf8(output.stdout).unwrap();
    nibkit::dslot::check("nibcase-snes9x", &canonical).unwrap();
    assert!(!canonical.contains("fixture core stdout diagnostic"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("fixture core stdout diagnostic"));
    assert!(canonical.contains("/usr/lib/libretro/snes9x_libretro.so"));
    assert!(!canonical.contains(temp.to_str().unwrap()));
    // Every installed core gets its own system, options and executable probe.
    for (name, system) in [
        ("nestopia", "FC"),
        ("blastem", "MD"),
        ("gearsystem", "GG"),
        ("handy", "LYNX"),
        ("gw", "GW"),
        ("mednafen_pce_fast", "PCE"),
        ("mednafen_ngp", "NGP"),
        ("mednafen_wswan", "WS"),
        ("pcsx_rearmed", "PS"),
    ] {
        let core_library = cores.join(format!("{name}_libretro.so"));
        std::fs::copy(&library, &core_library).unwrap();
        sync();
        let id = format!("nibcase-{name}");
        let generated = descriptions.join(format!("{id}.toml"));
        let emulator =
            nibkit::dslot::check(&id, &std::fs::read_to_string(&generated).unwrap()).unwrap();
        assert!(emulator.plays(system));
        assert!(!emulator.plays("SFC"));
        // A core registered for several systems is described for each of them.
        let also: &[&str] = match name {
            "nestopia" => &["FDS"],
            "gearsystem" => &["SMS", "SG1000"],
            "mednafen_pce_fast" => &["PCECD"],
            _ => &[],
        };
        for other in also {
            assert!(emulator.plays(other), "{name} plays {other}");
            let command = emulator.command(other).unwrap();
            assert!(command.windows(2).any(|w| w == ["--system", *other]));
        }
        // RetroAchievements, for the systems checked on the device only.
        for s in std::iter::once(&system).chain(also) {
            let checked = nibcase_libretro::systems::System::from_id(s).unwrap();
            assert_eq!(emulator.earns(s), checked.achievements(), "{name} {s}");
        }
        assert!(emulator
            .options_for(system)
            .any(|o| o.def.key == format!("{id}.fixture_format")));
        let command = emulator.command(system).unwrap();
        assert!(command.windows(2).any(|w| w == ["--system", system]));
        std::fs::remove_file(core_library).unwrap();
        sync();
        assert!(!generated.exists());
    }
    assert!(target.exists());
    std::fs::write(descriptions.join("personal.toml"), "keep me").unwrap();
    std::fs::remove_file(library).unwrap();
    sync();
    assert!(!target.exists());
    assert_eq!(
        std::fs::read_to_string(descriptions.join("personal.toml")).unwrap(),
        "keep me"
    );
    std::fs::remove_dir_all(temp).unwrap();
}
