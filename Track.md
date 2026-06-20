## Tokio 异步状态机跟踪要点

完整动态跟踪记录见 `track.log`，可复现实验脚本包括：

- `gdb_scripts/gdb_track_v1_single.gdb`：单并发，跟踪主要 `await` 的 poll / resume 流程，原始输出见 `logs/spider_gdb_raw.log`。
- `gdb_scripts/gdb_track_v2_multi4.gdb`：并发 4，观察多个子 future 状态机的存储和轮换，原始输出见 `logs/spider_gdb_multi_raw.log`。
- `gdb_scripts/gdb_track.gdb`：当前默认脚本，内容对应并发 4 版本。
- `gdb_scripts/gdb_stack_thread_v2.gdb`：thread 版栈 VMA 测量，原始输出见 `logs/gdb_stack_thread_v2_raw.log`。
- `gdb_scripts/gdb_stack_async_v2.gdb`：Tokio 版栈 VMA 测量，原始输出见 `logs/gdb_stack_async_v2_raw.log`。

### MIR 状态机的实验入口

本报告不只按“Rust async 会被编译成状态机”这个理论来解释，而是先看 gdb 实际打印出的状态机名字。`gdb_scripts/gdb_track_v1_single.gdb` 生成的 `logs/spider_gdb_raw.log` 中，第一次命中 `download_async.rs:16` 时，backtrace 第 4 帧直接显示外层 async future 处在 `Unresumed` 状态：

```text
raw log line 24:
download_async_inner_with_tasks_limited::{async_fn_env#0}<&str>::Unresumed {
  tasks: Vec(size=33),
  output_dir: "output/async",
  concurrency_limit: Some(1)
}
```

同一次命中里，locals 又显示了源码变量：

```text
raw log lines 29-34:
output_dir = "output/async"
concurrency_limit = Some(1)
tasks = Vec(size=33)
```

所以这里能由实验直接得到两点：第一，`async fn` 在 gdb 里不是显示成普通阻塞函数调用，而是显示成 `async_fn_env#0` 这种 future 环境；第二，future 环境里确实保存了跨 `await` 还要继续使用的 `tasks`、`output_dir` 和 `concurrency_limit`。

更内层的下载 future 也能看到类似结构。第 29 次命中 `download_async.rs:25` 时，locals 显示：

```text
raw log lines 31815-31816:
__awaitee =
  run_async_to::{async_fn_env#0}<PathBuf>::Suspend0 {
    __awaitee:
      downloader_async::{async_fn_env#0}<PathBuf>::Suspend2 {
        output: "output/async/北京大学.html",
        ...
      }
  }
```

第 29 次命中的后半段继续显示：

```text
raw log lines 32620-32633:
bytes = Bytes { ptr: 0x55555642e850, len: 110683, ... }
__awaitee = tokio::fs::create_dir_all::{async_fn_env#0}::Suspend0 { ... }

raw log lines 32661-32669:
url = "http://www.pku.edu.cn/"
uni = "北京大学"
_task_context = 0x7fffffff9c60
output = "output/async/北京大学.html"
```

这组数据支持的结论是：`Suspend0`、`Suspend2`、`__awaitee` 不是本文人为命名的概念，而是 gdb 从编译产物调试信息里还原出来的状态；`bytes`、`url`、`uni`、`output` 这些跨 `await` 仍要使用的值，也确实出现在对应 future 的 locals 中。后文所有关于“状态机 frame 保存跨 await 变量”的说法，都以这些 gdb locals 为实验依据。具体地址会随每次运行变化，所以本文只用地址判断“是否落在同一类状态区”，不把某个绝对地址当作固定事实。

### 调用链和状态机层次

Tokio 版本的核心调用链为：

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

因此，程序运行时不是只有一个“协程对象”，而是一组嵌套 future 状态机：外层调度 future 保存任务列表和 `buffer_unordered` 状态；`FuturesUnordered` 保存多个子 future；每个子 future 再保存自己的 `task`、`url`、`output`、`reqwest` 请求状态和文件写入状态。

