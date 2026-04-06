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

### Audio Backend

Syntherklaas uses **PulseAudio** (via `pacat`) as the primary audio backend. This provides low-latency, real-time audio output in Termux. The app spawns a `pacat` process and streams PCM audio samples directly to it.

- **Sample rate**: 48000 Hz
- **Format**: Signed 16-bit little-endian PCM (s16le)
- **Channels**: Mono (1 channel)

**Note**: `pacat` must be installed. On Termux, install it with:

```bash
pkg install pulseaudio
```

## Waveforms

- `sine` - Smooth sine wave
- `square` - Classic square wave
- `triangle` - Triangular wave
- `sawtooth` - Sawtooth wave
