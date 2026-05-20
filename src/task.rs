use crate::downloader::download_sync;
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
