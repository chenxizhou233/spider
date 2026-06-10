use crate::{
    profiler::RunStats,
    runner::effective_concurrency,
    task::task::{CrawlTask, creat_task_queue},
};
use std::path::Path;
use std::sync::mpsc;
use std::thread::spawn;
use std::time::{Duration, Instant};

type ThreadTaskResult = (String, String, Duration, anyhow::Result<()>);

pub fn download_thread_with_tasks_limited(
    tasks: Vec<CrawlTask>,
    output_dir: impl AsRef<Path>,
    concurrency_limit: Option<usize>,
) -> Result<RunStats, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref().to_path_buf();
    std::fs::create_dir_all(&output_dir)?;
    let total = tasks.len();
    let concurrency = effective_concurrency(concurrency_limit, total);
    let (tx, rx) = mpsc::channel::<ThreadTaskResult>();
    let mut pending = tasks.into_iter();
    let mut active = 0usize;

    for _ in 0..concurrency {
        if let Some(task) = pending.next() {
            spawn_thread_task(task, output_dir.clone(), tx.clone());
            active += 1;
        }
    }

    let mut latencies = Vec::new();
    let mut failed = 0;
    while active > 0 {
        let (uni, url, latency, result) = rx.recv()?;
        active -= 1;
        match result {
            Ok(()) => latencies.push(latency),
            Err(err) => {
                failed += 1;
                eprintln!("thread skip {uni} ({url}): {err}");
            }
        }

        if let Some(task) = pending.next() {
            spawn_thread_task(task, output_dir.clone(), tx.clone());
            active += 1;
        }
    }
    Ok(RunStats::new(total, latencies, failed))
}

fn spawn_thread_task(
    task: CrawlTask,
    output_dir: std::path::PathBuf,
    tx: mpsc::Sender<ThreadTaskResult>,
) {
    let output = task.output_path_in(output_dir);
    spawn(move || {
        let uni = task.uni.clone();
        let url = task.url.clone();
        let start = Instant::now();
        let result = task.run_sync_to(output);
        let _ = tx.send((uni, url, start.elapsed(), result));
    });
}

pub fn download_thread_with_tasks(
    tasks: Vec<CrawlTask>,
    output_dir: impl AsRef<Path>,
) -> Result<RunStats, Box<dyn std::error::Error>> {
    download_thread_with_tasks_limited(tasks, output_dir, None)
}

pub fn download_thread() -> Result<RunStats, Box<dyn std::error::Error>> {
    let tasks = creat_task_queue()?;
    download_thread_with_tasks(tasks, "output/thread")
}
