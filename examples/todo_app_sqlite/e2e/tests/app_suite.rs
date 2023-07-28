mod app;

use anyhow::Result;
use app::world::AppWorld;
use cucumber::World;

#[tokio::main]
async fn main() -> Result<()> {
    AppWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit("./features")
        .await;
    Ok(())
}
