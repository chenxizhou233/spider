use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs, process,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct ProfileReport {
    pub elapsed: Duration,
    pub memory: MemoryReport,
    pub workload: RunStats,
}

#[derive(Debug, Clone)]
pub struct MemoryReport {
    pub samples: usize,
    pub peak_bytes: u64,
    pub p99_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct RunStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub latencies: Vec<Duration>,
}

#[derive(Debug, Clone)]
pub struct LatencyReport {
    pub min: Duration,
    pub p50: Duration,
    pub p90: Duration,
    pub p99: Duration,
    pub max: Duration,
}

pub fn record_usage(
    name: &str,
    run: impl FnOnce() -> Result<RunStats, Box<dyn Error>>,
) -> Result<ProfileReport, Box<dyn Error>> {
    let sampler = MemorySampler::start(memory_sample_interval());
    let start = Instant::now();
    let result = run();
    let elapsed = start.elapsed();
    let memory = sampler.finish();
    let workload = match result {
        Ok(workload) => workload,
        Err(err) => {
            let report = ProfileReport {
                elapsed,
                memory,
                workload: RunStats::default(),
            };
            eprintln!("{name}: failed after {}: {err}", report.summary());
            return Err(err);
        }
    };

    Ok(ProfileReport {
        elapsed,
        memory,
        workload,
    })
}

impl ProfileReport {
    pub fn summary(&self) -> String {
        format!(
            "{:.3?}, {}, throughput {:.2} tasks/s, peak RSS {}, p99 RSS {}, samples {}",
            self.elapsed,
            self.workload.summary(self.elapsed),
            self.workload.throughput(self.elapsed),
            format_bytes(self.memory.peak_bytes),
            format_bytes(self.memory.p99_bytes),
            self.memory.samples,
        )
    }
}

impl RunStats {
    pub fn new(total: usize, latencies: Vec<Duration>, failed: usize) -> Self {
        Self {
            total,
            completed: latencies.len(),
            failed,
            latencies,
        }
    }

    pub fn throughput(&self, elapsed: Duration) -> f64 {
        if elapsed.is_zero() {
            0.0
        } else {
            self.completed as f64 / elapsed.as_secs_f64()
        }
    }

    pub fn latency_report(&self) -> Option<LatencyReport> {
        let mut latencies = self.latencies.clone();
        if latencies.is_empty() {
            return None;
        }

        latencies.sort_unstable();
        let len = latencies.len();
        Some(LatencyReport {
            min: latencies[0],
            p50: percentile_duration(&latencies, 50),
            p90: percentile_duration(&latencies, 90),
            p99: percentile_duration(&latencies, 99),
            max: latencies[len - 1],
        })
    }

    pub fn summary(&self, elapsed: Duration) -> String {
        match self.latency_report() {
            Some(latency) => format!(
                "tasks {}/{}, failed {}, latency min {:.3?} p50 {:.3?} p90 {:.3?} p99 {:.3?} max {:.3?}",
                self.completed,
                self.total,
                self.failed,
                latency.min,
                latency.p50,
                latency.p90,
                latency.p99,
                latency.max,
            ),
            None => format!(
                "tasks 0/{}, failed {}, latency n/a, throughput {:.2} tasks/s",
                self.total,
                self.failed,
                self.throughput(elapsed),
            ),
        }
    }
}

struct MemorySampler {
    running: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<u64>>>,
    handle: Option<JoinHandle<()>>,
}

impl MemorySampler {
    fn start(interval: Duration) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let samples = Arc::new(Mutex::new(Vec::new()));

        let sampler_running = Arc::clone(&running);
        let sampler_samples = Arc::clone(&samples);
        let handle = thread::spawn(move || {
            while sampler_running.load(Ordering::Relaxed) {
                if let Ok(rss_bytes) = process_tree_rss_bytes() {
                    if let Ok(mut samples) = sampler_samples.lock() {
                        samples.push(rss_bytes);
                    }
                }
                thread::sleep(interval);
            }
        });

        Self {
            running,
            samples,
            handle: Some(handle),
        }
    }

    fn finish(mut self) -> MemoryReport {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        let mut samples = self
            .samples
            .lock()
            .map(|samples| samples.clone())
            .unwrap_or_default();
        if samples.is_empty() {
            if let Ok(rss_bytes) = process_tree_rss_bytes() {
                samples.push(rss_bytes);
            }
        }

        memory_report(samples)
    }
}

