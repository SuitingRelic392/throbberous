use throbberous::Bar;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Indeterminate Progress Bar Demo:");
    
    let loading = Bar::indeterminate("Working...");
    // Shows bouncing animation: [    ====    ]
    sleep(Duration::from_secs(3)).await;
    loading.finish().await;
    println!("Done!");
}
