use std::path::Path;

pub async fn downloader_async(url: &String, output: impl AsRef<Path>) -> anyhow::Result<()> {
    let bytes = reqwest::get(url).await?.bytes().await?;

    if let Some(parent) = output.as_ref().parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(output, bytes).await?;

    Ok(())
}
