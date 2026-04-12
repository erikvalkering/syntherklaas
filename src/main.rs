mod audio;
mod keyboard;
mod tui;
mod waveform;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "syntherklaas")]
#[command(about = "A simple synthesizer for generating sounds", long_about = None)]
struct Args {
    /// Audio backend: cpal or pulse (default: auto-fallback from cpal to pulse)
    #[arg(long)]
    backend: Option<String>,

    /// Show verbose output (backend selection, etc.)
    #[arg(long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let backend = if let Some(backend_str) = args.backend {
        match backend_str.to_lowercase().as_str() {
            "cpal" => Some(audio::AudioBackend::Cpal),

            #[cfg(target_os = "android")]
            "pulse" => Some(audio::AudioBackend::PulseAudio),

            _ => return Err(format!("Unknown backend '{}'. Use: cpal, pulse", backend_str).into()),
        }
    } else {
        None
    };

    tui::run_tui(backend, args.verbose)?;
    Ok(())
}
