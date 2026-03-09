mod config;
mod graph;
mod input;
mod mcp;
mod metrics;
mod platform;
mod process;
mod render;

use std::time::Instant;

use clap::{Parser, Subcommand};
use egaku::Theme;
use madori::event::AppEvent;
use madori::{App, AppConfig, EventResponse};
use tracing_subscriber::EnvFilter;

use crate::config::MyakuConfig;
use crate::input::{Action, Mode};
use crate::metrics::MetricsCollector;
use crate::process::{ProcessList, SortColumn};
use crate::render::MyakuRenderer;

#[derive(Parser)]
#[command(name = "myaku", about = "Myaku (\u{8108}) \u{2014} GPU system monitor")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the metrics collection daemon.
    Daemon,
    /// Print current system metrics as JSON to stdout.
    Snapshot,
    /// Run as MCP server (stdio transport) for Claude Code integration.
    Mcp,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load config via shikumi
    let config = load_config();

    match cli.command {
        Some(Command::Mcp) => {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            rt.block_on(mcp::run())
                .expect("MCP server error");
        }
        Some(Command::Daemon) => run_daemon(&config),
        Some(Command::Snapshot) => run_snapshot(&config),
        None => run_gui(config),
    }
}

fn load_config() -> MyakuConfig {
    match shikumi::ConfigDiscovery::new("myaku")
        .env_override("MYAKU_CONFIG")
        .discover()
    {
        Ok(path) => {
            tracing::info!("loading config from {}", path.display());
            let store =
                shikumi::ConfigStore::<MyakuConfig>::load(&path, "MYAKU_").unwrap_or_else(|e| {
                    tracing::warn!("failed to load config: {e}, using defaults");
                    let tmp = std::env::temp_dir().join("myaku-default.yaml");
                    std::fs::write(&tmp, "{}").ok();
                    shikumi::ConfigStore::load(&tmp, "MYAKU_").unwrap()
                });
            MyakuConfig::clone(&store.get())
        }
        Err(_) => {
            tracing::info!("no config file found, using defaults");
            MyakuConfig::default()
        }
    }
}

fn run_gui(config: MyakuConfig) {
    tracing::info!("launching myaku GUI");
    tracing::info!(
        "refresh rate: {}ms, history: {}s",
        config.appearance.refresh_rate_ms,
        config.monitoring.history_seconds
    );

    let refresh_interval_ms = config.appearance.refresh_rate_ms;
    let app_config = AppConfig {
        title: String::from("Myaku - System Monitor"),
        width: config.appearance.width,
        height: config.appearance.height,
        resizable: true,
        vsync: true,
        transparent: false,
    };

    // Initialize metrics collector
    let mut collector = MetricsCollector::new(&config);
    collector.refresh();

    // Initialize process list
    let sort_col = SortColumn::from_str(&config.processes.sort_by);
    let mut proc_list = ProcessList::new(sort_col, 30);
    proc_list.sort_ascending = config.processes.sort_direction == "asc";

    // Load initial processes
    let procs = collector.processes(&config.processes.sort_by, proc_list.sort_ascending);
    proc_list.update(procs);

    // Current mode
    let mut mode = Mode::Dashboard;
    let mut filter_input = String::new();
    let mut last_refresh = Instant::now();

    // Create renderer
    let renderer = MyakuRenderer::new(Theme::default());

    App::builder(renderer)
        .config(app_config)
        .on_event(move |event: &AppEvent, renderer: &mut MyakuRenderer| -> EventResponse {
            match event {
                AppEvent::Key(key_event) => {
                    let action = input::map_key(key_event, mode);
                    match action {
                        Action::Quit => {
                            return EventResponse {
                                consumed: true,
                                exit: true,
                                set_title: None,
                            };
                        }
                        Action::SwitchDashboard => {
                            mode = Mode::Dashboard;
                        }
                        Action::SwitchProcess => {
                            mode = Mode::Process;
                        }
                        Action::ForceRefresh => {
                            collector.refresh();
                            let procs = collector.processes(
                                proc_list.sort_column.label(),
                                proc_list.sort_ascending,
                            );
                            proc_list.update(procs);
                            last_refresh = Instant::now();
                        }
                        Action::FocusNext | Action::FocusPrev => {
                            // Toggle between dashboard and process view
                            mode = match mode {
                                Mode::Dashboard => Mode::Process,
                                Mode::Process => Mode::Dashboard,
                                Mode::Filter => Mode::Filter,
                            };
                        }
                        Action::Down => {
                            if mode == Mode::Process {
                                proc_list.select_next();
                            }
                        }
                        Action::Up => {
                            if mode == Mode::Process {
                                proc_list.select_prev();
                            }
                        }
                        Action::PageDown => proc_list.page_down(),
                        Action::PageUp => proc_list.page_up(),
                        Action::First => proc_list.select_first(),
                        Action::Last => proc_list.select_last(),
                        Action::CycleSort => {
                            proc_list.cycle_sort();
                        }
                        Action::ToggleSortDirection => {
                            proc_list.toggle_sort_direction();
                        }
                        Action::EnterFilter => {
                            mode = Mode::Filter;
                            filter_input.clear();
                        }
                        Action::Back => {
                            match mode {
                                Mode::Filter => {
                                    mode = Mode::Process;
                                    // Keep the filter applied
                                }
                                Mode::Process => {
                                    mode = Mode::Dashboard;
                                }
                                Mode::Dashboard => {}
                            }
                        }
                        Action::Confirm => {
                            if mode == Mode::Filter {
                                proc_list.set_filter(filter_input.clone());
                                mode = Mode::Process;
                            }
                        }
                        Action::Char(c) => {
                            if mode == Mode::Filter {
                                filter_input.push(c);
                                proc_list.set_filter(filter_input.clone());
                            }
                        }
                        Action::Backspace => {
                            if mode == Mode::Filter {
                                filter_input.pop();
                                proc_list.set_filter(filter_input.clone());
                            }
                        }
                        Action::None => {}
                    }
                    EventResponse::consumed()
                }
                AppEvent::RedrawRequested => {
                    // Periodic refresh based on configured interval
                    let elapsed_ms = last_refresh.elapsed().as_millis() as u32;
                    if elapsed_ms >= refresh_interval_ms {
                        collector.refresh();
                        let procs = collector.processes(
                            proc_list.sort_column.label(),
                            proc_list.sort_ascending,
                        );
                        proc_list.update(procs);
                        last_refresh = Instant::now();
                    }

                    // Build render state
                    renderer.state.mode = mode;
                    renderer.state.dashboard_lines =
                        renderer.build_dashboard_state(&collector);
                    renderer.state.process_lines =
                        renderer.build_process_state(&proc_list);
                    renderer.state.sparklines =
                        renderer.build_sparklines(&collector);

                    // Status bar
                    let mode_label = match mode {
                        Mode::Dashboard => "DASHBOARD",
                        Mode::Process => "PROCESS",
                        Mode::Filter => "FILTER",
                    };
                    renderer.state.status_line = match mode {
                        Mode::Filter => {
                            format!(
                                "  [FILTER] /{}_  | Enter: apply | Esc: cancel",
                                filter_input
                            )
                        }
                        Mode::Dashboard => {
                            format!(
                                "  [{mode_label}]  q: quit | p: processes | r: refresh | Tab: switch view | hjkl: navigate"
                            )
                        }
                        Mode::Process => {
                            format!(
                                "  [{mode_label}]  q: quit | d: dashboard | jk: navigate | s: sort | /: filter | Esc: back"
                            )
                        }
                    };

                    EventResponse::default()
                }
                _ => EventResponse::default(),
            }
        })
        .run()
        .expect("myaku GUI exited with error");
}

