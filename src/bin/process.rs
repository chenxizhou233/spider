use spider::profiler::{print_csv_report, record_usage};
use spider::runner::download_process::download_process;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = record_usage("process", download_process)?;
    print_csv_report(&[("process", report)])?;
    Ok(())
}
