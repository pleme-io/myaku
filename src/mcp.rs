//! MCP server for myaku system monitor.
//!
//! Provides tools for querying CPU, memory, disk, network metrics,
//! listing processes, and sending signals.

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;
use sysinfo::System;

// ── Tool input types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ListProcessesInput {
    #[schemars(description = "Sort by: 'cpu', 'memory', 'pid', or 'name' (default: 'cpu').")]
    sort_by: Option<String>,
    #[schemars(description = "Maximum number of processes to return (default: 30).")]
    limit: Option<usize>,
    #[schemars(description = "Filter processes by name pattern.")]
    filter: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct KillProcessInput {
    #[schemars(description = "Process ID to send a signal to.")]
    pid: u32,
    #[schemars(description = "Signal to send: 'SIGTERM' (default), 'SIGKILL', or 'SIGINT'.")]
    signal: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ConfigGetInput {
    #[schemars(description = "Config key to retrieve. Omit for full config.")]
    key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ConfigSetInput {
    #[schemars(description = "Config key to set.")]
    key: String,
    #[schemars(description = "Value to set (as JSON string).")]
    value: String,
}

// ── MCP Server ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct MyakuMcp {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MyakuMcp {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    // ── Standard tools ──────────────────────────────────────────────────────

    #[tool(description = "Get myaku application status and health information. Returns system uptime and overview.")]
    async fn status(&self) -> String {
        let mut sys = System::new();
        sys.refresh_memory();

        serde_json::json!({
            "status": "running",
            "app": "myaku",
            "uptime_seconds": System::uptime(),
            "total_memory_bytes": sys.total_memory(),
            "used_memory_bytes": sys.used_memory(),
        })
        .to_string()
    }

    #[tool(description = "Get myaku version information.")]
    async fn version(&self) -> String {
        serde_json::json!({
            "name": "myaku",
            "version": env!("CARGO_PKG_VERSION"),
            "description": env!("CARGO_PKG_DESCRIPTION"),
            "sysinfo_version": "0.33",
        })
        .to_string()
    }

    #[tool(description = "Get a myaku configuration value. Pass a key for a specific value, or omit for the full config.")]
    async fn config_get(&self, Parameters(input): Parameters<ConfigGetInput>) -> String {
        match input.key {
            Some(key) => serde_json::json!({
                "key": key,
                "value": null,
                "note": "Config queries require a running myaku instance."
            })
            .to_string(),
            None => serde_json::json!({
                "config_path": "~/.config/myaku/myaku.yaml"
            })
            .to_string(),
        }
    }

    #[tool(description = "Set a myaku configuration value at runtime.")]
    async fn config_set(&self, Parameters(input): Parameters<ConfigSetInput>) -> String {
        serde_json::json!({
            "key": input.key,
            "value": input.value,
            "applied": false,
            "note": "Config mutations require a running myaku instance."
        })
        .to_string()
    }

    // ── System metrics tools ────────────────────────────────────────────────

    #[tool(description = "Get CPU usage metrics. Returns total usage, per-core usage, core count, and CPU brand.")]
    async fn get_cpu(&self) -> String {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        // Need a brief sleep for accurate readings on first call
        std::thread::sleep(std::time::Duration::from_millis(200));
        sys.refresh_cpu_all();

        let global = sys.global_cpu_usage();
        let cores: Vec<serde_json::Value> = sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| {
                serde_json::json!({
                    "core": i,
                    "usage_percent": cpu.cpu_usage(),
                    "frequency_mhz": cpu.frequency(),
                })
            })
            .collect();

        let brand = sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default();

        serde_json::json!({
            "total_usage_percent": global,
            "core_count": sys.cpus().len(),
            "brand": brand,
            "cores": cores,
        })
        .to_string()
    }

    #[tool(description = "Get memory usage metrics. Returns RAM and swap usage in bytes and percentages.")]
    async fn get_memory(&self) -> String {
        let mut sys = System::new();
        sys.refresh_memory();

        let total = sys.total_memory();
        let used = sys.used_memory();
        let available = sys.available_memory();
        let swap_total = sys.total_swap();
        let swap_used = sys.used_swap();

        serde_json::json!({
            "ram": {
                "total_bytes": total,
                "used_bytes": used,
                "available_bytes": available,
                "usage_percent": if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 },
            },
            "swap": {
                "total_bytes": swap_total,
                "used_bytes": swap_used,
                "usage_percent": if swap_total > 0 { (swap_used as f64 / swap_total as f64) * 100.0 } else { 0.0 },
            },
        })
        .to_string()
    }

    #[tool(description = "Get disk usage metrics. Returns usage per mount point with total, used, and available space.")]
    async fn get_disk(&self) -> String {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let entries: Vec<serde_json::Value> = disks
            .iter()
            .map(|disk| {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total.saturating_sub(available);
                serde_json::json!({
                    "mount_point": disk.mount_point().display().to_string(),
                    "name": disk.name().to_string_lossy(),
                    "file_system": String::from_utf8_lossy(disk.file_system().as_encoded_bytes()),
                    "total_bytes": total,
                    "used_bytes": used,
                    "available_bytes": available,
                    "usage_percent": if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 },
                })
            })
            .collect();

        serde_json::json!({
            "count": entries.len(),
            "disks": entries,
        })
        .to_string()
    }

    #[tool(description = "Get network interface metrics. Returns per-interface received and transmitted bytes.")]
    async fn get_network(&self) -> String {
        let networks = sysinfo::Networks::new_with_refreshed_list();
        let entries: Vec<serde_json::Value> = networks
            .iter()
            .map(|(name, data)| {
                serde_json::json!({
                    "interface": name,
                    "received_bytes": data.total_received(),
                    "transmitted_bytes": data.total_transmitted(),
                })
            })
            .collect();

        serde_json::json!({
            "count": entries.len(),
            "interfaces": entries,
        })
        .to_string()
    }

    #[tool(description = "List running processes. Sort by CPU, memory, PID, or name. Optionally filter by name.")]
    async fn list_processes(&self, Parameters(input): Parameters<ListProcessesInput>) -> String {
        let mut sys = System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let sort_by = input.sort_by.unwrap_or_else(|| "cpu".to_string());
        let limit = input.limit.unwrap_or(30);
        let filter = input.filter.map(|f| f.to_lowercase());

        let mut procs: Vec<serde_json::Value> = sys
            .processes()
            .values()
            .filter(|p| {
                if let Some(ref f) = filter {
                    p.name().to_string_lossy().to_lowercase().contains(f)
                } else {
                    true
                }
            })
            .map(|p| {
                serde_json::json!({
                    "pid": p.pid().as_u32(),
                    "name": p.name().to_string_lossy(),
                    "cpu_percent": p.cpu_usage(),
                    "memory_bytes": p.memory(),
                    "status": format!("{:?}", p.status()),
                })
            })
            .collect();

        // Sort
        match sort_by.as_str() {
            "memory" | "mem" => {
                procs.sort_by(|a, b| {
                    let ma = a["memory_bytes"].as_u64().unwrap_or(0);
                    let mb = b["memory_bytes"].as_u64().unwrap_or(0);
                    mb.cmp(&ma)
                });
            }
            "pid" => {
                procs.sort_by(|a, b| {
                    let pa = a["pid"].as_u64().unwrap_or(0);
                    let pb = b["pid"].as_u64().unwrap_or(0);
                    pa.cmp(&pb)
                });
            }
            "name" => {
                procs.sort_by(|a, b| {
                    let na = a["name"].as_str().unwrap_or("");
                    let nb = b["name"].as_str().unwrap_or("");
                    na.to_lowercase().cmp(&nb.to_lowercase())
                });
            }
            _ => {
                // Default: sort by CPU descending
                procs.sort_by(|a, b| {
                    let ca = a["cpu_percent"].as_f64().unwrap_or(0.0);
                    let cb = b["cpu_percent"].as_f64().unwrap_or(0.0);
                    cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        procs.truncate(limit);

        serde_json::json!({
            "sort_by": sort_by,
            "count": procs.len(),
            "processes": procs,
        })
        .to_string()
    }

    #[tool(description = "Send a signal to a process by PID. Default signal is SIGTERM. Use SIGKILL for force kill.")]
    async fn kill_process(&self, Parameters(input): Parameters<KillProcessInput>) -> String {
        let pid = sysinfo::Pid::from_u32(input.pid);
        let signal_str = input.signal.unwrap_or_else(|| "SIGTERM".to_string());

        let signal = match signal_str.to_uppercase().as_str() {
            "SIGKILL" | "KILL" | "9" => sysinfo::Signal::Kill,
            "SIGINT" | "INT" | "2" => sysinfo::Signal::Interrupt,
            _ => sysinfo::Signal::Term,
        };

        let sys = System::new_all();
        match sys.process(pid) {
            Some(process) => {
                if process.kill_with(signal).unwrap_or(false) {
                    serde_json::json!({
                        "ok": true,
                        "pid": input.pid,
                        "signal": signal_str,
                    })
                    .to_string()
                } else {
                    serde_json::json!({
                        "ok": false,
                        "pid": input.pid,
                        "error": "failed to send signal (permission denied?)",
                    })
                    .to_string()
                }
            }
            None => serde_json::json!({
                "error": format!("process not found: {}", input.pid),
            })
            .to_string(),
        }
    }
}

#[tool_handler]
impl ServerHandler for MyakuMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Myaku GPU system monitor — CPU, memory, disk, network metrics and process management."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let server = MyakuMcp::new().serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}
