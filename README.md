# Practice for the async programming in Rust

## Task Description

Given a xlsx, extracting the url and university name, output the corresponding main page in plane text.

## File structure

├── Cargo.lock
├── Cargo.toml
├── README.md
├── src
│   ├── download_thread.rs // The thread version of concurrent downloading
│   ├── downloader.rs // The naive implementation of downloading
│   ├── main.rs. 
│   └── task.rs. // The abstraction of tasks
└── 高校名称和官方网站.xlsx // The task xlsx

## Tech details

### Concurrent with thread

We use the `std::thread::spawn` as our concurrent run time. It is based on the thread model. So the cost is a little bit expensive. But it requires little modification on the naive implementation.
