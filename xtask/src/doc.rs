use clap::ArgMatches;
use duct::cmd;

pub fn doc_crate(matches: &ArgMatches) -> anyhow::Result<()> {
    let mut build_args = vec!["doc"];
    if let Some(private) = matches.get_one::<bool>("private")
        && *private
    {
        build_args.push("--document-private-items");
    }
    if let Some(open) = matches.get_one::<bool>("open")
        && *open
    {
        build_args.push("--open");
    }

    cmd("cargo", build_args).run()?;
    Ok(())
}
