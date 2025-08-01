use clap::ArgMatches;
use duct::cmd;

pub fn build_all_crates(matches: &ArgMatches) -> anyhow::Result<()> {
    let mut build_args = vec!["build", "--target"];

    match matches.get_one::<String>("target") {
        Some(target) => build_args.push(target),
        None => build_args.push("x86_64-unknown-uefi"),
    }

    if matches.contains_id("release") {
        build_args.extend(["--profile", "release-lto"]);
    }

    cmd("cargo", build_args).run()?;
    Ok(())
}
