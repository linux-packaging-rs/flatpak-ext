fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("cosmic".into());
    slint_build::compile_with_config("ui/ui.slint", config).unwrap();
}
