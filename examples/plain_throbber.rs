use throbberous::Throbber;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Throbber");
    
    let throbber = Throbber::new_plain();
    throbber.start().await;
    // Spins: | / - \ with "Working..."
    sleep(Duration::from_secs(5)).await;
    throbber.stop().await;
}
