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
}
