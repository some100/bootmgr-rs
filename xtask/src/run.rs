// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

use clap::Subcommand;
use duct::cmd;

#[derive(Subcommand)]
pub enum Frontend {
    /// Run ratatui frontend
    Ratatui,

    /// Run minimal frontend
    Minimal,

    /// Run Slint frontend
    Slint,
}

pub fn run_bootmgr(
    ovmf_code: Option<&str>,
    release: bool,
    add_file: Option<&str>,
    frontend: Frontend,
) -> anyhow::Result<()> {
    let bin = match frontend {
        Frontend::Minimal => "bootmgr-rs-minimal",
        Frontend::Ratatui => "bootmgr-rs-ratatui",
        Frontend::Slint => "bootmgr-rs-slint",
    };
    let mut run_args = vec!["-d"];
    let mut build_args = vec![
        "build",
        "--bin",
        bin,
        "--target",
        "x86_64-unknown-uefi",
        "--features",
        "global_allocator,panic_handler",
    ];

    if let Some(ovmf_code) = ovmf_code {
        run_args.append(&mut vec!["-b", &ovmf_code]);
    }

    if let Some(add_file) = add_file {
        run_args.append(&mut vec!["-f", &add_file]);
    }

    let app = if release {
        build_args.extend(["--profile", "release-lto"]);
        format!("target/x86_64-unknown-uefi/release-lto/{bin}.efi")
    } else {
        format!("target/x86_64-unknown-uefi/debug/{bin}.efi")
    };

    run_args.push(&app);

    cmd!("cargo", "install", "uefi-run").run()?; // will not install if its already installed
    cmd("cargo", build_args).run()?;
    if let Err(e) = cmd("uefi-run", run_args).run() {
        println!(
            "hint: if the error was that the PC BIOS could not be loaded, you may have to specify ovmf-code"
        );
        return Err(e.into());
    }
    Ok(())
}
