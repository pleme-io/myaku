//! Platform abstraction traits.
//!
//! System metrics collection: CPU, memory, disk, network, and process info.
//! Platform-specific implementations live in submodules.

#[cfg(target_os = "macos")]
pub mod macos;

/// CPU usage information.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CpuInfo {
    /// Per-core usage percentages (0.0 - 100.0).
    pub per_core: Vec<f32>,
    /// Total (average) CPU usage percentage.
    pub total: f32,
    /// CPU brand name.
    pub brand: String,
    /// Number of physical cores.
    pub core_count: usize,
}

/// Memory usage information.
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Total physical memory in bytes.
    pub total: u64,
    /// Used memory in bytes.
    pub used: u64,
    /// Available memory in bytes.
    pub available: u64,
    /// Total swap in bytes.
    pub swap_total: u64,
    /// Used swap in bytes.
    pub swap_used: u64,
}

/// Disk usage information.
#[derive(Debug, Clone)]
pub struct DiskInfo {
    /// Disk name / device identifier.
    pub name: String,
    /// Mount point.
    pub mount_point: String,
    /// File system type (e.g. "apfs", "ext4").
    pub fs_type: String,
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
    /// Total bytes received (cumulative).
    pub rx_bytes: u64,
    /// Total bytes transmitted (cumulative).
    pub tx_bytes: u64,
}

/// Information about a running process.
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    /// Parent process ID (0 if unknown).
    pub parent_pid: u32,
    /// User who owns the process.
    pub user: String,
}

/// System metrics collection.
pub trait SystemMetrics: Send + Sync {
    /// Refresh all metrics. Must be called periodically to get updated values.
    fn refresh(&mut self);

    /// Get CPU usage info (per-core and total).
    fn cpu_info(&self) -> CpuInfo;

    /// Get memory usage information.
    fn memory_info(&self) -> MemoryInfo;

    /// Get disk usage for all mounted volumes.
    fn disk_info(&self) -> Vec<DiskInfo>;

    /// Get network interface statistics.
    fn network_info(&self) -> Vec<NetworkInfo>;

    /// Get list of running processes.
    fn process_list(&self) -> Vec<ProcessInfo>;

    /// Get system uptime in seconds.
    fn uptime_secs(&self) -> u64;

    /// Get load averages (1, 5, 15 minute).
    fn load_average(&self) -> [f64; 3];
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