### `await` 切换观察

单并发跟踪中，关键切换点如下：

- `download_async.rs:16 -> 17`：外层 `tokio::fs::create_dir_all(&output_dir).await` 返回后，状态机继续计算 `tasks.len()`。
- `download_async.rs:25`：per-task future 反复被 poll；当 `reqwest` I/O 未 ready 时，子 future 返回 `Pending`。
- `downloader_async.rs:13-14 -> 15`：`send().await` 完成后，状态机恢复并执行 `error_for_status()`。
- `downloader_async.rs:16-17 -> 19`：`bytes().await` 完成后，可以在 gdb locals 中看到 `Bytes` 的 `ptr/len/vtable`。
- `downloader_async.rs:20` 和 `23`：分别对应内部建目录和写文件的异步 fs future。

这说明 `await` 的本质是：当前 future 把后续还需要的局部变量保存到自己的状态机 frame，poll 子 future；如果返回 `Pending`，runtime 暂停该任务；waker 被 I/O 事件触发后，runtime 再次 poll，并按状态跳回 `await` 之后继续执行。

#### 从 v1 single raw log 读状态机

`gdb_scripts/gdb_track_v1_single.gdb` 重新生成的 `logs/spider_gdb_raw.log` 里，单并发链路一共记录了 32 次源码断点命中。虽然脚本名叫 single，但 `tasks` 仍然是完整的 33 个 crawl task；single 指的是 `SPIDER_CONCURRENCY=1`，也就是 `buffer_unordered` 一次只让一个下载子 future 活跃。

第一次进入外层 future 时，gdb 能在 backtrace 参数里看到：

```text
download_async_inner_with_tasks_limited::{async_fn_env#0}<&str>::Unresumed {
  tasks: Vec(size=33),
  output_dir: "output/async",
  concurrency_limit: Some(1)
}
```

这正对应“async fn 被编译成 future frame，但还没有真正跑过”的初始状态。进入下载子任务后，locals 里会出现更具体的暂停状态，例如：

```text
run_async_to::{async_fn_env#0}<PathBuf>::Suspend0 {
  __awaitee: downloader_async::{async_fn_env#0}<PathBuf>::Suspend2 {
    output: "output/async/北京大学.html",
    bytes: Bytes { ... },
    __awaitee: tokio::fs::create_dir_all::{async_fn_env#0}::Suspend0 { ... }
  }
}
```

这段信息很有价值：它不是源码里手写出来的结构，而是 gdb 根据编译器生成的 async 状态机还原出的调试视图。`run_async_to` 自己停在 `Suspend0`，因为它正在等待内部的 `downloader_async(...) .await`；`downloader_async` 停在 `Suspend2`，说明它已经越过了前面的 `send().await` 和 `bytes().await`，现在正持有 `bytes` 并等待内部的 `create_dir_all(parent).await`。

同一份 raw log 后面还能看到：

```text
bytes = bytes::bytes::Bytes {
  ptr: ...,
  len: 110683,
  vtable: 0x555556303788 <bytes::bytes::PROMOTABLE_EVEN_VTABLE>
}
output = "output/async/北京大学.html"
_task_context = 0x7fffffff9c60
```

这把理论和现场连起来了：`bytes` 和 `output` 在 `bytes().await` 之后、`write(output, bytes).await` 之前仍然要用，所以它们被保存在 `downloader_async` 的 future frame 中；`_task_context` 则是这一次 poll 时传进来的 `Context`，用于把 waker 继续传给内部的 reqwest/tokio fs 子 future。

#### 带寄存器的切换例子 1：`send().await` 从网络 I/O 回到 per-task future

在 `downloader_async.rs:13-14` 的 `client.get(url).send().await?` 附近，断点停在 reqwest `send` 子 future 被 poll 的位置时，寄存器现场为：

```text
pc  = 0x5555559df5c1
sp  = 0x7fffffff7020
bp  = 0x7fffffffcee0
rdi = 0x7fffffff8240
rsi = 0x7fffffff8350
rdx = 0x7fffffff8238
rcx = 0x7fffffff81a8
r8  = 0x1
r9  = 0x280
_task_context = 0x7fffffff9c70
url/output 捕获区约在 0x555556355d10 附近
```

