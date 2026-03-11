use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::System;

/// CPU requires two samples ~200ms apart for accurate measurement.
const CPU_SAMPLE_MS: u64 = 200;
/// Total refresh cycle after publishing metrics.
const REFRESH_INTERVAL: Duration = Duration::from_millis(500);
/// Latency probe target: TCP connect to Cloudflare DNS (low latency, reliable).
const LATENCY_HOST: &str = "1.1.1.1:80";
const LATENCY_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub cpu_percent: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    /// TCP connect RTT in ms, None if unreachable.
    pub latency_ms: Option<u32>,
}

impl SystemMetrics {
    pub fn ram_display(&self) -> String {
        if self.ram_total_gb < 0.1 {
            return "--".to_string();
        }
        format!("{:.1}GB", self.ram_used_gb)
    }

    pub fn cpu_display(&self) -> String {
        format!("{:.0}%", self.cpu_percent)
    }

    pub fn latency_display(&self) -> String {
        match self.latency_ms {
            Some(ms) => format!("{}ms", ms),
            None => "--".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemMetricsHandle {
    inner: Arc<Mutex<SystemMetrics>>,
}

impl SystemMetricsHandle {
    pub fn start() -> Self {
        let inner = Arc::new(Mutex::new(SystemMetrics::default()));
        let shared = Arc::clone(&inner);
        thread::spawn(move || run_metrics_loop(shared));
        Self { inner }
    }

    pub fn snapshot(&self) -> SystemMetrics {
        self.inner.lock().map(|m| m.clone()).unwrap_or_default()
    }
}

fn measure_latency_ms() -> Option<u32> {
    let start = Instant::now();
    TcpStream::connect_timeout(&LATENCY_HOST.parse().ok()?, LATENCY_TIMEOUT).ok()?;
    Some(start.elapsed().as_millis() as u32)
}

fn run_metrics_loop(shared: Arc<Mutex<SystemMetrics>>) {
    let mut sys = System::new();
    let mut tick: u32 = 0;

    loop {
        // CPU: two samples separated by CPU_SAMPLE_MS for accurate reading
        sys.refresh_cpu_usage();
        thread::sleep(Duration::from_millis(CPU_SAMPLE_MS));
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let cpu = sys.global_cpu_usage();
        let ram_used = sys.used_memory() as f32 / (1024.0_f32.powi(3));
        let ram_total = sys.total_memory() as f32 / (1024.0_f32.powi(3));

        // Probe latency every 5 ticks (~2.5s) to avoid hammering the network
        let latency_ms = if tick % 5 == 0 {
            Some(measure_latency_ms())
        } else {
            None // None = keep previous value
        };

        if let Ok(mut m) = shared.lock() {
            m.cpu_percent = cpu;
            m.ram_used_gb = ram_used;
            m.ram_total_gb = ram_total;
            if let Some(lat) = latency_ms {
                m.latency_ms = lat;
            }
        }

        tick = tick.wrapping_add(1);
        thread::sleep(REFRESH_INTERVAL);
    }
}
