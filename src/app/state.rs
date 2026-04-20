use super::focus::FocusPosition;
use crate::music::PianoKey;
use crate::waveform::WaveShape;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct SynthState {
    pub frequency: f32,
    pub volume: f32,
    pub shape: WaveShape,
    pub is_playing: bool,
    pub keep_playing: bool,
    pub should_exit: bool,
    pub focus: FocusPosition,
    pub last_play_button_press: Instant,
    pub mouse_dragging: bool,
    pub mouse_start_x: u16,
    // Monophonic piano: track single key and octave/semitone offsets
    pub current_piano_key: Option<PianoKey>,
    pub current_octave: i32,
    pub semitone_offset: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedField {
    Frequency,
    Volume,
    Shape,
}

impl SynthState {
    pub fn new() -> Self {
        SynthState {
            frequency: 440.0,
            volume: 0.5,
            shape: WaveShape::Sine,
            is_playing: false,
            keep_playing: false,
            should_exit: false,
            focus: FocusPosition::new(0, 0),
            last_play_button_press: Instant::now(),
            mouse_dragging: false,
            mouse_start_x: 0,
            current_piano_key: None,
            current_octave: 4,
            semitone_offset: 0,
        }
    }

    /// Get the current focused field as the old enum for backward compatibility
    pub fn focused_field(&self) -> FocusedField {
        match self.focus.row {
            0 => FocusedField::Frequency,
            1 => FocusedField::Volume,
            _ => FocusedField::Shape,
        }
    }
}

impl Default for SynthState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synth_state_default_focus() {
        let state = SynthState::new();
        assert_eq!(state.focus.row, 0);
        assert_eq!(state.focus.col, 0);
    }

    #[test]
    fn test_focused_field_frequency() {
        let state = SynthState::new();
        assert_eq!(state.focused_field(), FocusedField::Frequency);
    }

    #[test]
    fn test_focused_field_volume() {
        let mut state = SynthState::new();
        state.focus = FocusPosition::new(1, 0);
        assert_eq!(state.focused_field(), FocusedField::Volume);
    }

    #[test]
    fn test_focused_field_shape() {
        let mut state = SynthState::new();
        state.focus = FocusPosition::new(2, 0);
        assert_eq!(state.focused_field(), FocusedField::Shape);
    }

    #[test]
    fn test_synth_state_initialization() {
        let state = SynthState::new();
        assert_eq!(state.frequency, 440.0);
        assert_eq!(state.volume, 0.5);
        assert_eq!(state.shape, WaveShape::Sine);
        assert!(!state.is_playing);
        assert!(!state.should_exit);
    }
}
