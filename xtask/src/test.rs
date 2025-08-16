// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

use clap::Subcommand;
use duct::cmd;

#[derive(Subcommand)]
pub enum Test {
    /// Run integration test with uefi-run
    Run {
        /// Path to the OVMF code file
        #[arg(long)]
        ovmf_code: Option<String>,
    },
}

pub fn test_crate(command: Option<Test>) -> anyhow::Result<()> {
    if let Some(command) = command {
        let Test::Run { ovmf_code } = command;
        test_on_vm(ovmf_code.as_deref())
    } else {
        test_on_host()
    }
}

pub fn test_on_host() -> anyhow::Result<()> {
    cmd!(
        "cargo",
        "clippy",
        "--all-features",
        "--",
        "-C",
        "panic=abort"
    )
    .run()?;
    cmd!("cargo", "test", "--lib").run()?;
    cmd!("cargo", "test", "--doc").run()?;
    cmd!("cargo", "fmt", "--all", "--check").run()?;
    Ok(())
}

pub fn test_on_vm(ovmf_code: Option<&str>) -> anyhow::Result<()> {
    let mut run_args = vec!["-d"];

    if let Some(ovmf_code) = ovmf_code {
        run_args.append(&mut vec!["-b", ovmf_code]);
    }

    run_args.push("target/x86_64-unknown-uefi/debug/bootmgr-rs-tests.efi");
    cmd!("cargo", "install", "uefi-run").run()?; // will not install if its already installed
    cmd!(
        "cargo",
        "build",
        "--bin",
        "bootmgr-rs-tests",
        "--target",
        "x86_64-unknown-uefi",
        "--features",
        "global_allocator,panic_handler",
    )
    .run()?;
    if let Err(e) = cmd("uefi-run", run_args).run() {
        println!(
            "hint: if the error was that the PC BIOS could not be loaded, you may have to specify ovmf-code"
        );
        return Err(e.into());
    }
    Ok(())
}
