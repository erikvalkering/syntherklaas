use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
pub struct KeyboardHandler {
    spacebar_pressed: Arc<AtomicBool>,
    should_exit: Arc<AtomicBool>,
    keyboard_thread: Option<std::thread::JoinHandle<()>>,
}

impl KeyboardHandler {
    #[allow(dead_code)]
    pub fn new() -> Self {
        KeyboardHandler {
            spacebar_pressed: Arc::new(AtomicBool::new(false)),
            should_exit: Arc::new(AtomicBool::new(false)),
            keyboard_thread: None,
        }
    }

    #[allow(dead_code)]
    pub fn spacebar_pressed(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.spacebar_pressed)
    }

    #[allow(dead_code)]
    pub fn should_exit(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.should_exit)
    }

    #[allow(dead_code)]
    pub fn start(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let spacebar = Arc::clone(&self.spacebar_pressed);
        let exit = Arc::clone(&self.should_exit);

        // Spawn a thread to handle keyboard input
        let handle = thread::spawn(move || {
            let mut last_spacebar_press = Instant::now();
            let key_repeat_interval = Duration::from_millis(100);

            loop {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    if let Ok(Event::Key(key_event)) = event::read() {
                        match key_event.code {
                            KeyCode::Char(' ') => {
                                match key_event.kind {
                                    event::KeyEventKind::Press => {
                                        spacebar.store(true, Ordering::Relaxed);
                                        last_spacebar_press = Instant::now();
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

                // Detect key release by timeout (for systems like Termux that don't send release events)
                if spacebar.load(Ordering::Relaxed) && last_spacebar_press.elapsed() > key_repeat_interval {
                    spacebar.store(false, Ordering::Relaxed);
                }

                if exit.load(Ordering::Relaxed) {
                    break;
                }
            }
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
        });

        self.keyboard_thread = Some(handle);

        Ok(())
    }

    #[allow(dead_code)]
    pub fn wait_and_cleanup(&mut self) -> io::Result<()> {
        if let Some(handle) = self.keyboard_thread.take() {
            let _ = handle.join();
        }
        
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }
}
