use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::panic;

use crate::waveform::{Oscillator, WaveShape};

pub struct AudioPlayer {
    frequency: f32,
    volume: f32,
    shape: WaveShape,
    duration: f32,
}

impl AudioPlayer {
    pub fn new(frequency: f32, volume: f32, shape: WaveShape, duration: f32) -> Self {
        AudioPlayer {
            frequency,
            volume,
            shape,
            duration,
        }
    }

    pub fn play(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Try to play through audio device, catching any panics from NDK/Android context
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.play_device()
        }));

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                eprintln!("Warning: Could not initialize audio device: {}", e);
                eprintln!("Generating WAV file instead...\n");
                self.play_to_file("output.wav")
            }
            Err(_) => {
                eprintln!("Warning: Audio device initialization failed");
                eprintln!("Generating WAV file instead...\n");
                self.play_to_file("output.wav")
            }
        }
    }

    fn play_device(&self) -> Result<(), Box<dyn std::error::Error>> {
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

    fn play_to_file(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate = 44100u32;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(filename, spec)?;
        let mut osc = Oscillator::new(sample_rate as f32, self.frequency);
        let total_samples = (sample_rate as f32 * self.duration) as u32;

        for _ in 0..total_samples {
            let value = osc.next_sample(self.shape);
            let sample = (value * self.volume * i16::MAX as f32) as i16;
            writer.write_sample(sample)?;
        }

        writer.finalize()?;
        println!("Generated audio file: {}", filename);
        println!("Duration: {:.1}s at {} Hz, {} wave, {:.0}% volume", 
                 self.duration, self.frequency, 
                 shape_name(self.shape), self.volume * 100.0);

        Ok(())
    }
}

fn shape_name(shape: WaveShape) -> &'static str {
    match shape {
        WaveShape::Sine => "sine",
        WaveShape::Square => "square",
        WaveShape::Triangle => "triangle",
        WaveShape::Sawtooth => "sawtooth",
    }
}
