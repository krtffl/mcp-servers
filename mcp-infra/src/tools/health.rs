//! `get_server_health` tool — host metrics via sysinfo.

use serde::Serialize;
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Serialize)]
pub struct ServerHealth {
    pub cpu_usage_percent: f64,
    pub memory_used_mb: f64,
    pub memory_total_mb: f64,
    pub memory_usage_percent: f64,
    pub disks: Vec<DiskInfo>,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub uptime_seconds: u64,
    pub hostname: String,
}

#[derive(Debug, Serialize)]
pub struct DiskInfo {
    pub mount: String,
    pub used_gb: f64,
    pub total_gb: f64,
    pub usage_percent: f64,
}

/// Collect host health metrics (CPU, memory, disks, network, uptime).
///
/// # Errors
///
/// Returns an error if the collected metrics fail to serialize to JSON.
// Byte counters are converted to f64 for human-readable MB/GB reporting. Values
// only lose precision above 2^53 bytes (~9 PB), far beyond any real host.
#[allow(clippy::cast_precision_loss)]
pub fn execute() -> Result<String, String> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_usage = f64::from(sys.global_cpu_usage());
    let mem_used = sys.used_memory() as f64 / 1_048_576.0;
    let mem_total = sys.total_memory() as f64 / 1_048_576.0;
    let mem_pct = if mem_total > 0.0 {
        (mem_used / mem_total) * 100.0
    } else {
        0.0
    };

    let disks = Disks::new_with_refreshed_list();
    let disk_info: Vec<DiskInfo> = disks
        .iter()
        .map(|d| {
            let total = d.total_space() as f64 / 1_073_741_824.0;
            let available = d.available_space() as f64 / 1_073_741_824.0;
            let used = total - available;
            DiskInfo {
                mount: d.mount_point().to_string_lossy().to_string(),
                used_gb: (used * 100.0).round() / 100.0,
                total_gb: (total * 100.0).round() / 100.0,
                usage_percent: if total > 0.0 {
                    ((used / total) * 100.0 * 10.0).round() / 10.0
                } else {
                    0.0
                },
            }
        })
        .collect();

    let networks = Networks::new_with_refreshed_list();
    let (rx, tx) = networks
        .iter()
        .fold((0u64, 0u64), |(rx, tx), (_name, data)| {
            (rx + data.total_received(), tx + data.total_transmitted())
        });

    let health = ServerHealth {
        cpu_usage_percent: (cpu_usage * 10.0).round() / 10.0,
        memory_used_mb: (mem_used * 10.0).round() / 10.0,
        memory_total_mb: (mem_total * 10.0).round() / 10.0,
        memory_usage_percent: (mem_pct * 10.0).round() / 10.0,
        disks: disk_info,
        network_rx_bytes: rx,
        network_tx_bytes: tx,
        uptime_seconds: System::uptime(),
        hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
    };

    serde_json::to_string_pretty(&health).map_err(|e| format!("JSON error: {e}"))
}
