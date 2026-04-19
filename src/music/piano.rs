#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Note {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Note {
    /// Get the semitone offset from C within an octave (0-11)
    pub fn semitone_in_octave(&self) -> i32 {
        match self {
            Note::C => 0,
            Note::D => 2,
            Note::E => 4,
            Note::F => 5,
            Note::G => 7,
            Note::A => 9,
            Note::B => 11,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Note::C => "C",
            Note::D => "D",
            Note::E => "E",
            Note::F => "F",
            Note::G => "G",
            Note::A => "A",
            Note::B => "B",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PianoKey {
    pub note: Note,
    pub sharp: bool,
    pub octave: i32,
}

impl PianoKey {
    /// Create a new piano key                
    #[cfg(test)]
    pub fn new(note: Note, sharp: bool, octave: i32) -> Self {
        PianoKey {
            note,
            sharp,
            octave,
        }
    }

    /// Calculate the frequency in Hz for this piano key.
    /// Uses A4 = 440 Hz as the reference.
    pub fn frequency(&self) -> f32 {
        // Calculate semitones from A4
        // C4 is 9 semitones below A4
        let octave_offset = (self.octave - 4) * 12;
        let note_offset = self.note.semitone_in_octave();
        let sharp_offset = if self.sharp { 1 } else { 0 };
        let semitones_from_c4 = note_offset + sharp_offset;
        let semitones_from_a4 = octave_offset + semitones_from_c4 - 9;

        // Equal temperament: f = 440 * 2^(n/12)
        440.0 * 2.0_f32.powf(semitones_from_a4 as f32 / 12.0)
    }

    /// Get the name of this note for display
    pub fn name(&self) -> String {
        if self.sharp {
            format!("{}#{}", self.note.as_str(), self.octave)
        } else {
            format!("{}{}", self.note.as_str(), self.octave)
        }
    }
}

/// Get a PianoKey for a given octave and semitone position within octave (0-11)
/// where 0=C, 1=C#, 2=D, 3=D#, 4=E, 5=F, 6=F#, 7=G, 8=G#, 9=A, 10=A#, 11=B
pub fn get_key_for_octave_and_semitone(octave: i32, semitone: i32) -> Option<PianoKey> {
    let semitone = semitone.rem_euclid(12) as usize;
    let octave = octave.clamp(0, 8);

    // Map semitone to note and sharp
    let (note, sharp) = match semitone {
        0 => (Note::C, false),
        1 => (Note::C, true),
        2 => (Note::D, false),
        3 => (Note::D, true),
        4 => (Note::E, false),
        5 => (Note::F, false),
        6 => (Note::F, true),
        7 => (Note::G, false),
        8 => (Note::G, true),
        9 => (Note::A, false),
        10 => (Note::A, true),
        11 => (Note::B, false),
        _ => return None,
    };

    // Check bounds for grand piano (A0-C8)
    if octave == 0 && semitone < 9 {
        return None; // Before A0
    }
    if octave == 8 && semitone != 0 {
        return None; // After C8
    }

    Some(PianoKey {
        note,
        sharp,
        octave,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a4_frequency() {
        let a4 = PianoKey::new(Note::A, false, 4);
        assert!((a4.frequency() - 440.0).abs() < 0.1);
    }

    #[test]
    fn test_octave_doubling() {
        let c4 = PianoKey::new(Note::C, false, 4);
        let c5 = PianoKey::new(Note::C, false, 5);
        assert!((c5.frequency() / c4.frequency() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_key_name() {
        let c_sharp = PianoKey::new(Note::C, true, 4);
        assert_eq!(c_sharp.name(), "C#4");
        let a = PianoKey::new(Note::A, false, 4);
        assert_eq!(a.name(), "A4");
    }
}
