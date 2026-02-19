use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use context_map::{RenderConfig, RenderProfile};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProfileArg {
    Compact,
    Balanced,
    Detailed,
}

impl From<ProfileArg> for RenderProfile {
    fn from(value: ProfileArg) -> Self {
        match value {
            ProfileArg::Compact => RenderProfile::Compact,
            ProfileArg::Balanced => RenderProfile::Balanced,
            ProfileArg::Detailed => RenderProfile::Detailed,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "context-map")]
#[command(about = "Scan TS/TSX/Vue exports and write a Markdown context map")]
struct Args {
    #[arg(long, default_value = ".")]
    root: PathBuf,

    #[arg(long)]
    out: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = ProfileArg::Balanced)]
    profile: ProfileArg,

    #[arg(long, default_value_t = false)]
    no_types: bool,

    #[arg(long, default_value_t = 10)]
    tree_depth: usize,
}

fn main() {
    let args = Args::parse();
    let output = args.out.unwrap_or_else(|| args.root.join("REPO.md"));
    let profile: RenderProfile = args.profile.into();
    let config = RenderConfig {
        profile,
        include_types: !args.no_types,
        tree_depth: args.tree_depth,
    };

    match context_map::run_with_config(&args.root, &output, config) {
        Ok(summary) => {
            println!(
                "Profile={:?}, types={}, tree_depth={} -> wrote {} exported functions and {} exported types from {} scanned files to {}",
                profile,
                config.include_types,
                config.tree_depth,
                summary.exported_functions,
                summary.exported_types,
                summary.scanned,
                output.display()
            );
        }
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    }
}
