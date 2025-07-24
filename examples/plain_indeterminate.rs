use throbberous::Bar;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Indeterminate Progress Bar:");

    let loading = Bar::indeterminate_plain("Working...");
    // Shows bouncing animation: [    ====    ]
    sleep(Duration::from_secs(6)).await;
    loading.finish().await;
    println!("Done!");
}

