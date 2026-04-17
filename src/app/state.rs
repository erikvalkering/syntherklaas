use crate::music::PianoKey;
use crate::waveform::WaveShape;
use std::collections::HashSet;
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
    pub piano_active_keys: HashSet<PianoKey>,
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
            piano_active_keys: HashSet::new(),
        }
    }
}

impl Default for SynthState {
    fn default() -> Self {
        Self::new()
    }
}
