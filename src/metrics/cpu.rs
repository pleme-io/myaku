//! CPU metrics collection and history tracking.

use crate::graph::{SeriesGroup, SparklineData};
use crate::platform::CpuInfo;

/// CPU metrics with historical data per-core and total.
#[derive(Debug, Clone)]
pub struct CpuMetrics {
    /// Per-core usage history + summary.
    pub cores: SeriesGroup,
    /// CPU brand name.
    pub brand: String,
    /// Number of physical cores.
    pub core_count: usize,
}

#[allow(dead_code)]
impl CpuMetrics {
    /// Create a new CPU metrics tracker.
    #[must_use]
    pub fn new(core_count: usize, history_len: usize) -> Self {
        Self {
            cores: SeriesGroup::new("Core", core_count, history_len),
            brand: String::new(),
            core_count,
        }
    }

    /// Update with new CPU information.
    pub fn update(&mut self, info: &CpuInfo) {
        // Resize if core count changed (unlikely but handle it)
        if info.per_core.len() != self.cores.series.len() {
            self.cores
                .resize(info.per_core.len(), self.cores.summary.capacity());
        }
        self.cores.push_all(&info.per_core);
        self.brand.clone_from(&info.brand);
        self.core_count = info.core_count;
    }

    /// Get the total (average) CPU usage percentage.
    #[must_use]
    pub fn total_usage(&self) -> f32 {
        self.cores.summary.latest().unwrap_or(0.0)
    }

    /// Get per-core sparkline data for rendering.
    #[must_use]
    pub fn sparklines(&self, color: [f32; 4]) -> Vec<SparklineData> {
        self.cores
            .series
            .iter()
            .enumerate()
            .map(|(i, (_, buf))| {
                SparklineData::from_ring_buffer(buf, format!("Core {i}"), 100.0, color)
            })
            .collect()
    }

    /// Get the total usage sparkline.
    #[must_use]
    pub fn total_sparkline(&self, color: [f32; 4]) -> SparklineData {
        SparklineData::from_ring_buffer(&self.cores.summary, "CPU Total", 100.0, color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::CpuInfo;

    fn sample_cpu_info() -> CpuInfo {
        CpuInfo {
            per_core: vec![10.0, 20.0, 30.0, 40.0],
            total: 25.0,
            brand: "Apple M1".into(),
            core_count: 4,
        }
    }

    #[test]
    fn initial_state() {
        let cpu = CpuMetrics::new(4, 60);
        assert!((cpu.total_usage() - 0.0).abs() < f32::EPSILON);
        assert_eq!(cpu.core_count, 4);
    }

    #[test]
    fn update_records_data() {
        let mut cpu = CpuMetrics::new(4, 60);
        cpu.update(&sample_cpu_info());
        assert!((cpu.total_usage() - 25.0).abs() < f32::EPSILON);
        assert_eq!(cpu.brand, "Apple M1");
    }

    #[test]
    fn sparklines_generated() {
        let mut cpu = CpuMetrics::new(4, 60);
        cpu.update(&sample_cpu_info());
        let sparks = cpu.sparklines([1.0, 0.0, 0.0, 1.0]);
        assert_eq!(sparks.len(), 4);
        assert!((sparks[0].current - 10.0).abs() < f32::EPSILON);
    }
}
