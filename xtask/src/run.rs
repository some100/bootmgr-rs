use duct::cmd;

pub fn run_bootmgr(
    ovmf_code: Option<&str>,
    release: bool,
    add_file: Option<&str>,
) -> anyhow::Result<()> {
    let mut run_args = vec!["-d"];
    let mut build_args = vec![
        "build",
        "--bin",
        "bootmgr-rs-ratatui",
        "--target",
        "x86_64-unknown-uefi",
    ];

    if let Some(ovmf_code) = ovmf_code {
        run_args.append(&mut vec!["-b", &ovmf_code]);
    }

    if let Some(add_file) = add_file {
        run_args.append(&mut vec!["-f", &add_file]);
    }

    if release {
        build_args.extend(["--profile", "release-lto"]);
        run_args.push("target/x86_64-unknown-uefi/release-lto/bootmgr-rs-ratatui.efi");
    } else {
        run_args.push("target/x86_64-unknown-uefi/debug/bootmgr-rs-ratatui.efi");
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
