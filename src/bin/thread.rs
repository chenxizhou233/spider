use spider::runner::download_thread::download_thread;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let result = download_thread();
    let elapsed = start.elapsed();

    match &result {
        Ok(_) => println!("thread: {:.3?}", elapsed),
        Err(err) => eprintln!("thread: failed after {:.3?}: {err}", elapsed),
    }

    result
}
