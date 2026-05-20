mod download_process;
mod download_thread;
mod downloader;
mod task;

use crate::{download_process::download_process, download_thread::download_thread};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // download_thread()
    download_process()
}
