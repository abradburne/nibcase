use nibcase_libretro::testing::{fixture, game};
use nibcase_libretro::{ffi::*, Config, Core};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());
fn config() -> Config {
    let dir = fixture().parent().unwrap();
    Config {
        core: fixture().into(),
        system_dir: dir.join("BIOS"),
        save_dir: dir.join("Saves"),
        options: BTreeMap::new(),
    }
}
#[test]
fn lifecycle_options_input_audio_and_states() {
    let _lock = SERIAL.lock().unwrap();
    let mut core = Core::open(config()).unwrap();
    assert_eq!(core.info().name, "nibcase test core");
    assert_eq!(core.value("fixture_format"), Some("565"));
    assert_eq!(
        core.options()
            .find(|d| d.key == "fixture_format")
            .unwrap()
            .category,
        "Video"
    );
    assert!(
        !core
            .options()
            .find(|d| d.key == "fixture_hidden")
            .unwrap()
            .visible
    );
    assert!(
        Core::open(config()).is_err(),
        "process-wide single-core guard"
    );
    core.load_game(&game(0)).unwrap();
    let audio = core
        .run(1 << RETRO_DEVICE_ID_JOYPAD_A, [0; 2], false, 50)
        .unwrap();
    assert_eq!(audio, [0, 0, 100, -100, 200, -200, 300, -300, 400, -400]);
    assert_eq!(
        core.frame().rgba,
        [255, 0, 0, 255, 0, 0, 255, 255, 0, 255, 0, 255, 255, 255, 255, 255]
    );
    assert_eq!(&core.memory(RETRO_MEMORY_SAVE_RAM)[..2], &[1, 0]);
    let state = core.serialize().unwrap();
    let before = core.frame().clone();
    core.run(1, [0; 2], false, 50).unwrap();
    assert_eq!(
        core.frame(),
        &before,
        "NULL means duplicate, not a black frame"
    );
    let audio_after = core.run(0, [0; 2], false, 50).unwrap();
    core.unserialize(&state).unwrap();
    core.run(1, [0; 2], false, 50).unwrap();
    assert_eq!(core.run(0, [0; 2], false, 50).unwrap(), audio_after);
    for format in ["1555", "8888", "565"] {
        core.set_option("fixture_format", format).unwrap();
        core.run(0, [0; 2], false, 50).unwrap();
        core.run(0, [0; 2], false, 50).unwrap();
        assert_eq!(&core.frame().rgba[..4], &[255, 0, 0, 255]);
    }
    core.set_option("fixture_format", "8888").unwrap();
    assert!(
        core.options()
            .find(|d| d.key == "fixture_hidden")
            .unwrap()
            .visible
    );
    assert!(core.set_option("fixture_format", "bad").is_err());
    assert!(core.unserialize(&[0; 2]).is_err());
    assert!(core.restore_memory(RETRO_MEMORY_SAVE_RAM, &[0; 2]).is_err());
    core.restore_memory(RETRO_MEMORY_SAVE_RAM, &[7; 16])
        .unwrap();
    assert_eq!(core.memory(RETRO_MEMORY_SAVE_RAM), [7; 16]);
}

#[test]
fn geometry_change_and_hardware_refusal() {
    let _lock = SERIAL.lock().unwrap();
    {
        let mut core = Core::open(config()).unwrap();
        core.load_game(&game(2)).unwrap();
        core.run(0, [0; 2], false, 50).unwrap();
        core.run(0, [0; 2], false, 50).unwrap();
        assert_eq!((core.frame().width, core.frame().height), (3, 2));
        assert_eq!(core.av().geometry.base_width, 3);
        assert_eq!(
            &core.frame().rgba[..12],
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255]
        );
    }
    let mut core = Core::open(config()).unwrap();
    core.load_game(&game(3)).unwrap();
    assert!(core
        .run(0, [0; 2], false, 50)
        .unwrap_err()
        .contains("hardware"));
}

#[test]
fn bad_load_releases_process_lease_and_quirks_are_exposed() {
    let _lock = SERIAL.lock().unwrap();
    let mut bad = config();
    bad.core = PathBuf::from("/not/a/core.so");
    assert!(Core::open(bad).is_err());
    let mut core = Core::open(config()).unwrap();
    assert!(core.load_game(Path::new("/missing/rom.sfc")).is_err());
    core.load_game(&game(4)).unwrap();
    assert!(core.single_session_states());
}

#[test]
fn zip_skips_non_rom_members_and_accepts_stored_and_deflated() {
    let _lock = SERIAL.lock().unwrap();
    use std::io::Write;
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        let path = fixture().parent().unwrap().join("packed.smc.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default().compression_method(method);
        zip.start_file("readme.txt", options).unwrap();
        zip.write_all(b"fixture").unwrap();
        zip.add_directory("roms/", options).unwrap();
        zip.start_file("roms/game.smc", options).unwrap();
        zip.write_all(&[0; 4]).unwrap();
        zip.finish().unwrap();
        // Its bytes, before any core loads it.
        assert_eq!(nibcase_libretro::peek(&path, &["smc"]).unwrap(), [0; 4]);
        assert!(nibcase_libretro::peek(&path, &["ws"]).is_err());
        let mut core = Core::open(config()).unwrap();
        core.load_game(&path).unwrap();
        core.run(0, [0; 2], false, 50).unwrap();
        assert_eq!(core.frame().width, 2);
    }
    assert_eq!(nibcase_libretro::peek(&game(1), &[]).unwrap(), [1, 0, 0, 0]);
}

