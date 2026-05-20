use crate::task::CrawlTask;
use calamine::{Reader, open_workbook_auto};
use std::{thread::spawn, vec};

pub fn download_thread() -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<CrawlTask> = vec![];
    let mut workbook =
        open_workbook_auto("/Users/chenxizhou/CS/2026s-OS/AsyncOS/spider/高校名称和官方网站.xlsx")?;
    let range = workbook.worksheet_range_at(0).unwrap()?;
    for row in range.rows().skip(1) {
        let uni = row.get(0).map(|c| c.to_string()).unwrap_or_default();
        let url = row.get(1).map(|c| c.to_string()).unwrap_or_default();
        tasks.push(CrawlTask { uni, url });
    }
    let handles: Vec<_> = tasks.into_iter().map(|x| spawn(move || x.run())).collect();
    for handle in handles {
        handle.join().unwrap();
    }
    Ok(())
}
