use crate::task::task::creat_task_queue;
use std::thread::spawn;

pub fn download_thread() -> Result<(), Box<dyn std::error::Error>> {
    let tasks = creat_task_queue().unwrap();
    let handles: Vec<_> = tasks
        .into_iter()
        .map(|x| spawn(move || x.run_sync()))
        .collect();
    for handle in handles {
        let _ = handle.join().unwrap();
    }
    Ok(())
}
