use std::path::Path;
use std::time::Duration;

pub fn downloader_sync(url: &String, output: impl AsRef<Path>) -> anyhow::Result<()> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?;

    let bytes = client
        .get(url)
        .headers(super::default_headers())
        .send()?
        .error_for_status()?
        .bytes()?;

    if let Some(parent) = output.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(output, bytes)?;

    Ok(())
}
