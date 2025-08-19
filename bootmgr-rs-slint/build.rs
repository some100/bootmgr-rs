// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Build script for `bootmgr-rs-slint`.
//!
//! Embeds ui/main.slint, which defines the design of the program.

use std::{env, path::PathBuf};

/// The files that are required by the program or user interface.
const REQUIRED_FILES: [&str; 7] = [
    "ui/fonts/Roboto-Regular.ttf",
    "ui/icons/fallback.png",
    "ui/icons/linux.png",
    "ui/icons/osx.png",
    "ui/icons/shell.png",
    "ui/icons/special.png",
    "ui/icons/windows.png",
];

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR variable not set"));

    for file in REQUIRED_FILES {
        let file = manifest_dir.join(file);
        assert!(
            matches!(std::fs::exists(&file), Ok(true)),
            "Required file did not exist: {}",
            file.display()
        );
    }
    slint_build::compile_with_config(
        "ui/main.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .expect("Failed to build slint UIs");
}
