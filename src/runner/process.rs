use crate::task::creat_task_queue;
use std::process::{Child, Command};

pub fn download_process() -> Result<(), Box<dyn std::error::Error>> {
    let tasks = creat_task_queue().unwrap();
    let handles: Vec<Child> = tasks
        .into_iter()
        .map(move |x| {
            Command::new("curl")
                .arg("-L")
                .arg(&x.url)
                .arg("-o")
                .arg(x.output_path())
                .spawn()
                .unwrap()
        })
        .collect();
    for mut handle in handles {
        handle.wait()?;
    }
    Ok(())
}
