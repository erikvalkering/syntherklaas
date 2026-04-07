use crate::waveform::{Oscillator, WaveShape};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioBackend {
    Cpal,
    PulseAudio,
}

pub struct AudioPlayer {
    pub frequency: f32,
    pub volume: f32,
    pub shape: WaveShape,
    #[allow(dead_code)]
    pub duration: f32,
    pub backend: Option<AudioBackend>,
    pub verbose: bool,
}

impl AudioPlayer {
    pub fn new(frequency: f32, volume: f32, shape: WaveShape, duration: f32) -> Self {
        AudioPlayer {
            frequency,
            volume,
            shape,
            duration,
            backend: None,
            verbose: false,
        }
    }

    pub fn with_backend(mut self, backend: AudioBackend) -> Self {
        self.backend = Some(backend);
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    #[allow(dead_code)]
    pub fn play(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self.backend {
            Some(AudioBackend::Cpal) => self.play_cpal(),
            Some(AudioBackend::PulseAudio) => self.play_pulseaudio(),
            None => {
                use std::panic;
                
                // Suppress panic output unless verbose
                if !self.verbose {
                    std::panic::set_hook(Box::new(|_| {}));
                }
                
                let result = match panic::catch_unwind(panic::AssertUnwindSafe(|| self.play_cpal())) {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(cpal_err)) => {
                        if self.verbose {
                            eprintln!("cpal unavailable: {}", cpal_err);
                            eprintln!("Falling back to PulseAudio...");
                        }
                        self.play_pulseaudio()
                    }
                    Err(_) => {
                        if self.verbose {
                            eprintln!("cpal panicked");
                            eprintln!("Falling back to PulseAudio...");
                        }
                        self.play_pulseaudio()
                    }
                };
                
                // Restore default panic hook
                let _ = std::panic::take_hook();
                result
            }
        }
    }

    pub fn play_realtime_cpal(
        &self,
        should_play: Arc<AtomicBool>,
        should_exit: Arc<AtomicBool>,
        freq_param: Option<Arc<Mutex<f32>>>,
        vol_param: Option<Arc<Mutex<f32>>>,
        shape_param: Option<Arc<Mutex<crate::waveform::WaveShape>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device found")?;

        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate() as f32;

        // Use provided parameters or fall back to self parameters
        let frequency = freq_param.unwrap_or_else(|| Arc::new(Mutex::new(self.frequency)));
        let volume = vol_param.unwrap_or_else(|| Arc::new(Mutex::new(self.volume)));
        let shape = shape_param.unwrap_or_else(|| Arc::new(Mutex::new(self.shape)));
        let oscillator = Arc::new(Mutex::new(Oscillator::new(sample_rate, self.frequency)));

        let freq_clone = Arc::clone(&frequency);
        let vol_clone = Arc::clone(&volume);
        let shape_clone = Arc::clone(&shape);
        let osc_clone = Arc::clone(&oscillator);
        let should_play_clone = Arc::clone(&should_play);

        let callback = move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let freq = *freq_clone.lock().unwrap();
            let vol = *vol_clone.lock().unwrap();
            let current_shape = *shape_clone.lock().unwrap();
            let mut osc = osc_clone.lock().unwrap();
            
            if osc.frequency != freq {
                osc.frequency = freq;
            }
            
            let playing = should_play_clone.load(Ordering::Relaxed);

            for sample in output.iter_mut() {
                if playing {
                    let value = osc.next_sample(current_shape);
                    *sample = value * vol;
                } else {
                    *sample = 0.0;
                }
            }
        };

        let stream = device.build_output_stream(&config.config(), callback, |err| {
            eprintln!("Stream error: {}", err);
        }, None)?;

        stream.play()?;
        
        // Keep stream alive until exit signal
        while !should_exit.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
        }

        Ok(())
    }

    pub fn play_realtime_pulseaudio(
        &self,
        should_play: Arc<AtomicBool>,
        should_exit: Arc<AtomicBool>,
        freq_param: Option<Arc<Mutex<f32>>>,
        vol_param: Option<Arc<Mutex<f32>>>,
        shape_param: Option<Arc<Mutex<crate::waveform::WaveShape>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use libpulse_simple_binding::Simple;
        use libpulse_binding::stream::Direction;
        use libpulse_binding::sample::{Spec, Format};

        let sample_rate = 48000u32;
        let mut osc = Oscillator::new(sample_rate as f32, self.frequency);

        let spec = Spec {
            format: Format::S16le,
            channels: 1,
            rate: sample_rate,
        };

        let s = Simple::new(
            None,
            "syntherklaas",
            Direction::Playback,
            None,
            "tone",
            &spec,
            None,
            None
        )?;

        let chunk_size = sample_rate as usize / 10; // 100ms chunks
        let mut buffer = vec![0i16; chunk_size];

        while !should_exit.load(Ordering::Relaxed) {
            // Read current parameters
            let freq = freq_param.as_ref().map(|f| *f.lock().unwrap()).unwrap_or(self.frequency);
            let vol = vol_param.as_ref().map(|v| *v.lock().unwrap()).unwrap_or(self.volume);
            let shape = shape_param.as_ref().map(|s| *s.lock().unwrap()).unwrap_or(self.shape);

            // Update oscillator frequency if changed
            if osc.frequency != freq {
                osc.frequency = freq;
            }

            for sample in buffer.iter_mut() {
                if should_play.load(Ordering::Relaxed) {
                    let value = osc.next_sample(shape);
                    *sample = (value * vol * i16::MAX as f32) as i16;
                } else {
                    *sample = 0i16;
                }
            }
            
            let bytes: Vec<u8> = buffer.iter()
                .flat_map(|s| s.to_le_bytes().to_vec())
                .collect();
            s.write(&bytes)?;
        }

        Ok(())
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
            None,
            "syntherklaas",
            Direction::Playback,
            None,
            "tone",
            &spec,
            None,
            None
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
