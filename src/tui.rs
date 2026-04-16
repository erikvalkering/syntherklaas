use crate::app::{update, Message, SynthState};
use crate::app::state::FocusedField;
use crate::audio::AudioPlayer;
use crate::waveform::WaveShape;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent,
    },
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
    let (playing_arc, _keep_playing_arc, should_exit_arc) =
        start_audio_thread(&mut state, verbose);

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
                        *state = update(state.clone(), Message::ReleasePlayButton);
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
    match key.code {
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
        KeyCode::Left | KeyCode::Char('a') => Message::PrevWaveform,
        KeyCode::Right | KeyCode::Char('d') => Message::NextWaveform,
        KeyCode::Char(' ') | KeyCode::Enter => {
            if state.focused_field == FocusedField::PlayButton {
                Message::PressPlayButton
            } else if state.focused_field == FocusedField::PlayToggleButton {
                Message::TogglePlay
            } else {
                Message::FocusNext
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => Message::Exit,
        _ => Message::FocusNext,
    }
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
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
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

    // Waveform shape
    let shape_style = if state.focused_field == FocusedField::Shape {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let shape_block = Block::default()
        .title("Waveform - Left/Right or drag to change")
        .borders(Borders::ALL)
        .style(shape_style);
    let shape_text = match shape {
        WaveShape::Sine => "Sine",
        WaveShape::Square => "Square",
        WaveShape::Triangle => "Triangle",
        WaveShape::Sawtooth => "Sawtooth",
    };
    let shape_para = Paragraph::new(shape_text)
        .block(shape_block)
        .alignment(Alignment::Center);
    f.render_widget(shape_para, chunks[2]);

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

    // Instructions
    let instructions = vec![
        Line::from("Controls:"),
        Line::from("  Tab - Switch field"),
        Line::from("  ↑/↓ - Adjust frequency/volume"),
        Line::from("  ←/→ - Change waveform"),
        Line::from("  Space/Enter - Toggle play button"),
        Line::from("  q/Esc - Quit"),
    ];
    let instructions_block = Block::default().title("Instructions").borders(Borders::ALL);
    let instructions_para = Paragraph::new(instructions).block(instructions_block);
    f.render_widget(instructions_para, chunks[5]);
}
