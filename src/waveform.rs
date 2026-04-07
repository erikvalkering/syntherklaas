use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveShape {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl WaveShape {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sine" => Some(WaveShape::Sine),
            "square" => Some(WaveShape::Square),
            "triangle" => Some(WaveShape::Triangle),
            "sawtooth" => Some(WaveShape::Sawtooth),
            _ => None,
        }
    }
}

pub struct Oscillator {
    pub sample_rate: f32,
    pub frequency: f32,
    pub phase: f32,
}

impl Oscillator {
    pub fn new(sample_rate: f32, frequency: f32) -> Self {
        Oscillator {
            sample_rate,
            frequency,
            phase: 0.0,
        }
    }

    pub fn next_sample(&mut self, shape: WaveShape) -> f32 {
        let sample = match shape {
            WaveShape::Sine => self.sine(),
            WaveShape::Square => self.square(),
            WaveShape::Triangle => self.triangle(),
            WaveShape::Sawtooth => self.sawtooth(),
        };

        self.phase += (self.frequency / self.sample_rate) * 2.0 * PI;
        if self.phase > 2.0 * PI {
            self.phase -= 2.0 * PI;
        }

        sample
    }

    fn sine(&self) -> f32 {
        self.phase.sin()
    }

    fn square(&self) -> f32 {
        if self.phase < PI {
            1.0
        } else {
            -1.0
        }
    }

    fn triangle(&self) -> f32 {
        let normalized = self.phase / (2.0 * PI);
        if normalized < 0.25 {
            4.0 * normalized
        } else if normalized < 0.75 {
            2.0 - 4.0 * normalized
        } else {
            4.0 * normalized - 4.0
        }
    }

    fn sawtooth(&self) -> f32 {
        2.0 * (self.phase / (2.0 * PI) - (self.phase / (2.0 * PI) + 0.5).floor())
    }
}
