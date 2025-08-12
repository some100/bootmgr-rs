use duct::cmd;

pub fn build_all_crates(
    release: bool,
    target: &str,
    features: Option<Vec<String>>,
    no_default_features: bool,
) -> anyhow::Result<()> {
    let mut build_args = vec!["build", "--target", target];

    if release {
        build_args.extend(["--profile", "release-lto"]);
    }

    let mut all_features = vec!["global_allocator,panic_handler,"];
    if let Some(extra_features) = &features {
        all_features.extend(extra_features.iter().map(String::as_str));
    }

    let all_features = all_features.join(",");
    build_args.push("--features");
    build_args.push(&all_features);

    if no_default_features {
        build_args.push("--no-default-features");
    }

    cmd("cargo", build_args).run()?;
    Ok(())
}
