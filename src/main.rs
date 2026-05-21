mod download_async;
mod downloader_async;
mod task;

use crate::download_async::download_thread;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    download_thread().await;
    Ok(())
}
