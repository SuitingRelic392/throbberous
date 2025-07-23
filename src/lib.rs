//! throbberous
//!
//! An async-native CLI progress bar and throbber (spinner) library for Rust.
//!
//! # Example
//!
//! ```rust
//! use throbberous::Throbber;
//! use tokio_test::block_on;
//!
//! block_on(async {
//!     let throbber = Throbber::new("Testing...");
//!     throbber.start().await;
//!     // Simulate work...
//!     throbber.stop().await;
//! });
//! ```

use std::{io, sync::Arc, time::Duration};
use crossterm::{
    execute,
    style::{Color, Print, SetForegroundColor, ResetColor},
    terminal::{Clear, ClearType},
    cursor::MoveToColumn,
};
use tokio::{
    sync::{Mutex, Notify},
    task::{self, JoinHandle},
    time::sleep,
};

// --- Progress Bar Implementation ---

#[derive(Clone)]
pub struct BarConfig {
    pub colors: Vec<Color>,
    pub color_cycle_delay: u64,
    pub width: usize,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            colors: vec![Color::Green, Color::Yellow, Color::Magenta, Color::Cyan],
            color_cycle_delay: 600,
            width: 40,
        }
    }
}

struct BarState {
    current: u64,
    total: u64,
    finished: bool,
    message: String,
    color_index: usize,
}

pub struct Bar {
    inner: Arc<Mutex<BarState>>,
    notify: Arc<Notify>,
    _draw_task: JoinHandle<()>,
}

impl Bar {
    pub fn new(total: u64) -> Self {
        Self::with_config(total, BarConfig::default())
    }

    pub fn with_config(total: u64, config: BarConfig) -> Self {
        let state = BarState {
            current: 0,
            total,
            finished: false,
            message: String::new(),
            color_index: 0,
        };

        let inner = Arc::new(Mutex::new(state));
        let notify = Arc::new(Notify::new());
        
        let draw_task = Self::spawn_draw_task(inner.clone(), notify.clone(), config);

        Bar { 
            inner, 
            notify, 
            _draw_task: draw_task,
        }
    }

    fn spawn_draw_task(
        inner: Arc<Mutex<BarState>>, 
        notify: Arc<Notify>, 
        config: BarConfig
    ) -> JoinHandle<()> {
        task::spawn(async move {
            let mut stdout = io::stdout();
            
            loop {
                notify.notified().await;
                let mut state = inner.lock().await;
                
                if state.finished {
                    Self::draw_bar(&state, &config, &mut stdout);
                    println!();
                    break;
                }

                Self::draw_bar(&state, &config, &mut stdout);
                state.color_index = (state.color_index + 1) % config.colors.len();
            }
        })
    }

    pub async fn inc(&self, delta: u64) {
        let mut state = self.inner.lock().await;
        if !state.finished {
            state.current = (state.current + delta).min(state.total);
            if state.current == state.total {
                state.finished = true;
            }
            drop(state);
            self.notify.notify_one();
        }
    }

    pub async fn finish_with_message(&self, msg: &str) {
        {
            let mut state = self.inner.lock().await;
            state.current = state.total;
            state.finished = true;
            state.message = msg.to_string();
        }
        self.notify.notify_one();
    }

    fn draw_bar(state: &BarState, config: &BarConfig, stdout: &mut io::Stdout) {
        let progress = (state.current as f64 / state.total as f64).min(1.0);
        let filled_len = (progress * config.width as f64).round() as usize;
        
        let color = config.colors.get(state.color_index).unwrap_or(&Color::White);
        let percent = (progress * 100.0).round();

        let _ = execute!(
            stdout,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(*color),
            Print(format!(
                "[{:=<filled$}{:width$}] {:.0}% {}", 
                "", 
                "", 
                percent, 
                state.message,
                filled = filled_len,
                width = config.width - filled_len
            )),
            ResetColor,
        );
    }
}

// --- Throbber (Spinner) Implementation ---

#[derive(Clone)]
pub struct ThrobberConfig {
    pub frames: Vec<&'static str>,
    pub colors: Vec<Color>,
    pub frame_delay: u64,
}

impl Default for ThrobberConfig {
    fn default() -> Self {
        Self {
            frames: vec!["|", "/", "-", "\\"],
            colors: vec![
                Color::Green, Color::Yellow, Color::Magenta, Color::Cyan,
                Color::Blue, Color::Red, Color::White, Color::DarkGrey,
            ],
            frame_delay: 150,
        }
    }
}

struct ThrobberState {
    frame_index: usize,
    color_index: usize,
    running: bool,
    message: String,
}

pub struct Throbber {
    inner: Arc<Mutex<ThrobberState>>,
    _draw_task: JoinHandle<()>,
    _animate_task: JoinHandle<()>,
}

impl Throbber {
    pub fn new(msg: impl Into<String>) -> Self {
        Self::with_config(msg, ThrobberConfig::default())
    }

    pub fn with_config(msg: impl Into<String>, config: ThrobberConfig) -> Self {
        let state = ThrobberState {
            frame_index: 0,
            color_index: 0,
            running: false, // Start stopped
            message: msg.into(),
        };

        let inner = Arc::new(Mutex::new(state));
        let notify = Arc::new(Notify::new());
        
        let draw_task = Self::spawn_draw_task(inner.clone(), notify.clone(), config.clone());
        let animate_task = Self::spawn_animate_task(inner.clone(), notify, config);

        Throbber {
            inner,
            _draw_task: draw_task,
            _animate_task: animate_task,
        }
    }

    fn spawn_draw_task(
        inner: Arc<Mutex<ThrobberState>>, 
        notify: Arc<Notify>, 
        config: ThrobberConfig
    ) -> JoinHandle<()> {
        task::spawn(async move {
            let mut stdout = io::stdout();
            
            loop {
                notify.notified().await;
                let state = inner.lock().await;
                
                if !state.running {
                    let _ = execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine));
                    break;
                }
                
                Self::draw_frame(&state, &config, &mut stdout);
            }
        })
    }

    fn spawn_animate_task(
        inner: Arc<Mutex<ThrobberState>>, 
        notify: Arc<Notify>, 
        config: ThrobberConfig
    ) -> JoinHandle<()> {
        task::spawn(async move {
            loop {
                sleep(Duration::from_millis(config.frame_delay)).await;
                
                let running = {
                    let mut state = inner.lock().await;
                    if !state.running {
                        false
                    } else {
                        state.frame_index = (state.frame_index + 1) % config.frames.len();
                        state.color_index = (state.color_index + 1) % config.colors.len();
                        true
                    }
                };
                
                if !running {
                    break;
                }
                
                notify.notify_one();
            }
        })
    }

    pub async fn start(&self) {
        {
            let mut state = self.inner.lock().await;
            if !state.running {
                state.running = true;
                state.frame_index = 0;
                state.color_index = 0;
            }
        }
    }

    pub async fn stop(&self) {
        {
            let mut state = self.inner.lock().await;
            state.running = false;
        }
        println!("\nFinished");
    }

    fn draw_frame(state: &ThrobberState, config: &ThrobberConfig, stdout: &mut io::Stdout) {
        let frame = config.frames[state.frame_index];
        let color = config.colors.get(state.color_index).unwrap_or(&Color::White);

        let _ = execute!(
            stdout,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(*color),
            Print(format!("{} {}", frame, state.message)),
            ResetColor,
        );
    }
}
