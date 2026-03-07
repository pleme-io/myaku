//! macOS system metrics implementation using sysinfo.

use sysinfo::{Disks, Networks, System};

use crate::platform::{DiskInfo, MemoryInfo, NetworkInfo, ProcessInfo, SystemMetrics};

/// macOS-specific system metrics collector.
pub struct MacOSMetrics {
    system: System,
}

impl MacOSMetrics {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }
}

impl SystemMetrics for MacOSMetrics {
    fn cpu_usage(&self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        Ok(self.system.cpus().iter().map(|cpu| cpu.cpu_usage()).collect())
    }

    fn memory_usage(&self) -> Result<MemoryInfo, Box<dyn std::error::Error>> {
        Ok(MemoryInfo {
            total: self.system.total_memory(),
            used: self.system.used_memory(),
            available: self.system.available_memory(),
        })
    }

    fn disk_usage(&self) -> Result<Vec<DiskInfo>, Box<dyn std::error::Error>> {
        let disks = Disks::new_with_refreshed_list();
        Ok(disks
            .iter()
            .map(|d| DiskInfo {
                mount_point: d.mount_point().to_string_lossy().into_owned(),
                total: d.total_space(),
                used: d.total_space() - d.available_space(),
                available: d.available_space(),
            })
            .collect())
    }

    fn network_usage(&self) -> Result<Vec<NetworkInfo>, Box<dyn std::error::Error>> {
        let networks = Networks::new_with_refreshed_list();
        Ok(networks
            .iter()
            .map(|(name, data)| NetworkInfo {
                interface: name.clone(),
                rx_bytes: data.total_received(),
                tx_bytes: data.total_transmitted(),
            })
            .collect())
    }

    fn process_list(&self) -> Result<Vec<ProcessInfo>, Box<dyn std::error::Error>> {
        Ok(self
            .system
            .processes()
            .iter()
            .map(|(pid, proc_)| ProcessInfo {
                pid: pid.as_u32(),
                name: proc_.name().to_string_lossy().into_owned(),
                cpu: proc_.cpu_usage(),
                memory: proc_.memory(),
                status: format!("{:?}", proc_.status()),
            })
            .collect())
    }
}
