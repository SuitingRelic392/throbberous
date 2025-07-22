#[tokio::test]
async fn test_bar() {
    let bar = throbberous::Bar::new(100);
    for _ in 0..100 {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        bar.inc(1).await;
    }
    bar.finish_with_message("Done!").await;
}

