use clap::ArgMatches;
use duct::cmd;

pub fn run_bootmgr(matches: &ArgMatches) -> anyhow::Result<()> {
    let mut run_args = vec!["-d"];
    let mut build_args = vec![
        "build",
        "--bin",
        "bootmgr-rs",
        "--target",
        "x86_64-unknown-uefi",
    ];

    if let Some(ovmf_code) = matches.get_one::<String>("ovmf-code") {
        run_args.append(&mut vec!["-b", ovmf_code]);
    }

    if let Some(add_file) = matches.get_one::<String>("add-file") {
        run_args.append(&mut vec!["-f", add_file]);
    }

    if matches.contains_id("release") {
        build_args.push("-r");
        run_args.push("target/x86_64-unknown-uefi/release/bootmgr-rs.efi");
    } else {
        run_args.push("target/x86_64-unknown-uefi/debug/bootmgr-rs.efi");
    }

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
