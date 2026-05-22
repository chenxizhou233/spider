mod downloader;
mod runner;
mod task;

use crate::runner::{
    download_async::download_async, download_process::download_process,
    download_thread::download_thread,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // download_thread();
    // download_process();
    download_async()
}
