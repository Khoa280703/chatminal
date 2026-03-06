mod config;
mod input;
mod ipc;
mod terminal_pane_adapter;
mod terminal_quality_benchmark;
mod terminal_session_commands;
mod terminal_wezterm_attach_frame_renderer;
mod terminal_wezterm_attach_tui;
mod terminal_wezterm_commands;
mod terminal_wezterm_core;
mod terminal_wezterm_dashboard_tui;
mod terminal_wezterm_dashboard_watch;
mod terminal_workspace_ascii_renderer;
mod terminal_workspace_binding_runtime;
mod terminal_workspace_view_model;
mod window;

use std::time::Duration;

use chatminal_protocol::Request;
use config::{AppConfig, parse_usize, usage};
use ipc::ChatminalClient;
use terminal_pane_adapter::{SessionPaneRegistry, StdoutJsonTerminalPaneAdapter, pump_events};
use terminal_quality_benchmark::{run_bench_rtt_wezterm, summary_line};
use terminal_session_commands::{
    activate_session_with_snapshot, fetch_snapshot_for_session, resize_session,
    write_input_for_session,
};
use terminal_wezterm_attach_tui::run_attach_tui_wezterm;
use terminal_wezterm_commands::{
    handle_activate_wezterm, handle_dashboard_wezterm, handle_events_wezterm,
    handle_workspace_wezterm,
};
use terminal_wezterm_dashboard_tui::run_dashboard_tui_wezterm;
use terminal_wezterm_dashboard_watch::run_dashboard_watch_wezterm;

const SUPPORTED_COMMANDS: &[&str] = &[
    "workspace",
    "sessions",
    "create",
    "activate",
    "snapshot",
    "input",
    "resize",
    "events",
    "workspace-wezterm",
    "activate-wezterm",
    "events-wezterm",
    "dashboard-wezterm",
    "dashboard-watch-wezterm",
    "dashboard-tui-wezterm",
    "attach-wezterm",
    "window",
    "bench-rtt-wezterm",
];

struct SilentTerminalPaneAdapter;
impl terminal_pane_adapter::TerminalPaneAdapter for SilentTerminalPaneAdapter {}

