use std::path::Path;

pub fn downloader_sync(url: &String, output: impl AsRef<Path>) -> anyhow::Result<()> {
    let bytes = reqwest::blocking::get(url)?.error_for_status()?.bytes()?;

    if let Some(parent) = output.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(output, bytes)?;

    Ok(())
}
