use spider::profiler::{print_summary_csv_report, record_usage_repeated};
use spider::runner::download_thread::download_thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = record_usage_repeated("thread", 100, download_thread)?;
    print_summary_csv_report(&[("thread", report)])?;
    Ok(())
}
