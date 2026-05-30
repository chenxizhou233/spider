use spider::profiler::{print_csv_report, record_usage};
use spider::runner::download_process::{download_process, run_process_worker_from_args};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if run_process_worker_from_args()? {
        return Ok(());
    }

    let report = record_usage("process", download_process)?;
    print_csv_report(&[("process", report)])?;
    Ok(())
}
