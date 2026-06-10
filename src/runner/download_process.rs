use crate::{
    downloader::downloader_sync::downloader_sync,
    profiler::RunStats,
    runner::effective_concurrency,
    task::task::{CrawlTask, creat_task_queue},
};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const PROCESS_WORKER_ARG: &str = "--spider-process-worker";

struct ChildTask {
    child: Child,
    start: Instant,
    uni: String,
    url: String,
}

pub fn run_process_worker_from_args() -> Result<bool, Box<dyn std::error::Error>> {
    let mut args = env::args_os();
    let _program = args.next();
    if args.next().as_deref() != Some(std::ffi::OsStr::new(PROCESS_WORKER_ARG)) {
        return Ok(false);
    }

    let url = args
        .next()
        .ok_or("missing process worker url")?
        .into_string()
        .map_err(|_| "process worker url is not valid unicode")?;
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing process worker output path")?;

    downloader_sync(&url, output)?;
    Ok(true)
}

pub fn download_process_with_tasks_limited(
    tasks: Vec<CrawlTask>,
    output_dir: impl AsRef<Path>,
    concurrency_limit: Option<usize>,
) -> Result<RunStats, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref().to_path_buf();
    std::fs::create_dir_all(&output_dir)?;
    let worker_exe = env::current_exe()?;
    let total = tasks.len();
    let concurrency = effective_concurrency(concurrency_limit, total);
    let mut pending = tasks.into_iter();
    let mut handles = Vec::new();

    for _ in 0..concurrency {
        if let Some(task) = pending.next() {
            handles.push(spawn_process_task(task, &output_dir, &worker_exe)?);
        }
    }

    let mut latencies = Vec::new();
    let mut failed = 0;
    while !handles.is_empty() {
        let mut index = 0;
        while index < handles.len() {
            if let Some(status) = handles[index].child.try_wait()? {
                let finished = handles.swap_remove(index);
                if status.success() {
                    latencies.push(finished.start.elapsed());
                } else {
                    failed += 1;
                    eprintln!(
                        "process skip {} ({}): worker exited with {status}",
                        finished.uni, finished.url
                    );
                }
                if let Some(task) = pending.next() {
                    handles.push(spawn_process_task(task, &output_dir, &worker_exe)?);
                }
            } else {
                index += 1;
            }
        }

        if !handles.is_empty() {
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    Ok(RunStats::new(total, latencies, failed))
}

fn spawn_process_task(
    task: CrawlTask,
    output_dir: &Path,
    worker_exe: &Path,
) -> std::io::Result<ChildTask> {
    let output = task.output_path_in(output_dir.to_path_buf());
    let child = Command::new(worker_exe)
        .arg(PROCESS_WORKER_ARG)
        .arg(&task.url)
        .arg(output)
        .spawn()?;
    Ok(ChildTask {
        child,
        start: Instant::now(),
        uni: task.uni,
        url: task.url,
    })
}

pub fn download_process_with_tasks(
    tasks: Vec<CrawlTask>,
    output_dir: impl AsRef<Path>,
) -> Result<RunStats, Box<dyn std::error::Error>> {
    download_process_with_tasks_limited(tasks, output_dir, None)
}

pub fn download_process() -> Result<RunStats, Box<dyn std::error::Error>> {
    let tasks = creat_task_queue()?;
    download_process_with_tasks(tasks, "output/process")
}
