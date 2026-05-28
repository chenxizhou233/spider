use spider::runner::download_process::download_process;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let result = download_process();
    let elapsed = start.elapsed();

    match &result {
        Ok(_) => println!("process: {:.3?}", elapsed),
        Err(err) => eprintln!("process: failed after {:.3?}: {err}", elapsed),
    }

    result
}