fn main() {
    if let Err(err) = run() {
        eprintln!("chatminal-app error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        println!("{}", usage());
        return Ok(());
    }

    let command = args[1].as_str();
    let is_supported = SUPPORTED_COMMANDS.contains(&command);
    if matches!(command, "--help" | "-h" | "help") {
        println!("{}", usage());
        return Ok(());
    }
    if !is_supported {
        println!("{}", usage());
        return Err(format!("unsupported command: {command}"));
    }

    let config = AppConfig::from_env()?;
    if is_window_command(command) {
        return run_window_by_backend(&config, &args);
    }

    let client = ChatminalClient::connect(&config.endpoint)?;
    let mut pane_registry = SessionPaneRegistry::new();
    let mut silent_adapter = SilentTerminalPaneAdapter;

    match command {
        "workspace" => {
            let response = client.request(Request::WorkspaceLoad, Duration::from_secs(4))?;
            print_pretty_json(&response)
        }
        "sessions" => {
            let response = client.request(Request::SessionList, Duration::from_secs(4))?;
            print_pretty_json(&response)
        }
        "create" => {
            let name = args
                .get(2)
                .cloned()
                .ok_or_else(|| "missing session name".to_string())?;
            let response = client.request(
                Request::SessionCreate {
                    name: Some(name),
                    cols: 120,
                    rows: 32,
                    cwd: None,
                    persist_history: Some(false),
                },
                Duration::from_secs(5),
            )?;
            print_pretty_json(&response)
        }
        "activate" => {
            let session_id = args
                .get(2)
                .cloned()
                .ok_or_else(|| "missing session id".to_string())?;
            let cols = parse_usize(args.get(3), 120);
            let rows = parse_usize(args.get(4), 32);
            let preview_lines = parse_usize(args.get(5), 200);
            let activation = activate_session_with_snapshot(
                &client,
                &mut pane_registry,
                &mut silent_adapter,
                &session_id,
                cols,
                rows,
                preview_lines,
            )?;
            print_pretty_json(&activation)
        }
        "snapshot" => {
            let session_id = args
                .get(2)
                .cloned()
                .ok_or_else(|| "missing session id".to_string())?;
            let preview_lines = parse_usize(args.get(3), 200);
            let snapshot = fetch_snapshot_for_session(&client, &session_id, preview_lines)?;
            print_pretty_json(&snapshot)
        }
        "input" => {
            let session_id = args
                .get(2)
                .cloned()
                .ok_or_else(|| "missing session id".to_string())?;
            let data = args
                .get(3..)
                .map(|value| value.join(" "))
                .ok_or_else(|| "missing input payload".to_string())?;
            if data.is_empty() {
                return Err("missing input payload".to_string());
            }
            write_input_for_session(
                &client,
                &mut pane_registry,
                &mut silent_adapter,
                &session_id,
                &data,
            )?;
            print_pretty_json(&serde_json::json!({ "ok": true }))
        }
        "resize" => {
            let session_id = args
                .get(2)
                .cloned()
                .ok_or_else(|| "missing session id".to_string())?;
            let cols = parse_usize(args.get(3), 120);
            let rows = parse_usize(args.get(4), 32);
            resize_session(
                &client,
                &mut pane_registry,
                &mut silent_adapter,
                &session_id,
                cols,
                rows,
            )?;
            print_pretty_json(&serde_json::json!({ "ok": true }))
        }
        "events" => {
            let seconds = parse_usize(args.get(2), 15);
            let _ = client.request(Request::WorkspaceLoad, Duration::from_secs(4));
            println!(
                "Listening daemon events for {}s at endpoint {}",
                seconds, config.endpoint
            );
            let mut adapter = StdoutJsonTerminalPaneAdapter;
            let processed =
                pump_events(&client, &mut adapter, Duration::from_secs(seconds as u64))?;
            println!("Processed {processed} events");
            Ok(())
        }
        "workspace-wezterm" => {
            let payload = handle_workspace_wezterm(&client, &args, &mut pane_registry)?;
            print_pretty_json(&payload)
        }
        "activate-wezterm" => {
            let payload = handle_activate_wezterm(&client, &args, &mut pane_registry)?;
            print_pretty_json(&payload)
        }
        "events-wezterm" => {
            let payload = handle_events_wezterm(&client, &args, &mut pane_registry)?;
            print_pretty_json(&payload)
        }
        "dashboard-wezterm" => {
            let payload = handle_dashboard_wezterm(&client, &args, &mut pane_registry)?;
            print_pretty_json(&payload)
        }
        "dashboard-watch-wezterm" => {
            run_dashboard_watch_wezterm(&client, &args, &mut pane_registry)
        }
        "dashboard-tui-wezterm" => run_dashboard_tui_wezterm(&client, &args, &mut pane_registry),
        "attach-wezterm" => run_attach_tui_wezterm(
            &client,
            &args,
            &mut pane_registry,
            config.input_pipeline_mode,
        ),
        "bench-rtt-wezterm" => {
            let report = run_bench_rtt_wezterm(&client, &args)?;
            println!("{}", summary_line(&report));
            print_pretty_json(&report)
        }
        _ => Ok(()),
    }
}

fn is_window_command(command: &str) -> bool {
    command == "window"
}

fn run_window_by_backend(config: &AppConfig, args: &[String]) -> Result<(), String> {
    window::run_window_wezterm(&config.endpoint, args, config.input_pipeline_mode)
}

fn print_pretty_json<T: serde::Serialize>(value: &T) -> Result<(), String> {
    let encoded =
        serde_json::to_string_pretty(value).map_err(|err| format!("encode json failed: {err}"))?;
    println!("{encoded}");
    Ok(())
}
