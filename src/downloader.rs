use std::path::Path;

pub async fn download_async(url: &String, output: impl AsRef<Path>) -> anyhow::Result<()> {
    let bytes = reqwest::get(url).await?.bytes().await?;

    if let Some(parent) = output.as_ref().parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(output, bytes).await?;

    Ok(())
}

pub fn download_sync(url: &String, output: impl AsRef<Path>) -> anyhow::Result<()> {
    let bytes = reqwest::blocking::get(url)?.error_for_status()?.bytes()?;

    if let Some(parent) = output.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(output, bytes)?;

    Ok(())
}