这里可以这样读：

- `_task_context = 0x7fffffff9c70` 是当前子 future poll 时使用的 `Context`，里面间接带着 waker。
- `rdi/rsi/rdx/rcx` 在这个源码行断点上更像 reqwest send 子 future 和临时返回槽所在的栈上状态区，不应简单当成源码函数参数。
- `0x555556355d10` 附近是 per-task async block 捕获的 `task/url/output` 状态区，例如北京大学任务的 `output/async/北京大学.html` 就保存在这一组状态里。

如果网络 I/O 还没有 ready，reqwest/hyper 会从 `Context` 中取出 waker 并注册到 I/O reactor。后续 I/O ready 时，调用链在符号层面可以对应到：

```text
RawWakerVTable.wake_by_ref
-> wake_by_ref_arc_raw::<Task<...>>       @ 0x0000000000421210
-> <Task<...> as ArcWake>::wake_by_ref    @ 0x00000000003f23a0
-> 把 FuturesUnordered 的子 task 放回 ready 队列
```

调完 waker 之后，runtime 并不是直接跳回 `downloader_async` 的下一条指令，而是下一轮从外层 poll 链路重新进入：

```text
Runtime::block_on
-> collect::<Vec<_>>().await
-> BufferUnordered::poll_next
-> FuturesUnordered::poll_next
-> per-task async block
-> CrawlTask::run_async_to
-> downloader_async
```

对应再次进入 per-task future 的现场之一是 `download_async.rs:25`：

```text
pc  = 0x5555559d9efe
sp  = 0x7fffffff92f0
bp  = 0x7fffffffcee0
rdi = 0x7fffffff9c48
rsi = 0x555556355d10
rdx = 0x7fffffff9c70
rcx = 0x2e6b51
r8  = 0x2
r9  = 0x7fffffffa868
_task_context = 0x7fffffff9c70
```

这组寄存器很有代表性：`rsi = 0x555556355d10` 正好落到北京大学这个子任务的捕获状态区，`rdx = 0x7fffffff9c70` 仍然指向这次 poll 使用的上下文。也就是说，waker 唤醒的是 `FuturesUnordered` 里的某个子 task；下一轮 poll 进入的是这个 ready 子 task 的 async block，然后才继续向内 poll 到 `run_async_to -> downloader_async`。

#### 带寄存器的切换例子 2：`bytes().await` 从响应体 I/O 回到 `downloader_async`

在 `downloader_async.rs:16-17` 的 `let bytes = response.bytes().await?;` 附近，进入响应体读取 future 时的现场为：

```text
pc  = 0x5555559dfa0e
sp  = 0x7fffffff7020
bp  = 0x7fffffffcee0
rdi = 0x7fffffff8ae8
rsi = 0x7fffffff8ae8
rdx = 0x88
rcx = 0x1
_task_context = 0x7fffffff9c70
```

这里 `rdi/rsi = 0x7fffffff8ae8` 指向当次 poll 中 `bytes()` 子 future 或返回临时值相关的栈上区域；`rdx = 0x88` 更像编译器生成状态/大小/分支处理时用到的临时值；真正稳定地跨 `await` 保存的是外层 `downloader_async` future frame 里的 `response`、`output` 等字段。这个位置如果返回 `Pending`，响应体读取 future 同样通过 `_task_context` 注册 waker。

当响应体 I/O ready 后，waker 的调用仍走 `RawWakerVTable.wake_by_ref -> wake_by_ref_arc_raw::<Task<...>> -> ArcWake::wake_by_ref`，区别在于这次 ready 的是同一个 per-task future 内部的 `bytes()` 子状态。重新 poll 后，状态机不会从 `downloader_async` 函数开头重新执行，而是按照保存的 discriminant 直接恢复到 `bytes().await` 之后。断点在 `downloader_async.rs:19` 的现场为：

