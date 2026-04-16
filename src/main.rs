mod app;
mod audio;
mod keyboard;
mod music;
mod tui;
mod waveform;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "syntherklaas")]
#[command(about = "A simple synthesizer for generating sounds", long_about = None)]
struct Args {
    /// Show verbose output (backend selection, etc.)
    #[arg(long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    tui::run_tui(args.verbose)?;
    Ok(())
}
