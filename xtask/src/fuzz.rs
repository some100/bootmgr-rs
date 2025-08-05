use clap::Subcommand;
use duct::cmd;

#[derive(Subcommand)]
pub enum Fuzz {
    /// Run BLS type #1 parser
    Bls,

    /// Run bootloader config parser
    Boot,

    /// Run BLS type #2 (UKI) parser
    Uki,

    /// Run Windows BCD parser
    Win,
}

pub fn fuzz_parsers(command: Fuzz) -> anyhow::Result<()> {
    let mut args = vec!["fuzz", "run"];
    match command {
        Fuzz::Bls => args.push("bls"),
        Fuzz::Boot => args.push("boot"),
        Fuzz::Uki => args.push("uki"),
        Fuzz::Win => args.push("win"),
    }

    cmd!("cargo", "install", "cargo-fuzz").run()?; // will not install if its already installed
    cmd("cargo", args).run()?;
    Ok(())
}