```text
pc  = 0x5555559dfc65
sp  = 0x7fffffff7020
bp  = 0x7fffffffcee0
rdi = 0x555556355db8
rsi = 0x7fffffff8b80
rdx = 0x555556303788
rcx = 0x1
bytes.ptr    = 0x55555642e850
bytes.len    = 110683
bytes.vtable = 0x555556303788
parent.inner = 0x555556355cf0
```

这组数据可以和源码变量更直接地对应起来：`bytes.ptr/len/vtable` 就是 `let bytes = ...` 得到的 `Bytes` 值；`rdx = 0x555556303788` 与 `bytes.vtable` 一致，说明当时寄存器里正带着这个异步 body/bytes 相关对象的虚表式指针；`rdi = 0x555556355db8` 落在同一个任务状态区附近，表示恢复后继续操作的仍是这个 downloader future 的保存状态。换句话说，waker 负责把任务重新排队，真正决定“从 `bytes().await` 后面继续跑”的，是 future frame 中保存的状态编号和局部变量。

### 寄存器和变量地址

在 x86_64 SysV ABI 下，函数前几个参数通常通过 `rdi/rsi/rdx/rcx/r8/r9` 传递。对真正的 `Future::poll` 边界，通常可以近似理解为：

- `rdi`：`Pin<&mut Future>` 或 future/frame 指针。
- `rsi`：`&mut Context`，间接关联 waker。

不过本实验的断点打在源码行上，不总是精确停在 `poll` 函数入口，所以寄存器有时只是编译器临时值。更可靠的判断方式是结合 backtrace、`info locals` 和 `_task_context`。

单并发中可对应的关键地址包括：

- 外层 poll `Context`：`0x7fffffffaff8`。
- 外层 `output_dir = "output/async"` 相关状态区：`0x555556351ce0`，附近内存能读出 `output/async`。
- per-task `_task_context`：`0x7fffffff9c70`。
- per-task 捕获变量区：`0x555556355d10`，对应 `task/url/output`。
- `bytes.ptr`：`0x55555642e850`。
- `bytes.len`：`110683`。
- `bytes.vtable`：`0x555556303788`。

这些地址表明 Rust async 状态机会把 `output_dir`、`task`、`url`、`output`、`bytes` 等跨 `await` 仍需使用的数据保存到 future frame 中，而不是依赖普通函数栈帧一直存在。

### 多并发状态机存储

并发 4 跟踪中，`buffer_unordered(4)` 同时保留 4 个子 future。前四个任务分别出现了不同的捕获状态区：

```text
北京大学       -> 0x555556355d10
清华大学       -> 0x555556353f10
中国人民大学   -> 0x555556353b00
北京师范大学   -> 0x555556340590
```

后续调度中，`rsi` 会在这些地址之间轮换，表示当前被 poll 的是哪个任务的 async block；而 `rdi=0x7fffffff9c68` 和 `rdx=0x7fffffff9c90` 更像当前 poll 调用的栈上 frame/context，并不是某个任务独占的堆上状态区。

这验证了多并发 Tokio 程序的存储模型：

- 同一种 async block 状态机可以同时存在多个实例。
- 每个实例都有独立的捕获变量存储区。
- `FuturesUnordered` 保存这些子 future，并通过 ready queue 和 waker 在它们之间切换。
- 外层 `Context`/poll 栈可以复用，真正区分任务的是子 future 自己的状态存储地址。

### Thread 与 Tokio task 的栈使用对比

这个项目里 thread 版本和 Tokio 版本的并发形态有一个关键差别：

```text
thread 版本:
std::thread::spawn
-> 每个活跃下载任务对应一个 OS thread
-> 每个 OS thread 有自己的线程栈
-> 阻塞 I/O 等待时，整个线程连同调用栈一起被挂起

Tokio 版本:
Runtime::new().block_on(...)
-> worker thread 负责反复 poll 多个 future
-> async task 没有独立 OS 栈
-> await 之后还要用的变量保存在 future frame / task node 中
```

