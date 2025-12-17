// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - NVIDIA GPU Integration
//
// Copyright (C) 2025 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::{Context, Result};
use log::{debug, info};
use std::fs;
use std::path::Path;

/// GPU power state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpuPowerState {
    D0,     // Fully on
    D1,     // Light sleep
    D2,     // Deep sleep
    D3Hot,  // Powered but suspended
    D3Cold, // Powered off
    Unknown,
}

impl std::fmt::Display for GpuPowerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuPowerState::D0 => write!(f, "D0 (active)"),
            GpuPowerState::D1 => write!(f, "D1"),
            GpuPowerState::D2 => write!(f, "D2"),
            GpuPowerState::D3Hot => write!(f, "D3hot"),
            GpuPowerState::D3Cold => write!(f, "D3cold"),
            GpuPowerState::Unknown => write!(f, "unknown"),
        }
    }
}

/// NVIDIA GPU information
#[derive(Debug)]
#[allow(dead_code)]
pub struct NvidiaGpuInfo {
    /// PCI device address (e.g., "0000:01:00.0")
    pub pci_address: String,
    /// GPU model name
    pub model: String,
    /// GPU UUID
    pub uuid: Option<String>,
    /// Whether Resizable BAR is enabled
    pub rebar_enabled: bool,
    /// BAR1 size in bytes (VRAM mapping)
    pub bar1_size: u64,
    /// Current power state
    pub power_state: GpuPowerState,
    /// NUMA node (-1 if not assigned)
    pub numa_node: i32,
    /// PCIe link speed (e.g., "32.0 GT/s")
    pub pcie_speed: String,
    /// PCIe link width (e.g., "x16")
    pub pcie_width: String,
}

/// Detect NVIDIA GPUs on the system
pub fn detect_nvidia_gpus() -> Result<Vec<NvidiaGpuInfo>> {
    let mut gpus = Vec::new();

    // Check if NVIDIA driver is loaded
    let nvidia_params = Path::new("/proc/driver/nvidia/params");
    if !nvidia_params.exists() {
        debug!("NVIDIA driver not loaded");
        return Ok(gpus);
    }

    // Check ReBAR status from driver params
    let rebar_enabled = check_rebar_enabled();
    if rebar_enabled {
        info!("NVIDIA Resizable BAR: enabled");
    }

    // Find NVIDIA GPUs in /proc/driver/nvidia/gpus/
    let gpus_dir = Path::new("/proc/driver/nvidia/gpus");
    if !gpus_dir.exists() {
        return Ok(gpus);
    }

    for entry in fs::read_dir(gpus_dir)? {
        let entry = entry?;
        let pci_address = entry.file_name().to_string_lossy().to_string();

        if let Ok(gpu_info) = read_gpu_info(&pci_address, rebar_enabled) {
            info!(
                "Detected NVIDIA GPU: {} at {}",
                gpu_info.model, gpu_info.pci_address
            );
            gpus.push(gpu_info);
        }
    }

    Ok(gpus)
}

/// Check if Resizable BAR is enabled in NVIDIA driver
fn check_rebar_enabled() -> bool {
    let params_path = "/proc/driver/nvidia/params";

    if let Ok(content) = fs::read_to_string(params_path) {
        for line in content.lines() {
            if line.starts_with("EnableResizableBar:") {
                return line.contains("1");
            }
        }
    }

    false
}

/// Read detailed info for a specific GPU
fn read_gpu_info(pci_address: &str, rebar_enabled: bool) -> Result<NvidiaGpuInfo> {
    let base_path = format!("/proc/driver/nvidia/gpus/{}", pci_address);

    // Read GPU information
    let info_path = format!("{}/information", base_path);
    let info_content = fs::read_to_string(&info_path)
        .with_context(|| format!("Failed to read GPU info at {}", info_path))?;

    let model = parse_gpu_field(&info_content, "Model:")
        .unwrap_or_else(|| "Unknown NVIDIA GPU".to_string());
    let uuid = parse_gpu_field(&info_content, "GPU UUID:");

    // Read power state
    let power_state = read_gpu_power_state(pci_address);

    // Read PCI info
    let (numa_node, pcie_speed, pcie_width, bar1_size) = read_pci_info(pci_address);

    Ok(NvidiaGpuInfo {
        pci_address: pci_address.to_string(),
        model,
        uuid,
        rebar_enabled,
        bar1_size,
        power_state,
        numa_node,
        pcie_speed,
        pcie_width,
    })
}

/// Parse a field from GPU information output
fn parse_gpu_field(content: &str, field: &str) -> Option<String> {
    for line in content.lines() {
        if line.contains(field) {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                return Some(parts[1].trim().to_string());
            }
        }
    }
    None
}

/// Read GPU power state from /proc/driver/nvidia
fn read_gpu_power_state(pci_address: &str) -> GpuPowerState {
    // Try NVIDIA driver power info
    let power_path = format!("/proc/driver/nvidia/gpus/{}/power", pci_address);
    if let Ok(content) = fs::read_to_string(&power_path)
        && content.contains("Runtime D3 status:")
        && (content.contains("Disabled") || content.contains("Not supported"))
    {
        return GpuPowerState::D0; // Assume active if D3 disabled
    }

    // Check PCI power state
    let pci_power_path = format!("/sys/bus/pci/devices/{}/power_state", pci_address);
    if let Ok(state) = fs::read_to_string(&pci_power_path) {
        match state.trim() {
            "D0" => return GpuPowerState::D0,
            "D1" => return GpuPowerState::D1,
            "D2" => return GpuPowerState::D2,
            "D3hot" => return GpuPowerState::D3Hot,
            "D3cold" => return GpuPowerState::D3Cold,
            _ => {}
        }
    }

    GpuPowerState::Unknown
}

