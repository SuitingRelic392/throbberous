use throbberous::{Bar, Throbber};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    
    // Throbber
    println!("\n1. Throbber");
    let throbber = Throbber::new();
    throbber.start().await;
    sleep(Duration::from_secs(5)).await;
    throbber.stop().await;
    
    // Determinate progress
    println!("\n2. Progress Bar:");
    let bar = Bar::new(50);
    for _i in 0..50 {
        bar.inc(1).await;
        sleep(Duration::from_millis(100)).await;
    }
    bar.finish().await;
    
    // Indeterminate progress
    println!("\n3. Indeterminate Bar:");
    let loading = Bar::indeterminate("Working...");
    sleep(Duration::from_secs(6)).await;
    loading.finish().await;
    
    println!("All demos complete!");
}
