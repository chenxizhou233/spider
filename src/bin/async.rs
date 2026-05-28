use spider::runner::download_async::download_async;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let result = download_async();
    let elapsed = start.elapsed();

    match &result {
        Ok(_) => println!("async: {:.3?}", elapsed),
        Err(err) => eprintln!("async: failed after {:.3?}: {err}", elapsed),
    }

    result
}
