use spider::profiler::{print_csv_report, record_usage};
use spider::runner::download_async::download_async;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = record_usage("async", download_async)?;
    print_csv_report(&[("async", report)])?;
    Ok(())
}
