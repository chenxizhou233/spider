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

The profiler compares the process, thread and coroutine crawlers with cached local HTTP input. It records latency distribution, throughput and RSS memory usage, then writes the result to `Result.csv`.
