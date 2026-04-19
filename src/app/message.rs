use crate::music::PianoKey;
use crate::waveform::WaveShape;
use crossterm::event::MouseEvent;

#[derive(Debug, Clone)]
pub enum Message {
    // Frequency adjustments
    IncreaseFrequency,
    DecreaseFrequency,
    #[allow(unused)]
    SetFrequency(f32),

    // Volume adjustments
    IncreaseVolume,
    DecreaseVolume,
    #[allow(unused)]
    SetVolume(f32),

    // Waveform selection
    SetWaveform(WaveShape),

    // Playback control
    #[allow(unused)]
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
