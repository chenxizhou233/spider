use crate::{
    profiler::RunStats,
    task::task::{CrawlTask, creat_task_queue},
};
use std::path::Path;
use std::thread::spawn;
use std::time::Instant;

pub fn download_thread_with_tasks(
    tasks: Vec<CrawlTask>,
    output_dir: impl AsRef<Path>,
) -> Result<RunStats, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref().to_path_buf();
    std::fs::create_dir_all(&output_dir)?;
    let total = tasks.len();
    let handles: Vec<_> = tasks
        .into_iter()
        .map(|task| {
            let output = task.output_path_in(output_dir.clone());
            spawn(move || {
                let start = Instant::now();
                let result = task.run_sync_to(output);
                (start.elapsed(), result)
            })
        })
        .collect();

    let mut latencies = Vec::new();
    let mut failed = 0;
    for handle in handles {
        match handle.join() {
            Ok((latency, Ok(()))) => latencies.push(latency),
            Ok((_, Err(err))) => {
                failed += 1;
                eprintln!("thread task failed: {err}");
            }
            Err(_) => failed += 1,
        }
    }
    Ok(RunStats::new(total, latencies, failed))
}

pub fn download_thread() -> Result<RunStats, Box<dyn std::error::Error>> {
    let tasks = creat_task_queue()?;
    download_thread_with_tasks(tasks, "output/thread")
}
