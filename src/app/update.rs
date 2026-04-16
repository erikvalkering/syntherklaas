use super::message::Message;
use super::state::{FocusedField, SynthState};
use crate::waveform::WaveShape;
use crossterm::event::{MouseEventKind, MouseEvent};
use std::time::Duration;

pub fn update(mut state: SynthState, msg: Message) -> SynthState {
    match msg {
        Message::IncreaseFrequency => {
            if state.focused_field == FocusedField::Frequency {
                state.frequency = (state.frequency + 10.0).min(20000.0);
            }
        }
        Message::DecreaseFrequency => {
            if state.focused_field == FocusedField::Frequency {
                state.frequency = (state.frequency - 10.0).max(20.0);
            }
        }
        Message::SetFrequency(freq) => {
            state.frequency = freq.clamp(20.0, 20000.0);
        }
        Message::IncreaseVolume => {
            if state.focused_field == FocusedField::Volume {
                state.volume = (state.volume + 0.05).min(1.0);
            }
        }
        Message::DecreaseVolume => {
            if state.focused_field == FocusedField::Volume {
                state.volume = (state.volume - 0.05).max(0.0);
            }
        }
        Message::SetVolume(vol) => {
            state.volume = vol.clamp(0.0, 1.0);
        }
        Message::NextWaveform => {
            if state.focused_field == FocusedField::Shape {
                state.shape = match state.shape {
                    WaveShape::Sine => WaveShape::Square,
                    WaveShape::Square => WaveShape::Triangle,
                    WaveShape::Triangle => WaveShape::Sawtooth,
                    WaveShape::Sawtooth => WaveShape::Sine,
                };
            }
        }
        Message::PrevWaveform => {
            if state.focused_field == FocusedField::Shape {
                state.shape = match state.shape {
                    WaveShape::Sine => WaveShape::Sawtooth,
                    WaveShape::Square => WaveShape::Sine,
                    WaveShape::Triangle => WaveShape::Square,
                    WaveShape::Sawtooth => WaveShape::Triangle,
                };
            }
        }
        Message::SetWaveform(shape) => {
            state.shape = shape;
        }
        Message::PressPlayButton => {
            if state.focused_field == FocusedField::PlayButton {
                state.is_playing = true;
                state.last_play_button_press = std::time::Instant::now();
            }
        }
        Message::ReleasePlayButton => {
            if state.focused_field == FocusedField::PlayButton {
                state.is_playing = false;
            }
        }
        Message::TogglePlay => {
            if state.focused_field == FocusedField::PlayToggleButton {
                let new_state = !state.is_playing;
                state.is_playing = new_state;
                state.keep_playing = new_state;
                state.last_play_button_press = std::time::Instant::now();
            }
        }
        Message::FocusNext => {
            state.focused_field = match state.focused_field {
                FocusedField::Frequency => FocusedField::Volume,
                FocusedField::Volume => FocusedField::Shape,
                FocusedField::Shape => FocusedField::PlayButton,
                FocusedField::PlayButton => FocusedField::PlayToggleButton,
                FocusedField::PlayToggleButton => FocusedField::Frequency,
            };
        }
        Message::FocusPrev => {
            state.focused_field = match state.focused_field {
                FocusedField::Frequency => FocusedField::PlayToggleButton,
                FocusedField::Volume => FocusedField::Frequency,
                FocusedField::Shape => FocusedField::Volume,
                FocusedField::PlayButton => FocusedField::Shape,
                FocusedField::PlayToggleButton => FocusedField::PlayButton,
            };
        }
        Message::PianoPressKey(key) => {
            state.piano_active_keys.insert(key);
            state.frequency = key.frequency();
            state.is_playing = true;
        }
        Message::PianoReleaseKey(key) => {
            state.piano_active_keys.remove(&key);
            if state.piano_active_keys.is_empty() {
                state.is_playing = false;
            } else {
                // Play the highest remaining key
                if let Some(&highest_key) = state.piano_active_keys.iter().max() {
                    state.frequency = highest_key.frequency();
                }
            }
        }
        Message::CheckTimeoutRelease => {
            if !state.keep_playing
                && state.is_playing
                && state.last_play_button_press.elapsed() > Duration::from_millis(100)
            {
                state.is_playing = false;
            }
        }
        Message::MouseEvent(mouse) => {
            handle_mouse_event(&mut state, mouse);
        }
        Message::Exit => {
            state.is_playing = false;
            state.should_exit = true;
        }
    }
    state
}

fn is_in_rect(rect: ratatui::layout::Rect, x: u16, y: u16) -> bool {
    x >= rect.left() && x < rect.right() && y >= rect.top() && y < rect.bottom()
}

pub fn handle_mouse_event(state: &mut SynthState, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::Down(_) => {
            state.mouse_dragging = true;
            state.mouse_start_x = mouse.column;
        }
        MouseEventKind::Up(_) => {
            state.mouse_dragging = false;
        }
        MouseEventKind::Drag(_) => {
            if !state.mouse_dragging {
                return;
            }

            let delta = mouse.column as i16 - state.mouse_start_x as i16;
            state.mouse_start_x = mouse.column;

            match state.focused_field {
                FocusedField::Frequency => {
                    state.frequency = (state.frequency + (delta as f32 * 10.0)).clamp(20.0, 20000.0);
                }
                FocusedField::Volume => {
                    state.volume = (state.volume + (delta as f32 * 0.01)).clamp(0.0, 1.0);
                }
                FocusedField::Shape => {
                    if delta > 0 {
                        state.shape = match state.shape {
                            WaveShape::Sine => WaveShape::Square,
                            WaveShape::Square => WaveShape::Triangle,
                            WaveShape::Triangle => WaveShape::Sawtooth,
                            WaveShape::Sawtooth => WaveShape::Sine,
                        };
                    } else if delta < 0 {
                        state.shape = match state.shape {
                            WaveShape::Sine => WaveShape::Sawtooth,
                            WaveShape::Square => WaveShape::Sine,
                            WaveShape::Triangle => WaveShape::Square,
                            WaveShape::Sawtooth => WaveShape::Triangle,
                        };
                    }
                }
                _ => {}
            }
        }
        MouseEventKind::ScrollUp => match state.focused_field {
            FocusedField::Frequency => {
                state.frequency = (state.frequency + 10.0).min(20000.0);
            }
            FocusedField::Volume => {
                state.volume = (state.volume + 0.05).min(1.0);
            }
            _ => {}
        },
        MouseEventKind::ScrollDown => match state.focused_field {
            FocusedField::Frequency => {
                state.frequency = (state.frequency - 10.0).max(20.0);
            }
            FocusedField::Volume => {
                state.volume = (state.volume - 0.05).max(0.0);
            }
            _ => {}
        },
        _ => {}
    }
}
