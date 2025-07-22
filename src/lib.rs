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

use std::{sync::Arc, time::Duration};
use crossterm::{
    execute,
    style::{Color, Print},
    terminal::{Clear, ClearType},
    cursor::MoveToColumn,
};
use tokio::{
    sync::{Mutex, Notify},
    task,
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
    config: BarConfig,
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
        let draw_inner = inner.clone();
        let draw_notify = notify.clone();
        let config_clone = config.clone();

        task::spawn(async move {
            loop {
                draw_notify.notified().await;
                let mut state = draw_inner.lock().await;
                if state.finished {
                    break;
                }

                Bar::draw(&state, &config_clone);
                state.color_index = (state.color_index + 1) % config_clone.colors.len();
                drop(state);
            }

            let state = draw_inner.lock().await;
            Bar::draw(&state, &config_clone);
            println!();
        });

        Bar { inner, notify, config }
    }

    pub async fn inc(&self, delta: u64) {
        let mut state = self.inner.lock().await;
        if !state.finished {
            state.current = (state.current + delta).min(state.total);
            if state.current == state.total {
                state.finished = true;
            }
        }
        drop(state);
        self.notify.notify_one();
    }

    pub async fn finish_with_message(&self, msg: &str) {
        let mut state = self.inner.lock().await;
        state.current = state.total;
        state.finished = true;
        state.message = msg.to_string();
        drop(state);
        self.notify.notify_one();
    }

    fn draw(state: &BarState, config: &BarConfig) {
        let width = config.width;
        let progress = (state.current as f64 / state.total as f64).min(1.0);
        let filled_len = (progress * width as f64).round() as usize;
        let bar = "=".repeat(filled_len) + &" ".repeat(width - filled_len);
        let percent = (progress * 100.0).round();

        let color = config.colors.get(state.color_index).unwrap_or(&Color::White);
        let mut stdout = std::io::stdout();
        execute!(
            stdout,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            crossterm::style::SetForegroundColor(*color),
            Print(format!("[{}] {:.0}% {}", bar, percent, state.message)),
            crossterm::style::ResetColor,
        )
        .unwrap();
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
                Color::Green,
                Color::Yellow,
                Color::Magenta,
                Color::Cyan,
                Color::Blue,
                Color::Red,
                Color::White,
                Color::DarkGrey,
            ],
            frame_delay: 150,
        }
    }
}

struct ThrobberState {
    index: usize,
    color_index: usize,
    running: bool,
    message: String,
}

pub struct Throbber {
    inner: Arc<Mutex<ThrobberState>>,
    notify: Arc<Notify>,
    config: ThrobberConfig,
}

impl Throbber {
    pub fn new(msg: impl Into<String>) -> Self {
        Self::with_config(msg, ThrobberConfig::default())
    }

    pub fn with_config(msg: impl Into<String>, config: ThrobberConfig) -> Self {
        let state = ThrobberState {
            index: 0,
            color_index: 0,
            running: true,
            message: msg.into(),
        };

        let inner = Arc::new(Mutex::new(state));
        let notify = Arc::new(Notify::new());
        let config = Arc::new(config);

        let inner_clone = inner.clone();
        let notify_clone = notify.clone();
        let config_clone1 = config.clone();

        task::spawn(async move {
            loop {
                notify_clone.notified().await;
                let state = inner_clone.lock().await;
                if !state.running {
                    break;
                }
                drop(state);
                Throbber::draw(&inner_clone, &config_clone1).await;
            }

            let mut stdout = std::io::stdout();
            execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine)).unwrap();
        });

        let inner_clone2 = inner.clone();
        let notify_clone2 = notify.clone();
        let config_clone2 = config.clone();

        task::spawn(async move {
            while {
                let state = inner_clone2.lock().await;
                state.running
            } {
                {
                    let mut state = inner_clone2.lock().await;
                    state.index = (state.index + 1) % config_clone2.frames.len();
                    state.color_index = (state.color_index + 1) % config_clone2.colors.len();
                }
                notify_clone2.notify_one();
                sleep(Duration::from_millis(config_clone2.frame_delay)).await;
            }
        });

        Throbber {
            inner,
            notify,
            config: Arc::try_unwrap(config).unwrap_or_else(|arc| (*arc).clone()),
        }
    }

    pub async fn stop(&self) {
        {
            let mut state = self.inner.lock().await;
            state.running = false;
        }
        self.notify.notify_one();
    }

    pub async fn start(&self) {
        {
            let mut state = self.inner.lock().await;
            if !state.running {
                state.running = true;
                state.index = 0;
                state.color_index = 0;
            }
        }
        self.notify.notify_one();
    }

    async fn draw(inner: &Arc<Mutex<ThrobberState>>, config: &ThrobberConfig) {
        let state = inner.lock().await;
        let frame = config.frames[state.index];
        let color = config.colors.get(state.color_index).unwrap_or(&Color::White);

        let mut stdout = std::io::stdout();
        execute!(
            stdout,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            crossterm::style::SetForegroundColor(*color),
            Print(format!("{} {}", frame, state.message)),
            crossterm::style::ResetColor,
        )
        .unwrap();
    }
}