/// Read PCI device info from sysfs
fn read_pci_info(pci_address: &str) -> (i32, String, String, u64) {
    let base = format!("/sys/bus/pci/devices/{}", pci_address);

    // NUMA node
    let numa_node = fs::read_to_string(format!("{}/numa_node", base))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(-1);

    // PCIe speed
    let pcie_speed = fs::read_to_string(format!("{}/current_link_speed", base))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // PCIe width
    let pcie_width = fs::read_to_string(format!("{}/current_link_width", base))
        .map(|s| format!("x{}", s.trim()))
        .unwrap_or_else(|_| "unknown".to_string());

    // BAR1 size from resource file
    let bar1_size = read_bar1_size(&base);

    (numa_node, pcie_speed, pcie_width, bar1_size)
}

/// Read BAR1 (VRAM) size from PCI resource file
fn read_bar1_size(pci_base: &str) -> u64 {
    let resource_path = format!("{}/resource", pci_base);

    if let Ok(content) = fs::read_to_string(&resource_path) {
        // resource file format: start end flags (one line per BAR)
        // BAR1 is typically the second line (index 1)
        for (idx, line) in content.lines().enumerate() {
            if idx == 1 {
                // BAR1
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2
                    && let (Ok(start), Ok(end)) = (
                        u64::from_str_radix(parts[0].trim_start_matches("0x"), 16),
                        u64::from_str_radix(parts[1].trim_start_matches("0x"), 16),
                    )
                    && end > start
                {
                    return end - start + 1;
                }
            }
        }
    }

    0
}

/// GPU state tracker for monitoring changes
pub struct GpuMonitor {
    gpus: Vec<NvidiaGpuInfo>,
    last_power_states: Vec<GpuPowerState>,
}

impl GpuMonitor {
    pub fn new() -> Result<Self> {
        let gpus = detect_nvidia_gpus()?;
        let last_power_states = gpus.iter().map(|g| g.power_state).collect();

        Ok(Self {
            gpus,
            last_power_states,
        })
    }

    /// Check if any GPU is active (D0 state)
    pub fn any_gpu_active(&self) -> bool {
        self.gpus.iter().any(|g| g.power_state == GpuPowerState::D0)
    }

    /// Check if ReBAR is enabled on any GPU
    pub fn rebar_enabled(&self) -> bool {
        self.gpus.iter().any(|g| g.rebar_enabled)
    }

    /// Get total VRAM mapping size (BAR1) across all GPUs
    pub fn total_bar1_size(&self) -> u64 {
        self.gpus.iter().map(|g| g.bar1_size).sum()
    }

    /// Update GPU power states and return true if any changed
    pub fn update_power_states(&mut self) -> bool {
        let mut changed = false;

        for (idx, gpu) in self.gpus.iter_mut().enumerate() {
            let new_state = read_gpu_power_state(&gpu.pci_address);

            if idx < self.last_power_states.len() && new_state != self.last_power_states[idx] {
                debug!(
                    "GPU {} power state changed: {} -> {}",
                    gpu.pci_address, self.last_power_states[idx], new_state
                );
                self.last_power_states[idx] = new_state;
                changed = true;
            }

            gpu.power_state = new_state;
        }

        changed
    }

    /// Get summary for logging
    pub fn summary(&self) -> String {
        if self.gpus.is_empty() {
            return "No NVIDIA GPUs detected".to_string();
        }

        let rebar = if self.rebar_enabled() {
            "ReBAR"
        } else {
            "no ReBAR"
        };
        let bar1_gb = self.total_bar1_size() as f64 / (1024.0 * 1024.0 * 1024.0);

        format!(
            "{} GPU(s), {}, {:.0}GB BAR1",
            self.gpus.len(),
            rebar,
            bar1_gb
        )
    }

    /// Get GPU count
    pub fn gpu_count(&self) -> usize {
        self.gpus.len()
    }

    /// Get first GPU info (primary)
    pub fn primary_gpu(&self) -> Option<&NvidiaGpuInfo> {
        self.gpus.first()
    }
}

impl Default for GpuMonitor {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            gpus: Vec::new(),
            last_power_states: Vec::new(),
        })
    }
}

/// GPU-feeding thread patterns (for BPF detection hints)
#[allow(dead_code)]
pub const GPU_THREAD_PATTERNS: &[&str] = &[
    "vk", // Vulkan threads
    "VkThread",
    "vulkan",
    "gl", // OpenGL threads
    "GLThread",
    "opengl",
    "nvidia", // NVIDIA driver threads
    "nv_queue",
    "threaded_gl",
    "dxvk",  // DXVK (Vulkan translation layer)
    "vkd3d", // VKD3D (D3D12 translation)
];

/// Check if a process name looks like a GPU-feeding thread
#[allow(dead_code)]
pub fn is_gpu_thread_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    GPU_THREAD_PATTERNS
        .iter()
        .any(|p| lower.contains(&p.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_nvidia_gpus() {
        // Just verify it doesn't panic
        let _result = detect_nvidia_gpus();
    }

    #[test]
    fn test_is_gpu_thread_name() {
        assert!(is_gpu_thread_name("VkThread-0"));
        assert!(is_gpu_thread_name("dxvk-submit"));
        assert!(is_gpu_thread_name("nvidia-modeset"));
        assert!(!is_gpu_thread_name("bash"));
    }
}
