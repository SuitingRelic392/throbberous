use crank::{Bar, BarConfig, Throbber, ThrobberConfig};
use crossterm::style::Color;
use std::{env, time::Duration};
use tokio::time::sleep;

// Constants controlling color cycle timing and frame delay
const COLOR_CYCLE_DELAY_MS: u64 = 600;      // How often colors change
const THROBBER_FRAME_DELAY_MS: u64 = 150;   // Throbber frame update interval

#[tokio::main]
async fn main() {
    // Collect CLI args
    let args: Vec<String> = env::args().collect();

    // Determine which parts to run
    // Default: run both if no flags are given
    let run_bar = args.contains(&"--bar".to_string()) || !args.iter().any(|a| a.starts_with("--"));
    let run_throbber = args.contains(&"--throbber".to_string()) || !args.iter().any(|a| a.starts_with("--"));

    if run_bar {
        let bar_config = BarConfig {
            colors: vec![
                Color::Cyan,
                Color::Green,
                Color::Yellow,
                Color::Magenta,
            ],
            color_cycle_delay: COLOR_CYCLE_DELAY_MS,
            width: 40,
        };

        let bar = Bar::with_config(100, bar_config);

        for _ in 0..100 {
            bar.inc(1).await;
            sleep(Duration::from_millis(25)).await;
        }

        bar.finish_with_message("Done with progress bar!").await;
    }

    if run_throbber {
        let throbber_config = ThrobberConfig {
            frames: vec!["|", "/", "-", "\\"],
            colors: vec![
                Color::Cyan,
                Color::Green,
                Color::Yellow,
                Color::Magenta,
                Color::Blue,
                Color::Red,
                Color::White,
                Color::DarkGrey,
            ],
            frame_delay: THROBBER_FRAME_DELAY_MS,
        };

        let throbber = Throbber::with_config("Throbbing...", throbber_config);

        throbber.start().await;

        sleep(Duration::from_secs(3)).await;

        throbber.stop().await;

        println!("Finished!");
    }
}

