//! Platform abstraction traits.
//!
//! System metrics collection: CPU, memory, disk, network, and process info.
//! Platform-specific implementations live in submodules.

#[cfg(target_os = "macos")]
pub mod macos;

/// Memory usage information.
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Total physical memory in bytes.
    pub total: u64,
    /// Used memory in bytes.
    pub used: u64,
    /// Available memory in bytes.
    pub available: u64,
}

/// Disk usage information.
#[derive(Debug, Clone)]
pub struct DiskInfo {
    /// Mount point.
    pub mount_point: String,
    /// Total space in bytes.
    pub total: u64,
    /// Used space in bytes.
    pub used: u64,
    /// Available space in bytes.
    pub available: u64,
}

/// Network interface information.
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    /// Interface name.
    pub interface: String,
    /// Bytes received.
    pub rx_bytes: u64,
    /// Bytes transmitted.
    pub tx_bytes: u64,
}

/// Information about a running process.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Process ID.
    pub pid: u32,
    /// Process name.
    pub name: String,
    /// CPU usage percentage.
    pub cpu: f32,
    /// Memory usage in bytes.
    pub memory: u64,
    /// Process status (running, sleeping, etc.).
    pub status: String,
}

/// System metrics collection.
pub trait SystemMetrics: Send + Sync {
    /// Get per-core CPU usage percentages.
    fn cpu_usage(&self) -> Result<Vec<f32>, Box<dyn std::error::Error>>;

    /// Get memory usage information.
    fn memory_usage(&self) -> Result<MemoryInfo, Box<dyn std::error::Error>>;

    /// Get disk usage for all mounted volumes.
    fn disk_usage(&self) -> Result<Vec<DiskInfo>, Box<dyn std::error::Error>>;

    /// Get network interface statistics.
    fn network_usage(&self) -> Result<Vec<NetworkInfo>, Box<dyn std::error::Error>>;

    /// Get list of running processes.
    fn process_list(&self) -> Result<Vec<ProcessInfo>, Box<dyn std::error::Error>>;
}

/// Create a platform-specific system metrics implementation.
pub fn create_metrics() -> Box<dyn SystemMetrics> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOSMetrics::new())
    }
    #[cfg(not(target_os = "macos"))]
    {
        panic!("system metrics not implemented for this platform")
    }
}
