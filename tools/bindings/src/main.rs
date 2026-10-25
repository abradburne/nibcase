fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    bindgen::Builder::default()
        .header(root.join("third_party/libretro/libretro.h").to_str().unwrap())
        .allowlist_type("retro_.*")
        .allowlist_var("RETRO_.*")
        .allowlist_function("^$")
        .generate_comments(false)
        .layout_tests(false)
        .derive_default(true)
        .generate()
        .expect("generate the pinned API header")
        .write_to_file(root.join("crates/nibcase-libretro/src/ffi.rs"))
        .expect("write bindings");
}
