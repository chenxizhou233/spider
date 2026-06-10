use spider::profiler::{print_summary_csv_report_to_path, record_usage_repeated, result_csv_path};
use spider::runner::{
    concurrency_limit_from_env, download_thread::download_thread_with_tasks_limited,
};
use spider::task::task::creat_task_queue;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tasks = creat_task_queue()?;
    let concurrency_limit = concurrency_limit_from_env()?;
    let report = record_usage_repeated("thread", 100, || {
        download_thread_with_tasks_limited(tasks.clone(), "output/thread", concurrency_limit)
    })?;
    print_summary_csv_report_to_path(&[("thread", report)], result_csv_path(concurrency_limit))?;
    Ok(())
}
