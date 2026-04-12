use crate::audio::AudioPlayer;
use crate::waveform::WaveShape;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph},
};
use std::cell::Cell;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct AppState {
    frequency: Arc<Mutex<f32>>,
    volume: Arc<Mutex<f32>>,
    shape: Arc<Mutex<WaveShape>>,
    playing: Arc<AtomicBool>,
    keep_playing: Arc<AtomicBool>,
    should_exit_tui: bool,
    should_exit_audio: Arc<AtomicBool>,
    focused_field: FocusedField,
    verbose: bool,
    audio_thread: Option<std::thread::JoinHandle<()>>,
    last_play_button_press: Instant,
    // Mouse interaction tracking
    mouse_dragging: bool,
    mouse_start_x: u16,
    freq_chunk_rect: Cell<Rect>,
    vol_chunk_rect: Cell<Rect>,
    shape_chunk_rect: Cell<Rect>,
    play_chunk_rect: Cell<Rect>,
    playtoggle_chunk_rect: Cell<Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FocusedField {
    Frequency,
    Volume,
    Shape,
    PlayButton,
    PlayToggleButton,
}

impl AppState {
    fn new(verbose: bool) -> Self {
        AppState {
            frequency: Arc::new(Mutex::new(440.0)),
            volume: Arc::new(Mutex::new(0.5)),
            shape: Arc::new(Mutex::new(WaveShape::Sine)),
            playing: Arc::new(AtomicBool::new(false)),
            keep_playing: Arc::new(AtomicBool::new(false)),
            should_exit_tui: false,
            should_exit_audio: Arc::new(AtomicBool::new(false)),
            focused_field: FocusedField::Frequency,
            verbose,
            audio_thread: None,
            last_play_button_press: Instant::now(),
            mouse_dragging: false,
            mouse_start_x: 0,
            freq_chunk_rect: Cell::new(Rect::default()),
            vol_chunk_rect: Cell::new(Rect::default()),
            shape_chunk_rect: Cell::new(Rect::default()),
            play_chunk_rect: Cell::new(Rect::default()),
            playtoggle_chunk_rect: Cell::new(Rect::default()),
        }
    }

    fn start_audio_thread(&mut self) {
        let frequency = Arc::clone(&self.frequency);
        let volume = Arc::clone(&self.volume);
        let shape = Arc::clone(&self.shape);
        let verbose = self.verbose;
        let playing = Arc::clone(&self.playing);
        let should_exit = Arc::clone(&self.should_exit_audio);

        let audio_thread = thread::spawn(move || {
            let init_freq = *frequency.lock().unwrap();
            let init_vol = *volume.lock().unwrap();
            let init_shape = *shape.lock().unwrap();

            let player = AudioPlayer::new(init_freq, init_vol, init_shape).with_verbose(verbose);

            let result = player.play_realtime(
                Arc::clone(&playing),
                Arc::clone(&should_exit),
                Some(Arc::clone(&frequency)),
                Some(Arc::clone(&volume)),
                Some(Arc::clone(&shape)),
            );

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
                    FocusedField::PlayButton => FocusedField::PlayToggleButton,
                    FocusedField::PlayToggleButton => FocusedField::Frequency,
                };
            }
            KeyCode::Up => match self.focused_field {
                FocusedField::Frequency => {
                    let mut freq = self.frequency.lock().unwrap();
                    *freq = (*freq + 10.0).min(20000.0);
                }
                FocusedField::Volume => {
                    let mut vol = self.volume.lock().unwrap();
                    *vol = (*vol + 0.05).min(1.0);
                }
                _ => {}
            },
            KeyCode::Down => match self.focused_field {
                FocusedField::Frequency => {
                    let mut freq = self.frequency.lock().unwrap();
                    *freq = (*freq - 10.0).max(20.0);
                }
                FocusedField::Volume => {
                    let mut vol = self.volume.lock().unwrap();
                    *vol = (*vol - 0.05).max(0.0);
                }
                _ => {}
            },
            KeyCode::Left | KeyCode::Char('a') => {
                if self.focused_field == FocusedField::Shape {
                    let mut shape = self.shape.lock().unwrap();
                    *shape = match *shape {
                        WaveShape::Sine => WaveShape::Sawtooth,
                        WaveShape::Square => WaveShape::Sine,
                        WaveShape::Triangle => WaveShape::Square,
                        WaveShape::Sawtooth => WaveShape::Triangle,
                    };
                }
            }
            KeyCode::Right | KeyCode::Char('d') => {
                if self.focused_field == FocusedField::Shape {
                    let mut shape = self.shape.lock().unwrap();
                    *shape = match *shape {
                        WaveShape::Sine => WaveShape::Square,
                        WaveShape::Square => WaveShape::Triangle,
                        WaveShape::Triangle => WaveShape::Sawtooth,
                        WaveShape::Sawtooth => WaveShape::Sine,
                    };
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => match self.focused_field {
                FocusedField::PlayButton => {
                    self.playing.store(true, Ordering::Relaxed);
                    self.last_play_button_press = Instant::now();
                }
                FocusedField::PlayToggleButton => {
                    let currently_playing = self.playing.load(Ordering::Relaxed);
                    self.playing.store(!currently_playing, Ordering::Relaxed);
                    self.keep_playing
                        .store(!currently_playing, Ordering::Relaxed);
                    self.last_play_button_press = Instant::now();
                }
                _ => {}
            },
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
            KeyCode::Char(' ') | KeyCode::Enter => match self.focused_field {
                FocusedField::PlayButton => self.playing.store(false, Ordering::Relaxed),
                FocusedField::PlayToggleButton => {
                    let currently_playing = self.playing.load(Ordering::Relaxed);
                    self.playing.store(!currently_playing, Ordering::Relaxed);
                    self.keep_playing
                        .store(!currently_playing, Ordering::Relaxed);
                    self.last_play_button_press = Instant::now();
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn check_timeout_release(&mut self) {
        // Detect key release by timeout (for systems like Termux that don't send release events)
        if !self.keep_playing.load(Ordering::Relaxed)
            && self.playing.load(Ordering::Relaxed)
            && self.last_play_button_press.elapsed() > Duration::from_millis(100)
        {
            self.playing.store(false, Ordering::Relaxed);
        }
    }

    fn is_in_rect(rect: Rect, x: u16, y: u16) -> bool {
        x >= rect.left() && x < rect.right() && y >= rect.top() && y < rect.bottom()
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                self.mouse_dragging = true;
                self.mouse_start_x = mouse.column;

                // Check if clicked on frequency field
                if Self::is_in_rect(self.freq_chunk_rect.get(), mouse.column, mouse.row) {
                    self.focused_field = FocusedField::Frequency;
                }
                // Check if clicked on volume field
                else if Self::is_in_rect(self.vol_chunk_rect.get(), mouse.column, mouse.row) {
                    self.focused_field = FocusedField::Volume;
                }
                // Check if clicked on shape field
                else if Self::is_in_rect(self.shape_chunk_rect.get(), mouse.column, mouse.row) {
                    self.focused_field = FocusedField::Shape;
                }
                // Check if clicked on play button
                else if Self::is_in_rect(self.play_chunk_rect.get(), mouse.column, mouse.row) {
                    self.focused_field = FocusedField::PlayButton;
                    self.playing.store(true, Ordering::Relaxed);
                    self.last_play_button_press = Instant::now();
                } else if Self::is_in_rect(
                    self.playtoggle_chunk_rect.get(),
                    mouse.column,
                    mouse.row,
                ) {
                    self.focused_field = FocusedField::PlayToggleButton;
                    let currently_playing = self.playing.load(Ordering::Relaxed);
                    self.playing.store(!currently_playing, Ordering::Relaxed);
                    self.keep_playing
                        .store(!currently_playing, Ordering::Relaxed);
                    self.last_play_button_press = Instant::now();
                }
            }
            MouseEventKind::Up(_) => {
                self.mouse_dragging = false;
            }
            MouseEventKind::Drag(_) => {
                if !self.mouse_dragging {
                    return;
                }

                let delta = mouse.column as i16 - self.mouse_start_x as i16;
                self.mouse_start_x = mouse.column;

                match self.focused_field {
                    FocusedField::Frequency => {
                        let mut freq = self.frequency.lock().unwrap();
                        *freq = (*freq + (delta as f32 * 10.0)).clamp(20.0, 20000.0);
                    }
                    FocusedField::Volume => {
                        let mut vol = self.volume.lock().unwrap();
                        *vol = (*vol + (delta as f32 * 0.01)).clamp(0.0, 1.0);
                    }
                    FocusedField::Shape => {
                        // Horizontal drag cycles through shapes
                        if delta > 0 {
                            let mut shape = self.shape.lock().unwrap();
                            *shape = match *shape {
                                WaveShape::Sine => WaveShape::Square,
                                WaveShape::Square => WaveShape::Triangle,
                                WaveShape::Triangle => WaveShape::Sawtooth,
                                WaveShape::Sawtooth => WaveShape::Sine,
                            };
                        } else if delta < 0 {
                            let mut shape = self.shape.lock().unwrap();
                            *shape = match *shape {
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
            MouseEventKind::ScrollUp => match self.focused_field {
                FocusedField::Frequency => {
                    let mut freq = self.frequency.lock().unwrap();
                    *freq = (*freq + 10.0).min(20000.0);
                }
                FocusedField::Volume => {
                    let mut vol = self.volume.lock().unwrap();
                    *vol = (*vol + 0.05).min(1.0);
                }
                _ => {}
            },
            MouseEventKind::ScrollDown => match self.focused_field {
                FocusedField::Frequency => {
                    let mut freq = self.frequency.lock().unwrap();
                    *freq = (*freq - 10.0).max(20.0);
                }
                FocusedField::Volume => {
                    let mut vol = self.volume.lock().unwrap();
                    *vol = (*vol - 0.05).max(0.0);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

pub fn run_tui(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    let mut app = AppState::new(verbose);
    app.start_audio_thread();

    let result = run_app(terminal, &mut app);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

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
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == event::KeyEventKind::Press {
                        app.handle_key_event(key);
                    } else if key.kind == event::KeyEventKind::Release {
                        app.handle_key_release(key);
                    }
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse_event(mouse);
                }
                _ => {}
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
    let frequency = *app.frequency.lock().unwrap();
    let volume = *app.volume.lock().unwrap();
    let shape = *app.shape.lock().unwrap();

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

    // Store chunk rectangles for mouse interaction
    app.freq_chunk_rect.set(chunks[0]);
    app.vol_chunk_rect.set(chunks[1]);
    app.shape_chunk_rect.set(chunks[2]);
    app.play_chunk_rect.set(chunks[3]);
    app.playtoggle_chunk_rect.set(chunks[4]);

    // Frequency field
    let freq_style = if app.focused_field == FocusedField::Frequency {
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
    let vol_style = if app.focused_field == FocusedField::Volume {
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
    let shape_style = if app.focused_field == FocusedField::Shape {
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
    let play_style = if app.focused_field == FocusedField::PlayButton {
        Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let play_status = if app.playing.load(Ordering::Relaxed) {
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
    let playtoggle_style = if app.focused_field == FocusedField::PlayToggleButton {
        Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let playtoggle_status = if app.playing.load(Ordering::Relaxed) {
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
