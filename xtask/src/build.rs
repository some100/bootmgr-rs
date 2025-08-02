use duct::cmd;

pub fn build_all_crates(release: bool, target: &str) -> anyhow::Result<()> {
    let mut build_args = vec!["build", "--target", target];

    if release {
        build_args.extend(["--profile", "release-lto"]);
    }

    cmd("cargo", build_args).run()?;
    Ok(())
}
