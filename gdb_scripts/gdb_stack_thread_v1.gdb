set pagination off
set confirm off
set print thread-events off
set environment SPIDER_CONCURRENCY=20
set logging file logs/gdb_stack_thread_raw.log
set logging overwrite on
set logging enabled on

break src/runner/download_thread.rs:33
commands
  silent
  printf "\n=== thread stack snapshot at download_thread.rs:33 ===\n"
  printf "active worker threads should have just been spawned\n"
  info threads
  python
import gdb, os, re
inferior = gdb.selected_inferior()
pid = inferior.pid
threads = inferior.threads()
print(f"gdb inferior pid={pid}")
print(f"gdb thread count={len(threads)}")
total_stack_maps = 0
total_stack_bytes = 0
largest_stack = 0
rows = []
for th in threads:
    ptid = th.ptid
    tid = ptid[1] if len(ptid) > 1 and ptid[1] else ptid[0]
    maps_path = f"/proc/{pid}/task/{tid}/maps"
    try:
        lines = open(maps_path, "r", encoding="utf-8", errors="replace").read().splitlines()
    except OSError as exc:
        rows.append((th.num, tid, 0, 0, f"ERR {exc}"))
        continue
    stack_lines = [line for line in lines if "[stack" in line]
    size_sum = 0
    for line in stack_lines:
        m = re.match(r"([0-9a-f]+)-([0-9a-f]+)", line)
        if m:
            size_sum += int(m.group(2), 16) - int(m.group(1), 16)
    total_stack_maps += len(stack_lines)
    total_stack_bytes += size_sum
    largest_stack = max(largest_stack, size_sum)
    rows.append((th.num, tid, len(stack_lines), size_sum, " | ".join(stack_lines)))
for num, tid, count, size_sum, detail in rows:
    print(f"thread#{num} tid={tid} stack_maps={count} stack_bytes={size_sum} stack_kib={size_sum/1024:.1f}")
    if detail:
        print(f"  {detail}")
print(f"SUMMARY model=thread gdb_threads={len(threads)} stack_maps={total_stack_maps} mapped_stack_bytes={total_stack_bytes} mapped_stack_mib={total_stack_bytes/1024/1024:.3f} largest_stack_kib={largest_stack/1024:.1f}")
  end
  detach
  quit
end

run
