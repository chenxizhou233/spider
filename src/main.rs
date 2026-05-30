use spider::profiler::{prepare_cached_input, print_csv_report, record_usage};
use spider::runner::{
    download_async::download_async_with_tasks, download_process::download_process_with_tasks,
    download_thread::download_thread_with_tasks,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = prepare_cached_input()?;
    let tasks = input.tasks().to_vec();

    let results = [
        (
            "process",
            record_usage("process", || {
                download_process_with_tasks(tasks.clone(), "output/process")
            })?,
        ),
        (
            "thread",
            record_usage("thread", || {
                download_thread_with_tasks(tasks.clone(), "output/thread")
            })?,
        ),
        (
            "async",
            record_usage("async", || {
                download_async_with_tasks(tasks.clone(), "output/async")
            })?,
        ),
    ];

    print_csv_report(&results)?;

    Ok(())
}
