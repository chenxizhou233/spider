## v2: multi-concurrency state-machine storage trace.
## Focus: SPIDER_CONCURRENCY=4, observe several FuturesUnordered child futures.
set pagination off
set confirm off
set print pretty on
set print frame-arguments all
set logging file /tmp/spider_gdb_multi_raw.log
set logging overwrite on
set logging enabled on
set env SPIDER_CONCURRENCY 4
set $hits = 0

break src/runner/download_async.rs:25
commands
silent
set $hits = $hits + 1
printf "\n=== multi hit %d: per-task future at task.run_async_to(output).await ===\n", $hits
printf "pc=%p sp=%p bp=%p rdi=%p rsi=%p rdx=%p rcx=%p r8=%p r9=%p\n", $pc, $rsp, $rbp, $rdi, $rsi, $rdx, $rcx, $r8, $r9
bt 8
info locals
printf "future/frame candidate at rdi:\n"
if $rdi > 0x10000
x/32gx $rdi
end
printf "captured/context candidate at rsi:\n"
if $rsi > 0x10000
x/20gx $rsi
end
printf "context candidate at rdx:\n"
if $rdx > 0x10000
x/12gx $rdx
end
if $hits >= 32
  quit
end
continue
end

run
