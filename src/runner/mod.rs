pub mod download_async;
pub mod download_process;
pub mod download_thread;

pub fn concurrency_limit_from_env() -> Result<Option<usize>, Box<dyn std::error::Error>> {
    match std::env::var("SPIDER_CONCURRENCY") {
        Ok(value) if value.trim().is_empty() => Ok(None),
        Ok(value) => {
            let limit = value
                .parse::<usize>()
                .map_err(|_| "SPIDER_CONCURRENCY must be a positive integer")?;
            if limit == 0 {
                return Err("SPIDER_CONCURRENCY must be greater than 0".into());
            }
            Ok(Some(limit))
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(Box::new(err)),
    }
}

pub(crate) fn effective_concurrency(limit: Option<usize>, total: usize) -> usize {
    limit.unwrap_or(total).max(1).min(total.max(1))
}
