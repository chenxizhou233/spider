use spider::profiler::{prepare_cached_input, print_summary_csv_report, record_usage_repeated};
use spider::runner::{
    download_async::download_async_with_tasks, download_process::download_process_with_tasks,
    download_process::run_process_worker_from_args, download_thread::download_thread_with_tasks,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if run_process_worker_from_args()? {
        return Ok(());
    }

    let input = prepare_cached_input()?;
    let tasks = input.tasks().to_vec();
    let runs = 100;

    let results = [
        (
            "process",
            record_usage_repeated("process", runs, || {
                download_process_with_tasks(tasks.clone(), "output/process")
            })?,
        ),
        (
            "thread",
            record_usage_repeated("thread", runs, || {
                download_thread_with_tasks(tasks.clone(), "output/thread")
            })?,
        ),
        (
            "async",
            record_usage_repeated("async", runs, || {
                download_async_with_tasks(tasks.clone(), "output/async")
            })?,
        ),
    ];

    print_summary_csv_report(&results)?;

    Ok(())
}