在 thread 版本中，`spawn_thread_task` 会为每个活跃任务创建一个真正的系统线程。`task.run_sync_to(output)` 内部如果阻塞在网络或文件 I/O 上，保存现场主要依赖 OS thread 自己的栈和内核调度：函数调用链、返回地址、栈上局部变量会随着这个线程一起停住。恢复时也是同一个线程继续执行，所以从程序模型上看更像“调用栈没有离开，只是线程睡着了”。

Tokio task 则不是这样。Rust async future 是 stackless coroutine：每次被 poll 时，它临时借用当前 worker thread 的普通调用栈；一旦遇到 `Pending`，这次 poll 调用返回，`sp/bp` 对应的栈帧可以被后续 poll 复用。gdb 中反复出现的 `0x7fffffff...` 地址，例如：

```text
sp  = 0x7fffffff7020
bp  = 0x7fffffffcee0
_task_context = 0x7fffffff9c70
```

更像“当前这次 poll 正在使用的线程栈现场”。这些地址不唯一属于某一个下载任务；下一次 poll 另一个 future 时，同一段 worker 栈仍然可以被使用。

真正区分 Tokio 子任务的是 future frame / 捕获状态区，例如并发 4 时看到的：

```text
北京大学       -> 0x555556355d10
清华大学       -> 0x555556353f10
中国人民大学   -> 0x555556353b00
北京师范大学   -> 0x555556340590
```

这些状态区保存了跨 `await` 仍然需要的内容：`task`、`url`、`output`、`response`、`bytes`，以及“现在应该从哪个 await 之后恢复”的状态编号。也就是说，Tokio 的“协程堆栈”并不是一条连续的私有栈，而是编译器生成的状态机对象；局部变量被拆成 future frame 里的字段，调用栈只在 poll 期间短暂存在。

可以把两者压缩成下面这个对比：

```text
OS thread:
  并发单位      = 内核线程
  保存现场      = 线程寄存器 + 独立线程栈
  阻塞 I/O      = 挂起整个线程
  局部变量位置  = 普通调用栈为主
  切换成本      = 内核调度 / 线程上下文切换

Tokio task:
  并发单位      = Future 状态机实例
  保存现场      = future frame + waker + ready queue
  阻塞 I/O      = 返回 Pending，线程继续 poll 别的 task
  局部变量位置  = 跨 await 的变量进 future frame；临时变量在 poll 栈上
  切换成本      = 用户态 poll 返回/再次 poll，必要时由 reactor 触发 waker
```

所以这里观测到的栈使用结论是：thread 版本用“多个独立 OS 栈”换取直观的阻塞式控制流；Tokio 版本用“少量 worker 栈 + 多个堆上 future 状态机”承载大量并发。`0x7fffffff...` 一类地址反映的是当前线程正在 poll 的瞬时栈；`0x555556...` 一类任务捕获区才更接近 Tokio task 自己的持久状态。

#### 实际测试数据验证

这部分结论后来又用两组运行数据验证了一次：

```text
cargo test
SPIDER_CONCURRENCY=20 cargo run
SPIDER_CONCURRENCY=20 target/debug/thread   # 外部 /proc 采样
SPIDER_CONCURRENCY=20 target/debug/async    # 外部 /proc 采样
```

`cargo test` 通过。完整基准在 `SPIDER_CONCURRENCY=20` 下跑了 100 轮，输出到 `Result_20.csv`。这次可用缓存任务数为 29，三种模型的结果为：

```text
model    runs  total  completed  elapsed_avg_ms  throughput_avg  peak_rss_max_bytes
process  100   29.00  29.00      406.106         71.58           251650048
thread   100   29.00  29.00      362.992         80.08           52621312
async    100   29.00  29.00      1122.478        25.84           49872896
```

这组性能数据不能简单理解成“async 一定更快”。在当前缓存/网络环境里，thread 版本的平均完成时间更短，Tokio 版本反而更慢；但这里讨论的是栈和状态保存方式，不是绝对吞吐胜负。对栈使用更直接的证据来自 `/proc` 线程采样：

