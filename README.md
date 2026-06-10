# Practice for the async programming in Rust

## Task Description

Given a xlsx, extracting the url and university name, output the corresponding main page in plane text.

## File structure

.
├── Cargo.lock
├── Cargo.toml
├── README.md
└── src
    ├── bin // Separate entry points for each model
    │   ├── async.rs
    │   ├── process.rs
    │   └── thread.rs
    ├── downloader // Downloader of different model
    │   ├── downloader_async.rs
    │   ├── downloader_sync.rs
    │   ├── parser.rs
    │   └── mod.rs
    ├── lib.rs
    ├── main.rs
    ├── profiler // Benchmarking, cache input and CSV output
    │   ├── downloader.rs
    │   ├── logic.rs
    │   ├── mod.rs
    │   └── output.rs
    ├── runner // The runner of different model
    │   ├── download_async.rs
    │   ├── download_process.rs
    │   ├── download_thread.rs
    │   └── mod.rs
    └── task // The structure and method for downloading task
        ├── mod.rs
        └── task.rs

## Tech details

### Concurrent with process

In this sector, we use the `std::process::Command` to spawn a lot of `curl` process for the concurrent downloading. This approach is relatively heavyweight because each task runs as an independent operating system process.

### Concurrent with thread

We use the `std::thread::spawn` as our concurrent run time. This model is based on native operating system threads. It is lighter than the process-based model and requires only minimal modifications to the naive implementation.implementation.

### Concurrent with coroutine

We use the `tokio` as our async run time. It is built on the model of the coroutine.
Tokio is built on lightweight asynchronous tasks (coroutines) scheduled in user space. Compared with the thread-based model, it usually consumes significantly fewer system resources because tasks are cooperatively scheduled and do not require blocking operating system threads while waiting for I/O.

### Profiler

The profiler compares the process, thread and coroutine crawlers with cached local HTTP input. Each benchmark runs 100 times. Scalar fields in `Result.csv` are reported as averages across those runs, while latency p50, p90 and p99 are computed with the percentile definition over all completed task latency samples.

Set `SPIDER_CONCURRENCY` to cap the number of downloads running at the same time. When it is not set, the runners keep the original behavior and run all tasks concurrently. A limited run writes its report to `Result_<limit>.csv`, such as `Result_10.csv`.

```sh
SPIDER_CONCURRENCY=10 cargo run
SPIDER_CONCURRENCY=10 cargo run --bin async
SPIDER_CONCURRENCY=10 cargo run --bin thread
SPIDER_CONCURRENCY=10 cargo run --bin process
```
