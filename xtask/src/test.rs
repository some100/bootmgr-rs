use anyhow::bail;
use clap::ArgMatches;
use duct::cmd;

pub fn test_on_host() -> anyhow::Result<()> {
    cmd!("cargo", "clippy", "--", "-C", "panic=abort").run()?;
    cmd!("cargo", "test", "--lib").run()?;
    Ok(())
}

pub fn test_on_vm(matches: &ArgMatches) -> anyhow::Result<()> {
    let mut run_args = vec!["-d"];

    if let Some(ovmf_code) = matches.get_one::<String>("ovmf-code") {
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

pub fn fuzz(matches: &ArgMatches) -> anyhow::Result<()> {
    if !matches.args_present() {
        bail!("-f was not specified (specify a parser with -f to fuzz)");
    }
    let mut run_args = vec!["fuzz", "run"];
    let parser = matches.get_one::<String>("fuzz").map(|x| &**x);
    match parser {
        Some("bls") => run_args.push("fuzz_bls_parser"),
        Some("boot") => run_args.push("fuzz_boot_parser"),
        Some("uki") => run_args.push("fuzz_uki_parser"),
        Some("win") => run_args.push("fuzz_win_parser"),
        _ => unreachable!(),
    }

    cmd("cargo", run_args).run()?;
    Ok(())
}
