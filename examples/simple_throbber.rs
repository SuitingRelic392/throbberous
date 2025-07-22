use crank::{Bar, Throbber};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let bar = Bar::new(50);
    for _ in 0..50 {
        sleep(Duration::from_millis(50)).await;
        bar.inc(1).await;
    }
    bar.finish_with_message("Finished!").await;

    let throbber = Throbber::new("Processing next step...");
    sleep(Duration::from_secs(5)).await;
    throbber.stop().await;
}

