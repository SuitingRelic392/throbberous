//! throbberous
//!
//! An async-native CLI progress bar and throbber (spinner) library for Rust.
//!
//! # Example
//!
//! ```rust
//! use throbberous::{Throbber, Bar};
//! use tokio_test::block_on;
//!
//! block_on(async {
//!     // Regular progress bar
//!     let bar = Bar::new(100);
//!     for _i in 0..100 {
//!         bar.inc(1).await;
//!         tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
//!     }
//!     bar.finish().await;
//!
//!     // Indeterminate progress bar
//!     let loading = Bar::indeterminate("Working...");
//!     tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
//!     loading.finish().await;
//!
//!     // Spinner
//!     let throbber = Throbber::new();
//!     throbber.start().await;
//!     tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
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

#[derive(Clone, Copy)]
pub enum BarMode {
    Determinate { current: u64, total: u64 },
    Indeterminate { position: usize, direction: i8 }, // direction: 1 or -1
}

struct BarState {
    mode: BarMode,
    finished: bool,
    message: String,
    color_index: usize,
}

pub struct Bar {
    inner: Arc<Mutex<BarState>>,
    notify: Arc<Notify>,
    _draw_task: JoinHandle<()>,
    _animate_task: Option<JoinHandle<()>>,
}

impl Bar {
    /// Creates a new determinate progress bar with a known total
    pub fn new(total: u64) -> Self {
        Self::with_config(total, BarConfig::default())
    }

    /// Creates a new determinate progress bar with custom configuration
    pub fn with_config(total: u64, config: BarConfig) -> Self {
        let state = BarState {
            mode: BarMode::Determinate { current: 0, total },
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
            _animate_task: None,
        }
    }

    /// Creates an indeterminate progress bar for unknown duration tasks
    pub fn indeterminate(message: impl Into<String>) -> Self {
        Self::indeterminate_with_config(message, BarConfig::default())
    }