#[test]
fn runtime_choice_survives_redeclared_core_options() {
    let _lock = SERIAL.lock().unwrap();
    let mut config = config();
    config.options.insert("fixture_format".into(), "565".into());
    let mut core = Core::open(config).unwrap();
    core.load_game(&game(5)).unwrap();
    core.set_option("fixture_format", "8888").unwrap();
    core.run(0, [0; 2], false, 50).unwrap();
    core.run(0, [0; 2], false, 50).unwrap();
    assert_eq!(core.value("fixture_format"), Some("8888"));
}

#[test]
fn rotation_environment_updates_frame_and_display_aspect() {
    let _lock = SERIAL.lock().unwrap();
    let mut core = Core::open(config()).unwrap();
    core.load_game(&game(7)).unwrap();
    core.run(0, [0; 2], false, 50).unwrap();
    assert_eq!(core.rotation(), 1);
    assert_eq!((core.frame().width, core.frame().height), (2, 3));
    let geometry = core.av().geometry;
    assert_eq!((geometry.base_width, geometry.base_height), (2, 3));
    assert!((geometry.aspect_ratio - 2.0 / 3.0).abs() < 0.00001);
    assert_eq!(&core.frame().rgba[..8], &[0, 0, 255, 255, 0, 0, 255, 255]);
}

#[test]
fn button_descriptions_and_the_pointer_reach_the_core() {
    let _lock = SERIAL.lock().unwrap();
    let mut core = Core::open(config()).unwrap();
    core.load_game(&game(8)).unwrap();
    // Only the first player's joypad is described to the player.
    assert_eq!(
        core.input_descriptors(),
        [
            (RETRO_DEVICE_ID_JOYPAD_LEFT, "Move left".to_string()),
            (RETRO_DEVICE_ID_JOYPAD_A, "Jump".to_string())
        ]
    );
    let pointer = |core: &mut Core| {
        let m = core.memory(RETRO_MEMORY_SAVE_RAM);
        (
            i16::from_le_bytes([m[4], m[5]]),
            i16::from_le_bytes([m[6], m[7]]),
            m[8],
        )
    };
    core.set_pointer(Some((-16000, 12000)));
    core.run(0, [0; 2], false, 50).unwrap();
    assert_eq!(pointer(&mut core), (-16000, 12000, 1));
    // Released: the last position stays, not pressed.
    core.set_pointer(None);
    core.run(0, [0; 2], false, 50).unwrap();
    assert_eq!(pointer(&mut core), (-16000, 12000, 0));
}

#[test]
fn a_core_without_states_says_so() {
    let _lock = SERIAL.lock().unwrap();
    let mut core = Core::open(config()).unwrap();
    core.load_game(&game(0)).unwrap();
    assert!(core.has_states());
    drop(core);
    let mut core = Core::open(config()).unwrap();
    core.load_game(&game(9)).unwrap();
    core.run(0, [0; 2], false, 50).unwrap();
    assert!(!core.has_states());
    assert!(core.serialize().is_err());
}

#[test]
fn a_game_of_several_discs_changes_disc() {
    let _lock = SERIAL.lock().unwrap();
    let mut core = Core::open(config()).unwrap();
    core.load_game(&game(0)).unwrap();
    assert_eq!(core.discs(), None);
    assert!(core.insert_disc(1).is_err());
    drop(core);
    let mut core = Core::open(config()).unwrap();
    core.load_game(&game(10)).unwrap();
    assert_eq!(core.discs(), Some((0, 2)));
    assert_eq!(core.disc_label(1).as_deref(), Some("Disc Two"));
    assert_eq!(core.disc_label(5), None);
    core.insert_disc(1).unwrap();
    core.run(0, [0; 2], false, 50).unwrap();
    assert_eq!(core.memory(RETRO_MEMORY_SAVE_RAM)[9], 1);
    assert_eq!(core.discs(), Some((1, 2)));
    assert!(core.insert_disc(2).is_err());
}

#[test]
fn the_first_port_takes_a_pad_the_core_offers() {
    let _lock = SERIAL.lock().unwrap();
    let mut core = Core::open(config()).unwrap();
    core.load_game(&game(0)).unwrap();
    // RETRO_DEVICE_SUBCLASS(RETRO_DEVICE_ANALOG, 1): the subclass + 1, shifted by 8.
    let dualshock = RETRO_DEVICE_ANALOG | (2 << 8);
    assert_eq!(
        core.controllers(),
        [
            ("Standard".to_string(), RETRO_DEVICE_JOYPAD),
            ("DualShock".to_string(), dualshock)
        ]
    );
    core.set_controller(dualshock).unwrap();
    core.run(0, [0; 2], false, 50).unwrap();
    assert_eq!(core.memory(RETRO_MEMORY_SAVE_RAM)[10], dualshock as u8);
    assert!(core.set_controller(RETRO_DEVICE_MOUSE).is_err());
}
