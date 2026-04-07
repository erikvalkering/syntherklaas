use crate::audio::{AudioPlayer, AudioBackend};
use crate::waveform::WaveShape;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Gauge},
    Frame, Terminal,
};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct AppState {
    frequency: f32,
    volume: f32,
    shape: WaveShape,
    playing: Arc<AtomicBool>,
    should_exit_tui: bool,
    should_exit_audio: Arc<AtomicBool>,
    focused_field: FocusedField,
    backend: Option<AudioBackend>,
    verbose: bool,
    audio_thread: Option<std::thread::JoinHandle<()>>,
    last_play_button_press: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FocusedField {
    Frequency,
    Volume,
    Shape,
    PlayButton,
}

impl AppState {
    fn new(backend: Option<AudioBackend>, verbose: bool) -> Self {
        AppState {
            frequency: 440.0,
            volume: 0.5,
            shape: WaveShape::Sine,
            playing: Arc::new(AtomicBool::new(false)),
            should_exit_tui: false,
            should_exit_audio: Arc::new(AtomicBool::new(false)),
            focused_field: FocusedField::Frequency,
            backend,
            verbose,
            audio_thread: None,
            last_play_button_press: Instant::now(),
        }
    }

    fn start_audio_thread(&mut self) {
        let frequency = self.frequency;
        let volume = self.volume;
        let shape = self.shape;
        let backend = self.backend;
        let verbose = self.verbose;
        let playing = Arc::clone(&self.playing);
        let should_exit = Arc::clone(&self.should_exit_audio);

        let audio_thread = thread::spawn(move || {
            // Set up panic hook to suppress output unless verbose
            if !verbose {
                std::panic::set_hook(Box::new(|_| {}));
            }

            let player = AudioPlayer::new(frequency, volume, shape, 999.0)
                .with_verbose(verbose);
            let player = if let Some(b) = backend {
                player.with_backend(b)
            } else {
                player
            };

            // Try realtime playback with fallback
            use std::panic;
            let result = if let Some(AudioBackend::PulseAudio) = backend {
                player.play_realtime_pulseaudio(Arc::clone(&playing), Arc::clone(&should_exit))
            } else if let Some(AudioBackend::Cpal) = backend {
                player.play_realtime_cpal(Arc::clone(&playing), Arc::clone(&should_exit))
            } else {
                match panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    player.play_realtime_cpal(Arc::clone(&playing), Arc::clone(&should_exit))
                })) {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(_)) | Err(_) => {
                        if verbose {
                            eprintln!("Switching to PulseAudio...");
                        }
                        player.play_realtime_pulseaudio(playing, should_exit)
                    }
                }
            };

            if let Err(e) = result {
                eprintln!("Audio error: {}", e);
            }
        });

        self.audio_thread = Some(audio_thread);
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.focused_field = match self.focused_field {
                    FocusedField::Frequency => FocusedField::Volume,
                    FocusedField::Volume => FocusedField::Shape,
                    FocusedField::Shape => FocusedField::PlayButton,
                    FocusedField::PlayButton => FocusedField::Frequency,
                };
            }
            KeyCode::Up => {
                match self.focused_field {
                    FocusedField::Frequency => {
                        self.frequency = (self.frequency + 10.0).min(20000.0);
                    }
                    FocusedField::Volume => {
                        self.volume = (self.volume + 0.05).min(1.0);
                    }
                    _ => {}
                }
            }
            KeyCode::Down => {
                match self.focused_field {
                    FocusedField::Frequency => {
                        self.frequency = (self.frequency - 10.0).max(20.0);
                    }
                    FocusedField::Volume => {
                        self.volume = (self.volume - 0.05).max(0.0);
                    }
                    _ => {}
                }
            }
            KeyCode::Left | KeyCode::Char('a') => {
                match self.focused_field {
                    FocusedField::Shape => {
                        self.shape = match self.shape {
                            WaveShape::Sine => WaveShape::Sawtooth,
                            WaveShape::Square => WaveShape::Sine,
                            WaveShape::Triangle => WaveShape::Square,
                            WaveShape::Sawtooth => WaveShape::Triangle,
                        };
                    }
                    _ => {}
                }
            }
            KeyCode::Right | KeyCode::Char('d') => {
                match self.focused_field {
                    FocusedField::Shape => {
                        self.shape = match self.shape {
                            WaveShape::Sine => WaveShape::Square,
                            WaveShape::Square => WaveShape::Triangle,
                            WaveShape::Triangle => WaveShape::Sawtooth,
                            WaveShape::Sawtooth => WaveShape::Sine,
                        };
                    }
                    _ => {}
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if self.focused_field == FocusedField::PlayButton {
                    self.playing.store(true, Ordering::Relaxed);
                    self.last_play_button_press = Instant::now();
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.playing.store(false, Ordering::Relaxed);
                self.should_exit_audio.store(true, Ordering::Relaxed);
                self.should_exit_tui = true;
            }
            _ => {}
        }
    }

    fn handle_key_release(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => {
                if self.focused_field == FocusedField::PlayButton {
                    self.playing.store(false, Ordering::Relaxed);
                }
            }
            _ => {}
        }
    }

    fn check_timeout_release(&mut self) {
        // Detect key release by timeout (for systems like Termux that don't send release events)
        if self.playing.load(Ordering::Relaxed) && self.last_play_button_press.elapsed() > Duration::from_millis(100) {
            self.playing.store(false, Ordering::Relaxed);
        }
    }
}

