# throbberous

An async-native CLI progress bar and throbber (spinner) library for Rust.

To run: either cargo run --example progress_and_spinner or cargo run --example progress_and_spinner --  --throbber  

## Features

- Async-friendly progress bars and spinners (throbbers)
- Customizable colors and animation speeds
- Easy to integrate into any Rust async project
- Minimal dependencies (`tokio`, `crossterm`)

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
throbberous = "0.1.0"

