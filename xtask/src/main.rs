use clap::{Parser, Subcommand};

use crate::test::Test;

mod build;
mod doc;
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
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Build { release, target } => build::build_all_crates(release, &target)?,
        Commands::Doc { private, open, lib } => doc::doc_crate(private, open, lib)?,
        Commands::Run {
            ovmf_code,
            release,
            add_file,
        } => run::run_bootmgr(ovmf_code.as_deref(), release, add_file.as_deref())?,
        Commands::Test { command } => test::test_crate(command)?,
    }
    Ok(())
}
