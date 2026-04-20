use super::message::Message;
use super::state::{FocusedField, SynthState};
use crate::music;
use crate::waveform::WaveShape;
use crossterm::event::{MouseEvent, MouseEventKind};
use std::time::Duration;

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
            // Move down one row, then wrap to top if at bottom
            if state.focus.row < 2 {
                state.focus = state.focus.move_down(2);
                // Reset column to 0 when moving to a new row
                state.focus.col = 0;
            } else {
                state.focus = state.focus.move_down(2);
                // Don't wrap; stay at row 2
            }
        }
        Message::FocusPrev => {
            // Move up one row, wrapping from row 0 to row 2
            if state.focus.row > 0 {
                state.focus = state.focus.move_up();
                state.focus.col = 0;
            } else {
                // Wrap to row 2 (waveform)
                state.focus = super::focus::FocusPosition::new(2, 0);
            }
        }
        Message::MoveUp => {
            state.focus = state.focus.move_up();
            state.focus.col = 0;
        }
        Message::MoveDown => {
            state.focus = state.focus.move_down(2);
            state.focus.col = 0;
        }
        Message::MoveLeft => {
            let max_col = if state.focus.row == 2 { 3 } else { 0 };
            state.focus = state.focus.move_left();
            if state.focus.col > max_col {
                state.focus.col = max_col;
            }
        }
        Message::MoveRight => {
            let max_col = if state.focus.row == 2 { 3 } else { 0 };
            state.focus = state.focus.move_right(max_col);
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
                if let Some(new_key) =
                    music::get_key_for_octave_and_semitone(state.current_octave, semitone_steps)
                {
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
            // Detect which piano key was clicked
            // Piano keyboard layout (with margins):
            //   "    ╔═══╦═══╦═══╦═══╦═══╦═══╦═══╗"
            //   "    ║ a ║ s ║ d ║ f ║ g ║ h ║ j ║"
            //   "    ║ C ║ D ║ E ║ F ║ G ║ A ║ B ║"
            //   "    ╚═══╩═══╩═══╩═══╩═══╩═══╩═══╝"
            // Each key cell is 4 characters wide
            // Key centers: C≈5, D≈9, E≈13, F≈17, G≈21, A≈25, B≈29

            let col = mouse.column;

            // Map column to semitone (0-11 for C through B)
            let semitone = if (4..=31).contains(&col) {
                // Determine key from column position
                match col {
                    3..=6 => Some(0),    // C
                    7..=10 => Some(2),   // D
                    11..=14 => Some(4),  // E
                    15..=18 => Some(5),  // F
                    19..=22 => Some(7),  // G
                    23..=26 => Some(9),  // A
                    27..=31 => Some(11), // B
                    _ => None,
                }
            } else {
                None
            };

            if let Some(st) = semitone
                && let Some(key) = music::get_key_for_octave_and_semitone(state.current_octave, st)
            {
                state.current_piano_key = Some(key);
                state.frequency = key.frequency();
                state.is_playing = true;
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

            match state.focused_field() {
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
            }
        }
        MouseEventKind::ScrollUp => match state.focused_field() {
            FocusedField::Frequency => {
                state.frequency = (state.frequency + 10.0).min(20000.0);
            }
            FocusedField::Volume => {
                state.volume = (state.volume + 0.05).min(1.0);
            }
            _ => {}
        },
        MouseEventKind::ScrollDown => match state.focused_field() {
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
