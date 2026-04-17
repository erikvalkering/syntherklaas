#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PianoKey {
    C3,
    CSharp3,
    D3,
    DSharp3,
    E3,
    F3,
    FSharp3,
    G3,
    GSharp3,
    A3,
    ASharp3,
    B3,
    C4,
    CSharp4,
    D4,
    DSharp4,
    E4,
    F4,
    FSharp4,
    G4,
    GSharp4,
    A4,
    ASharp4,
    B4,
    C5,
    CSharp5,
    D5,
    DSharp5,
    E5,
    F5,
    FSharp5,
    G5,
}

impl PianoKey {
    /// Calculate the frequency in Hz for this piano key.
    /// Uses A4 = 440 Hz as the reference.
    pub fn frequency(&self) -> f32 {
        let semitones_from_a4 = match self {
            PianoKey::C3 => -39.0,
            PianoKey::CSharp3 => -38.0,
            PianoKey::D3 => -37.0,
            PianoKey::DSharp3 => -36.0,
            PianoKey::E3 => -35.0,
            PianoKey::F3 => -34.0,
            PianoKey::FSharp3 => -33.0,
            PianoKey::G3 => -32.0,
            PianoKey::GSharp3 => -31.0,
            PianoKey::A3 => -30.0,
            PianoKey::ASharp3 => -29.0,
            PianoKey::B3 => -28.0,

            PianoKey::C4 => -27.0,
            PianoKey::CSharp4 => -26.0,
            PianoKey::D4 => -25.0,
            PianoKey::DSharp4 => -24.0,
            PianoKey::E4 => -23.0,
            PianoKey::F4 => -22.0,
            PianoKey::FSharp4 => -21.0,
            PianoKey::G4 => -20.0,
            PianoKey::GSharp4 => -19.0,
            PianoKey::A4 => 0.0,
            PianoKey::ASharp4 => 1.0,
            PianoKey::B4 => 2.0,

            PianoKey::C5 => 3.0,
            PianoKey::CSharp5 => 4.0,
            PianoKey::D5 => 5.0,
            PianoKey::DSharp5 => 6.0,
            PianoKey::E5 => 7.0,
            PianoKey::F5 => 8.0,
            PianoKey::FSharp5 => 9.0,
            PianoKey::G5 => 10.0,
        };

        // Equal temperament: f = 440 * 2^(n/12)
        440.0 * 2.0_f32.powf(semitones_from_a4 / 12.0)
    }

    /// Get the name of this note for display
    pub fn name(&self) -> &'static str {
        match self {
            PianoKey::C3 => "C3",
            PianoKey::CSharp3 => "C#3",
            PianoKey::D3 => "D3",
            PianoKey::DSharp3 => "D#3",
            PianoKey::E3 => "E3",
            PianoKey::F3 => "F3",
            PianoKey::FSharp3 => "F#3",
            PianoKey::G3 => "G3",
            PianoKey::GSharp3 => "G#3",
            PianoKey::A3 => "A3",
            PianoKey::ASharp3 => "A#3",
            PianoKey::B3 => "B3",

            PianoKey::C4 => "C4",
            PianoKey::CSharp4 => "C#4",
            PianoKey::D4 => "D4",
            PianoKey::DSharp4 => "D#4",
            PianoKey::E4 => "E4",
            PianoKey::F4 => "F4",
            PianoKey::FSharp4 => "F#4",
            PianoKey::G4 => "G4",
            PianoKey::GSharp4 => "G#4",
            PianoKey::A4 => "A4",
            PianoKey::ASharp4 => "A#4",
            PianoKey::B4 => "B4",

            PianoKey::C5 => "C5",
            PianoKey::CSharp5 => "C#5",
            PianoKey::D5 => "D5",
            PianoKey::DSharp5 => "D#5",
            PianoKey::E5 => "E5",
            PianoKey::F5 => "F5",
            PianoKey::FSharp5 => "F#5",
            PianoKey::G5 => "G5",
        }
    }

    /// Returns true if this is a black key (sharp/flat)
    pub fn is_black(&self) -> bool {
        match self {
            PianoKey::CSharp3
            | PianoKey::DSharp3
            | PianoKey::FSharp3
            | PianoKey::GSharp3
            | PianoKey::ASharp3
            | PianoKey::CSharp4
            | PianoKey::DSharp4
            | PianoKey::FSharp4
            | PianoKey::GSharp4
            | PianoKey::ASharp4
            | PianoKey::CSharp5
            | PianoKey::DSharp5
            | PianoKey::FSharp5 => true,
            _ => false,
        }
    }
}

/// Get all piano keys in order
pub fn all_keys() -> [PianoKey; 32] {
    [
        PianoKey::C3,
        PianoKey::CSharp3,
        PianoKey::D3,
        PianoKey::DSharp3,
        PianoKey::E3,
        PianoKey::F3,
        PianoKey::FSharp3,
        PianoKey::G3,
        PianoKey::GSharp3,
        PianoKey::A3,
        PianoKey::ASharp3,
        PianoKey::B3,
        PianoKey::C4,
        PianoKey::CSharp4,
        PianoKey::D4,
        PianoKey::DSharp4,
        PianoKey::E4,
        PianoKey::F4,
        PianoKey::FSharp4,
        PianoKey::G4,
        PianoKey::GSharp4,
        PianoKey::A4,
        PianoKey::ASharp4,
        PianoKey::B4,
        PianoKey::C5,
        PianoKey::CSharp5,
        PianoKey::D5,
        PianoKey::DSharp5,
        PianoKey::E5,
        PianoKey::F5,
        PianoKey::FSharp5,
        PianoKey::G5,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a4_frequency() {
        assert!((PianoKey::A4.frequency() - 440.0).abs() < 0.1);
    }

    #[test]
    fn test_octave_doubling() {
        let c4_freq = PianoKey::C4.frequency();
        let c5_freq = PianoKey::C5.frequency();
        assert!((c5_freq / c4_freq - 2.0).abs() < 0.01);
    }
}
