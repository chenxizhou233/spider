use std::path::Path;
use std::time::Duration;

pub async fn downloader_async(url: &String, output: impl AsRef<Path>) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?;

    let bytes = client
        .get(url)
        .headers(super::default_headers())
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    if let Some(parent) = output.as_ref().parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(output, bytes).await?;

    Ok(())
}
