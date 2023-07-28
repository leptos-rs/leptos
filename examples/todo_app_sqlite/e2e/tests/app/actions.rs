use super::world::HOST;
use crate::app::world::AppWorld;
use anyhow::Result;

pub async fn goto_path(world: &mut AppWorld, path: &str) -> Result<()> {
    let url = format!("{}{}", HOST, path);
    let client = &world.client;

    client.goto(&url).await?;
    let _: () = client.wait().for_url(url::Url::parse(&url)?).await?;

    Ok(())
}
