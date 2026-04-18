use crate::app::state::FocusedField;
use crate::app::{Message, SynthState, update};
use crate::audio::AudioPlayer;
use crate::waveform::WaveShape;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph},
};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn run_tui(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    let mut state = SynthState::new();
    let (playing_arc, _keep_playing_arc, should_exit_arc) = start_audio_thread(&mut state, verbose);

    let result = run_app(terminal, &mut state);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    playing_arc.store(false, Ordering::Relaxed);
    should_exit_arc.store(true, Ordering::Relaxed);

    result
}

fn start_audio_thread(
    state: &mut SynthState,
    verbose: bool,
) -> (Arc<AtomicBool>, Arc<AtomicBool>, Arc<AtomicBool>) {
    let frequency = Arc::new(Mutex::new(state.frequency));
    let volume = Arc::new(Mutex::new(state.volume));
    let shape = Arc::new(Mutex::new(state.shape));
    let playing = Arc::new(AtomicBool::new(false));
    let keep_playing = Arc::new(AtomicBool::new(false));
    let should_exit = Arc::new(AtomicBool::new(false));

    let frequency_clone = Arc::clone(&frequency);
    let volume_clone = Arc::clone(&volume);
    let shape_clone = Arc::clone(&shape);
    let playing_clone = Arc::clone(&playing);
    let should_exit_clone = Arc::clone(&should_exit);

    thread::spawn(move || {
        let init_freq = *frequency_clone.lock().unwrap();
        let init_vol = *volume_clone.lock().unwrap();
        let init_shape = *shape_clone.lock().unwrap();

        let player = AudioPlayer::new(init_freq, init_vol, init_shape).with_verbose(verbose);

        let result = player.play_realtime(
            Arc::clone(&playing_clone),
            Arc::clone(&should_exit_clone),
            Some(Arc::clone(&frequency_clone)),
            Some(Arc::clone(&volume_clone)),
            Some(Arc::clone(&shape_clone)),
        );

        if let Err(e) = result {
            eprintln!("Audio error: {}", e);
        }
    });

    (playing, keep_playing, should_exit)
}

fn run_app(
    mut terminal: Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut SynthState,
) -> Result<(), Box<dyn std::error::Error>> {
    // Shared state for audio thread
    let freq_arc = Arc::new(Mutex::new(state.frequency));
    let vol_arc = Arc::new(Mutex::new(state.volume));
    let shape_arc = Arc::new(Mutex::new(state.shape));
    let playing_arc = Arc::new(AtomicBool::new(false));
    let keep_playing_arc = Arc::new(AtomicBool::new(false));
    let should_exit_arc = Arc::new(AtomicBool::new(false));

    // Start audio thread
    let _audio_handle = {
        let freq_clone = Arc::clone(&freq_arc);
        let vol_clone = Arc::clone(&vol_arc);
        let shape_clone = Arc::clone(&shape_arc);
        let playing_clone = Arc::clone(&playing_arc);
        let should_exit_clone = Arc::clone(&should_exit_arc);

        thread::spawn(move || {
            let player = AudioPlayer::new(440.0, 0.5, WaveShape::Sine).with_verbose(false);
            let _ = player.play_realtime(
                playing_clone,
                should_exit_clone,
                Some(freq_clone),
                Some(vol_clone),
                Some(shape_clone),
            );
        })
    };

    loop {
        terminal.draw(|f| render_ui(f, state))?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == event::KeyEventKind::Press {
                        *state = update(state.clone(), key_to_message(key, state));
                    } else if key.kind == event::KeyEventKind::Release {
                        if let Some(msg) = key_to_release_message(key) {
                            *state = update(state.clone(), msg);
                        } else {
                            *state = update(state.clone(), Message::ReleasePlayButton);
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    *state = update(state.clone(), Message::MouseEvent(mouse));
                }
                _ => {}
            }
        }

        *state = update(state.clone(), Message::CheckTimeoutRelease);

        // Sync state to audio thread
        if let Ok(mut freq) = freq_arc.lock() {
            *freq = state.frequency;
        }
        if let Ok(mut vol) = vol_arc.lock() {
            *vol = state.volume;
        }
        if let Ok(mut shape) = shape_arc.lock() {
            *shape = state.shape;
        }
        playing_arc.store(state.is_playing, Ordering::Relaxed);
        keep_playing_arc.store(state.keep_playing, Ordering::Relaxed);

        if state.should_exit {
            break;
        }
    }

    should_exit_arc.store(true, Ordering::Relaxed);
    Ok(())
}

