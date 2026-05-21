use std::path::Path;
use tokio::fs;

pub async fn download_async(url: &String, output: impl AsRef<Path>) -> anyhow::Result<()> {
    let bytes = reqwest::get(url).await?.bytes().await?;

    if let Some(parent) = output.as_ref().parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::write(output, bytes).await?;

    Ok(())
}
