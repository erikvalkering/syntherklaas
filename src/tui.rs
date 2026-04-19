use crate::app::state::FocusedField;
use crate::app::{Message, SynthState, update};
use crate::audio::AudioPlayer;
use crate::waveform::WaveShape;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
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

const PIANO_KEYS: &str = "awsedftgyhuj";

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

    execute!(
        io::stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    )?;

    loop {
        terminal.draw(|f| render_ui(f, state))?;

        let msg = if crossterm::event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => match key.kind {
                    event::KeyEventKind::Press => key_to_message(key, state),
                    event::KeyEventKind::Release
                        if PIANO_KEYS.contains(key.code.as_char().unwrap()) =>
                    {
                        Some(Message::ReleasePlayButton)
                    }

                    _ => None,
                },

                Event::Mouse(mouse) => Some(Message::MouseEvent(mouse)),

                _ => None,
            }
        } else {
            Some(Message::CheckTimeoutRelease)
        };

        if let Some(msg2) = msg {
            *state = update(state.clone(), msg2);
        }

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

    execute!(io::stdout(), PopKeyboardEnhancementFlags)?;

    should_exit_arc.store(true, Ordering::Relaxed);
    Ok(())
}

fn key_to_message(key: KeyEvent, state: &SynthState) -> Option<Message> {
    use crate::music;

    match key.code {
        // Piano keys a-j map to C-B (semitones 0-11) in current octave
        KeyCode::Char(c) if PIANO_KEYS.contains(c) => Some(Message::KeyboardKeyDown(
            music::get_key_for_octave_and_semitone(
                state.current_octave,
                PIANO_KEYS.chars().position(|k| k == c).unwrap() as i32,
            ),
        )),

        // Octave navigation
        KeyCode::Char('k') => Some(Message::ChangeOctave(-1)),
        KeyCode::Char('l') => Some(Message::ChangeOctave(1)),

        // Semitone navigation
        KeyCode::Char('o') => Some(Message::ChangeSemitone(-1)),
        KeyCode::Char('p') => Some(Message::ChangeSemitone(1)),

        // Waveform selection
        KeyCode::Char('1') => Some(Message::SetWaveform(WaveShape::Sine)),
        KeyCode::Char('2') => Some(Message::SetWaveform(WaveShape::Square)),
        KeyCode::Char('3') => Some(Message::SetWaveform(WaveShape::Triangle)),
        KeyCode::Char('4') => Some(Message::SetWaveform(WaveShape::Sawtooth)),

        // UI controls
        KeyCode::Tab => Some(Message::FocusNext),
        KeyCode::BackTab => Some(Message::FocusPrev),
        KeyCode::Up => {
            if state.focused_field == FocusedField::Frequency {
                Some(Message::IncreaseFrequency)
            } else if state.focused_field == FocusedField::Volume {
                Some(Message::IncreaseVolume)
            } else {
                None
            }
        }
        KeyCode::Down => {
            if state.focused_field == FocusedField::Frequency {
                Some(Message::DecreaseFrequency)
            } else if state.focused_field == FocusedField::Volume {
                Some(Message::DecreaseVolume)
            } else {
                None
            }
        }
        KeyCode::Char(' ') => Some(Message::TogglePlay),
        KeyCode::Esc | KeyCode::Char('q') => Some(Message::Exit),

        _ => None,
    }
}

fn render_piano_widget(f: &mut Frame, area: ratatui::layout::Rect, state: &SynthState) {
    if area.height < 4 {
        return;
    }

    let block = Block::default()
        .title("Piano Keyboard - Click keys or use a-j, k/l, o/p")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Top row: status info
    let mut display_text = format!(
        "Octave: {}  Semitone offset: {}  ",
        state.current_octave, state.semitone_offset
    );
    if let Some(key) = state.current_piano_key {
        display_text.push_str(&format!(
            "Playing: [{}] @ {:.1}Hz",
            key.name(),
            key.frequency()
        ));
    } else {
        display_text.push_str("Playing: (none)");
    }

    let para = Paragraph::new(display_text);
    let mut status_area = inner;
    status_area.height = 1;
    f.render_widget(para, status_area);

    // Middle rows: visual keyboard with black and white keys
    if inner.height > 3 {
        let mut keyboard_display = String::new();

        // Draw black keys on first line (upper row)
        keyboard_display.push_str("       ╔═╗ ╔═╗     ╔═╗ ╔═╗ ╔═╗\n");
        keyboard_display.push_str("       ║#║ ║#║     ║#║ ║#║ ║#║\n");
        keyboard_display.push_str("       ╚═╝ ╚═╝     ╚═╝ ╚═╝ ╚═╝\n");

        // Draw white keys on second line (lower row)
        keyboard_display.push_str("    ╔═══╦═══╦═══╦═══╦═══╦═══╦═══╗\n");
        keyboard_display.push_str("    ║ a ║ s ║ d ║ f ║ g ║ h ║ j ║\n");
        keyboard_display.push_str("    ║ C ║ D ║ E ║ F ║ G ║ A ║ B ║\n");
        keyboard_display.push_str("    ╚═══╩═══╩═══╩═══╩═══╩═══╩═══╝");

        let mut keyboard_area = inner;
        keyboard_area.y += 1;
        keyboard_area.height = keyboard_area.height.saturating_sub(1);

        let keyboard_para = Paragraph::new(keyboard_display).alignment(Alignment::Left);
        f.render_widget(keyboard_para, keyboard_area);
    }
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
            Constraint::Length(15),
            Constraint::Min(1),
            Constraint::Max(16),
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

    // Piano keyboard
    render_piano_widget(f, chunks[3], state);

    // Instructions
    let instructions = vec![
        Line::from("Piano: a-j keys | Octave: k/l | Semitone: o/p"),
        Line::from("Waveforms: 1=Sine | 2=Square | 3=Triangle | 4=Sawtooth"),
        Line::from("Controls: Tab=Switch field | ↑/↓=Adjust | Space=Toggle audio"),
        Line::from("Mouse: Click piano keys | Esc/Q=Quit"),
    ];

    let instructions_block = Block::default().title("Instructions").borders(Borders::ALL);
    let instructions_para = Paragraph::new(instructions).block(instructions_block);
    f.render_widget(instructions_para, chunks[4]);

    let logo = String::from(
        "
███████╗██╗   ██╗███╗   ██╗████████╗██╗  ██╗███████╗██████╗ 
██╔════╝╚██╗ ██╔╝████╗  ██║╚══██╔══╝██║  ██║██╔════╝██╔══██╗
███████╗ ╚████╔╝ ██╔██╗ ██║   ██║   ███████║█████╗  ██████╔╝
╚════██║  ╚██╔╝  ██║╚██╗██║   ██║   ██╔══██║██╔══╝  ██╔══██╗
███████║   ██║   ██║ ╚████║   ██║   ██║  ██║███████╗██║  ██║
╚══════╝   ╚═╝   ╚═╝  ╚═══╝   ╚═╝   ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
██╗  ██╗██╗      █████╗  █████╗ ███████╗
██║ ██╔╝██║     ██╔══██╗██╔══██╗██╔════╝
█████╔╝ ██║     ███████║███████║███████╗
██╔═██╗ ██║     ██╔══██║██╔══██║╚════██║
██║  ██╗███████╗██║  ██║██║  ██║███████║
╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝",
    );
    let logo_block = Block::default().borders(Borders::ALL);
    let logo_para = Paragraph::new(logo)
        .block(logo_block)
        .centered()
        .style(Color::Red);
    f.render_widget(logo_para, chunks[5]);
}