```text
thread 完整运行采样: peak_threads = 47
thread 短时直接采样: peak_threads = 45
async  完整运行采样: peak_threads = 7
async  短时直接采样: peak_threads = 7
```

这里的 `thread` 峰值明显高于设置的并发 20，是因为进程里还包含主线程、profiler 采样线程、缓存 HTTP server 相关线程，以及下载 worker 线程。关键点是：thread 版本会随着活跃下载任务额外增加大量 OS thread；Tokio 版本则稳定在少量 runtime/helper 线程上，用这些线程轮流 poll 多个 future。

短时采样还读取了 `/proc/<pid>/task/<tid>/maps` 里的 `[stack]` 映射。由于线程创建和退出很快，按所有 TID 聚合的 stack map 数会有竞态，不能当作精确峰值；但它能辅助确认每个被采到的 OS thread 都有自己的栈映射。结合前面的 gdb 地址现象，可以得到更稳妥的判断：Tokio 的并发任务没有各自独立的 OS 栈，持久状态主要落在 future frame；thread 版本的并发任务则以 OS thread 和线程栈为单位保存阻塞现场。

#### GDB 堆栈大小实测

当前环境中主线程栈限制为：

```text
ulimit -s = 8192 KiB = 8 MiB
RUST_MIN_STACK 未设置
代码中没有自定义 thread_stack_size
```

为了避免只靠默认值估算，后面又用 `rust-gdb` 做了一次现场测量。使用的脚本是：

```text
gdb_scripts/gdb_stack_thread_v2.gdb
gdb_scripts/gdb_stack_async_v2.gdb
```

脚本逻辑是：在 gdb 里暂停程序，遍历 `info threads` 中的每个线程，切换到该线程后读取 `$rsp`，再到 `/proc/<pid>/maps` 里找到包含这个 `$rsp` 的 VMA。这样得到的是“当前线程栈指针实际落在哪一段栈映射里”，比单纯 grep `[stack]` 更可靠。

thread 版本停在 `download_thread.rs:33`，也就是首批 `SPIDER_CONCURRENCY=20` 个 worker spawn 完之后：

```text
SUMMARY model=thread
gdb_threads=52
unique_stack_vmas=52
mapped_stack_bytes_by_rsp=107237376
mapped_stack_mib_by_rsp=102.270
```

其中主线程当前 `[stack]` VMA 是 `136 KiB`，其余线程大多是 `2048 KiB` 左右的匿名栈映射，例如：

```text
thread#1  stack_vma size = 136.0 KiB
thread#3  stack_vma size = 2048.0 KiB
thread#4  stack_vma size = 2048.0 KiB
...
thread#52 stack_vma size = 2048.0 KiB
```

async 版本停在 `download_async.rs:25`，即一个 Tokio child future 正在被 poll：

```text
SUMMARY model=async
gdb_threads=7
unique_stack_vmas=7
mapped_stack_bytes_by_rsp=12730368
mapped_stack_mib_by_rsp=12.141
```

对应的栈映射结构是 1 个当前主栈 VMA 加 6 个约 `2 MiB` 的 runtime/helper 线程栈：

```text
thread#1 stack_vma size = 136.0 KiB
thread#2 stack_vma size = 2056.0 KiB
thread#3 stack_vma size = 2048.0 KiB
thread#4 stack_vma size = 2048.0 KiB
thread#5 stack_vma size = 2048.0 KiB
thread#6 stack_vma size = 2048.0 KiB
thread#7 stack_vma size = 2048.0 KiB
```

因此，这次 gdb 实测得到的 OS 线程栈映射大小对比是：

```text
thread 栈 VMA 总量 = 107237376 bytes = 102.270 MiB
async  栈 VMA 总量 =  12730368 bytes =  12.141 MiB

差值 = 94507008 bytes = 90.129 MiB
比例 = 107237376 / 12730368 = 8.42x
```

