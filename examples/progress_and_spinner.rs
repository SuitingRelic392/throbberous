use throbberous::{Bar, Throbber};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // === Progress Bar Phase ===
    let total = 50;
    let bar = Bar::new(total);

    for _ in 0..total {
        bar.inc(1).await;
        sleep(Duration::from_millis(50)).await; // simulate work
    }

    bar.finish_with_message("Completed loading!").await;
    sleep(Duration::from_millis(500)).await; // pause before throbber

    // === Throbber Phase ===
    let throbber = Throbber::new("Throbbing...");
    throbber.start().await;

    sleep(Duration::from_secs(3)).await; // simulate async task

    throbber.stop().await;

   //println!("");
   //println!("Finished!");
}

