use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "context-map")]
#[command(about = "Scan TS/TSX/Vue exports and write a Markdown context map")]
struct Args {
    #[arg(long, default_value = ".")]
    root: PathBuf,

    #[arg(long)]
    out: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let output = args
        .out
        .unwrap_or_else(|| args.root.join("context-map.md"));

    match context_map::run(&args.root, &output) {
        Ok(summary) => {
            println!(
                "Wrote {} exported functions from {} scanned files to {}",
                summary.exported_functions,
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
