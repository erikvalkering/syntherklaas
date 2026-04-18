use super::message::Message;
use super::state::{FocusedField, SynthState};
use crate::waveform::WaveShape;
use crossterm::event::{MouseEvent, MouseEventKind};
use std::time::Duration;
use crate::music;

pub fn update(mut state: SynthState, msg: Message) -> SynthState {
    match msg {
        Message::IncreaseFrequency => {
            state.frequency = (state.frequency + 10.0).min(20000.0);
        }
        Message::DecreaseFrequency => {
            state.frequency = (state.frequency - 10.0).max(20.0);
        }
        Message::SetFrequency(freq) => {
            state.frequency = freq.clamp(20.0, 20000.0);
        }
        Message::IncreaseVolume => {
            state.volume = (state.volume + 0.05).min(1.0);
        }
        Message::DecreaseVolume => {
            state.volume = (state.volume - 0.05).max(0.0);
        }
        Message::SetVolume(vol) => {
            state.volume = vol.clamp(0.0, 1.0);
        }
        Message::SetWaveform(shape) => {
            state.shape = shape;
        }
        Message::PressPlayButton => {
            state.is_playing = true;
            state.last_play_button_press = std::time::Instant::now();
        }
        Message::ReleasePlayButton => {
            state.is_playing = false;
        }
        Message::TogglePlay => {
            let new_state = !state.is_playing;
            state.is_playing = new_state;
            state.keep_playing = new_state;
            state.last_play_button_press = std::time::Instant::now();
        }
        Message::FocusNext => {
            state.focused_field = match state.focused_field {
                FocusedField::Frequency => FocusedField::Volume,
                FocusedField::Volume => FocusedField::Shape,
                FocusedField::Shape => FocusedField::Frequency,
            };
        }
        Message::FocusPrev => {
            state.focused_field = match state.focused_field {
                FocusedField::Frequency => FocusedField::Shape,
                FocusedField::Volume => FocusedField::Frequency,
                FocusedField::Shape => FocusedField::Volume,
            };
        }
        Message::KeyboardKeyDown(key_option) => {
            if let Some(key) = key_option {
                state.current_piano_key = Some(key);
                state.frequency = key.frequency();
                state.is_playing = true;
            }
        }
        Message::KeyboardKeyUp => {
            state.current_piano_key = None;
            state.is_playing = false;
        }
        Message::ChangeOctave(delta) => {
            state.current_octave = (state.current_octave + delta).clamp(0, 8);
        }
        Message::ChangeSemitone(delta) => {
            state.semitone_offset = (state.semitone_offset + delta).rem_euclid(12);
            // If a key is currently pressed, update its frequency
            if let Some(_key) = state.current_piano_key {
                let semitone_steps = state.semitone_offset;
                if let Some(new_key) = music::get_key_for_octave_and_semitone(state.current_octave, semitone_steps) {
                    state.frequency = new_key.frequency();
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

pub fn handle_mouse_event(state: &mut SynthState, mouse: MouseEvent) {
    use crate::music;

    match mouse.kind {
        MouseEventKind::Down(_) => {
            // Check if click is on piano keyboard and convert to key
            // Keyboard layout has white keys starting around column 4
            // Each white key is 6 characters wide (╔════╗ = 6 chars)
            // Black keys are positioned above at specific offsets
            
            // Simple heuristic: if row is around line 7-8 and column matches a key position
            // Map column to semitone (0-11 for C through B)
            // Key positions: "a", "w", "s", "e", "d", "f", "t", "g", "y", "h", "u", "j"
            // Column positions approximately: 6, 12, 18, 24, 30, 36, 42, 48, 54, 60, 66, 72
            
            let col = mouse.column as i32;
            let semitone = if col >= 4 && col <= 80 {
                // Map column to semitone 0-11
                let relative_col = (col - 4) / 6;
                if relative_col >= 0 && relative_col < 12 {
                    Some(relative_col as i32)
                } else {
                    None
                }
            } else {
                None
            };
            
            if let Some(st) = semitone {
                if let Some(key) = music::get_key_for_octave_and_semitone(state.current_octave, st) {
                    state.current_piano_key = Some(key);
                    state.frequency = key.frequency();
                    state.is_playing = true;
                }
            }
            
            state.mouse_dragging = true;
            state.mouse_start_x = mouse.column;
        }
        MouseEventKind::Up(_) => {
            state.mouse_dragging = false;
            state.current_piano_key = None;
            state.is_playing = false;
        }
        MouseEventKind::Drag(_) => {
            if !state.mouse_dragging {
                return;
            }

            let delta = mouse.column as i16 - state.mouse_start_x as i16;
            state.mouse_start_x = mouse.column;

            match state.focused_field {
                FocusedField::Frequency => {
                    state.frequency =
                        (state.frequency + (delta as f32 * 10.0)).clamp(20.0, 20000.0);
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
