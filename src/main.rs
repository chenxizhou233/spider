use spider::runner::download_async::download_async;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 默认入口保留 async 版本，独立可执行文件见：
    // - src/bin/process.rs
    // - src/bin/thread.rs
    // - src/bin/async.rs
    download_async()
}
