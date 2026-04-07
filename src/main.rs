mod waveform;
mod audio;
mod keyboard;

use clap::Parser;
use waveform::WaveShape;
use audio::{AudioPlayer, AudioBackend};
use keyboard::KeyboardHandler;
use std::io::{self, Write};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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

    /// Realtime mode (hold spacebar to play, Ctrl+C to exit)
    #[arg(long)]
    realtime: bool,

    /// Audio backend: cpal or pulse (default: auto-fallback from cpal to pulse)
    #[arg(long)]
    backend: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Args::parse();

    if args.interactive && args.realtime {
        return Err("Cannot use both -i and --realtime flags".into());
    }

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

    let mut player = AudioPlayer::new(args.frequency, args.volume, shape, args.duration);

    if let Some(backend_str) = args.backend {
        let backend = match backend_str.to_lowercase().as_str() {
            "cpal" => AudioBackend::Cpal,
            "pulse" => AudioBackend::PulseAudio,
            _ => return Err(format!("Unknown backend '{}'. Use: cpal, pulse", backend_str).into()),
        };
        player = player.with_backend(backend);
    }

    if args.realtime {
        run_realtime_mode(&player)?;
    } else {
        eprintln!(
            "Playing {} Hz {} wave at {:.0}% volume for {:.1} seconds...",
            args.frequency, args.shape, args.volume * 100.0, args.duration
        );
        player.play()?;
        eprintln!("Done!");
    }

    Ok(())
}

fn run_realtime_mode(player: &AudioPlayer) -> Result<(), Box<dyn std::error::Error>> {
    let mut kb = KeyboardHandler::new();
    kb.start()?;

    let spacebar = kb.spacebar_pressed();
    let should_exit = kb.should_exit();

    // Give keyboard handler time to initialize
    thread::sleep(Duration::from_millis(100));

    // Spawn audio thread
    let frequency = player.frequency;
    let volume = player.volume;
    let shape = player.shape;
    let duration = player.duration;
    let backend = player.backend;

    let spacebar_audio = Arc::clone(&spacebar);

    let audio_thread = thread::spawn(move || {
        let player = AudioPlayer::new(frequency, volume, shape, duration);
        let player = if let Some(b) = backend {
            player.with_backend(b)
        } else {
            player
        };

        // Try realtime playback with fallback
        let result = if let Some(AudioBackend::PulseAudio) = backend {
            player.play_realtime_pulseaudio(spacebar_audio.clone())
        } else if let Some(AudioBackend::Cpal) = backend {
            player.play_realtime_cpal(spacebar_audio.clone())
        } else {
            // Auto-detect: try cpal first
            use std::panic;
            match panic::catch_unwind(panic::AssertUnwindSafe(|| {
                player.play_realtime_cpal(spacebar_audio.clone())
            })) {
                Ok(Ok(())) => Ok(()),
                Ok(Err(_)) | Err(_) => {
                    eprintln!("Switching to PulseAudio...");
                    player.play_realtime_pulseaudio(spacebar_audio)
                }
            }
        };

        if let Err(e) = result {
            eprintln!("Audio error: {}", e);
        }
    });

    // Wait for exit signal
    while !should_exit.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));
    }

    eprintln!("\nExiting...");
    let _ = kb.wait_and_cleanup();
    let _ = audio_thread.join();

    Ok(())
}

fn get_interactive_input() -> Result<Args, Box<dyn std::error::Error>> {
    let mut shape = String::new();
    let mut frequency = String::new();
    let mut volume = String::new();
    let mut duration = String::new();

    eprint!("Enter waveform shape (sine/square/triangle/sawtooth) [sine]: ");
    io::stderr().flush()?;
    io::stdin().read_line(&mut shape)?;
    if shape.trim().is_empty() {
        shape = "sine".to_string();
    }

    eprint!("Enter frequency in Hz [440]: ");
    io::stderr().flush()?;
    io::stdin().read_line(&mut frequency)?;
    if frequency.trim().is_empty() {
        frequency = "440".to_string();
    }

    eprint!("Enter volume (0.0-1.0) [0.5]: ");
    io::stderr().flush()?;
    io::stdin().read_line(&mut volume)?;
    if volume.trim().is_empty() {
        volume = "0.5".to_string();
    }

    eprint!("Enter duration in seconds [2]: ");
    io::stderr().flush()?;
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
        realtime: false,
        backend: None,
    })
}
