mod downloader;
mod runner;
mod task;

use runner::{coroutine::download_async, process::download_process, thread::download_thread};

enum RuntimeMode {
    Process,
    Thread,
    Coroutine,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = RuntimeMode::Process;

    match mode {
        RuntimeMode::Process => download_process(),
        RuntimeMode::Thread => download_thread(),
        RuntimeMode::Coroutine => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(download_async())
        }
    }
}
