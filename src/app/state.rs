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
    pub focused_field: FocusedField,
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
    PlayButton,
    PlayToggleButton,
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
            focused_field: FocusedField::Frequency,
            last_play_button_press: Instant::now(),
            mouse_dragging: false,
            mouse_start_x: 0,
            current_piano_key: None,
            current_octave: 4,
            semitone_offset: 0,
        }
    }
}

impl Default for SynthState {
    fn default() -> Self {
        Self::new()
    }
}
