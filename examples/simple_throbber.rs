use throbberous::{Bar, Throbber};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let throbber = Throbber::new("Throbbing...");
    throbber.start().await;

    sleep(Duration::from_secs(3)).await; // simulate async task

    throbber.stop().await;

    println!("");
    println!("Finished!");

}

