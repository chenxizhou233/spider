use spider::profiler::{print_summary_csv_report, record_usage_repeated};
use spider::runner::download_process::{download_process, run_process_worker_from_args};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if run_process_worker_from_args()? {
        return Ok(());
    }

    let report = record_usage_repeated("process", 100, download_process)?;
    print_summary_csv_report(&[("process", report)])?;
    Ok(())
}