pub fn run_tui(audio_backend: Option<AudioBackend>, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    let mut app = AppState::new(audio_backend, verbose);
    app.start_audio_thread();

    let result = run_app(terminal, &mut app);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    app.playing.store(false, Ordering::Relaxed);
    app.should_exit_audio.store(true, Ordering::Relaxed);
    if let Some(handle) = app.audio_thread {
        let _ = handle.join();
    }

    result
}

fn run_app(
    mut terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    app.handle_key_event(key);
                } else if key.kind == event::KeyEventKind::Release {
                    app.handle_key_release(key);
                }
            }
        }

        // Detect key release by timeout (for systems like Termux)
        app.check_timeout_release();

        if app.should_exit_tui {
            break;
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
        ])
        .split(f.size());

    // Frequency field
    let freq_style = if app.focused_field == FocusedField::Frequency {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let freq_block = Block::default()
        .title("Frequency (Hz) - Up/Down to adjust")
        .borders(Borders::ALL)
        .style(freq_style);
    let freq_text = format!("{:.0}", app.frequency);
    let freq_para = Paragraph::new(freq_text)
        .block(freq_block)
        .alignment(Alignment::Center);
    f.render_widget(freq_para, chunks[0]);

    // Volume field
    let vol_style = if app.focused_field == FocusedField::Volume {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let vol_block = Block::default()
        .title("Volume - Up/Down to adjust")
        .borders(Borders::ALL)
        .style(vol_style);
    let vol_gauge = Gauge::default()
        .block(vol_block)
        .ratio(app.volume as f64)
        .label(format!("{:.0}%", app.volume * 100.0))
        .style(Style::default().fg(Color::Green));
    f.render_widget(vol_gauge, chunks[1]);

    // Waveform shape
    let shape_style = if app.focused_field == FocusedField::Shape {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let shape_block = Block::default()
        .title("Waveform - Left/Right to change")
        .borders(Borders::ALL)
        .style(shape_style);
    let shape_text = match app.shape {
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
    let play_style = if app.focused_field == FocusedField::PlayButton {
        Style::default().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let play_status = if app.playing.load(Ordering::Relaxed) {
        "PLAYING (Press Space/Enter to release)"
    } else {
        "PRESS SPACE TO PLAY"
    };
    let play_block = Block::default()
        .title("Play Button")
        .borders(Borders::ALL)
        .style(play_style);
    let play_para = Paragraph::new(play_status)
        .block(play_block)
        .alignment(Alignment::Center);
    f.render_widget(play_para, chunks[3]);

    // Instructions
    let instructions = vec![
        Line::from("Controls:"),
        Line::from("  Tab - Switch field"),
        Line::from("  ↑/↓ - Adjust frequency/volume"),
        Line::from("  ←/→ - Change waveform"),
        Line::from("  Space/Enter - Toggle play button"),
        Line::from("  q/Esc - Quit"),
    ];
    let instructions_block = Block::default()
        .title("Instructions")
        .borders(Borders::ALL);
    let instructions_para = Paragraph::new(instructions).block(instructions_block);
    f.render_widget(instructions_para, chunks[4]);
}
