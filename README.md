# syntherklaas

Synthesizer packed with presents

## Building

```bash
cargo build --release
```

## Running

Generate a 440 Hz sine wave for 2 seconds at 50% volume:

```bash
cargo run -- -f 440 -d 2 -v 0.5
```

### Interactive Mode

```bash
cargo run -- -i
```

You'll be prompted to choose waveform shape (sine/square/triangle/sawtooth), frequency, volume, and duration.

### Real-time Mode

```bash
cargo run -- --realtime
```

Press and hold **SPACEBAR** to continuously play the synthesizer. Release to stop. Press **Ctrl+C** to exit.

```bash
# Play a 440 Hz sine wave in realtime with PulseAudio
cargo run -- --realtime --backend pulse

# Play a 523 Hz square wave in realtime
cargo run -- -f 523 -s square --realtime
```

### Audio Backend

Syntherklaas supports two audio backends:

**cpal** (primary, cross-platform)
- Uses cpal for audio output on systems where it works

**PulseAudio** (fallback)
- Uses pacat (PulseAudio client) for low-latency streaming
- Automatically used as fallback if cpal fails or panics

#### Selecting a Backend

By default, cpal is tried first, then falls back to PulseAudio if cpal fails:

```bash
cargo run -- -f 440 -d 2  # Uses cpal if available, falls back to PulseAudio
```

Force a specific backend without fallback:

```bash
cargo run -- -f 440 -d 2 --backend cpal      # Use cpal only
cargo run -- -f 440 -d 2 --backend pulse     # Use PulseAudio only
```

**Note on Termux**: cpal panics on Termux due to lack of Android NDK context initialization. Use `--backend pulse` for immediate audio playback, or omit the flag to auto-fallback.

#### Audio Format

- **Sample rate**: 48000 Hz
- **Format**: Signed 16-bit little-endian PCM (s16le)
- **Channels**: Mono (1 channel)

## Waveforms

- `sine` - Smooth sine wave
- `square` - Classic square wave
- `triangle` - Triangular wave
- `sawtooth` - Sawtooth wave