fn memory_report(mut samples: Vec<u64>) -> MemoryReport {
    if samples.is_empty() {
        return MemoryReport {
            samples: 0,
            peak_bytes: 0,
            p99_bytes: 0,
        };
    }

    let sample_count = samples.len();
    samples.sort_unstable();
    let peak_bytes = *samples.last().unwrap_or(&0);
    let p99_index = ((sample_count * 99).div_ceil(100)).saturating_sub(1);
    let p99_bytes = samples[p99_index.min(sample_count - 1)];

    MemoryReport {
        samples: sample_count,
        peak_bytes,
        p99_bytes,
    }
}

fn percentile_duration(sorted: &[Duration], percentile: usize) -> Duration {
    let index = ((sorted.len() * percentile).div_ceil(100)).saturating_sub(1);
    sorted[index.min(sorted.len() - 1)]
}

#[cfg(target_os = "macos")]
fn memory_sample_interval() -> Duration {
    Duration::from_millis(50)
}

#[cfg(not(target_os = "macos"))]
fn memory_sample_interval() -> Duration {
    Duration::from_millis(10)
}

#[cfg(target_os = "linux")]
fn process_tree_rss_bytes() -> std::io::Result<u64> {
    let root_pid = process::id();
    let mut rss_by_pid = HashMap::new();
    let mut children_by_ppid: HashMap<u32, Vec<u32>> = HashMap::new();

    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry.file_name().to_string_lossy().parse::<u32>().ok() else {
            continue;
        };
        let Ok(status) = fs::read_to_string(entry.path().join("status")) else {
            continue;
        };
        let Some((ppid, rss_bytes)) = parse_status(&status) else {
            continue;
        };

        rss_by_pid.insert(pid, rss_bytes);
        children_by_ppid.entry(ppid).or_default().push(pid);
    }

    let mut total = 0;
    let mut seen = HashSet::new();
    let mut stack = vec![root_pid];

    while let Some(pid) = stack.pop() {
        if !seen.insert(pid) {
            continue;
        }
        total += rss_by_pid.get(&pid).copied().unwrap_or(0);
        if let Some(children) = children_by_ppid.get(&pid) {
            stack.extend(children);
        }
    }

    Ok(total)
}

#[cfg(target_os = "linux")]
fn parse_status(status: &str) -> Option<(u32, u64)> {
    let mut ppid = None;
    let mut rss_kb = None;

    for line in status.lines() {
        if let Some(value) = line.strip_prefix("PPid:") {
            ppid = value.trim().parse::<u32>().ok();
        } else if let Some(value) = line.strip_prefix("VmRSS:") {
            rss_kb = value.split_whitespace().next()?.parse::<u64>().ok();
        }
    }

    Some((ppid?, rss_kb.unwrap_or(0) * 1024))
}

#[cfg(target_os = "macos")]
fn process_tree_rss_bytes() -> std::io::Result<u64> {
    let root_pid = process::id();
    let output = process::Command::new("ps")
        .args(["-axo", "pid=,ppid=,rss="])
        .output()?;
    if !output.status.success() {
        return Ok(0);
    }

    let mut rss_by_pid = HashMap::new();
    let mut children_by_ppid: HashMap<u32, Vec<u32>> = HashMap::new();
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        let Some(pid) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
            continue;
        };
        let Some(ppid) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
            continue;
        };
        let rss_bytes = parts
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0)
            * 1024;

        rss_by_pid.insert(pid, rss_bytes);
        children_by_ppid.entry(ppid).or_default().push(pid);
    }

    let mut total = 0;
    let mut seen = HashSet::new();
    let mut stack = vec![root_pid];

    while let Some(pid) = stack.pop() {
        if !seen.insert(pid) {
            continue;
        }
        total += rss_by_pid.get(&pid).copied().unwrap_or(0);
        if let Some(children) = children_by_ppid.get(&pid) {
            stack.extend(children);
        }
    }

    Ok(total)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn process_tree_rss_bytes() -> std::io::Result<u64> {
    Ok(0)
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes / KIB)
    } else {
        format!("{bytes:.0} B")
    }
}
