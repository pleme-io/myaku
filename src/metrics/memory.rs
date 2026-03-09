//! Memory (RAM + swap) metrics collection and history tracking.

use crate::graph::{RingBuffer, SparklineData};
use crate::platform::MemoryInfo;

/// Memory metrics with historical data.
#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    /// RAM usage percentage history.
    pub ram_history: RingBuffer,
    /// Swap usage percentage history.
    pub swap_history: RingBuffer,
    /// Latest raw memory info snapshot.
    pub latest: MemoryInfo,
}

#[allow(dead_code)]
impl MemoryMetrics {
    /// Create a new memory metrics tracker.
    #[must_use]
    pub fn new(history_len: usize) -> Self {
        Self {
            ram_history: RingBuffer::new(history_len),
            swap_history: RingBuffer::new(history_len),
            latest: MemoryInfo {
                total: 0,
                used: 0,
                available: 0,
                swap_total: 0,
                swap_used: 0,
            },
        }
    }

    /// Update with new memory information.
    pub fn update(&mut self, info: &MemoryInfo) {
        let ram_pct = if info.total > 0 {
            (info.used as f64 / info.total as f64 * 100.0) as f32
        } else {
            0.0
        };
        self.ram_history.push(ram_pct);

        let swap_pct = if info.swap_total > 0 {
            (info.swap_used as f64 / info.swap_total as f64 * 100.0) as f32
        } else {
            0.0
        };
        self.swap_history.push(swap_pct);

        self.latest = info.clone();
    }

    /// Current RAM usage percentage.
    #[must_use]
    pub fn ram_percent(&self) -> f32 {
        self.ram_history.latest().unwrap_or(0.0)
    }

    /// Current swap usage percentage.
    #[must_use]
    pub fn swap_percent(&self) -> f32 {
        self.swap_history.latest().unwrap_or(0.0)
    }

    /// Format total RAM as a human-readable string (e.g. "16.0 GB").
    #[must_use]
    pub fn total_ram_display(&self) -> String {
        format_bytes(self.latest.total)
    }

    /// Format used RAM as a human-readable string.
    #[must_use]
    pub fn used_ram_display(&self) -> String {
        format_bytes(self.latest.used)
    }

    /// Format total swap as a human-readable string.
    #[must_use]
    pub fn total_swap_display(&self) -> String {
        format_bytes(self.latest.swap_total)
    }

    /// Format used swap as a human-readable string.
    #[must_use]
    pub fn used_swap_display(&self) -> String {
        format_bytes(self.latest.swap_used)
    }

    /// Get RAM usage sparkline for rendering.
    #[must_use]
    pub fn ram_sparkline(&self, color: [f32; 4]) -> SparklineData {
        SparklineData::from_ring_buffer(&self.ram_history, "RAM", 100.0, color)
    }

    /// Get swap usage sparkline for rendering.
    #[must_use]
    pub fn swap_sparkline(&self, color: [f32; 4]) -> SparklineData {
        SparklineData::from_ring_buffer(&self.swap_history, "Swap", 100.0, color)
    }
}

/// Format bytes to a human-readable string with appropriate unit.
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_memory() -> MemoryInfo {
        MemoryInfo {
            total: 16 * 1024 * 1024 * 1024,     // 16 GB
            used: 8 * 1024 * 1024 * 1024,        // 8 GB
            available: 8 * 1024 * 1024 * 1024,   // 8 GB
            swap_total: 4 * 1024 * 1024 * 1024,  // 4 GB
            swap_used: 1024 * 1024 * 1024,        // 1 GB
        }
    }

    #[test]
    fn initial_state() {
        let mem = MemoryMetrics::new(60);
        assert!((mem.ram_percent() - 0.0).abs() < f32::EPSILON);
        assert!((mem.swap_percent() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn update_calculates_percentages() {
        let mut mem = MemoryMetrics::new(60);
        mem.update(&sample_memory());
        assert!((mem.ram_percent() - 50.0).abs() < 0.1);
        assert!((mem.swap_percent() - 25.0).abs() < 0.1);
    }

    #[test]
    fn display_strings() {
        let mut mem = MemoryMetrics::new(60);
        mem.update(&sample_memory());
        assert_eq!(mem.total_ram_display(), "16.0 GB");
        assert_eq!(mem.used_ram_display(), "8.0 GB");
    }

    #[test]
    fn format_bytes_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.0 TB");
    }
}
