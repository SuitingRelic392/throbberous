use throbberous::Bar;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let bar = Bar::new(100);
    for _ in 0..100 {
        sleep(Duration::from_millis(50)).await;
        bar.inc(1).await;
    }
    println!("");
    bar.finish_with_message("Done!").await;
}

