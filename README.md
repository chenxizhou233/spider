# Practice for the async programming in Rust

## Task Description

Given a xlsx, extracting the url and university name, output the corresponding main page in plane text.

## File structure

For the async contagion, we have two branch for the implementation:

### Process / Threads

.
├── Cargo.lock
├── Cargo.toml
├── README.md
├── src
│   ├── download_process.rs // The process version of concurrent downloading
│   ├── download_thread.rs // The thread version of concurrent downloading
│   ├── downloader.rs // The naive implementation of downloading
│   ├── main.rs
│   └── task.rs // The abstraction of tasks
└── 高校名称和官方网站.xlsx // The task xlsx

### Tokio Coroutine

.
├── Cargo.lock
├── Cargo.toml
├── README.md
├── src
│   ├── download_thread.rs // The function that creates the coroutines.
│   ├── downloader.rs // The async version of the naive downloading action
│   ├── main.rs // The async main function
│   └── task.rs // The struct of the Task
└── 高校名称和官方网站.xlsx

## Tech details

### Concurrent with process

In this sector, we use the `std::process::Command` to spawn a lot of `curl` process for the concurrent downloading. This approach is relatively heavyweight because each task runs as an independent operating system process.

### Concurrent with thread

We use the `std::thread::spawn` as our concurrent run time. This model is based on native operating system threads. It is lighter than the process-based model and requires only minimal modifications to the naive implementation.implementation.

### Concurrent with coroutine

We use the `tokio` as our async run time. It is built on the model of the coroutine.
Tokio is built on lightweight asynchronous tasks (coroutines) scheduled in user space. Compared with the thread-based model, it usually consumes significantly fewer system resources because tasks are cooperatively scheduled and do not require blocking operating system threads while waiting for I/O.
