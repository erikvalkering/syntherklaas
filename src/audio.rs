use crate::waveform::{Oscillator, WaveShape};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioBackend {
    Cpal,
    PulseAudio,
}

pub struct AudioPlayer {
    frequency: f32,
    volume: f32,
    shape: WaveShape,
    duration: f32,
    backend: Option<AudioBackend>,
}

impl AudioPlayer {
    pub fn new(frequency: f32, volume: f32, shape: WaveShape, duration: f32) -> Self {
        AudioPlayer {
            frequency,
            volume,
            shape,
            duration,
            backend: None,
        }
    }

    pub fn with_backend(mut self, backend: AudioBackend) -> Self {
        self.backend = Some(backend);
        self
    }

    pub fn play(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self.backend {
            Some(AudioBackend::Cpal) => self.play_cpal(),
            Some(AudioBackend::PulseAudio) => self.play_pulseaudio(),
            None => {
                use std::panic;
                match panic::catch_unwind(panic::AssertUnwindSafe(|| self.play_cpal())) {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(cpal_err)) => {
                        eprintln!("cpal unavailable: {}", cpal_err);
                        eprintln!("Falling back to PulseAudio...");
                        self.play_pulseaudio()
                    }
                    Err(_) => {
                        eprintln!("cpal panicked");
                        eprintln!("Falling back to PulseAudio...");
                        self.play_pulseaudio()
                    }
                }
            }
        }
    }

    fn play_cpal(&self) -> Result<(), Box<dyn std::error::Error>> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device found")?;

        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate() as f32;

        let frequency = Arc::new(Mutex::new(self.frequency));
        let volume = self.volume;
        let shape = self.shape;
        let duration = self.duration;
        let total_samples = (sample_rate * duration) as u32;
        let sample_count = Arc::new(Mutex::new(0u32));

        let freq_clone = Arc::clone(&frequency);
        let sample_count_clone = Arc::clone(&sample_count);

        let callback = move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let freq = *freq_clone.lock().unwrap();
            let mut osc = Oscillator::new(sample_rate, freq);
            let mut count = sample_count_clone.lock().unwrap();

            for sample in output.iter_mut() {
                if *count >= total_samples {
                    *sample = 0.0;
                } else {
                    let value = osc.next_sample(shape);
                    *sample = value * volume;
                    *count += 1;
                }
            }
        };

        let stream = device.build_output_stream(&config.config(), callback, |err| {
            eprintln!("Stream error: {}", err);
        }, None)?;

        stream.play()?;
        std::thread::sleep(Duration::from_secs_f32(duration + 0.1));

        Ok(())
    }

    fn play_pulseaudio(&self) -> Result<(), Box<dyn std::error::Error>> {
        use libpulse_simple_binding::Simple;
        use libpulse_binding::stream::Direction;
        use libpulse_binding::sample::{Spec, Format};

        let sample_rate = 48000u32;
        let mut osc = Oscillator::new(sample_rate as f32, self.frequency);
        let total_samples = (sample_rate as f32 * self.duration) as u32;

        let spec = Spec {
            format: Format::S16le,
            channels: 1,
            rate: sample_rate,
        };

        let s = Simple::new(
            None,                    // Use the default server
            "syntherklaas",          // Application name
            Direction::Playback,     // Playback stream
            None,                    // Use the default device
            "tone",                  // Stream description
            &spec,                   // Sample format
            None,                    // Use default channel map
            None                     // Use default buffering attributes
        )?;

        // Generate and write samples
        let mut buffer = Vec::with_capacity((total_samples as usize) * 2);
        for _ in 0..total_samples {
            let value = osc.next_sample(self.shape);
            let sample = (value * self.volume * i16::MAX as f32) as i16;
            buffer.extend_from_slice(&sample.to_le_bytes());
        }

        s.write(&buffer)?;
        s.drain()?;

        Ok(())
    }
}
