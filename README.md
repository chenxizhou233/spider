# Practice for async/concurrent programming in Rust

## Task Description

Given an xlsx file, extract the university name and URL, then download each university homepage into local html files.

## Project Structure

This repository now uses a **single unified structure** for three runtime models (process / thread / coroutine):

```text
.
├── Cargo.lock
├── Cargo.toml
├── README.md
└── src
    ├── downloader
    │   ├── async_reqwest.rs   # Async HTTP download implementation (reqwest + tokio)
    │   ├── mod.rs
    │   └── sync.rs            # Blocking HTTP download implementation
    ├── runner
    │   ├── coroutine.rs       # Coroutine-based concurrent scheduling (tokio tasks)
    │   ├── mod.rs
    │   ├── process.rs         # Process-based concurrent scheduling (curl subprocesses)
    │   └── thread.rs          # Thread-based concurrent scheduling (std::thread)
    ├── task
    │   └── mod.rs             # Task model + xlsx loader + run_sync/run_async entry
    └── main.rs                # Program entry and runtime mode selection
```

## Runtime Models

### Process model

Uses `std::process::Command` to spawn many `curl` subprocesses concurrently. This is the heaviest model because each task is an OS process.

### Thread model

Uses `std::thread::spawn` to execute tasks concurrently. This model is lighter than process-based scheduling while preserving a synchronous implementation style.

### Coroutine model

Uses `tokio` tasks and async I/O (`reqwest` async client). This model is usually the most resource-efficient for large I/O-bound concurrency.

## Notes

- Task data is read from `高校名称和官方网站.xlsx` in the project root.
- Output html files are written to the `output/` directory.
- In `main.rs`, you can switch runtime mode by changing `RuntimeMode`.
