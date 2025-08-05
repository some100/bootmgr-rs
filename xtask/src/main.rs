use clap::{Parser, Subcommand};

use crate::{fuzz::Fuzz, test::Test};

mod build;
mod doc;
mod fuzz;
mod run;
mod test;

#[derive(Parser)]
#[command(about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build all crates in workspace
    Build {
        /// Build with release profile
        #[arg(short, long, default_value_t = false)]
        release: bool,

        /// Build with target architecture
        #[arg(short, long, default_value = "x86_64-unknown-uefi")]
        target: String,

        /// Space separated list of features
        #[arg(short, long)]
        features: Option<Vec<String>>,

        /// Build with no default features (except global allocator and panic handler)
        #[arg(long, default_value_t = false)]
        no_default_features: bool,
    },

    /// Build docs for bootmgr-rs crate
    Doc {
        /// Document private items in crate
        #[arg(short, long, default_value_t = false)]
        private: bool,

        /// Open in web browser after documenting
        #[arg(short, long, default_value_t = false)]
        open: bool,

        /// Document the core crate, or the application
        #[arg(long, default_value_t = true)]
        lib: bool,
    },

    /// Run bootmgr-rs in VM with uefi-run
    Run {
        /// Path to the OVMF code file
        #[arg(long)]
        ovmf_code: Option<String>,

        /// Build with release profile
        #[arg(short, long, default_value_t = false)]
        release: bool,

        /// Add an additional file to the root of the image
        #[arg(long)]
        add_file: Option<String>,
    },

    /// Run unit tests and clippy on host
    Test {
        #[command(subcommand)]
        command: Option<Test>,
    },

    Fuzz {
        #[command(subcommand)]
        command: Fuzz,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Build {
            release,
            target,
            features,
            no_default_features,
        } => build::build_all_crates(release, &target, features, no_default_features)?,
        Commands::Doc { private, open, lib } => doc::doc_crate(private, open, lib)?,
        Commands::Run {
            ovmf_code,
            release,
            add_file,
        } => run::run_bootmgr(ovmf_code.as_deref(), release, add_file.as_deref())?,
        Commands::Test { command } => test::test_crate(command)?,
        Commands::Fuzz { command } => fuzz::fuzz_parsers(command)?,
    }
    Ok(())
}
