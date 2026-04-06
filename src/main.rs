mod waveform;
mod audio;

use clap::Parser;
use waveform::WaveShape;
use audio::AudioPlayer;
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(name = "syntherklaas")]
#[command(about = "A simple synthesizer for generating sounds", long_about = None)]
struct Args {
    /// Waveform shape: sine, square, triangle, sawtooth
    #[arg(short, long, default_value = "sine")]
    shape: String,

    /// Frequency in Hz (20-20000)
    #[arg(short, long, default_value = "440")]
    frequency: f32,

    /// Volume (0.0-1.0)
    #[arg(short, long, default_value = "0.5")]
    volume: f32,

    /// Duration in seconds
    #[arg(short, long, default_value = "2")]
    duration: f32,

    /// Interactive mode (prompt for values)
    #[arg(short, long)]
    interactive: bool,

    /// Stream raw audio to stdout (for piping to pacat)
    #[arg(long)]
    stream: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Args::parse();

    if args.interactive {
        args = get_interactive_input()?;
    }

    let shape = WaveShape::from_str(&args.shape)
        .ok_or(format!("Invalid shape '{}'. Use: sine, square, triangle, sawtooth", args.shape))?;

    if args.frequency < 20.0 || args.frequency > 20000.0 {
        return Err("Frequency must be between 20 and 20000 Hz".into());
    }

    if args.volume < 0.0 || args.volume > 1.0 {
        return Err("Volume must be between 0.0 and 1.0".into());
    }

    if args.stream {
        let player = AudioPlayer::new(args.frequency, args.volume, shape, args.duration);
        player.stream_audio()?;
    } else {
        eprintln!(
            "Playing {} Hz {} wave at {:.0}% volume for {:.1} seconds...",
            args.frequency, args.shape, args.volume * 100.0, args.duration
        );

        let player = AudioPlayer::new(args.frequency, args.volume, shape, args.duration);
        player.play()?;

        eprintln!("Done!");
    }

    Ok(())
}

fn get_interactive_input() -> Result<Args, Box<dyn std::error::Error>> {
    let mut shape = String::new();
    let mut frequency = String::new();
    let mut volume = String::new();
    let mut duration = String::new();

    print!("Enter waveform shape (sine/square/triangle/sawtooth) [sine]: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut shape)?;
    if shape.trim().is_empty() {
        shape = "sine".to_string();
    }

    print!("Enter frequency in Hz [440]: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut frequency)?;
    if frequency.trim().is_empty() {
        frequency = "440".to_string();
    }

    print!("Enter volume (0.0-1.0) [0.5]: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut volume)?;
    if volume.trim().is_empty() {
        volume = "0.5".to_string();
    }

    print!("Enter duration in seconds [2]: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut duration)?;
    if duration.trim().is_empty() {
        duration = "2".to_string();
    }

    Ok(Args {
        shape: shape.trim().to_string(),
        frequency: frequency.trim().parse()?,
        volume: volume.trim().parse()?,
        duration: duration.trim().parse()?,
        interactive: false,
        stream: false,
    })
}