fn key_to_message(key: KeyEvent, state: &SynthState) -> Message {
    use crate::music::PianoKey;

    // Piano keyboard mapping - using QWERTY layout
    // Row 1: Q W E R T Y U I  (C C# D D# E F F# G)
    // Row 2: Z X C V B N M    (G# A A# B C C# D)
    match key.code {
        // Piano keys - QWERTY layout (Octave 4) - CHECK THESE FIRST!
        KeyCode::Char('q') => Message::PianoPressKey(PianoKey::C4),
        KeyCode::Char('2') => Message::PianoPressKey(PianoKey::CSharp4),
        KeyCode::Char('w') => Message::PianoPressKey(PianoKey::D4),
        KeyCode::Char('3') => Message::PianoPressKey(PianoKey::DSharp4),
        KeyCode::Char('e') => Message::PianoPressKey(PianoKey::E4),
        KeyCode::Char('r') => Message::PianoPressKey(PianoKey::F4),
        KeyCode::Char('5') => Message::PianoPressKey(PianoKey::FSharp4),
        KeyCode::Char('t') => Message::PianoPressKey(PianoKey::G4),
        KeyCode::Char('6') => Message::PianoPressKey(PianoKey::GSharp4),
        KeyCode::Char('y') => Message::PianoPressKey(PianoKey::A4),
        KeyCode::Char('7') => Message::PianoPressKey(PianoKey::ASharp4),
        KeyCode::Char('u') => Message::PianoPressKey(PianoKey::B4),
        KeyCode::Char('i') => Message::PianoPressKey(PianoKey::C5),

        // Alt row: Z X C V B N M (G3 and higher octave)
        KeyCode::Char('z') => Message::PianoPressKey(PianoKey::G3),
        KeyCode::Char('x') => Message::PianoPressKey(PianoKey::GSharp3),
        KeyCode::Char('c') => Message::PianoPressKey(PianoKey::A3),
        KeyCode::Char('v') => Message::PianoPressKey(PianoKey::ASharp3),
        KeyCode::Char('b') => Message::PianoPressKey(PianoKey::B3),
        KeyCode::Char('n') => Message::PianoPressKey(PianoKey::C4),
        KeyCode::Char('m') => Message::PianoPressKey(PianoKey::CSharp4),

        // Waveform selection
        KeyCode::Char('1') => Message::SetWaveform(WaveShape::Sine),
        KeyCode::Char('2') => Message::SetWaveform(WaveShape::Square),
        KeyCode::Char('3') => Message::SetWaveform(WaveShape::Triangle),
        KeyCode::Char('4') => Message::SetWaveform(WaveShape::Sawtooth),

        // UI controls
        KeyCode::Tab => Message::FocusNext,
        KeyCode::Up => {
            if state.focused_field == FocusedField::Frequency {
                Message::IncreaseFrequency
            } else if state.focused_field == FocusedField::Volume {
                Message::IncreaseVolume
            } else {
                Message::FocusNext
            }
        }
        KeyCode::Down => {
            if state.focused_field == FocusedField::Frequency {
                Message::DecreaseFrequency
            } else if state.focused_field == FocusedField::Volume {
                Message::DecreaseVolume
            } else {
                Message::FocusNext
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            if state.focused_field == FocusedField::PlayButton {
                Message::PressPlayButton
            } else if state.focused_field == FocusedField::PlayToggleButton {
                Message::TogglePlay
            } else {
                Message::FocusNext
            }
        }
        KeyCode::Esc => Message::Exit,

        _ => Message::FocusNext,
    }
}

fn key_to_release_message(key: KeyEvent) -> Option<Message> {
    use crate::music::PianoKey;

    match key.code {
        // Piano keys - QWERTY layout (Octave 4)
        KeyCode::Char('q') => Some(Message::PianoReleaseKey(PianoKey::C4)),
        KeyCode::Char('2') => Some(Message::PianoReleaseKey(PianoKey::CSharp4)),
        KeyCode::Char('w') => Some(Message::PianoReleaseKey(PianoKey::D4)),
        KeyCode::Char('3') => Some(Message::PianoReleaseKey(PianoKey::DSharp4)),
        KeyCode::Char('e') => Some(Message::PianoReleaseKey(PianoKey::E4)),
        KeyCode::Char('r') => Some(Message::PianoReleaseKey(PianoKey::F4)),
        KeyCode::Char('5') => Some(Message::PianoReleaseKey(PianoKey::FSharp4)),
        KeyCode::Char('t') => Some(Message::PianoReleaseKey(PianoKey::G4)),
        KeyCode::Char('6') => Some(Message::PianoReleaseKey(PianoKey::GSharp4)),
        KeyCode::Char('y') => Some(Message::PianoReleaseKey(PianoKey::A4)),
        KeyCode::Char('7') => Some(Message::PianoReleaseKey(PianoKey::ASharp4)),
        KeyCode::Char('u') => Some(Message::PianoReleaseKey(PianoKey::B4)),
        KeyCode::Char('i') => Some(Message::PianoReleaseKey(PianoKey::C5)),

        // Alt row: Z X C V B N M
        KeyCode::Char('z') => Some(Message::PianoReleaseKey(PianoKey::G3)),
        KeyCode::Char('x') => Some(Message::PianoReleaseKey(PianoKey::GSharp3)),
        KeyCode::Char('c') => Some(Message::PianoReleaseKey(PianoKey::A3)),
        KeyCode::Char('v') => Some(Message::PianoReleaseKey(PianoKey::ASharp3)),
        KeyCode::Char('b') => Some(Message::PianoReleaseKey(PianoKey::B3)),
        KeyCode::Char('n') => Some(Message::PianoReleaseKey(PianoKey::C4)),
        KeyCode::Char('m') => Some(Message::PianoReleaseKey(PianoKey::CSharp4)),

        _ => None,
    }
}

fn render_piano_widget(f: &mut Frame, area: ratatui::layout::Rect, state: &SynthState) {
    use crate::music::PianoKey;

    if area.height < 2 {
        return;
    }

    let block = Block::default()
        .title("Piano Keyboard (Q-I and Z-M)")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    // Display piano keys - show which ones are active
    let mut key_display = String::new();

    // Show octave 3 bottom row and octave 4 top row
    let keys_to_show = [
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
    ];

    for (i, key) in keys_to_show.iter().enumerate() {
        if i > 0 && i % 12 == 0 {
            key_display.push('\n');
        }
        if state.piano_active_keys.contains(key) {
            key_display.push_str(&format!("[{}]", key.name()));
        } else {
            key_display.push_str(&format!(" {} ", key.name()));
        }
    }

    let para = Paragraph::new(key_display).alignment(Alignment::Center);
    f.render_widget(para, inner);
}

fn render_waveform_button(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    wave_type: WaveShape,
    active_wave: WaveShape,
) {
    let is_active = wave_type == active_wave;

    // ASCII art for each waveform
    let (label, ascii_art) = match wave_type {
        WaveShape::Sine => (
            "Sine wave",
            " \
 ╭──╮    ╭──╮    ╭──╮
╭╯  ╰╮  ╭╯  ╰╮  ╭╯  ╰╮
╯    ╰──╯    ╰──╯    ╰ ",
        ),
        WaveShape::Square => (
            "Square wave",
            "  \
  ┌───┐   ┌───┐   ┌───┐
  │   │   │   │   │   │
──┘   └───┘   └───┘   └──",
        ),
        WaveShape::Triangle => (
            "Triangle wave",
            "
  ╱╲  ╱╲  ╱╲  ╱╲  ╱╲
 ╱  ╲╱  ╲╱  ╲╱  ╲╱  ╲",
        ),
        WaveShape::Sawtooth => (
            "Sawtooth wave",
            "
 ╱│ ╱│ ╱│ ╱│ ╱│ ╱│ ╱│
╱ │╱ │╱ │╱ │╱ │╱ │╱ │",
        ),
    };

    let style = if is_active {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(label)
        .borders(Borders::ALL)
        .style(style);

    let para = Paragraph::new(ascii_art)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(para, area);
}

fn render_ui(f: &mut Frame, state: &SynthState) {
    let frequency = state.frequency;
    let volume = state.volume;
    let shape = state.shape;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(1),
        ])
        .split(f.size());

    // Frequency field
    let freq_style = if state.focused_field == FocusedField::Frequency {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let freq_block = Block::default()
        .title("Frequency (Hz) - Up/Down to adjust, drag to change")
        .borders(Borders::ALL)
        .style(freq_style);
    let freq_text = format!("{:.0}", frequency);
    let freq_para = Paragraph::new(freq_text)
        .block(freq_block)
        .alignment(Alignment::Center);
    f.render_widget(freq_para, chunks[0]);

    // Volume field
    let vol_style = if state.focused_field == FocusedField::Volume {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let vol_block = Block::default()
        .title("Volume - Up/Down or scroll to adjust, drag to change")
        .borders(Borders::ALL)
        .style(vol_style);
    let vol_gauge = Gauge::default()
        .block(vol_block)
        .ratio(volume as f64)
        .label(format!("{:.0}%", volume * 100.0))
        .style(Style::default().fg(Color::Green));
    f.render_widget(vol_gauge, chunks[1]);

    // Waveform buttons - split into 4 columns for each waveform
    let waveform_area = chunks[2];
    let wave_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(waveform_area);

    render_waveform_button(f, wave_chunks[0], WaveShape::Sine, shape);
    render_waveform_button(f, wave_chunks[1], WaveShape::Square, shape);
    render_waveform_button(f, wave_chunks[2], WaveShape::Triangle, shape);
    render_waveform_button(f, wave_chunks[3], WaveShape::Sawtooth, shape);

    // Play button
    let play_style = if state.focused_field == FocusedField::PlayButton {
        Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let play_status = if state.is_playing {
        "PLAYING (Click or press Space/Enter to release)"
    } else {
        "CLICK TO PLAY (or press Space)"
    };
    let play_block = Block::default()
        .title("Play Button")
        .borders(Borders::ALL)
        .style(play_style);
    let play_para = Paragraph::new(play_status)
        .block(play_block)
        .alignment(Alignment::Center);
    f.render_widget(play_para, chunks[3]);

    // Play toggle button
    let playtoggle_style = if state.focused_field == FocusedField::PlayToggleButton {
        Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let playtoggle_status = if state.is_playing {
        "Playing (Click or press Space/Enter to toggle)"
    } else {
        "Stopped (Click or press Space to toggle)"
    };
    let playtoggle_block = Block::default()
        .title("Play Toggle Button")
        .borders(Borders::ALL)
        .style(playtoggle_style);
    let playtoggle_para = Paragraph::new(playtoggle_status)
        .block(playtoggle_block)
        .alignment(Alignment::Center);
    f.render_widget(playtoggle_para, chunks[4]);

    // Piano keyboard
    render_piano_widget(f, chunks[5], state);

    // Instructions
    let instructions = vec![
        Line::from("Piano: Q W E R T Y U I (Z X C V B N M)"),
        Line::from("Waveforms: 1=Sine | 2=Square | 3=Triangle | 4=Sawtooth"),
        Line::from("Controls: Tab - Switch field | ↑/↓ - Adjust freq/vol"),
        Line::from("  Space - Play button | Esc - Quit"),
    ];
    let instructions_block = Block::default().title("Instructions").borders(Borders::ALL);
    let instructions_para = Paragraph::new(instructions).block(instructions_block);
    f.render_widget(instructions_para, chunks[6]);
}
