mod download_thread;
mod downloader;
mod task;

use crate::download_thread::download_thread;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    download_thread().await;
    Ok(())
}
