//! Myaku configuration — uses shikumi for discovery and hot-reload.

use serde::{Deserialize, Serialize};

/// Top-level configuration.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct MyakuConfig {
    pub appearance: AppearanceConfig,
    pub monitoring: MonitoringConfig,
    pub processes: ProcessConfig,
    pub alerts: AlertConfig,
    pub daemon: DaemonConfig,
}

impl Default for MyakuConfig {
    fn default() -> Self {
        Self {
            appearance: AppearanceConfig::default(),
            monitoring: MonitoringConfig::default(),
            processes: ProcessConfig::default(),
            alerts: AlertConfig::default(),
            daemon: DaemonConfig::default(),
        }
    }
}

/// Visual appearance settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct AppearanceConfig {
    /// Window width in pixels.
    pub width: u32,
    /// Window height in pixels.
    pub height: u32,
    /// Font size in points.
    pub font_size: f32,
    /// Background opacity (0.0-1.0).
    pub opacity: f32,
    /// Refresh rate in milliseconds.
    pub refresh_rate_ms: u32,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 800,
            font_size: 13.0,
            opacity: 0.95,
            refresh_rate_ms: 1000,
        }
    }
}

/// Which subsystems to monitor.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct MonitoringConfig {
    /// Show CPU usage.
    pub show_cpu: bool,
    /// Show memory usage.
    pub show_memory: bool,
    /// Show disk usage.
    pub show_disk: bool,
    /// Show network activity.
    pub show_network: bool,
    /// Show GPU usage (macOS only).
    pub show_gpu: bool,
    /// How many seconds of history to keep in graphs.
    pub history_seconds: u32,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            show_cpu: true,
            show_memory: true,
            show_disk: true,
            show_network: true,
            show_gpu: false,
            history_seconds: 300,
        }
    }
}

/// Process list settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct ProcessConfig {
    /// Sort column (cpu, memory, pid, name).
    pub sort_by: String,
    /// Sort direction (asc, desc).
    pub sort_direction: String,
    /// Show per-process threads.
    pub show_threads: bool,
    /// Auto-refresh the process list.
    pub auto_refresh: bool,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            sort_by: "cpu".into(),
            sort_direction: "desc".into(),
            show_threads: false,
            auto_refresh: true,
        }
    }
}

/// Resource usage alert thresholds.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct AlertConfig {
    /// CPU usage threshold percentage (0-100).
    pub cpu_threshold: f32,
    /// Memory usage threshold percentage (0-100).
    pub memory_threshold: f32,
    /// Disk usage threshold percentage (0-100).
    pub disk_threshold: f32,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            cpu_threshold: 90.0,
            memory_threshold: 85.0,
            disk_threshold: 90.0,
        }
    }
}

/// Daemon mode configuration.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct DaemonConfig {
    /// Enable metrics collection daemon.
    pub enable: bool,
    /// Port for metrics endpoint.
    pub metrics_port: u16,
    /// Hours of metric history to retain.
    pub history_retention_hours: u32,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            enable: false,
            metrics_port: 9100,
            history_retention_hours: 24,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = MyakuConfig::default();
        assert_eq!(config.appearance.width, 1200);
        assert_eq!(config.appearance.refresh_rate_ms, 1000);
        assert!(config.monitoring.show_cpu);
        assert!(!config.daemon.enable);
        assert_eq!(config.monitoring.history_seconds, 300);
    }

    #[test]
    fn alert_thresholds_are_sane() {
        let config = MyakuConfig::default();
        assert!(config.alerts.cpu_threshold > 0.0);
        assert!(config.alerts.cpu_threshold <= 100.0);
        assert!(config.alerts.memory_threshold > 0.0);
        assert!(config.alerts.memory_threshold <= 100.0);
    }

    #[test]
    fn process_config_defaults() {
        let config = MyakuConfig::default();
        assert_eq!(config.processes.sort_by, "cpu");
        assert_eq!(config.processes.sort_direction, "desc");
        assert!(config.processes.auto_refresh);
    }
}
