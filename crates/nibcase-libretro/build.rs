fn main() {
    println!("cargo:rerun-if-changed=src/log.c");
    println!("cargo:rerun-if-changed=../../third_party/libretro/libretro.h");
    cc::Build::new()
        .file("src/log.c")
        .include("../../third_party/libretro")
        .warnings(true)
        .compile("nibcase_core_log");
}
