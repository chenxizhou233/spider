use crate::downloader::download_sync;
use calamine::{Reader, open_workbook_auto};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CrawlTask {
    pub uni: String,
    pub url: String,
}

impl CrawlTask {
    pub fn output_path(&self) -> PathBuf {
        PathBuf::from("output").join(format!("{}.html", (&self.uni)))
    }
    pub fn run(&self) -> anyhow::Result<()> {
        download_sync(&self.url, self.output_path())
    }
}
pub fn creat_task_queue() -> anyhow::Result<Vec<CrawlTask>> {
    let mut tasks: Vec<CrawlTask> = vec![];
    let mut workbook =
        open_workbook_auto("/Users/chenxizhou/CS/2026s-OS/AsyncOS/spider/高校名称和官方网站.xlsx")?;
    let range = workbook.worksheet_range_at(0).unwrap()?;
    for row in range.rows().skip(1) {
        let uni = row.first().map(|c| c.to_string()).unwrap_or_default();
        let url = row.get(1).map(|c| c.to_string()).unwrap_or_default();
        tasks.push(CrawlTask { uni, url });
    }
    Ok(tasks)
}
