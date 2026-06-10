use spider::profiler::{print_summary_csv_report_to_path, record_usage_repeated, result_csv_path};
use spider::runner::{
    concurrency_limit_from_env,
    download_process::{download_process_with_tasks_limited, run_process_worker_from_args},
};
use spider::task::task::creat_task_queue;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if run_process_worker_from_args()? {
        return Ok(());
    }

    let tasks = creat_task_queue()?;
    let concurrency_limit = concurrency_limit_from_env()?;
    let report = record_usage_repeated("process", 100, || {
        download_process_with_tasks_limited(tasks.clone(), "output/process", concurrency_limit)
    })?;
    print_summary_csv_report_to_path(&[("process", report)], result_csv_path(concurrency_limit))?;
    Ok(())
}