这个比例比前面按线程数粗估的 `4.8x - 5.0x` 更大，原因是 gdb 断点停在 thread 版本首批任务刚启动、reqwest blocking 内部线程和 Tokio helper 线程也已经出现的瞬间；也就是说，thread 版并不只是“20 个下载 worker + 主线程”，还额外带出了一批 `reqwest-interna` 和 `tokio-rt-worker` 线程。

这也解释了为什么 async task 的 stackless 特性更关键：`SPIDER_CONCURRENCY=20` 下，Tokio 的 20 个并发下载 task 没有变成 20 条独立 OS 栈；它们的跨 `await` 状态在 future frame 中，poll 时复用这 7 个线程的栈。thread 版本则在 gdb 现场出现了 52 个可见 OS 线程，对应约 `102.270 MiB` 的栈 VMA。

完整基准中的 RSS 也能看到这个方向，但幅度较小：

```text
thread peak_rss_max_bytes = 52621312  ≈ 50.2 MiB
async  peak_rss_max_bytes = 49872896  ≈ 47.6 MiB
```

RSS 只差约 `2.6 MiB`，是因为大部分线程栈只是地址空间/VMA 预留，并没有全部被写入提交；真正能明显拉开的是“可用/预留的 OS 栈空间”和线程数量。gdb 的 `$rsp -> VMA` 结果更适合回答“每种模型实际保留了多少线程栈空间”。

### Waker 相关结论

符号表中能看到 `FuturesUnordered` 的 RawWaker / ArcWake 相关函数，例如：

```text
futures_util::stream::futures_unordered::task::waker_ref::clone_arc_raw
futures_util::stream::futures_unordered::task::waker_ref::wake_arc_raw
futures_util::stream::futures_unordered::task::waker_ref::wake_by_ref_arc_raw
futures_util::stream::futures_unordered::task::waker_ref::drop_arc_raw
futures_util::stream::futures_unordered::task::waker_ref::waker_vtable
<...Task... as futures_task::arc_wake::ArcWake>::wake
<tokio::runtime::park::UnparkThread>::unpark
```

`Waker` 底层可以理解成两部分：

```text
RawWaker {
  data:   *const (),
  vtable: *const RawWakerVTable,
}
```

这里的 `vtable` 可以看作一张“虚函数表式”的函数指针表。它和 trait object 的 vtable 不是同一个类型，但思想很像：调用方并不知道具体 task 类型，只通过表里的函数指针执行对应操作。`RawWakerVTable` 里核心槽位是：

```text
clone       -> clone_arc_raw
wake        -> wake_arc_raw
wake_by_ref -> wake_by_ref_arc_raw
drop        -> drop_arc_raw
```

在本程序中，`FuturesUnordered` 会把每个子 future 包装成自己的 task node。`RawWaker.data` 指向这个 task node，`RawWaker.vtable` 指向由 `waker_vtable::<Task<...>>` 生成/返回的函数表。符号表中相关地址如下：

```text
0x0000000000421130 drop_arc_raw::<Task<...>>
0x0000000000421160 wake_arc_raw::<Task<...>>
0x0000000000421190 waker_vtable::<Task<...>>
0x00000000004211a0 clone_arc_raw::<Task<...>>
0x0000000000421210 wake_by_ref_arc_raw::<Task<...>>
0x00000000003f23a0 <Task<...> as ArcWake>::wake_by_ref
0x00000000003f2640 <Task<...> as ArcWake>::wake
```

也就是说，当 I/O 事件触发 `wake_by_ref` 时，实际路径不是直接调用一个写死的函数，而是通过 `RawWakerVTable` 取出 `wake_by_ref` 槽位对应的函数指针，再调用到 `wake_by_ref_arc_raw::<Task<...>>`，最终进入 `Task<...> as ArcWake>::wake_by_ref`，把这个子 future 标记为 ready。

当 reqwest 网络 I/O 或 tokio fs I/O 未 ready 时，子 future 返回 `Pending` 并注册 waker；I/O ready 后，waker 将对应子 task 放回 ready 队列；runtime 再次进入 `FuturesUnordered::poll_next`，选择 ready 的子 future 继续 poll。这就是 Tokio 在用户态调度大量异步任务的关键机制。
