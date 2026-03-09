//! Metrics collection engine.
//!
//! Periodically collects system metrics via the platform abstraction layer,
//! stores history in ring buffers, and provides data to the rendering layer.

pub mod cpu;
pub mod disk;
pub mod memory;
pub mod network;

use crate::config::MyakuConfig;
use crate::platform::{self, SystemMetrics};

use cpu::CpuMetrics;
use disk::DiskMetrics;
use memory::MemoryMetrics;
use network::NetworkMetrics;

/// Central metrics collector that orchestrates all subsystem metrics.
pub struct MetricsCollector {
    /// Platform-specific metrics source.
    source: Box<dyn SystemMetrics>,
    /// CPU metrics with history.
    pub cpu: CpuMetrics,
    /// Memory metrics with history.
    pub memory: MemoryMetrics,
    /// Disk metrics with history.
    pub disk: DiskMetrics,
    /// Network metrics with history.
    pub network: NetworkMetrics,
    /// System uptime in seconds.
    pub uptime_secs: u64,
    /// Load averages (1, 5, 15 min).
    pub load_average: [f64; 3],
}

impl MetricsCollector {
    /// Create a new metrics collector.
    #[must_use]
    pub fn new(config: &MyakuConfig) -> Self {
        let source = platform::create_metrics();
        let history_len = config.monitoring.history_seconds as usize;

        // Get initial CPU info to know core count.
        let cpu_info = source.cpu_info();
        let core_count = cpu_info.per_core.len();

        Self {
            source,
            cpu: CpuMetrics::new(core_count, history_len),
            memory: MemoryMetrics::new(history_len),
            disk: DiskMetrics::new(history_len),
            network: NetworkMetrics::new(history_len),
            uptime_secs: 0,
            load_average: [0.0; 3],
        }
    }

    /// Refresh all metrics from the platform. Call this periodically.
    pub fn refresh(&mut self) {
        self.source.refresh();

        self.cpu.update(&self.source.cpu_info());
        self.memory.update(&self.source.memory_info());
        self.disk.update(&self.source.disk_info());
        self.network.update(&self.source.network_info());
        self.uptime_secs = self.source.uptime_secs();
        self.load_average = self.source.load_average();
    }

    /// Get a sorted, filtered process list snapshot.
    #[must_use]
    pub fn processes(&self, sort_by: &str, ascending: bool) -> Vec<platform::ProcessInfo> {
        let mut procs = self.source.process_list();

        match sort_by {
            "cpu" => procs.sort_by(|a, b| a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal)),
            "memory" => procs.sort_by(|a, b| a.memory.cmp(&b.memory)),
            "pid" => procs.sort_by(|a, b| a.pid.cmp(&b.pid)),
            "name" => procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            _ => procs.sort_by(|a, b| a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal)),
        }

        if !ascending {
            procs.reverse();
        }

        procs
    }

    /// Format uptime as "Xd Xh Xm Xs".
    #[must_use]
    pub fn uptime_display(&self) -> String {
        let s = self.uptime_secs;
        let days = s / 86400;
        let hours = (s % 86400) / 3600;
        let minutes = (s % 3600) / 60;
        let seconds = s % 60;
        if days > 0 {
            format!("{days}d {hours}h {minutes}m")
        } else if hours > 0 {
            format!("{hours}h {minutes}m {seconds}s")
        } else {
            format!("{minutes}m {seconds}s")
        }
    }

    /// Format load average.
    #[must_use]
    pub fn load_display(&self) -> String {
        format!(
            "{:.2} {:.2} {:.2}",
            self.load_average[0], self.load_average[1], self.load_average[2]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_format_seconds() {
        let config = MyakuConfig::default();
        let mut mc = MetricsCollector::new(&config);
        mc.uptime_secs = 45;
        assert_eq!(mc.uptime_display(), "0m 45s");
    }

    #[test]
    fn uptime_format_hours() {
        let config = MyakuConfig::default();
        let mut mc = MetricsCollector::new(&config);
        mc.uptime_secs = 3661;
        assert_eq!(mc.uptime_display(), "1h 1m 1s");
    }

    #[test]
    fn uptime_format_days() {
        let config = MyakuConfig::default();
        let mut mc = MetricsCollector::new(&config);
        mc.uptime_secs = 90061;
        assert_eq!(mc.uptime_display(), "1d 1h 1m");
    }

    #[test]
    fn load_display() {
        let config = MyakuConfig::default();
        let mut mc = MetricsCollector::new(&config);
        mc.load_average = [1.5, 2.0, 1.8];
        assert_eq!(mc.load_display(), "1.50 2.00 1.80");
    }
}
