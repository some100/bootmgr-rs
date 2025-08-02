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

    /// Fuzz boot configuration parsers on host
    Fuzz {
        #[arg(short, long)]
        fuzz: String,
    },
}

pub fn test_crate(command: Option<Test>) -> anyhow::Result<()> {
    if let Some(command) = command {
        match command {
            Test::Run { ovmf_code } => test_on_vm(ovmf_code.as_deref()),
            Test::Fuzz { fuzz } => fuzz_on_host(&fuzz),
        }
    } else {
        test_on_host()
    }
}

pub fn test_on_host() -> anyhow::Result<()> {
    cmd!("cargo", "clippy", "--", "-C", "panic=abort").run()?;
    cmd!("cargo", "test", "--lib").run()?;
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
        "x86_64-unknown-uefi"
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

pub fn fuzz_on_host(fuzz: &str) -> anyhow::Result<()> {
    let mut run_args = vec!["fuzz", "run"];
    match fuzz {
        "bls" => run_args.push("fuzz_bls_parser"),
        "boot" => run_args.push("fuzz_boot_parser"),
        "uki" => run_args.push("fuzz_uki_parser"),
        "win" => run_args.push("fuzz_win_parser"),
        _ => unreachable!(),
    }

    cmd("cargo", run_args).run()?;
    Ok(())
}