    /// Creates an indeterminate progress bar with custom configuration
    pub fn indeterminate_with_config(message: impl Into<String>, config: BarConfig) -> Self {
        let state = BarState {
            mode: BarMode::Indeterminate { position: 0, direction: 1 },
            finished: false,
            message: message.into(),
            color_index: 0,
        };

        let inner = Arc::new(Mutex::new(state));
        let notify = Arc::new(Notify::new());
        
        let draw_task = Self::spawn_draw_task(inner.clone(), notify.clone(), config.clone());
        let animate_task = Self::spawn_indeterminate_task(inner.clone(), notify.clone(), config);

        Bar { 
            inner, 
            notify, 
            _draw_task: draw_task,
            _animate_task: Some(animate_task),
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

    fn spawn_indeterminate_task(
        inner: Arc<Mutex<BarState>>, 
        notify: Arc<Notify>, 
        config: BarConfig
    ) -> JoinHandle<()> {
        task::spawn(async move {
            let bounce_width = config.width / 4; // Size of the moving block
            
            loop {
                sleep(Duration::from_millis(100)).await;
                
                let finished = {
                    let mut state = inner.lock().await;
                    if state.finished {
                        true
                    } else if let BarMode::Indeterminate { ref mut position, ref mut direction } = state.mode {
                        *position = (*position as i32 + *direction as i32) as usize;
                        
                        // Bounce off the edges
                        if *position >= config.width - bounce_width {
                            *direction = -1;
                            *position = config.width - bounce_width;
                        } else if *position == 0 {
                            *direction = 1;
                        }
                        false
                    } else {
                        true // Wrong mode, stop animating
                    }
                };
                
                if finished {
                    break;
                }
                
                notify.notify_one();
            }
        })
    }

    /// Increment the progress bar by the specified amount (determinate mode only)
    pub async fn inc(&self, delta: u64) {
        let mut state = self.inner.lock().await;
        if !state.finished {
            if let BarMode::Determinate { current, total } = &mut state.mode {
                *current = (*current + delta).min(*total);
                
                // Check if we need to update message and if finished - extract values first
                let progress = *current as f64 / *total as f64;
                let current_val = *current;
                let total_val = *total;
                let message_empty = state.message.is_empty();
                
                // Now we can safely update state without conflicting borrows
                if message_empty {
                    state.message = match progress {
                        p if p >= 1.0 => "Complete!".to_string(),
                        p if p >= 0.75 => "Almost there...".to_string(),
                        p if p >= 0.5 => "Halfway done".to_string(),
                        p if p >= 0.25 => "Quarter done".to_string(),
                        _ => "Working...".to_string(),
                    };
                }
                
                if current_val == total_val {
                    state.finished = true;
                }
            }
        }
        drop(state);
        self.notify.notify_one();
    }

    /// Set the current progress directly (determinate mode only)
    pub async fn set_position(&self, pos: u64) {
        let mut state = self.inner.lock().await;
        if !state.finished {
            if let BarMode::Determinate { current, total } = &mut state.mode {
                *current = pos.min(*total);
                
                // Check if we need to update message and if finished - extract values first
                let progress = *current as f64 / *total as f64;
                let current_val = *current;
                let total_val = *total;
                let message_empty = state.message.is_empty();
                
                // Now we can safely update state without conflicting borrows
                if message_empty {
                    state.message = match progress {
                        p if p >= 1.0 => "Complete!".to_string(),
                        p if p >= 0.75 => "Almost there...".to_string(),
                        p if p >= 0.5 => "Halfway done".to_string(),
                        p if p >= 0.25 => "Quarter done".to_string(),
                        _ => "Working...".to_string(),
                    };
                }
                
                if current_val == total_val {
                    state.finished = true;
                }
            }
        }
        drop(state);
        self.notify.notify_one();
    }

    /// Update the message displayed with the progress bar
    pub async fn set_message(&self, msg: impl Into<String>) {
        {
            let mut state = self.inner.lock().await;
            state.message = msg.into();
        }
        self.notify.notify_one();
    }

    /// Finish the progress bar
    pub async fn finish(&self) {
        {
            let mut state = self.inner.lock().await;
            // Set to 100% if determinate
            if let BarMode::Determinate { ref mut current, total } = state.mode {
                *current = total;
            }
            state.finished = true;
        }
        self.notify.notify_one();
    }

    /// Finish the progress bar with a custom message
    pub async fn finish_with_message(&self, msg: impl Into<String>) {
        {
            let mut state = self.inner.lock().await;
            // Set to 100% if determinate
            if let BarMode::Determinate { ref mut current, total } = state.mode {
                *current = total;
            }
            state.finished = true;
            state.message = msg.into();
        }
        self.notify.notify_one();
    }

    fn draw_bar(state: &BarState, config: &BarConfig, stdout: &mut io::Stdout) {
        let color = config.colors.get(state.color_index).unwrap_or(&Color::White);
        
        let display = match state.mode {
            BarMode::Determinate { current, total } => {
                let progress = if total == 0 { 1.0 } else { (current as f64 / total as f64).min(1.0) };
                let filled_len = (progress * config.width as f64).round() as usize;
                let percent = (progress * 100.0).round();
                
                format!(
                    "[{:=<filled$}{:width$}] {:.0}% {}", 
                    "", 
                    "", 
                    percent, 
                    state.message,
                    filled = filled_len,
                    width = config.width - filled_len
                )
            },
            BarMode::Indeterminate { position, .. } => {
                let bounce_width = config.width / 4;
                let mut bar = vec![' '; config.width];
                
                // Fill the bouncing section
                for i in position..=(position + bounce_width).min(config.width - 1) {
                    if i < config.width {
                        bar[i] = '=';
                    }
                }
                
                format!("[{}] {}", bar.iter().collect::<String>(), state.message)
            }
        };

        let _ = execute!(
            stdout,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(*color),
            Print(display),
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
    pub fn new() -> Self {
        Self::with_config(ThrobberConfig::default())
    }

    pub fn with_config(config: ThrobberConfig) -> Self {
        let state = ThrobberState {
            frame_index: 0,
            color_index: 0,
            running: false,
            message: "Working...".to_string(),
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

    pub async fn set_message(&self, msg: impl Into<String>) {
        {
            let mut state = self.inner.lock().await;
            state.message = msg.into();
        }
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
