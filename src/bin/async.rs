use spider::profiler::{print_summary_csv_report, record_usage_repeated};
use spider::runner::download_async::download_async;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = record_usage_repeated("async", 100, download_async)?;
    print_summary_csv_report(&[("async", report)])?;
    Ok(())
}
