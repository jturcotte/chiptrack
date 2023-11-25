#[cfg(not(feature = "gba"))]
fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent-light".to_owned());
    slint_build::compile_with_config("ui/main.slint", config).unwrap();
}

#[cfg(feature = "gba")]
fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config("ui/gba_main.slint", config).unwrap();
    slint_build::print_rustc_flags().unwrap();
}
