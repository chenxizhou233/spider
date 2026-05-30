use spider::profiler::{print_csv_report, record_usage};
use spider::runner::download_thread::download_thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = record_usage("thread", download_thread)?;
    print_csv_report(&[("thread", report)])?;
    Ok(())
}
