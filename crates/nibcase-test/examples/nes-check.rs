//! Black-box NES smoke check. Generate the original cartridge with tools/nes-smoke.py first.
//! cargo run -p nibcase-test --release --example nes-check -- CORE ROM
use nibcase_libretro::{ffi::*, Config, Core};
use std::collections::BTreeMap;
use std::path::PathBuf;
fn main() {
    check().unwrap();
}
fn check() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: nes-check CORE ROM".into());
    }
    let temp = std::env::temp_dir().join(format!("nibcase-nes-check-{}", std::process::id()));
    let result = (|| {
        let mut core = Core::open(Config {
            core: PathBuf::from(&args[0]),
            system_dir: temp.join("BIOS"),
            save_dir: temp.join("Saves"),
            options: BTreeMap::from([("nestopia_blargg_ntsc_filter".into(), "disabled".into())]),
        })?;
        core.load_game(&PathBuf::from(&args[1]))?;
        let mut audible = false;
        for _ in 0..120 {
            audible |= core.run(0, [0; 2], false, 50)?.iter().any(|s| *s != 0);
        }
        assert!(audible, "the test cartridge must emit nonzero audio");
        assert_eq!((core.frame().width, core.frame().height), (256, 224));
        let memory = core.memory(RETRO_MEMORY_SAVE_RAM);
        assert_eq!(memory.len(), 8192);
        assert_eq!(memory[1], 0);
        for (button, expected) in [
            (RETRO_DEVICE_ID_JOYPAD_A, 0x80),
            (RETRO_DEVICE_ID_JOYPAD_B, 0x40),
            (RETRO_DEVICE_ID_JOYPAD_START, 0x10),
            (RETRO_DEVICE_ID_JOYPAD_RIGHT, 1),
        ] {
            for _ in 0..3 {
                core.run(1 << button, [0; 2], false, 50)?;
            }
            assert_eq!(
                core.memory(RETRO_MEMORY_SAVE_RAM)[1],
                expected,
                "button {button}"
            );
        }
        let saved = core.serialize()?;
        for _ in 0..10 {
            core.run(0, [0; 2], false, 50)?;
        }
        let expected = core.memory(RETRO_MEMORY_SAVE_RAM);
        let picture = core.frame().clone();
        core.unserialize(&saved)?;
        for _ in 0..10 {
            core.run(0, [0; 2], false, 50)?;
        }
        assert_eq!(core.memory(RETRO_MEMORY_SAVE_RAM), expected);
        assert_eq!(*core.frame(), picture);
        let mut battery = vec![0; 8192];
        battery[0] = 73;
        core.restore_memory(RETRO_MEMORY_SAVE_RAM, &battery)?;
        assert_eq!(core.memory(RETRO_MEMORY_SAVE_RAM), battery);
        println!("NES check: video, nonzero audio, A/B/Start/Right, SRAM restore, state video/RAM continuation passed");
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(temp);
    result
}
