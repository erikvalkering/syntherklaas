use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct KeyboardHandler {
    spacebar_pressed: Arc<AtomicBool>,
    should_exit: Arc<AtomicBool>,
}

impl KeyboardHandler {
    pub fn new() -> Self {
        KeyboardHandler {
            spacebar_pressed: Arc::new(AtomicBool::new(false)),
            should_exit: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn spacebar_pressed(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.spacebar_pressed)
    }

    pub fn should_exit(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.should_exit)
    }

    pub fn start(&self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        eprintln!("Real-time synthesizer mode");
        eprintln!("Press and hold SPACEBAR to play");
        eprintln!("Press Ctrl+C to exit\n");

        let spacebar = Arc::clone(&self.spacebar_pressed);
        let exit = Arc::clone(&self.should_exit);

        // Spawn a thread to handle keyboard input
        thread::spawn(move || {
            loop {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    if let Ok(Event::Key(key_event)) = event::read() {
                        match key_event.code {
                            KeyCode::Char(' ') => {
                                // Toggle on space press (since termux might not support release events)
                                match key_event.kind {
                                    event::KeyEventKind::Press => {
                                        spacebar.store(true, Ordering::Relaxed);
                                    }
                                    event::KeyEventKind::Release => {
                                        spacebar.store(false, Ordering::Relaxed);
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::Char('c') if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                exit.store(true, Ordering::Relaxed);
                                break;
                            }
                            KeyCode::Esc => {
                                exit.store(true, Ordering::Relaxed);
                                break;
                            }
                            _ => {}
                        }
                    }
                }

                if exit.load(Ordering::Relaxed) {
                    break;
                }
            }
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
        });

        Ok(())
    }

    pub fn cleanup(&self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }
}
