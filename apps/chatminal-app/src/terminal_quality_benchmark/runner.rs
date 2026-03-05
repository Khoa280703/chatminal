use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chatminal_protocol::{CreateSessionResponse, Event, Request, Response};

use crate::config::parse_usize;
use crate::ipc::ChatminalClient;

use super::stats::{RttBenchmarkReport, build_report};

pub fn run_bench_rtt_wezterm(
    client: &ChatminalClient,
    args: &[String],
) -> Result<RttBenchmarkReport, String> {
    let samples = parse_usize(args.get(2), 80).clamp(10, 2_000);
    let warmup = parse_usize(args.get(3), 15).clamp(0, 500);
    let timeout_ms = parse_usize(args.get(4), 2_000).clamp(200, 10_000) as u64;
    let cols = parse_usize(args.get(5), 120).max(20);
    let rows = parse_usize(args.get(6), 32).max(5);
    let timeout = Duration::from_millis(timeout_ms);

    let session_name = format!("bench-rtt-{}", now_millis());
    let created = expect_session_create(client.request(
        Request::SessionCreate {
            name: Some(session_name),
            cols,
            rows,
            cwd: None,
            persist_history: Some(false),
        },
        Duration::from_secs(5),
    )?)?;

    let session_id = created.session_id;
    let values = run_benchmark_samples(client, &session_id, samples, warmup, timeout, cols, rows);
    let _ = client.request(
        Request::SessionClose {
            session_id: session_id.clone(),
        },
        Duration::from_secs(3),
    );
    let values = values?;
    build_report(session_id, samples, warmup, timeout_ms, &values)
}

fn run_benchmark_samples(
    client: &ChatminalClient,
    session_id: &str,
    samples: usize,
    warmup: usize,
    timeout: Duration,
    cols: usize,
    rows: usize,
) -> Result<Vec<f64>, String> {
    expect_empty(client.request(
        Request::SessionActivate {
            session_id: session_id.to_string(),
            cols,
            rows,
        },
        Duration::from_secs(5),
    )?)?;

    drain_events(client, Duration::from_millis(250));
    let total = samples + warmup;
    let mut values = Vec::with_capacity(samples);
    for index in 0..total {
        let marker = format!("__CHATMINAL_BENCH_{}_{}__", now_millis(), index);
        let payload = format!("echo {marker}\r");
        let started = Instant::now();
        expect_empty(client.request(
            Request::SessionInputWrite {
                session_id: session_id.to_string(),
                data: payload,
            },
            Duration::from_secs(2),
        )?)?;
        wait_for_marker(client, session_id, &marker, timeout)?;
        if index >= warmup {
            values.push(started.elapsed().as_secs_f64() * 1000.0);
        }
    }
    Ok(values)
}

fn wait_for_marker(
    client: &ChatminalClient,
    session_id: &str,
    marker: &str,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    let mut rolling = String::with_capacity(marker.len() * 2 + 1024);
    loop {
        if Instant::now() >= deadline {
            return Err(format!(
                "benchmark marker timeout for session '{session_id}'"
            ));
        }
        let wait = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(5));
        let Some(event) = client.recv_event(wait)? else {
            continue;
        };
        match event {
            Event::PtyOutput(value) if value.session_id == session_id => {
                rolling.push_str(&value.chunk);
                trim_rolling_buffer(&mut rolling, 65_536);
                if rolling.contains(marker) {
                    return Ok(());
                }
            }
            Event::PtyError(value) if value.session_id == session_id => {
                return Err(format!("benchmark session error: {}", value.message));
            }
            Event::PtyExited(value) if value.session_id == session_id => {
                return Err(format!(
                    "benchmark session exited unexpectedly: {}",
                    value.reason
                ));
            }
            _ => {}
        }
    }
}

fn expect_session_create(response: Response) -> Result<CreateSessionResponse, String> {
    match response {
        Response::SessionCreate(value) => Ok(value),
        other => Err(format!("unexpected session_create response: {:?}", other)),
    }
}

fn expect_empty(response: Response) -> Result<(), String> {
    match response {
        Response::Empty => Ok(()),
        other => Err(format!("unexpected empty response: {:?}", other)),
    }
}

fn trim_rolling_buffer(buffer: &mut String, max_bytes: usize) {
    if buffer.len() <= max_bytes {
        return;
    }
    let mut cut = buffer.len().saturating_sub(max_bytes);
    while cut < buffer.len() && !buffer.is_char_boundary(cut) {
        cut += 1;
    }
    buffer.drain(..cut.min(buffer.len()));
}

fn drain_events(client: &ChatminalClient, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match client.recv_event(Duration::from_millis(5)) {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}
