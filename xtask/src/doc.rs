// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

use duct::cmd;

pub fn doc_crate(private: bool, open: bool, lib: bool) -> anyhow::Result<()> {
    let mut build_args = vec!["doc"];
    if private {
        build_args.push("--document-private-items");
    }

    if open {
        build_args.push("--open");
    }

    if lib {
        build_args.push("--lib")
    }

    cmd("cargo", build_args).run()?;
    Ok(())
}
