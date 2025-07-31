use clap::{Arg, ArgAction, Command};

mod build;
mod doc;
mod run;
mod test;

fn main() -> anyhow::Result<()> {
    let args = clap::command!()
        .subcommand(
            Command::new("test")
                .about("Run unit tests and clippy on host")
                .subcommand(
                    Command::new("run")
                        .about("Run integration test with uefi-run")
                        .arg(
                            Arg::new("ovmf-code")
                                .long("ovmf-code")
                                .value_name("FILE")
                                .help("path to the OVMF code file"),
                        ),
                )
                .subcommand(
                    Command::new("fuzz")
                        .about("Fuzz boot configuration parsers on host")
                        .arg(
                            Arg::new("fuzz")
                                .short('f')
                                .long("fuzz")
                                .value_name("PARSER_NAME")
                                .help("name of parser")
                                .value_parser(["bls", "boot", "uki", "win"]),
                        ),
                ),
        )
        .subcommand(
            Command::new("build")
                .about("Build all crates in workspace")
                .args([
                    Arg::new("release")
                        .short('r')
                        .long("release")
                        .help("Build with release profile")
                        .action(ArgAction::SetTrue),
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .help("Build with target architecture"),
                ]),
        )
        .subcommand(
            Command::new("doc")
                .about("Build docs for bootmgr-rs crate")
                .args([
                    Arg::new("private")
                        .short('p')
                        .long("document-private-items")
                        .help("Document private items in crate")
                        .action(ArgAction::SetTrue),
                    Arg::new("open")
                        .short('o')
                        .long("open")
                        .help("Open in web browser after documenting")
                        .action(ArgAction::SetTrue),
                ]),
        )
        .subcommand(
            Command::new("run")
                .about("Run bootmgr-rs in VM with uefi-run")
                .args([
                    Arg::new("ovmf-code")
                        .long("ovmf-code")
                        .value_name("FILE")
                        .help("path to the OVMF code file"),
                    Arg::new("release")
                        .short('r')
                        .long("release")
                        .help("Build with release profile")
                        .action(ArgAction::SetTrue),
                    Arg::new("add-file")
                        .long("add-file")
                        .value_name("FILE")
                        .help("Add an additional file to the root of the image"),
                ]),
        )
        .arg_required_else_help(true)
        .get_matches();

    match args.subcommand() {
        Some(("build", sub_m)) => build::build_all_crates(sub_m)?,
        Some(("doc", sub_m)) => doc::doc_crate(sub_m)?,
        Some(("run", sub_m)) => run::run_bootmgr(sub_m)?,
        Some(("test", sub_m)) => {
            if let Some(run) = sub_m.subcommand_matches("run") {
                test::test_on_vm(run)?;
            } else if let Some(fuzz) = sub_m.subcommand_matches("fuzz") {
                test::fuzz(fuzz)?;
            } else {
                test::test_on_host()?;
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
