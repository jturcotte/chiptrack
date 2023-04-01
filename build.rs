#[cfg(not(feature = "gba"))]
fn main() {
    slint_build::compile("ui/main.slint").unwrap();
}

#[cfg(feature = "gba")]
fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config("ui/gba_main.slint", config).unwrap();
    slint_build::print_rustc_flags().unwrap();
}
