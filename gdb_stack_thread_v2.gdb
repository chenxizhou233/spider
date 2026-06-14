set pagination off
set confirm off
set print thread-events off
set environment SPIDER_CONCURRENCY=20
set logging file gdb_stack_thread_v2_raw.log
set logging overwrite on
set logging enabled on

break src/runner/download_thread.rs:33
commands
  silent
  printf "\n=== thread stack VMA snapshot at download_thread.rs:33 ===\n"
  printf "method: switch each gdb thread, read $rsp, find containing VMA in /proc/<pid>/maps\n"
  info threads
  python
import gdb, re
inferior = gdb.selected_inferior()
pid = inferior.pid
maps = []
with open(f"/proc/{pid}/maps", "r", encoding="utf-8", errors="replace") as f:
    for line in f:
        m = re.match(r"([0-9a-f]+)-([0-9a-f]+)\s+(\S+)\s+\S+\s+\S+\s+\S+\s*(.*)", line.rstrip())
        if m:
            start = int(m.group(1), 16)
            end = int(m.group(2), 16)
            maps.append((start, end, m.group(3), m.group(4), line.rstrip()))
rows = []
unique = {}
for th in inferior.threads():
    th.switch()
    tid = th.ptid[1] if len(th.ptid) > 1 and th.ptid[1] else th.ptid[0]
    rsp = int(gdb.parse_and_eval("$rsp"))
    hit = None
    for start, end, perms, name, line in maps:
        if start <= rsp < end:
            hit = (start, end, perms, name, line)
            break
    if hit:
        start, end, perms, name, line = hit
        size = end - start
        unique[(start, end)] = size
        rows.append((th.num, tid, rsp, start, end, size, line))
    else:
        rows.append((th.num, tid, rsp, 0, 0, 0, "NO_MAPPING_FOR_RSP"))
total = sum(unique.values())
print(f"gdb inferior pid={pid}")
print(f"gdb thread count={len(inferior.threads())}")
for num, tid, rsp, start, end, size, line in rows:
    print(f"thread#{num} tid={tid} rsp=0x{rsp:x} stack_vma=0x{start:x}-0x{end:x} size_bytes={size} size_kib={size/1024:.1f}")
    print(f"  {line}")
print(f"SUMMARY model=thread gdb_threads={len(inferior.threads())} unique_stack_vmas={len(unique)} mapped_stack_bytes_by_rsp={total} mapped_stack_mib_by_rsp={total/1024/1024:.3f}")
  end
  detach
  quit
end

run
