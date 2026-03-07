# Myaku (脈) — GPU System Monitor

## Build & Test

```bash
cargo build                    # compile
cargo test --lib               # unit tests
cargo run                      # launch GUI
cargo run -- daemon            # start metrics collection daemon
```

## Architecture

### Pipeline

```
sysinfo → Metrics Collector → History Store
                                    |
  Timer Event → Refresh → Alert Check → GPU Render
```

### Platform Isolation (`src/platform/`)

| Trait | macOS Impl | Purpose |
|-------|------------|---------|
| `SystemMetrics` | `MacOSMetrics` | CPU, memory, disk, network, process list |

Linux implementations will be added under `src/platform/linux/`.

### Configuration

Uses **shikumi** for config discovery and hot-reload:
- Config file: `~/.config/myaku/myaku.yaml`
- Env override: `$MYAKU_CONFIG`
- Env vars: `MYAKU_` prefix (e.g. `MYAKU_APPEARANCE__REFRESH_RATE_MS=500`)
- Hot-reload on file change (nix-darwin symlink aware)

## File Map

| Path | Purpose |
|------|---------|
| `src/config.rs` | Config struct (uses shikumi) |
| `src/platform/mod.rs` | Platform trait definitions (SystemMetrics, MemoryInfo, ProcessInfo) |
| `src/platform/macos/mod.rs` | macOS metrics via sysinfo |
| `src/main.rs` | CLI entry point (GUI + daemon subcommands) |
| `src/lib.rs` | Library root |
| `module/default.nix` | HM module with typed options + daemon |

## Design Decisions

### Configuration Language: YAML
- YAML is the primary and only configuration format
- Config file: `~/.config/myaku/myaku.yaml`
- Nix HM module generates YAML via `lib.generators.toYAML` from typed options
- Typed options mirror `MyakuConfig` struct: appearance, monitoring, processes, alerts, daemon
- `extraSettings` escape hatch for raw attrset merge on top of typed options

### Nix Integration
- Flake exports: `packages`, `overlays.default`, `homeManagerModules.default`, `devShells`
- HM module at `blackmatter.components.myaku` with fully typed options:
  - `appearance.{width, height, font_size, opacity, refresh_rate_ms}`
  - `monitoring.{show_cpu, show_memory, show_disk, show_network, show_gpu}`
  - `processes.{sort_by, show_threads, auto_refresh}`
  - `alerts.{cpu_threshold, memory_threshold, disk_threshold}`
  - `daemon.{enable, metrics_port, history_retention_hours}` with launchd/systemd service
  - `extraSettings` — raw attrset escape hatch
- YAML generated via `lib.generators.toYAML` -> `xdg.configFile."myaku/myaku.yaml"`
- Cross-platform: `mkLaunchdService` (macOS) + `mkSystemdService` (Linux) for daemon
- Uses substrate's `hm-service-helpers.nix` for service generation

### Cross-Platform Strategy
- Platform-specific: behind trait boundaries in `src/platform/`
- System metrics: sysinfo crate (cross-platform)
- Config: shikumi for discovery and hot-reload
