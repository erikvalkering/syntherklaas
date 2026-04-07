# Copilot Instructions for syntherklaas

## Build, Test & Run

### Building
```bash
cargo build --release
```

### Running
All commands use `cargo run --`:

**Basic tone generation** (440 Hz sine wave, 2 seconds, 50% volume):
```bash
cargo run -- -f 440 -d 2 -v 0.5
```

**Interactive mode** (prompts for waveform, frequency, volume, duration):
```bash
cargo run -- -i
```

**Real-time mode** (hold SPACEBAR to play, Ctrl+C to exit):
```bash
cargo run -- --realtime
```

**Verbose output** (show backend detection details):
```bash
cargo run -- --realtime --verbose
```

**Testing**
No automated tests exist. Manual testing involves running the application with various parameters and verifying audio output.

## Architecture Overview

Syntherklaas is a CLI synthesizer with four main modules:

**1. Oscillator & Waveforms** (`src/waveform.rs`)
- `Oscillator` struct generates samples via phase accumulation
- `WaveShape` enum: Sine, Square, Triangle, Sawtooth
- Phase wrapping at 2π ensures stability
- All waveforms output normalized samples in [-1.0, 1.0]

**2. Audio Backend System** (`src/audio.rs`)
- `AudioPlayer` struct holds synthesis parameters (frequency, volume, shape, duration)
- Two backends: **cpal** (cross-platform, primary) and **PulseAudio** (fallback)
- Auto-fallback: cpal is tried first; if it panics or fails, PulseAudio is used
- Can be forced via `--backend` flag
- Fixed audio config: 48 kHz, mono, signed 16-bit PCM
- Each backend spawns threads to fill sample buffers in real-time

**3. Keyboard Handler** (`src/keyboard.rs`)
- Uses `crossterm` for terminal input handling
- Detects SPACEBAR and Ctrl+C for real-time mode
- Uses atomic booleans to signal audio thread state changes
- Manages alternate screen and raw mode setup/teardown

**4. CLI Entry Point** (`src/main.rs`)
- Uses `clap` (derive API) to parse command-line arguments
- Routes to interactive, real-time, or standard modes
- Validates parameters (frequency range 20-20000 Hz, volume 0.0-1.0)

## Key Conventions

### Audio Generation Pipeline
- Phase accumulation formula: `phase += (frequency / sample_rate) * 2π`
- Phase wrapping prevents floating-point drift
- All calculations use `f32` for efficiency

### Real-time Mode Threading Model
- Main thread: CLI parsing and mode routing
- Keyboard thread: monitoring input (started in `KeyboardHandler::start()`)
- Audio thread: sample generation and playback (spawned per backend)
- Atomic booleans for inter-thread state (spacebar pressed, exit signal)

### Backend Fallback Strategy
- Default behavior (no `--backend` flag): Try cpal, catch panics/errors, fallback to PulseAudio
- Explicit backend: `--backend cpal` or `--backend pulse` skips fallback logic
- PulseAudio output via `pacat` utility; cpal uses system audio device

### Error Handling
- Functions return `Result<(), Box<dyn std::error::Error>>`
- Audio errors print to stderr with context before attempting fallback
- CLI parsing errors handled by clap automatically

## Terminux-Specific Notes

- cpal panics on Termux (no Android NDK context)
- Use `--backend pulse` for reliable audio on Termux
- PulseAudio must be running (`pulseaudio --start` if needed)

## Known Issues & Fixes

**Real-time mode exit (FIXED)**
- **Issue**: Ctrl+C could appear to hang if audio thread wasn't properly notified of exit signal
- **Root cause**: Audio playback functions (`play_realtime_cpal`, `play_realtime_pulseaudio`) weren't receiving the `should_exit` signal, causing them to wait indefinitely for the spacebar state rather than checking for exit
- **Fix**: Both realtime functions now accept both `should_play` (spacebar state) and `should_exit` (Ctrl+C signal) parameters, allowing clean shutdown when either backend is used

**Spacebar release detection (FIXED)**
- **Issue**: On Termux, key release events aren't reliably reported, so holding spacebar would continue playing even after physically releasing the key
- **Root cause**: Crossterm depends on OS-level key event reporting; Termux doesn't always report `KeyEventKind::Release` events
- **Fix**: Implemented timeout-based release detection in `KeyboardHandler`: if no new press event arrives within 100ms, assume the key was released. This maintains responsiveness while working around the Termux limitation.

**Verbose auto-detection (FIXED)**
- **Issue**: Backend auto-detection was overly verbose with panic stack traces and fallback messages cluttering output
- **Root cause**: `AudioPlayer` unconditionally printed fallback messages when cpal failed and it switched to PulseAudio. Additionally, Rust's panic hook prints panic details to stderr even when caught by `catch_unwind`.
- **Fix**: Added `--verbose` flag to control backend detection output. By default, auto-detection silently picks a working backend. When verbose is off, a custom panic hook suppresses panic output. With `--verbose`, it shows all backend selection details including panic traces, useful for debugging setup issues.

## MCP Servers

**Recommended MCP servers for this project:**

- **bash**: Run cargo commands, manage processes, test audio backends
- **filesystem**: Handle file operations, inspect Cargo.lock and build artifacts
