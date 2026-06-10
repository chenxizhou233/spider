use spider::profiler::{
    prepare_cached_input, print_summary_csv_report_to_path, record_usage_repeated, result_csv_path,
};
use spider::runner::{
    concurrency_limit_from_env, download_async::download_async_with_tasks_limited,
    download_process::download_process_with_tasks_limited,
    download_process::run_process_worker_from_args,
    download_thread::download_thread_with_tasks_limited,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if run_process_worker_from_args()? {
        return Ok(());
    }

    let input = prepare_cached_input()?;
    let tasks = input.tasks().to_vec();
    let runs = 100;
    let concurrency_limit = concurrency_limit_from_env()?;

    let results = [
        (
            "process",
            record_usage_repeated("process", runs, || {
                download_process_with_tasks_limited(
                    tasks.clone(),
                    "output/process",
                    concurrency_limit,
                )
            })?,
        ),
        (
            "thread",
            record_usage_repeated("thread", runs, || {
                download_thread_with_tasks_limited(
                    tasks.clone(),
                    "output/thread",
                    concurrency_limit,
                )
            })?,
        ),
        (
            "async",
            record_usage_repeated("async", runs, || {
                download_async_with_tasks_limited(tasks.clone(), "output/async", concurrency_limit)
            })?,
        ),
    ];

    print_summary_csv_report_to_path(&results, result_csv_path(concurrency_limit))?;

    Ok(())
}
