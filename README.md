# syntherklaas

Synthesizer packed with presents

## Building

```bash
cargo build --release
```

## Running

The synthesizer now runs in an interactive TUI (Text User Interface) where you can:
- Adjust frequency (20-20000 Hz)
- Adjust volume (0-100%)
- Select waveform (sine, square, triangle, sawtooth)
- Press and hold the Play button to produce sound

```bash
cargo run --release
```

### Using a Specific Audio Backend

By default, the synthesizer tries cpal first, then falls back to PulseAudio if needed:

```bash
cargo run -- --backend cpal      # Use cpal only
cargo run -- --backend pulse     # Use PulseAudio only
```

**Note on Termux**: cpal panics on Termux due to lack of Android NDK context initialization. Use `--backend pulse` for immediate audio playback, or omit the flag to auto-fallback.

### Verbose Mode

Show backend detection details for debugging:

```bash
cargo run -- --verbose
```

### Audio Backend

Syntherklaas supports two audio backends:

**cpal** (primary, cross-platform)
- Uses cpal for audio output on systems where it works

**PulseAudio** (fallback)
- Uses pacat (PulseAudio client) for low-latency streaming
- Automatically used as fallback if cpal fails or panics

#### Audio Format

- **Sample rate**: 48000 Hz
- **Format**: Signed 16-bit little-endian PCM (s16le)
- **Channels**: Mono (1 channel)

## Waveforms

- `sine` - Smooth sine wave
- `square` - Classic square wave
- `triangle` - Triangular wave
- `sawtooth` - Sawtooth wave

