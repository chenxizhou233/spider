# Practice for the async programming in Rust

## Task Description

Given a xlsx, extracting the url and university name, output the corresponding main page in plane text.

## File structure

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

## Tech details

### Concurrent with process

In this sector, we use the `std::process::Command` to spawn a lot of `curl` process for the concurrent downloading. It is heavier.

### Concurrent with thread

We use the `std::thread::spawn` as our concurrent run time. It is based on the thread model. It is litter than the process model and it requires little modification on the naive implementation.
