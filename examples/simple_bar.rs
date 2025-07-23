use throbberous::Bar;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Simple Progress Bar Demo:");
    
    let bar = Bar::new(100);
    
    for _i in 0..100 {
        bar.inc(1).await;
        sleep(Duration::from_millis(50)).await;
        // Messages automatically change: "Working..." -> "Quarter done" -> "Halfway done" -> "Almost there..." -> "Complete!"
    }
    
    bar.finish().await;
    println!("Done!");
}
