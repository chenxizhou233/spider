use crate::{
    profiler::RunStats,
    task::task::{CrawlTask, creat_task_queue},
};
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

struct ChildTask {
    child: Child,
    start: Instant,
    uni: String,
    url: String,
}

pub fn download_process_with_tasks(
    tasks: Vec<CrawlTask>,
    output_dir: impl AsRef<Path>,
) -> Result<RunStats, Box<dyn std::error::Error>> {
    let output_dir = output_dir.as_ref().to_path_buf();
    std::fs::create_dir_all(&output_dir)?;
    let total = tasks.len();
    let mut handles: Vec<ChildTask> = tasks
        .into_iter()
        .map(move |task| {
            let output = task.output_path_in(output_dir.clone());
            let child = Command::new("curl")
                .arg("-sS")
                .arg("-L")
                .arg(&task.url)
                .arg("-o")
                .arg(output)
                .spawn()?;
            Ok::<_, std::io::Error>(ChildTask {
                child,
                start: Instant::now(),
                uni: task.uni,
                url: task.url,
            })
        })
        .collect::<Result<_, _>>()?;

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
                        "process skip {} ({}): curl exited with {status}",
                        finished.uni, finished.url
                    );
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

pub fn download_process() -> Result<RunStats, Box<dyn std::error::Error>> {
    let tasks = creat_task_queue()?;
    download_process_with_tasks(tasks, "output/process")
}
