//! Disk usage and I/O metrics collection.

use crate::graph::RingBuffer;
use crate::platform::DiskInfo;
use crate::metrics::memory::format_bytes;

/// Per-mount disk usage with history.
#[derive(Debug, Clone)]
pub struct DiskMetrics {
    /// Per-mount usage info.
    pub mounts: Vec<MountMetrics>,
    /// History buffer capacity.
    history_len: usize,
}

/// Metrics for a single mount point.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MountMetrics {
    /// Mount point path.
    pub mount_point: String,
    /// Device/disk name.
    pub name: String,
    /// File system type.
    pub fs_type: String,
    /// Usage percentage history.
    pub usage_history: RingBuffer,
    /// Latest raw disk info.
    pub total: u64,
    pub used: u64,
    pub available: u64,
}

#[allow(dead_code)]
impl MountMetrics {
    fn new(info: &DiskInfo, history_len: usize) -> Self {
        let mut m = Self {
            mount_point: info.mount_point.clone(),
            name: info.name.clone(),
            fs_type: info.fs_type.clone(),
            usage_history: RingBuffer::new(history_len),
            total: info.total,
            used: info.used,
            available: info.available,
        };
        m.record_usage(info);
        m
    }

    fn record_usage(&mut self, info: &DiskInfo) {
        self.total = info.total;
        self.used = info.used;
        self.available = info.available;
        let pct = if info.total > 0 {
            (info.used as f64 / info.total as f64 * 100.0) as f32
        } else {
            0.0
        };
        self.usage_history.push(pct);
    }

    /// Current usage percentage.
    #[must_use]
    pub fn usage_percent(&self) -> f32 {
        self.usage_history.latest().unwrap_or(0.0)
    }

    /// Human-readable total size.
    #[must_use]
    pub fn total_display(&self) -> String {
        format_bytes(self.total)
    }

    /// Human-readable used size.
    #[must_use]
    pub fn used_display(&self) -> String {
        format_bytes(self.used)
    }

    /// Human-readable available size.
    #[must_use]
    pub fn available_display(&self) -> String {
        format_bytes(self.available)
    }
}

#[allow(dead_code)]
impl DiskMetrics {
    /// Create a new disk metrics tracker.
    #[must_use]
    pub fn new(history_len: usize) -> Self {
        Self {
            mounts: Vec::new(),
            history_len,
        }
    }

    /// Update with new disk information.
    pub fn update(&mut self, disks: &[DiskInfo]) {
        for info in disks {
            if let Some(existing) = self
                .mounts
                .iter_mut()
                .find(|m| m.mount_point == info.mount_point)
            {
                existing.record_usage(info);
            } else {
                self.mounts.push(MountMetrics::new(info, self.history_len));
            }
        }

        // Remove mounts that no longer exist.
        let active_mounts: Vec<&str> = disks.iter().map(|d| d.mount_point.as_str()).collect();
        self.mounts
            .retain(|m| active_mounts.contains(&m.mount_point.as_str()));
    }

    /// Get a summary line for each mount: "mount: XX% (used/total)".
    #[must_use]
    pub fn summary_lines(&self) -> Vec<String> {
        self.mounts
            .iter()
            .map(|m| {
                format!(
                    "{}: {:.0}% ({}/{})",
                    m.mount_point,
                    m.usage_percent(),
                    m.used_display(),
                    m.total_display()
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_disks() -> Vec<DiskInfo> {
        vec![
            DiskInfo {
                name: "disk0s1".into(),
                mount_point: "/".into(),
                fs_type: "apfs".into(),
                total: 500 * 1024 * 1024 * 1024,
                used: 250 * 1024 * 1024 * 1024,
                available: 250 * 1024 * 1024 * 1024,
            },
            DiskInfo {
                name: "disk1s1".into(),
                mount_point: "/Volumes/Data".into(),
                fs_type: "apfs".into(),
                total: 1000 * 1024 * 1024 * 1024,
                used: 100 * 1024 * 1024 * 1024,
                available: 900 * 1024 * 1024 * 1024,
            },
        ]
    }

    #[test]
    fn initial_empty() {
        let disk = DiskMetrics::new(60);
        assert!(disk.mounts.is_empty());
    }

    #[test]
    fn update_creates_mounts() {
        let mut disk = DiskMetrics::new(60);
        disk.update(&sample_disks());
        assert_eq!(disk.mounts.len(), 2);
        assert!((disk.mounts[0].usage_percent() - 50.0).abs() < 0.1);
    }

    #[test]
    fn update_tracks_existing() {
        let mut disk = DiskMetrics::new(60);
        disk.update(&sample_disks());
        disk.update(&sample_disks());
        assert_eq!(disk.mounts[0].usage_history.len(), 2);
    }

    #[test]
    fn removed_mounts_cleaned_up() {
        let mut disk = DiskMetrics::new(60);
        disk.update(&sample_disks());
        assert_eq!(disk.mounts.len(), 2);
        // Only keep root
        disk.update(&sample_disks()[..1]);
        assert_eq!(disk.mounts.len(), 1);
        assert_eq!(disk.mounts[0].mount_point, "/");
    }

    #[test]
    fn summary_lines() {
        let mut disk = DiskMetrics::new(60);
        disk.update(&sample_disks());
        let lines = disk.summary_lines();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains('/'));
        assert!(lines[0].contains('%'));
    }
}
