# Myaku (脈) — GPU System Monitor

> **★★★ CSE / Knowable Construction.** This repo operates under **Constructive Substrate Engineering** — canonical specification at [`pleme-io/theory/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md`](https://github.com/pleme-io/theory/blob/main/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md). The Compounding Directive (operational rules: solve once, load-bearing fixes only, idiom-first, models stay current, direction beats velocity) is in the org-level pleme-io/CLAUDE.md ★★★ section. Read both before non-trivial changes.


Crate: `myaku` | Binary: `myaku` | Config app name: `myaku`

GPU-rendered system monitor with real-time resource graphs, process management, and
customizable dashboard. Uses sysinfo for cross-platform metrics and garasu for GPU
chart rendering.

## Build & Test

```bash
cargo build                    # compile
cargo test --lib               # unit tests
cargo run                      # launch GUI
cargo run -- daemon            # start metrics collection daemon
cargo run -- snapshot          # print current metrics to stdout (JSON)
```

Nix build:
```bash
nix build                     # build via substrate rust-tool-release-flake
nix run                       # run
nix run .#regenerate           # regenerate Cargo.nix after Cargo.toml changes
```

## Competitive Position

| Competitor | Stack | Our advantage |
|-----------|-------|---------------|
| **btop++** | C++, optional GPU | Full wgpu GPU rendering, MCP-drivable, Rhai plugins |
| **bottom** | Rust, TUI | GPU-rendered (not TUI), MCP automation, Rhai plugins |
| **Zenith** | Rust, TUI | More customizable widgets, MCP, Rhai, Nix-configured |
| **htop** | C, TUI | GPU rendering, historical graphs, MCP automation |
| **glances** | Python, web UI | Native performance, GPU rendering, vim-modal |

Unique value: GPU-rendered system dashboard with customizable widget layout,
MCP for remote/AI-driven monitoring, and Rhai custom widgets.

## Architecture

### Module Map

```
src/
  main.rs                      ← CLI entry point (clap: open, daemon, snapshot)
  lib.rs                       ← Library root (re-exports config + platform)
  config.rs                    ← MyakuConfig via shikumi

  platform/
    mod.rs                     ← Platform trait definitions (SystemMetrics, ProcessInfo, etc.)
    macos/
      mod.rs                   ← macOS metrics via sysinfo + IOKit

  metrics/                     ← (planned) Metrics collection engine
    mod.rs                     ← MetricsCollector: periodic collection, broadcasting
    cpu.rs                     ← CPU usage (per-core, total, frequency, temperature)
    memory.rs                  ← Memory (used, free, swap, pressure)
    disk.rs                    ← Disk I/O (read/write bytes/s, usage per mount)
    network.rs                 ← Network I/O (rx/tx bytes/s per interface)
    gpu.rs                     ← GPU usage (Metal/Vulkan, VRAM, temperature)
    battery.rs                 ← Battery (charge%, charging, time remaining)
    temperature.rs             ← Temperature sensors (CPU, GPU, ambient)

  process/                     ← (planned) Process management
    mod.rs                     ← ProcessList: sort, filter, tree view
    tree.rs                    ← Process tree builder (parent-child hierarchy)
    actions.rs                 ← Kill, renice, signal operations

  graph/                       ← (planned) Historical data + chart rendering
    mod.rs                     ← GraphEngine: ring buffer store, chart types
    ring_buffer.rs             ← Fixed-size circular buffer for time series
    line_chart.rs              ← GPU line/area chart (garasu primitives)
    bar_chart.rs               ← GPU bar chart (CPU cores, disk usage)
    sparkline.rs               ← Compact sparkline for inline metrics

  dashboard/                   ← (planned) Composable widget layout
    mod.rs                     ← Dashboard: widget grid, layout manager
    widget.rs                  ← Widget trait + built-in widget catalog
    layout.rs                  ← Grid layout engine (rows, columns, spans)

  render/                      ← (planned) GPU rendering
    mod.rs                     ← MyakuRenderer: madori RenderCallback
    charts.rs                  ← Chart rendering helpers (axes, labels, legends)
    process_table.rs           ← Process list table rendering
    status_bar.rs              ← Bottom bar (uptime, load, CPU summary)

  mcp/                         ← (planned) MCP server via kaname
    mod.rs                     ← MyakuMcp server struct
    tools.rs                   ← Tool implementations

  scripting/                   ← (planned) Rhai scripting via soushi
    mod.rs                     ← Engine setup, myaku.* API registration

module/
  default.nix                  ← HM module (blackmatter.components.myaku)
```

### Data Flow

```
sysinfo crate + platform APIs
         │
         ▼
   MetricsCollector (periodic timer, configurable interval)
         │
         ├──▸ RingBuffer[] (time series history per metric)
         │         │
         │         ▼
         │    GraphEngine ──▸ GPU charts (line, area, bar, sparkline)
         │
         ├──▸ ProcessList (sorted, filtered, tree-structured)
         │         │
         │         ▼
         │    ProcessTable ──▸ GPU table rendering
         │
         └──▸ Dashboard (widget grid layout)
                   │
                   ▼
              GPU Render (garasu/madori/egaku)
                   │
              Input Events (awase hotkeys)
                   │
              Process Actions (kill, renice)
```

### Platform Isolation

The `SystemMetrics` trait abstracts platform-specific metric collection:

| Trait Method | Purpose |
|-------------|---------|
| `cpu()` | CPU usage per core + total, frequency, temperature |
| `memory()` | RAM used/free/total, swap used/free/total |
| `disks()` | Per-mount usage, I/O bytes/s |
| `networks()` | Per-interface rx/tx bytes/s |
| `processes()` | Process list with CPU%, MEM%, PID, name, user, state |
| `gpu()` | GPU usage%, VRAM, temperature (platform-specific) |
| `battery()` | Charge%, state, time remaining |
| `temperatures()` | All temperature sensors |

Primary implementation via `sysinfo` crate (cross-platform). Platform-specific
extensions for GPU metrics (IOKit on macOS, procfs on Linux).

### Current Implementation Status

**Done:**
- `config.rs` — shikumi integration with appearance/monitoring/processes/alerts/daemon
- `platform/mod.rs` — Platform trait definitions
- `platform/macos/mod.rs` — macOS metrics via sysinfo
- `main.rs` — CLI entry point with GUI + daemon + snapshot subcommands
- `lib.rs` — Library root
- `module/default.nix` — Full HM module with typed options + daemon service
- `flake.nix` — substrate rust-tool-release-flake + HM module

**Not started:**
- GPU rendering via madori/garasu/egaku
- Historical graph engine (ring buffers, GPU charts)
- Dashboard widget layout system
- Process tree view and actions (kill, renice)
- GPU-specific metrics (Metal/Vulkan)
- MCP server via kaname
- Rhai scripting via soushi
- Hotkey system via awase

## Configuration

Uses **shikumi** for config discovery and hot-reload:
- Config file: `~/.config/myaku/myaku.yaml`
- Env override: `$MYAKU_CONFIG`
- Env prefix: `MYAKU_` (e.g., `MYAKU_APPEARANCE__REFRESH_RATE_MS=500`)
- Hot-reload on file change (nix-darwin symlink aware)

### Config Schema

```yaml
appearance:
  width: 1200
  height: 800
  font_size: 13.0
  opacity: 0.95
  refresh_rate_ms: 1000              # UI refresh interval

monitoring:
  show_cpu: true
  show_memory: true
  show_disk: true
  show_network: true
  show_gpu: true
  show_battery: true
  show_temperature: true
  history_seconds: 300               # how many seconds of history to keep in graphs

processes:
  sort_by: "cpu"                     # cpu | memory | pid | name
  sort_direction: "desc"             # asc | desc
  show_threads: false
  auto_refresh: true
  tree_view: false

alerts:
  cpu_threshold: 90                  # alert above this % CPU
  memory_threshold: 85               # alert above this % memory
  disk_threshold: 95                 # alert above this % disk usage
  temperature_threshold: 80          # alert above this temp (Celsius)

dashboard:
  layout:                            # widget grid layout
    - row: [cpu_chart, memory_chart]
    - row: [disk_chart, network_chart]
    - row: [process_table]

daemon:
  enable: false
  metrics_port: 9100                 # Prometheus-compatible metrics endpoint
  history_retention_hours: 24
```

## Shared Library Integration

| Library | Usage |
|---------|-------|
| **shikumi** | Config discovery + hot-reload (`MyakuConfig`) |
| **garasu** | GPU rendering for charts, tables, dashboard |
| **madori** | App framework (event loop, render loop, timed refresh) |
| **egaku** | Widgets (split pane for dashboard, list for processes, tabs for views) |
| **irodzuki** | Theme: base16 to GPU uniforms (chart colors, backgrounds) |
| **tsunagu** | Daemon mode for metrics collection + Prometheus endpoint |
| **kaname** | MCP server framework |
| **soushi** | Rhai scripting engine (custom widgets, alert actions) |
| **awase** | Hotkey system for vim-modal navigation |
| **tsuuchi** | Notifications (threshold alerts) |

## MCP Server (kaname)

Standard tools: `status`, `config_get`, `config_set`, `version`

App-specific tools:
- `get_cpu()` — CPU usage (per-core, total, frequency)
- `get_memory()` — memory and swap usage
- `get_disk()` — disk usage and I/O per mount
- `get_network()` — network I/O per interface
- `get_processes(sort_by?, limit?)` — process list
- `kill_process(pid, signal?)` — send signal to process
- `get_gpu()` — GPU usage and VRAM
- `get_battery()` — battery state
- `get_temperature()` — all temperature sensors
- `get_uptime()` — system uptime
- `get_load_average()` — 1/5/15 minute load averages

## Rhai Scripting (soushi)

Scripts from `~/.config/myaku/scripts/*.rhai`

```rhai
// Available API:
myaku.cpu()                   // -> {total: 45.2, cores: [30.1, 60.5, ...], freq_mhz: 3200}
myaku.memory()                // -> {used_gb: 12.4, total_gb: 32.0, swap_used_gb: 0.5}
myaku.disk()                  // -> [{mount: "/", used_gb: 200, total_gb: 500, io_read_mb: 1.2}]
myaku.network()               // -> [{name: "en0", rx_mb: 5.2, tx_mb: 1.1}]
myaku.processes()             // -> [{pid: 1234, name: "rust-analyzer", cpu: 15.0, mem_mb: 400}]
myaku.kill(1234)              // kill process
myaku.kill(1234, "SIGTERM")   // kill with specific signal
myaku.gpu()                   // -> {usage: 30, vram_used_mb: 1024, temp: 65}
myaku.battery()               // -> {charge: 85, charging: true, time_remaining_min: 120}
myaku.refresh()               // force immediate metrics refresh
myaku.widget_add("sparkline", #{metric: "cpu", position: [0, 0]})
```

Event hooks: `on_startup`, `on_shutdown`, `on_alert(metric, value, threshold)`,
`on_process_exit(pid, name, exit_code)`

Example: alert when memory exceeds 90%:
```rhai
fn on_alert(metric, value, threshold) {
    if metric == "memory" {
        // Find top memory consumers
        let procs = myaku.processes();
        procs.sort(|a, b| b.mem_mb - a.mem_mb);
        let top3 = procs[0..3];
        notify("Memory alert", `${value}% used. Top: ${top3[0].name}`);
    }
}
```

## Hotkey System (awase)

### Modes

**Normal** (default — dashboard view):
| Key | Action |
|-----|--------|
| `Tab` | Cycle focus between widgets |
| `1-5` | Jump to widget by index |
| `p` | Switch to process view |
| `g` | Switch to graph view |
| `r` | Force refresh |
| `+/-` | Increase/decrease refresh interval |
| `q` | Quit |
| `:` | Command mode |

**Process** (process table view):
| Key | Action |
|-----|--------|
| `j/k` | Navigate processes |
| `Enter` | Expand process (children, threads, details) |
| `K` | Kill process (confirm) |
| `r` | Renice process |
| `f` | Follow process (keep highlighted on refresh) |
| `t` | Toggle tree view |
| `s` | Cycle sort column (CPU, MEM, PID, name) |
| `S` | Reverse sort direction |
| `/` | Filter processes by name |
| `Esc` | Back to dashboard |

**Command** (`:` prefix):
- `:sort cpu|mem|pid|name` — sort process list
- `:filter <pattern>` — filter processes
- `:kill <pid>` — kill process
- `:interval <ms>` — set refresh interval
- `:layout <preset>` — switch dashboard layout

## Nix Integration

### Flake Exports
- Multi-platform packages via substrate `rust-tool-release-flake.nix`
- `overlays.default` — `pkgs.myaku`
- `homeManagerModules.default` — `blackmatter.components.myaku`
- `devShells` — dev environment

### HM Module

Namespace: `blackmatter.components.myaku`

Fully implemented with typed options:
- `enable` — install package + generate config
- `package` — override package
- `appearance.{width, height, font_size, opacity, refresh_rate_ms}`
- `monitoring.{show_cpu, show_memory, show_disk, show_network, show_gpu}`
- `processes.{sort_by, show_threads, auto_refresh}`
- `alerts.{cpu_threshold, memory_threshold, disk_threshold}`
- `daemon.{enable, metrics_port, history_retention_hours}` — launchd/systemd service
- `extraSettings` — raw attrset escape hatch

YAML generated via `lib.generators.toYAML` -> `xdg.configFile."myaku/myaku.yaml"`.
Uses substrate's `hm-service-helpers.nix` for `mkLaunchdService`/`mkSystemdService`.

## Dashboard Design

### Default Layout

```
┌──────────────────────────┬──────────────────────────┐
│ CPU Usage (area chart)   │ Memory (area chart)      │
│ ████████████░░░░░░ 65%   │ ██████████░░░░░░░ 55%    │
│ [per-core sparklines]    │ RAM: 17.6/32 GB          │
│                          │ Swap: 0.5/4 GB           │
├──────────────────────────┼──────────────────────────┤
│ Disk I/O (line chart)    │ Network (line chart)     │
│ Read:  █░ 45 MB/s        │ ↓ 12.5 MB/s  en0        │
│ Write: ██░ 120 MB/s      │ ↑  3.2 MB/s  en0        │
├──────────────────────────┴──────────────────────────┤
│ Processes                              Sort: CPU ▼  │
│ PID    NAME              CPU%  MEM%  USER     STATE │
│ 1234   rust-analyzer     15.2  12.0  drzzln   R     │
│ 5678   chrome            12.8  18.5  drzzln   S     │
│ 9012   WindowServer       8.5   4.2  _window  R     │
│ ...                                                  │
└──────────────────────────────────────────────────────┘
```

### Chart Rendering (GPU)

Charts are rendered via garasu primitives:
- **Area chart** — filled polygon below line, semi-transparent fill
- **Line chart** — polyline with configurable thickness
- **Bar chart** — rectangles for per-core CPU, per-mount disk
- **Sparkline** — compact inline chart (single line, no axes)
- **Axes** — time on X (with labels), percentage/value on Y
- **Colors** — from irodzuki theme (semantic: good=green, warn=yellow, critical=red)

Ring buffers store history: configurable length (default 300 data points at 1s interval
= 5 minutes). Each metric has its own ring buffer. Charts render from ring buffer data.

### Widget System

Widgets are composable egaku components:
- Each widget implements a `DashboardWidget` trait: `update(metrics)`, `render(area)`, `handle_key(key)`
- Layout is a grid of rows, each row contains widgets with relative sizing
- Widgets can be added/removed/rearranged via Rhai scripts or config

Built-in widgets: `cpu_chart`, `memory_chart`, `disk_chart`, `network_chart`,
`process_table`, `gpu_chart`, `battery`, `temperature`, `uptime`, `load_average`

## Design Constraints

- **sysinfo as primary source** — use sysinfo crate for cross-platform metrics, extend with platform APIs only for GPU/battery
- **Ring buffer for history** — fixed-size circular buffers, never unbounded growth
- **Refresh interval is configurable** — default 1s, minimum 100ms, impacts CPU usage
- **Process kill requires confirm** — no accidental kills; `:kill!` to force without confirm
- **Dashboard layout is data** — layout defined in config YAML, not hardcoded
- **Chart rendering is batched** — all chart geometry submitted in minimal draw calls
- **Daemon mode is headless** — no GPU rendering, just metrics collection + optional Prometheus endpoint
- **Alerts are threshold-based** — simple value > threshold, no anomaly detection
