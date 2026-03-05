use serde::Serialize;

pub const TARGET_P95_MS: f64 = 30.0;
pub const TARGET_P99_MS: f64 = 60.0;
pub const FAIL_GATE_P95_MS: f64 = 50.0;

#[derive(Debug, Clone, Serialize)]
pub struct RttStatistics {
    pub min_ms: f64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RttTargets {
    pub p95_target_ms: f64,
    pub p99_target_ms: f64,
    pub p95_fail_gate_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RttBenchmarkReport {
    pub samples: usize,
    pub warmup: usize,
    pub timeout_ms: u64,
    pub session_id: String,
    pub stats: RttStatistics,
    pub targets: RttTargets,
    pub pass_targets: bool,
    pub pass_fail_gate: bool,
}

pub fn build_report(
    session_id: String,
    samples: usize,
    warmup: usize,
    timeout_ms: u64,
    values_ms: &[f64],
) -> Result<RttBenchmarkReport, String> {
    let stats = build_statistics(values_ms)?;
    let targets = RttTargets {
        p95_target_ms: TARGET_P95_MS,
        p99_target_ms: TARGET_P99_MS,
        p95_fail_gate_ms: FAIL_GATE_P95_MS,
    };
    Ok(RttBenchmarkReport {
        samples,
        warmup,
        timeout_ms,
        session_id,
        pass_targets: stats.p95_ms <= TARGET_P95_MS && stats.p99_ms <= TARGET_P99_MS,
        pass_fail_gate: stats.p95_ms <= FAIL_GATE_P95_MS,
        stats,
        targets,
    })
}

pub fn summary_line(report: &RttBenchmarkReport) -> String {
    format!(
        "RTT_BENCH samples={} warmup={} avg_ms={:.3} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} max_ms={:.3} pass_targets={} pass_fail_gate={}",
        report.samples,
        report.warmup,
        report.stats.avg_ms,
        report.stats.p50_ms,
        report.stats.p95_ms,
        report.stats.p99_ms,
        report.stats.max_ms,
        report.pass_targets,
        report.pass_fail_gate
    )
}

fn build_statistics(values: &[f64]) -> Result<RttStatistics, String> {
    if values.is_empty() {
        return Err("benchmark produced no samples".to_string());
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let sum = sorted.iter().sum::<f64>();
    Ok(RttStatistics {
        min_ms: *sorted.first().unwrap_or(&0.0),
        avg_ms: sum / sorted.len() as f64,
        p50_ms: percentile(&sorted, 50.0),
        p95_ms: percentile(&sorted, 95.0),
        p99_ms: percentile(&sorted, 99.0),
        max_ms: *sorted.last().unwrap_or(&0.0),
    })
}

fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    let len = sorted_values.len();
    let rank = ((p / 100.0) * len as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(len.saturating_sub(1));
    sorted_values[index]
}

#[cfg(test)]
mod tests {
    use super::{build_report, percentile};

    #[test]
    fn percentile_uses_nearest_rank() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&values, 50.0), 3.0);
        assert_eq!(percentile(&values, 95.0), 5.0);
    }

    #[test]
    fn report_contains_expected_aggregates() {
        let report = build_report("session-1".to_string(), 4, 0, 1000, &[1.0, 2.0, 3.0, 4.0])
            .expect("build report");
        assert_eq!(report.stats.min_ms, 1.0);
        assert_eq!(report.stats.max_ms, 4.0);
        assert_eq!(report.stats.p50_ms, 2.0);
        assert!((report.stats.avg_ms - 2.5).abs() < 0.001);
    }
}
