//! macOS system metrics implementation using sysinfo.

use sysinfo::{Disks, Networks, System};

use crate::platform::{CpuInfo, DiskInfo, MemoryInfo, NetworkInfo, ProcessInfo, SystemMetrics};

/// macOS-specific system metrics collector.
pub struct MacOSMetrics {
    system: System,
    disks: Disks,
    networks: Networks,
}

impl MacOSMetrics {
    pub fn new() -> Self {
        let mut system = System::new_all();
        // Two refreshes needed for CPU usage calculation — first refresh
        // establishes baseline, second computes actual deltas.
        system.refresh_all();
        std::thread::sleep(std::time::Duration::from_millis(200));
        system.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        Self {
            system,
            disks,
            networks,
        }
    }
}

impl SystemMetrics for MacOSMetrics {
    fn refresh(&mut self) {
        self.system.refresh_all();
        self.disks.refresh(true);
        self.networks.refresh(true);
    }

    fn cpu_info(&self) -> CpuInfo {
        let per_core: Vec<f32> = self
            .system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect();

        let total = if per_core.is_empty() {
            0.0
        } else {
            per_core.iter().sum::<f32>() / per_core.len() as f32
        };

        let brand = self
            .system
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default();

        let core_count = self.system.physical_core_count().unwrap_or(per_core.len());

        CpuInfo {
            per_core,
            total,
            brand,
            core_count,
        }
    }

    fn memory_info(&self) -> MemoryInfo {
        MemoryInfo {
            total: self.system.total_memory(),
            used: self.system.used_memory(),
            available: self.system.available_memory(),
            swap_total: self.system.total_swap(),
            swap_used: self.system.used_swap(),
        }
    }

    fn disk_info(&self) -> Vec<DiskInfo> {
        self.disks
            .iter()
            .map(|d| DiskInfo {
                name: d.name().to_string_lossy().into_owned(),
                mount_point: d.mount_point().to_string_lossy().into_owned(),
                fs_type: d.file_system().to_string_lossy().into_owned(),
                total: d.total_space(),
                used: d.total_space().saturating_sub(d.available_space()),
                available: d.available_space(),
            })
            .collect()
    }

    fn network_info(&self) -> Vec<NetworkInfo> {
        self.networks
            .iter()
            .map(|(name, data)| NetworkInfo {
                interface: name.clone(),
                rx_bytes: data.total_received(),
                tx_bytes: data.total_transmitted(),
            })
            .collect()
    }

    fn process_list(&self) -> Vec<ProcessInfo> {
        self.system
            .processes()
            .iter()
            .map(|(pid, proc_)| {
                let parent_pid = proc_
                    .parent()
                    .map(|p| p.as_u32())
                    .unwrap_or(0);

                let user = proc_
                    .user_id()
                    .map(|uid| uid.to_string())
                    .unwrap_or_else(|| String::from("-"));

                ProcessInfo {
                    pid: pid.as_u32(),
                    name: proc_.name().to_string_lossy().into_owned(),
                    cpu: proc_.cpu_usage(),
                    memory: proc_.memory(),
                    status: format!("{:?}", proc_.status()),
                    parent_pid,
                    user,
                }
            })
            .collect()
    }

    fn uptime_secs(&self) -> u64 {
        System::uptime()
    }

    fn load_average(&self) -> [f64; 3] {
        let load = System::load_average();
        [load.one, load.five, load.fifteen]
    }
}
