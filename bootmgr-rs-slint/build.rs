//! Build script for `bootmgr-rs-slint`.
//!
//! Embeds ui/main.slint, which defines the design of the program.

fn main() {
    slint_build::compile_with_config(
        "ui/main.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .expect("Failed to build slint UIs");
}
