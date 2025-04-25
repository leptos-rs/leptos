mod fixtures;

use anyhow::Result;
use cucumber::World;
use fixtures::world::AppWorld;

#[tokio::main]
async fn main() -> Result<()> {
    AppWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit("./features")
        .await;
    Ok(())
}
