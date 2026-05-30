use super::logic::ProfileReport;
use std::{fs, io};

const CSV_OUTPUT_PATH: &str = "Result.csv";
const CSV_HEADER: &str = "model,total,completed,failed,elapsed_ms,throughput_tasks_per_sec,latency_min_ms,latency_p50_ms,latency_p90_ms,latency_p99_ms,latency_max_ms,peak_rss_bytes,p99_rss_bytes,memory_samples";

pub fn print_csv_report(results: &[(&str, ProfileReport)]) -> io::Result<()> {
    let csv = csv_report(results);
    fs::write(CSV_OUTPUT_PATH, &csv)?;
    print!("{csv}");
    eprintln!("csv report written to {CSV_OUTPUT_PATH}");
    Ok(())
}

pub fn print_csv_rows(results: &[(&str, ProfileReport)]) {
    for (name, report) in results {
        println!("{}", csv_row(name, report));
    }
}

fn csv_report(results: &[(&str, ProfileReport)]) -> String {
    let mut rows = Vec::with_capacity(results.len() + 1);
    rows.push(CSV_HEADER.to_owned());
    rows.extend(results.iter().map(|(name, report)| csv_row(name, report)));
    rows.join("\n") + "\n"
}

fn csv_row(name: &str, report: &ProfileReport) -> String {
    let latency = report.workload.latency_report();
    let latency_min = latency
        .as_ref()
        .map(|latency| duration_ms(latency.min))
        .unwrap_or_default();
    let latency_p50 = latency
        .as_ref()
        .map(|latency| duration_ms(latency.p50))
        .unwrap_or_default();
    let latency_p90 = latency
        .as_ref()
        .map(|latency| duration_ms(latency.p90))
        .unwrap_or_default();
    let latency_p99 = latency
        .as_ref()
        .map(|latency| duration_ms(latency.p99))
        .unwrap_or_default();
    let latency_max = latency
        .as_ref()
        .map(|latency| duration_ms(latency.max))
        .unwrap_or_default();

    format!(
        "{},{},{},{},{},{:.2},{},{},{},{},{},{},{},{}",
        csv_field(name),
        report.workload.total,
        report.workload.completed,
        report.workload.failed,
        duration_ms(report.elapsed),
        report.workload.throughput(report.elapsed),
        latency_min,
        latency_p50,
        latency_p90,
        latency_p99,
        latency_max,
        report.memory.peak_bytes,
        report.memory.p99_bytes,
        report.memory.samples,
    )
}

fn duration_ms(duration: std::time::Duration) -> String {
    format!("{:.3}", duration.as_secs_f64() * 1000.0)
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