fn run_daemon(config: &MyakuConfig) {
    tracing::info!("starting myaku daemon");
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        tracing::info!(
            "metrics daemon on port {}, retention {}h",
            config.daemon.metrics_port,
            config.daemon.history_retention_hours
        );

        let mut collector = MetricsCollector::new(config);
        let interval_ms = config.appearance.refresh_rate_ms.max(500);

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("daemon shutting down");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(u64::from(interval_ms))) => {
                    collector.refresh();
                    tracing::debug!(
                        "cpu: {:.1}%, ram: {:.1}%, uptime: {}",
                        collector.cpu.total_usage(),
                        collector.memory.ram_percent(),
                        collector.uptime_display()
                    );
                }
            }
        }
    });
}

fn run_snapshot(config: &MyakuConfig) {
    let mut collector = MetricsCollector::new(config);
    collector.refresh();

    // Simple JSON snapshot to stdout
    let cpu = &collector.cpu;
    let mem = &collector.memory;
    let load = collector.load_average;

    println!("{{");
    println!("  \"cpu\": {{");
    println!("    \"total\": {:.1},", cpu.total_usage());
    println!("    \"brand\": \"{}\",", cpu.brand);
    println!("    \"cores\": {},", cpu.core_count);
    print!("    \"per_core\": [");
    let per_core: Vec<String> = cpu
        .cores
        .series
        .iter()
        .map(|(_, buf)| format!("{:.1}", buf.latest().unwrap_or(0.0)))
        .collect();
    print!("{}", per_core.join(", "));
    println!("]");
    println!("  }},");

    println!("  \"memory\": {{");
    println!("    \"total_bytes\": {},", mem.latest.total);
    println!("    \"used_bytes\": {},", mem.latest.used);
    println!("    \"available_bytes\": {},", mem.latest.available);
    println!("    \"ram_percent\": {:.1},", mem.ram_percent());
    println!("    \"swap_total_bytes\": {},", mem.latest.swap_total);
    println!("    \"swap_used_bytes\": {},", mem.latest.swap_used);
    println!("    \"swap_percent\": {:.1}", mem.swap_percent());
    println!("  }},");

    println!("  \"disks\": [");
    let disk_lines: Vec<String> = collector
        .disk
        .mounts
        .iter()
        .map(|m| {
            format!(
                "    {{\"mount\": \"{}\", \"total\": {}, \"used\": {}, \"percent\": {:.1}}}",
                m.mount_point, m.total, m.used, m.usage_percent()
            )
        })
        .collect();
    println!("{}", disk_lines.join(",\n"));
    println!("  ],");

    println!("  \"network\": [");
    let net_lines: Vec<String> = collector
        .network
        .interfaces
        .iter()
        .map(|i| {
            format!(
                "    {{\"interface\": \"{}\", \"rx_bytes\": {}, \"tx_bytes\": {}}}",
                i.name, i.total_rx, i.total_tx
            )
        })
        .collect();
    println!("{}", net_lines.join(",\n"));
    println!("  ],");

    println!("  \"uptime_seconds\": {},", collector.uptime_secs);
    println!(
        "  \"load_average\": [{:.2}, {:.2}, {:.2}]",
        load[0], load[1], load[2]
    );
    println!("}}");
}
