## Tokio 异步状态机跟踪要点

完整动态跟踪记录见 `track.log`，可复现实验脚本包括：

- `gdb_track_v1_single.gdb`：单并发，跟踪主要 `await` 的 poll / resume 流程。
- `gdb_track_v2_multi4.gdb`：并发 4，观察多个子 future 状态机的存储和轮换。
- `gdb_track.gdb`：当前默认脚本，内容对应并发 4 版本。

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
