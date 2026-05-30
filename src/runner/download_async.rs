use crate::{
    profiler::RunStats,
    task::task::{CrawlTask, creat_task_queue},
};
use futures::future::join_all;
use std::path::Path;
use std::time::Instant;
use tokio::spawn;

pub async fn download_async_inner_with_tasks(
    tasks: Vec<CrawlTask>,
    output_dir: impl AsRef<Path>,
) -> Result<RunStats, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref().to_path_buf();
    tokio::fs::create_dir_all(&output_dir).await?;
    let total = tasks.len();
    let handles: Vec<_> = tasks
        .into_iter()
        .map(|task| {
            let output = task.output_path_in(output_dir.clone());
            spawn(async move {
                let uni = task.uni.clone();
                let url = task.url.clone();
                let start = Instant::now();
                let result = task.run_async_to(output).await;
                let latency = start.elapsed();
                if let Err(err) = &result {
                    eprintln!("async skip {uni} ({url}): {err}");
                }
                (latency, result)
            })
        })
        .collect();

    let mut latencies = Vec::new();
    let mut failed = 0;
    let results = join_all(handles).await;
    for result in results {
        match result {
            Ok((latency, Ok(()))) => latencies.push(latency),
            Ok((_, Err(_))) => failed += 1,
            Err(_) => failed += 1,
        }
    }
    Ok(RunStats::new(total, latencies, failed))
}

pub async fn download_async_inner() -> Result<RunStats, Box<dyn std::error::Error>> {
    let tasks = creat_task_queue()?;
    download_async_inner_with_tasks(tasks, "output/async").await
}

pub fn download_async_with_tasks(
    tasks: Vec<CrawlTask>,
    output_dir: impl AsRef<Path>,
) -> Result<RunStats, Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(download_async_inner_with_tasks(tasks, output_dir))
}

pub fn download_async() -> Result<RunStats, Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(download_async_inner())
}
