# throbberous

An async-native CLI progress bar and throbber (spinner) library for Rust.
## Features

- Async-friendly progress bars and throbbers
- Customizable colors and animation speeds
- Easy to integrate into any Rust async project
- Minimal dependencies (`tokio`, `crossterm`)

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
throbberous = "0.1.4"
tokio = { version = "1", features = ["full"] }
crossterm = "0.29"
