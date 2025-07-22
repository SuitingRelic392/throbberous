use crank::Bar;
use tokio::time::{sleep, Duration};

/// Simulated async task that reports progress chunks
async fn simulated_task(bar: &Bar, total_chunks: u64) {
    for _ in 0..total_chunks {
        // Simulate work
        sleep(Duration::from_millis(50)).await;

        // Notify bar about completed chunk
        bar.inc(1).await;
    }

    // Task done — finish the bar for real
    bar.finish_with_message("All done!").await;
}

#[tokio::main]
async fn main() {
    let total_chunks = 100;
    let bar = Bar::new(total_chunks);

    // Run the task and bar progress in the same async flow
    simulated_task(&bar, total_chunks).await;
}

