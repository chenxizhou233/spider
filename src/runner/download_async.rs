use crate::task::task::creat_task_queue;
use futures::future::join_all;
use tokio::spawn;

pub async fn download_async_inner() -> Result<(), Box<dyn std::error::Error>> {
    let tasks = creat_task_queue().unwrap();
    let handles: Vec<_> = tasks
        .into_iter()
        .map(|x| {
            spawn(async move {
                let uni = x.uni.clone();
                let url = x.url.clone();
                if let Err(err) = x.run_async().await {
                    eprintln!("async skip {uni} ({url}): {err}");
                }
            })
        })
        .collect();
    let results = join_all(handles).await;
    for result in results {
        result?;
    }
    Ok(())
}

pub fn download_async() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(download_async_inner())
}
