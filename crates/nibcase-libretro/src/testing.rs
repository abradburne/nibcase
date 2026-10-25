//! The test core (`crates/testcore/testcore.c`), built once per test process with the system's
//! C compiler, and games for it. For this crate's tests and the player's (feature `testing`).
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

static FIXTURE: OnceLock<PathBuf> = OnceLock::new();

/// The test core's shared library, in a temporary folder of its own.
pub fn fixture() -> &'static Path {
    FIXTURE.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = std::env::temp_dir().join(format!("nibcase-host-tests-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let library = dir.join(format!("fixture.{}", std::env::consts::DLL_EXTENSION));
        let out = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()))
            .args([
                if cfg!(target_os = "macos") {
                    "-dynamiclib"
                } else {
                    "-shared"
                },
                "-fPIC",
                "-std=c99",
                "-Wall",
                "-Wextra",
                "-Werror",
            ])
            .arg("-I")
            .arg(root.join("third_party/libretro"))
            .arg(root.join("crates/testcore/testcore.c"))
            .arg("-o")
            .arg(&library)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        library
    })
}

/// A game for the test core: its first byte picks what the core does (see testcore.c).
pub fn game(mode: u8) -> PathBuf {
    let path = fixture().parent().unwrap().join(format!("game-{mode}.sfc"));
    std::fs::write(&path, [mode, 0, 0, 0]).unwrap();
    path
}
