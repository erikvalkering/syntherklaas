use crate::waveform::{Oscillator, WaveShape};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioBackend {
    Cpal,

    #[cfg(target_os = "android")]
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

        let stream = device.build_output_stream(
            &config.config(),
            callback,
            |err| {
                eprintln!("Stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;

        // Keep stream alive until exit signal
        while !should_exit.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
        }

        Ok(())
    }

    #[cfg(target_os = "android")]
    pub fn play_realtime_pulseaudio(
        &self,
        should_play: Arc<AtomicBool>,
        should_exit: Arc<AtomicBool>,
        freq_param: Option<Arc<Mutex<f32>>>,
        vol_param: Option<Arc<Mutex<f32>>>,
        shape_param: Option<Arc<Mutex<crate::waveform::WaveShape>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use libpulse_binding::sample::{Format, Spec};
        use libpulse_binding::stream::Direction;
        use libpulse_simple_binding::Simple;

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
            None,
        )?;

        let chunk_size = sample_rate as usize / 10; // 100ms chunks
        let mut buffer = vec![0i16; chunk_size];

        while !should_exit.load(Ordering::Relaxed) {
            // Read current parameters
            let freq = freq_param
                .as_ref()
                .map(|f| *f.lock().unwrap())
                .unwrap_or(self.frequency);
            let vol = vol_param
                .as_ref()
                .map(|v| *v.lock().unwrap())
                .unwrap_or(self.volume);
            let shape = shape_param
                .as_ref()
                .map(|s| *s.lock().unwrap())
                .unwrap_or(self.shape);

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

            let bytes: Vec<u8> = buffer
                .iter()
                .flat_map(|s| s.to_le_bytes().to_vec())
                .collect();
            s.write(&bytes)?;
        }

        Ok(())
    }
}
