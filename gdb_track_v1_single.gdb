## v1: single-concurrency full await transition trace.
## Focus: SPIDER_CONCURRENCY=1, observe one task through the major await points.
set pagination off
set confirm off
set print pretty on
set print frame-arguments all
set logging file /tmp/spider_gdb_raw.log
set logging overwrite on
set logging enabled on
set env SPIDER_CONCURRENCY 1
set $hits = 0

break src/runner/download_async.rs:16
commands
silent
set $hits = $hits + 1
printf "\n=== hit %d: outer future reaches tokio::fs::create_dir_all(...).await (download_async.rs:16) ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi(self/future?)=%p rsi(cx?)=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 8
info args
info locals
printf "memory at rdi/future candidate:\n"
if $rdi > 0x10000
x/32gx $rdi
end
printf "memory at rsi/context candidate:\n"
if $rsi > 0x10000
x/12gx $rsi
end
continue
end

break src/runner/download_async.rs:17
commands
silent
set $hits = $hits + 1
printf "\n=== hit %d: resumed after create_dir_all await, computing total (download_async.rs:17) ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi=%p rsi=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 8
info args
info locals
if $rdi > 0x10000
x/32gx $rdi
end
continue
end

break src/runner/download_async.rs:25
commands
silent
set $hits = $hits + 1
printf "\n=== hit %d: per-task async block reaches task.run_async_to(output).await (download_async.rs:25) ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi=%p rsi(cx?)=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 10
info args
info locals
printf "future/task memory candidate at rdi:\n"
if $rdi > 0x10000
x/40gx $rdi
end
printf "context/waker memory candidate at rsi:\n"
if $rsi > 0x10000
x/16gx $rsi
end
continue
end

break src/downloader/downloader_async.rs:13
commands
silent
set $hits = $hits + 1
printf "\n=== hit %d: reqwest RequestBuilder::send().await about to be polled (downloader_async.rs:13-14) ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi=%p rsi(cx?)=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 10
info args
info locals
if $rdi > 0x10000
x/40gx $rdi
end
if $rsi > 0x10000
x/16gx $rsi
end
continue
end

break src/downloader/downloader_async.rs:15
commands
silent
set $hits = $hits + 1
printf "\n=== hit %d: resumed after send().await; status check follows (downloader_async.rs:15) ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi=%p rsi=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 10
info args
info locals
if $rdi > 0x10000
x/40gx $rdi
end
continue
end

break src/downloader/downloader_async.rs:16
commands
silent
set $hits = $hits + 1
printf "\n=== hit %d: response.bytes().await about to be polled (downloader_async.rs:16-17) ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi=%p rsi(cx?)=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 10
info args
info locals
if $rdi > 0x10000
x/40gx $rdi
end
if $rsi > 0x10000
x/16gx $rsi
end
continue
end

break src/downloader/downloader_async.rs:19
commands
silent
set $hits = $hits + 1
printf "\n=== hit %d: resumed after bytes().await; parent dir check (downloader_async.rs:19) ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi=%p rsi=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 10
info args
info locals
if $rdi > 0x10000
x/40gx $rdi
end
continue
end

break src/downloader/downloader_async.rs:20
commands
silent
set $hits = $hits + 1
printf "\n=== hit %d: inner tokio::fs::create_dir_all(parent).await (downloader_async.rs:20) ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi=%p rsi(cx?)=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 10
info args
info locals
if $rdi > 0x10000
x/40gx $rdi
end
if $rsi > 0x10000
x/16gx $rsi
end
continue
end

break src/downloader/downloader_async.rs:23
commands
silent
set $hits = $hits + 1
printf "\n=== hit %d: tokio::fs::write(output, bytes).await (downloader_async.rs:23) ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi=%p rsi(cx?)=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 10
info args
info locals
if $rdi > 0x10000
x/40gx $rdi
end
if $rsi > 0x10000
x/16gx $rsi
end
if $hits > 18
  quit
end
continue
end

run
