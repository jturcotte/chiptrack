#[cfg(not(feature = "gba"))]
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("pkg/chiptrack.ico");
        res.compile().unwrap();
    }

    let config = slint_build::CompilerConfiguration::new().with_style("fluent-light".to_owned());
    slint_build::compile_with_config("ui/main.slint", config).unwrap();
}

#[cfg(feature = "gba")]
fn main() {
    // Run wat2wasm here so that we don't have to convert it to binary at runtime on the GBA
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::process::Command::new("wat2wasm")
        .arg("instruments/default-instruments.wat")
        .arg("-o")
        .arg(format!("{}/default-instruments.wasm", out_dir))
        .output()
        .expect("Failed to run wat2wasm command");
    println!("cargo:rerun-if-changed=instruments/default-instruments.wat");

    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config("ui/gba_main.slint", config).unwrap();
    slint_build::print_rustc_flags().unwrap();
}
