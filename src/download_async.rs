use crate::task::creat_task_queue;
use futures::future::join_all;
use tokio::spawn;

pub async fn download_thread() -> Result<(), Box<dyn std::error::Error>> {
    let tasks = creat_task_queue().unwrap();
    let handles: Vec<_> = tasks
        .into_iter()
        .map(|x| spawn(async move { x.run_async().await }))
        .collect();
    let results = join_all(handles).await;
    for result in results {
        result??;
    }
    Ok(())
}
