use spider::runner::{
    download_async::download_async, download_process::download_process,
    download_thread::download_thread,
};
use std::time::{Duration, Instant};

fn record_time(
    name: &str,
    run: fn() -> Result<(), Box<dyn std::error::Error>>,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let result = run();
    let elapsed = start.elapsed();

    match &result {
        Ok(_) => println!("{name}: {:.3?}", elapsed),
        Err(err) => eprintln!("{name}: failed after {:.3?}: {err}", elapsed),
    }

    result.map(|_| elapsed)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let results = [
        ("process", record_time("process", download_process)?),
        ("thread", record_time("thread", download_thread)?),
        ("async", record_time("async", download_async)?),
    ];

    println!("\nSummary:");
    for (name, elapsed) in results {
        println!("{name}: {:.3?}", elapsed);
    }

    Ok(())
}
