//! Network throughput metrics collection per interface.

use crate::graph::RingBuffer;
use crate::platform::NetworkInfo;

/// Network metrics with throughput history per interface.
#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    /// Per-interface metrics.
    pub interfaces: Vec<InterfaceMetrics>,
    /// History buffer capacity.
    history_len: usize,
}

/// Throughput metrics for a single network interface.
#[derive(Debug, Clone)]
pub struct InterfaceMetrics {
    /// Interface name (e.g. "en0").
    pub name: String,
    /// Receive throughput (bytes/s) history.
    pub rx_history: RingBuffer,
    /// Transmit throughput (bytes/s) history.
    pub tx_history: RingBuffer,
    /// Previous cumulative rx bytes (for computing delta).
    prev_rx: u64,
    /// Previous cumulative tx bytes (for computing delta).
    prev_tx: u64,
    /// Total cumulative received bytes.
    pub total_rx: u64,
    /// Total cumulative transmitted bytes.
    pub total_tx: u64,
}

impl InterfaceMetrics {
    fn new(info: &NetworkInfo, history_len: usize) -> Self {
        Self {
            name: info.interface.clone(),
            rx_history: RingBuffer::new(history_len),
            tx_history: RingBuffer::new(history_len),
            prev_rx: info.rx_bytes,
            prev_tx: info.tx_bytes,
            total_rx: info.rx_bytes,
            total_tx: info.tx_bytes,
        }
    }

    fn update(&mut self, info: &NetworkInfo) {
        // Calculate delta (bytes since last sample).
        let rx_delta = info.rx_bytes.saturating_sub(self.prev_rx);
        let tx_delta = info.tx_bytes.saturating_sub(self.prev_tx);

        self.rx_history.push(rx_delta as f32);
        self.tx_history.push(tx_delta as f32);

        self.prev_rx = info.rx_bytes;
        self.prev_tx = info.tx_bytes;
        self.total_rx = info.rx_bytes;
        self.total_tx = info.tx_bytes;
    }

    /// Current receive throughput (bytes/s). Since we sample once per
    /// refresh interval, this is bytes-per-interval.
    #[must_use]
    pub fn current_rx(&self) -> f32 {
        self.rx_history.latest().unwrap_or(0.0)
    }

    /// Current transmit throughput (bytes/s).
    #[must_use]
    pub fn current_tx(&self) -> f32 {
        self.tx_history.latest().unwrap_or(0.0)
    }

    /// Human-readable current receive rate.
    #[must_use]
    pub fn rx_display(&self) -> String {
        format_rate(self.current_rx())
    }

    /// Human-readable current transmit rate.
    #[must_use]
    pub fn tx_display(&self) -> String {
        format_rate(self.current_tx())
    }
}

#[allow(dead_code)]
impl NetworkMetrics {
    /// Create a new network metrics tracker.
    #[must_use]
    pub fn new(history_len: usize) -> Self {
        Self {
            interfaces: Vec::new(),
            history_len,
        }
    }

    /// Update with new network information.
    pub fn update(&mut self, networks: &[NetworkInfo]) {
        for info in networks {
            if let Some(existing) = self
                .interfaces
                .iter_mut()
                .find(|i| i.name == info.interface)
            {
                existing.update(info);
            } else {
                self.interfaces
                    .push(InterfaceMetrics::new(info, self.history_len));
            }
        }
    }

    /// Get a summary line for each active interface.
    #[must_use]
    pub fn summary_lines(&self) -> Vec<String> {
        self.interfaces
            .iter()
            .filter(|i| i.total_rx > 0 || i.total_tx > 0)
            .map(|i| {
                format!(
                    "{}: rx {} tx {}",
                    i.name,
                    i.rx_display(),
                    i.tx_display()
                )
            })
            .collect()
    }
}

/// Format a byte rate to human-readable string (e.g. "1.2 MB/s").
#[must_use]
fn format_rate(bytes_per_interval: f32) -> String {
    let b = bytes_per_interval;
    if b >= 1_073_741_824.0 {
        format!("{:.1} GB/s", b / 1_073_741_824.0)
    } else if b >= 1_048_576.0 {
        format!("{:.1} MB/s", b / 1_048_576.0)
    } else if b >= 1024.0 {
        format!("{:.1} KB/s", b / 1024.0)
    } else {
        format!("{:.0} B/s", b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_net_initial() -> Vec<NetworkInfo> {
        vec![
            NetworkInfo {
                interface: "en0".into(),
                rx_bytes: 1_000_000,
                tx_bytes: 500_000,
            },
            NetworkInfo {
                interface: "lo0".into(),
                rx_bytes: 100,
                tx_bytes: 100,
            },
        ]
    }

    fn sample_net_updated() -> Vec<NetworkInfo> {
        vec![
            NetworkInfo {
                interface: "en0".into(),
                rx_bytes: 2_000_000,
                tx_bytes: 600_000,
            },
            NetworkInfo {
                interface: "lo0".into(),
                rx_bytes: 200,
                tx_bytes: 200,
            },
        ]
    }

    #[test]
    fn initial_state() {
        let net = NetworkMetrics::new(60);
        assert!(net.interfaces.is_empty());
    }

    #[test]
    fn first_update_creates_interfaces() {
        let mut net = NetworkMetrics::new(60);
        net.update(&sample_net_initial());
        assert_eq!(net.interfaces.len(), 2);
        // First sample has no delta — just records baseline
        assert!((net.interfaces[0].current_rx() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn second_update_computes_delta() {
        let mut net = NetworkMetrics::new(60);
        net.update(&sample_net_initial());
        net.update(&sample_net_updated());
        // en0: rx delta = 2M - 1M = 1M
        assert!((net.interfaces[0].current_rx() - 1_000_000.0).abs() < 1.0);
        // en0: tx delta = 600K - 500K = 100K
        assert!((net.interfaces[0].current_tx() - 100_000.0).abs() < 1.0);
    }

    #[test]
    fn format_rate_units() {
        assert_eq!(format_rate(0.0), "0 B/s");
        assert_eq!(format_rate(500.0), "500 B/s");
        assert_eq!(format_rate(1024.0), "1.0 KB/s");
        assert_eq!(format_rate(1_048_576.0), "1.0 MB/s");
    }
}
