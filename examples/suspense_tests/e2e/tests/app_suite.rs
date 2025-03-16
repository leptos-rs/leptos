mod fixtures;

use anyhow::Result;
use cucumber::World;
use fixtures::world::AppWorld;
use std::{ffi::OsStr, fs::read_dir};

#[tokio::main]
async fn main() -> Result<()> {
    // Normally the below is done, but it's now gotten to the point of
    // having a sufficient number of tests where the resource contention
    // of the concurrently running browsers will cause failures on CI.
    // AppWorld::cucumber()
    //     .fail_on_skipped()
    //     .run_and_exit("./features")
    //     .await;

    // Mitigate the issue by manually stepping through each feature,
    // rather than letting cucumber glob them and dispatch all at once.
    for entry in read_dir("./features")? {
        let path = entry?.path();
        if path.extension() == Some(OsStr::new("feature")) {
            AppWorld::cucumber()
                .fail_on_skipped()
                .run_and_exit(path)
                .await;
        }
    }
    Ok(())
}
