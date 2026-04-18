use crate::music::PianoKey;
use crate::waveform::WaveShape;
use crossterm::event::MouseEvent;

#[derive(Debug, Clone)]
pub enum Message {
    // Frequency adjustments
    IncreaseFrequency,
    DecreaseFrequency,
    SetFrequency(f32),

    // Volume adjustments
    IncreaseVolume,
    DecreaseVolume,
    SetVolume(f32),

    // Waveform selection
    SetWaveform(WaveShape),

    // Playback control
    PressPlayButton,
    ReleasePlayButton,
    TogglePlay,

    // Piano control - monophonic
    KeyboardKeyDown(Option<PianoKey>),
    KeyboardKeyUp,
    ChangeOctave(i32),
    ChangeSemitone(i32),

    // UI focus
    FocusNext,
    FocusPrev,

    // Timeout-based release detection
    CheckTimeoutRelease,

    // Mouse interactions
    MouseEvent(MouseEvent),

    // Exit signal
    Exit,
}
