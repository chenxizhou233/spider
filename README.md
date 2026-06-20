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

## Tokio Async State Machine Trace Notes

The full dynamic trace is in `track.log`. The reproducible GDB scripts are under `gdb_scripts/`, and raw GDB logs are under `logs/`:

- `gdb_scripts/gdb_track_v1_single.gdb`: single-concurrency trace for the main `await` poll/resume transitions.
- `gdb_scripts/gdb_track_v2_multi4.gdb`: concurrency-4 trace for observing multiple child future state machines.
- `gdb_scripts/gdb_track.gdb`: current default script, matching the concurrency-4 version.

### Call Chain And State Machine Layers

The Tokio implementation follows this core chain:

```text
Runtime::block_on
-> download_async_inner_with_tasks_limited
-> collect::<Vec<_>>().await
-> BufferUnordered::poll_next
-> FuturesUnordered::poll_next
-> per-task async move block
-> CrawlTask::run_async_to
-> downloader_async
-> reqwest send / bytes / tokio fs child futures
```

At runtime this is not one single coroutine object. It is a nested set of future state machines: the outer scheduling future stores the task list and `buffer_unordered` state; `FuturesUnordered` stores multiple child futures; each child future stores its own `task`, `url`, `output`, reqwest request state, and file-writing state.

### Observed `await` Transitions

In the single-concurrency trace, the key transitions are:

- `download_async.rs:16 -> 17`: after `tokio::fs::create_dir_all(&output_dir).await`, the outer state machine resumes and computes `tasks.len()`.
- `download_async.rs:25`: the per-task future is repeatedly polled; while reqwest I/O is not ready, the child future returns `Pending`.
- `downloader_async.rs:13-14 -> 15`: after `send().await`, the downloader state machine resumes and runs `error_for_status()`.
- `downloader_async.rs:16-17 -> 19`: after `bytes().await`, GDB locals show the `Bytes` `ptr/len/vtable`.
- `downloader_async.rs:20` and `23`: these correspond to the async filesystem futures for directory creation and file writing.

The practical meaning of `await` here is: the current future stores the locals it will need later in its state-machine frame, polls a child future, returns `Pending` if the child is not ready, and resumes from the post-`await` state when the waker causes the runtime to poll it again.

### Registers And Variable Addresses

On x86_64 SysV, the first function arguments are usually passed in `rdi/rsi/rdx/rcx/r8/r9`. At a true `Future::poll` boundary, this roughly means:

- `rdi`: `Pin<&mut Future>` or a future/frame pointer.
- `rsi`: `&mut Context`, which is tied to the waker.

The trace uses source-line breakpoints, not exact `poll` function-entry breakpoints, so registers sometimes hold compiler temporaries. The reliable interpretation comes from combining registers with backtraces, `info locals`, and `_task_context`.

Important single-concurrency addresses include:

- Outer poll `Context`: `0x7fffffffaff8`.
- Outer `output_dir = "output/async"` state area: `0x555556351ce0`, where nearby memory contains `output/async`.
- Per-task `_task_context`: `0x7fffffff9c70`.
- Per-task captured-variable area: `0x555556355d10`, corresponding to `task/url/output`.
- `bytes.ptr`: `0x55555642e850`.
- `bytes.len`: `110683`.
- `bytes.vtable`: `0x555556303788`.

These addresses show that Rust async state machines store data needed across `await` points, such as `output_dir`, `task`, `url`, `output`, and `bytes`, inside future frames instead of relying on an ordinary function stack frame staying alive.

### Multi-Concurrency State Storage

In the concurrency-4 trace, `buffer_unordered(4)` keeps four child futures alive at the same time. The first four tasks have distinct captured-state areas:

```text
Peking University              -> 0x555556355d10
Tsinghua University            -> 0x555556353f10
Renmin University of China     -> 0x555556353b00
Beijing Normal University      -> 0x555556340590
```

During later scheduling, `rsi` rotates among these addresses, indicating which task's async block is currently being polled. Meanwhile, `rdi=0x7fffffff9c68` and `rdx=0x7fffffff9c90` look more like the current poll's stack frame/context rather than heap state owned by a specific task.

This confirms the storage model for concurrent Tokio tasks:

- The same async block state-machine type can have multiple live instances.
- Each instance has its own captured-variable storage.
- `FuturesUnordered` stores these child futures and switches among them through a ready queue and wakers.
- The outer `Context` and poll stack can be reused; the child future's stored state address is what identifies the task.

### Waker Notes

The symbol table exposes the `FuturesUnordered` RawWaker / ArcWake machinery, including symbols such as:

```text
futures_util::stream::futures_unordered::task::waker_ref::clone_arc_raw
futures_util::stream::futures_unordered::task::waker_ref::wake_arc_raw
futures_util::stream::futures_unordered::task::waker_ref::wake_by_ref_arc_raw
futures_util::stream::futures_unordered::task::waker_ref::drop_arc_raw
futures_util::stream::futures_unordered::task::waker_ref::waker_vtable
<...Task... as futures_task::arc_wake::ArcWake>::wake
<tokio::runtime::park::UnparkThread>::unpark
```

At the lowest level, a `Waker` is backed by a `RawWaker`, which can be viewed as:

```text
RawWaker {
  data:   *const (),
  vtable: *const RawWakerVTable,
}
```

The `RawWakerVTable` is a virtual-function-table-like set of function pointers. It is not the same type as a Rust trait-object vtable, but the dispatch idea is similar: the caller does not need to know the concrete task type and instead calls through function pointers in the table. The important slots are:

```text
clone       -> clone_arc_raw
wake        -> wake_arc_raw
wake_by_ref -> wake_by_ref_arc_raw
drop        -> drop_arc_raw
```

In this program, `FuturesUnordered` wraps each child future in a task node. `RawWaker.data` points to that task node, while `RawWaker.vtable` points to the table produced by `waker_vtable::<Task<...>>`. The relevant symbol addresses observed in the binary include:

```text
0x0000000000421130 drop_arc_raw::<Task<...>>
0x0000000000421160 wake_arc_raw::<Task<...>>
0x0000000000421190 waker_vtable::<Task<...>>
0x00000000004211a0 clone_arc_raw::<Task<...>>
0x0000000000421210 wake_by_ref_arc_raw::<Task<...>>
0x00000000003f23a0 <Task<...> as ArcWake>::wake_by_ref
0x00000000003f2640 <Task<...> as ArcWake>::wake
```

So when an I/O event triggers `wake_by_ref`, the call is dispatched through the `RawWakerVTable` slot to `wake_by_ref_arc_raw::<Task<...>>`, which then reaches `<Task<...> as ArcWake>::wake_by_ref` and marks that child future as ready.

When reqwest network I/O or Tokio filesystem I/O is not ready, the child future returns `Pending` and registers a waker. Once the I/O becomes ready, the waker puts the corresponding child task back into the ready queue. The runtime then enters `FuturesUnordered::poll_next` again and polls whichever child future is ready. This is the key mechanism behind Tokio's user-space scheduling of many asynchronous tasks.
