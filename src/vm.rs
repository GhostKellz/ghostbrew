// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - KVM/QEMU Virtualization Support
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::Result;
use log::{debug, info};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// VM workload classification
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum VmWorkloadType {
    Dev,     // Development VM - batch priority
    Gaming,  // Gaming VM with GPU passthrough - gaming priority
    Ai,      // AI/ML VM with GPU - batch priority, GPU affinity
    Unknown, // Unclassified VM
}

impl std::fmt::Display for VmWorkloadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmWorkloadType::Dev => write!(f, "dev"),
            VmWorkloadType::Gaming => write!(f, "gaming"),
            VmWorkloadType::Ai => write!(f, "AI"),
            VmWorkloadType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Information about a detected VM
#[derive(Debug)]
#[allow(dead_code)]
pub struct VmInfo {
    /// QEMU process PID
    pub qemu_pid: u32,
    /// VM name (from QEMU command line or libvirt)
    pub name: String,
    /// Workload type
    pub workload_type: VmWorkloadType,
    /// vCPU thread PIDs
    pub vcpu_pids: Vec<u32>,
    /// Whether VM has GPU passthrough
    pub has_gpu_passthrough: bool,
    /// PCI addresses of passed-through GPUs
    pub passthrough_gpus: Vec<String>,
    /// Whether vCPUs are pinned (don't override)
    pub vcpus_pinned: bool,
}

/// IOMMU group information
#[derive(Debug)]
pub struct IommuGroup {
    pub id: u32,
    pub devices: Vec<String>, // PCI addresses
    pub has_gpu: bool,
    pub is_isolated: bool, // Good grouping (no other devices)
}

/// Detect all IOMMU groups on the system
pub fn detect_iommu_groups() -> Result<Vec<IommuGroup>> {
    let mut groups = Vec::new();
    let iommu_path = Path::new("/sys/kernel/iommu_groups");

    if !iommu_path.exists() {
        debug!("IOMMU not available");
        return Ok(groups);
    }

    for entry in fs::read_dir(iommu_path)? {
        let entry = entry?;
        let group_name = entry.file_name().to_string_lossy().to_string();

        if let Ok(group_id) = group_name.parse::<u32>() {
            let devices_path = entry.path().join("devices");
            let mut devices = Vec::new();
            let mut has_gpu = false;

            if devices_path.exists() {
                for dev_entry in fs::read_dir(&devices_path)? {
                    let dev_entry = dev_entry?;
                    let pci_addr = dev_entry.file_name().to_string_lossy().to_string();

                    // Check if this is a GPU (class 0x03xxxx)
                    let class_path = dev_entry.path().join("class");
                    if let Ok(class) = fs::read_to_string(&class_path)
                        && class.trim().starts_with("0x03")
                    {
                        has_gpu = true;
                    }

                    devices.push(pci_addr);
                }
            }

            // Good isolation = only 1 device or only GPU + audio (common pairing)
            let is_isolated = devices.len() <= 2;

            groups.push(IommuGroup {
                id: group_id,
                devices,
                has_gpu,
                is_isolated,
            });
        }
    }

    // Sort by group ID
    groups.sort_by_key(|g| g.id);

    let gpu_groups: Vec<_> = groups.iter().filter(|g| g.has_gpu).collect();
    if !gpu_groups.is_empty() {
        info!(
            "IOMMU: {} groups, {} with GPU",
            groups.len(),
            gpu_groups.len()
        );
        for g in &gpu_groups {
            debug!(
                "  Group {}: {:?} ({})",
                g.id,
                g.devices,
                if g.is_isolated { "isolated" } else { "shared" }
            );
        }
    }

    Ok(groups)
}

/// Check if a PCI device is bound to vfio-pci (passed through to VM)
pub fn is_vfio_bound(pci_addr: &str) -> bool {
    let driver_path = format!("/sys/bus/pci/devices/{}/driver", pci_addr);

    if let Ok(driver_link) = fs::read_link(&driver_path)
        && let Some(driver_name) = driver_link.file_name()
    {
        return driver_name.to_string_lossy().contains("vfio");
    }

    false
}

/// Get all GPUs bound to vfio-pci (passed through to VMs)
pub fn get_passthrough_gpus(iommu_groups: &[IommuGroup]) -> Vec<String> {
    let mut passthrough = Vec::new();

    for group in iommu_groups {
        if group.has_gpu {
            for device in &group.devices {
                if is_vfio_bound(device) {
                    // Check if it's actually a GPU
                    let class_path = format!("/sys/bus/pci/devices/{}/class", device);
                    if let Ok(class) = fs::read_to_string(&class_path)
                        && class.trim().starts_with("0x03")
                    {
                        passthrough.push(device.clone());
                    }
                }
            }
        }
    }

    passthrough
}

/// Scan /proc for QEMU/KVM processes
pub fn scan_vms() -> Result<Vec<VmInfo>> {
    let mut vms = Vec::new();
    let proc_dir = fs::read_dir("/proc")?;

    for entry in proc_dir.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Skip non-numeric entries
        let pid: u32 = match name.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Check if this is a QEMU process
        if let Some(vm_info) = check_qemu_process(pid) {
            vms.push(vm_info);
        }
    }

    Ok(vms)
}

/// Check if a PID is a QEMU process and extract VM info
fn check_qemu_process(pid: u32) -> Option<VmInfo> {
    let comm_path = format!("/proc/{}/comm", pid);
    let comm = fs::read_to_string(&comm_path).ok()?;
    let comm = comm.trim();

    // Check for QEMU process names
    if !comm.contains("qemu") && !comm.contains("kvm") {
        return None;
    }

    // Read command line for VM details
    let cmdline_path = format!("/proc/{}/cmdline", pid);
    let cmdline = fs::read_to_string(&cmdline_path).ok()?;
    let args: Vec<&str> = cmdline.split('\0').collect();

    // Extract VM name
    let name = extract_vm_name(&args);

    // Find vCPU threads
    let vcpu_pids = find_vcpu_threads(pid);

    // Check for vCPU pinning
    let vcpus_pinned = check_vcpu_pinning(pid, &vcpu_pids);

    // Detect GPU passthrough
    let passthrough_gpus = detect_vm_passthrough_gpus(&args);
    let has_gpu_passthrough = !passthrough_gpus.is_empty();

    // Classify workload type
    let workload_type = classify_vm_workload(&args, &name, has_gpu_passthrough);

    debug!(
        "Detected VM: {} (PID {}) - {} vCPUs, type: {}, GPU: {}",
        name,
        pid,
        vcpu_pids.len(),
        workload_type,
        has_gpu_passthrough
    );

    Some(VmInfo {
        qemu_pid: pid,
        name,
        workload_type,
        vcpu_pids,
        has_gpu_passthrough,
        passthrough_gpus,
        vcpus_pinned,
    })
}

/// Extract VM name from QEMU command line
fn extract_vm_name(args: &[&str]) -> String {
    // Try -name argument
    for (i, arg) in args.iter().enumerate() {
        if *arg == "-name"
            && let Some(name) = args.get(i + 1)
        {
            // Handle "guest=name,..." format
            let name = name.split(',').next().unwrap_or(name);
            let name = name.strip_prefix("guest=").unwrap_or(name);
            return name.to_string();
        }
    }

    // Try to extract from -uuid or use generic name
    "unknown-vm".to_string()
}

/// Find vCPU thread PIDs for a QEMU process
fn find_vcpu_threads(qemu_pid: u32) -> Vec<u32> {
    let mut vcpus = Vec::new();
    let task_path = format!("/proc/{}/task", qemu_pid);

    if let Ok(tasks) = fs::read_dir(&task_path) {
        for task in tasks.flatten() {
            let tid: u32 = match task.file_name().to_string_lossy().parse() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Read thread comm
            let comm_path = format!("/proc/{}/task/{}/comm", qemu_pid, tid);
            if let Ok(comm) = fs::read_to_string(&comm_path) {
                let comm = comm.trim();
                // vCPU threads are named "CPU N/KVM" or similar
                if comm.contains("CPU") && comm.contains("KVM") {
                    vcpus.push(tid);
                }
            }
        }
    }

    vcpus
}

/// Check if vCPUs are pinned (via cgroups or taskset)
fn check_vcpu_pinning(qemu_pid: u32, vcpu_pids: &[u32]) -> bool {
    // Check if any vCPU has restricted CPU affinity
    for &vcpu_pid in vcpu_pids {
        let status_path = format!("/proc/{}/status", vcpu_pid);
        if let Ok(status) = fs::read_to_string(&status_path) {
            for line in status.lines() {
                if line.starts_with("Cpus_allowed:") {
                    let hex = line.split(':').nth(1).map(|s| s.trim());
                    if let Some(hex) = hex {
                        // If not all Fs, it's pinned
                        if !hex.chars().all(|c| c == 'f' || c == 'F' || c == ',') {
                            return true;
                        }
                    }
                }
            }
        }
    }

    // Also check libvirt cgroup for pinning
    let cgroup_path = format!("/proc/{}/cgroup", qemu_pid);
    if let Ok(cgroup) = fs::read_to_string(&cgroup_path)
        && cgroup.contains("vcpu")
        && cgroup.contains("emulator")
    {
        // Libvirt-style cgroups often indicate pinning
        return true;
    }

    false
}

/// Detect GPU passthrough from QEMU command line
fn detect_vm_passthrough_gpus(args: &[&str]) -> Vec<String> {
    let mut gpus = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        // Look for vfio-pci device arguments
        if *arg == "-device"
            && let Some(device_arg) = args.get(i + 1)
            && device_arg.contains("vfio-pci")
        {
            // Extract host= PCI address
            for part in device_arg.split(',') {
                if part.starts_with("host=") {
                    let addr = part.strip_prefix("host=").unwrap_or("");
                    // Normalize to full PCI address format
                    let addr = if addr.contains(':') && !addr.starts_with("0000:") {
                        format!("0000:{}", addr)
                    } else {
                        addr.to_string()
                    };
                    gpus.push(addr);
                }
            }
        }
    }

    gpus
}

/// Classify VM workload type based on command line and GPU
fn classify_vm_workload(args: &[&str], name: &str, has_gpu: bool) -> VmWorkloadType {
    let name_lower = name.to_lowercase();
    let args_str = args.join(" ").to_lowercase();

    // Gaming VM indicators
    if (name_lower.contains("gaming")
        || name_lower.contains("windows")
        || name_lower.contains("game"))
        && has_gpu
    {
        return VmWorkloadType::Gaming;
    }

    // AI/ML VM indicators
    if name_lower.contains("ollama")
        || name_lower.contains("ai")
        || name_lower.contains("ml")
        || name_lower.contains("cuda")
        || name_lower.contains("pytorch")
        || name_lower.contains("tensorflow")
    {
        return VmWorkloadType::Ai;
    }

    // Looking Glass indicator (gaming VM with display streaming)
    if args_str.contains("ivshmem") || args_str.contains("looking-glass") {
        return VmWorkloadType::Gaming;
    }

    // Dev VM indicators
    if name_lower.contains("dev")
        || name_lower.contains("build")
        || name_lower.contains("test")
        || name_lower.contains("linux")
        || name_lower.contains("ubuntu")
        || name_lower.contains("fedora")
        || name_lower.contains("arch")
        || name_lower.contains("debian")
    {
        return VmWorkloadType::Dev;
    }

    // Default: if has GPU passthrough, likely gaming
    if has_gpu {
        VmWorkloadType::Gaming
    } else {
        VmWorkloadType::Dev
    }
}

/// VM Monitor for tracking VMs and their state
pub struct VmMonitor {
    vms: Vec<VmInfo>,
    iommu_groups: Vec<IommuGroup>,
    passthrough_gpus: Vec<String>,
}

impl VmMonitor {
    pub fn new() -> Result<Self> {
        let iommu_groups = detect_iommu_groups()?;
        let passthrough_gpus = get_passthrough_gpus(&iommu_groups);
        let vms = scan_vms()?;

        if !vms.is_empty() {
            info!("VMs: {} detected", vms.len());
            for vm in &vms {
                info!(
                    "  {} ({}): {} vCPUs, GPU: {}",
                    vm.name,
                    vm.workload_type,
                    vm.vcpu_pids.len(),
                    vm.has_gpu_passthrough
                );
            }
        }

        if !passthrough_gpus.is_empty() {
            info!("GPU passthrough: {:?}", passthrough_gpus);
        }

        Ok(Self {
            vms,
            iommu_groups,
            passthrough_gpus,
        })
    }

    /// Rescan for VMs (call periodically)
    pub fn rescan(&mut self) -> Result<(Vec<VmInfo>, Vec<u32>)> {
        let current_vms = scan_vms()?;

        let current_pids: HashSet<u32> = current_vms.iter().map(|v| v.qemu_pid).collect();
        let old_pids: HashSet<u32> = self.vms.iter().map(|v| v.qemu_pid).collect();

        // Find new VMs
        let new_vms: Vec<VmInfo> = current_vms
            .into_iter()
            .filter(|v| !old_pids.contains(&v.qemu_pid))
            .collect();

        // Find removed VMs
        let removed_pids: Vec<u32> = old_pids.difference(&current_pids).copied().collect();

        // Update passthrough GPUs
        self.passthrough_gpus = get_passthrough_gpus(&self.iommu_groups);

        // Update VM list
        self.vms = scan_vms()?;

        Ok((new_vms, removed_pids))
    }

    /// Get all vCPU PIDs with their workload type
    pub fn get_vcpu_workloads(&self) -> HashMap<u32, VmWorkloadType> {
        let mut workloads = HashMap::new();

        for vm in &self.vms {
            for &vcpu_pid in &vm.vcpu_pids {
                workloads.insert(vcpu_pid, vm.workload_type);
            }
        }

        workloads
    }

    /// Get gaming VM vCPU count
    pub fn gaming_vcpu_count(&self) -> usize {
        self.vms
            .iter()
            .filter(|v| v.workload_type == VmWorkloadType::Gaming)
            .map(|v| v.vcpu_pids.len())
            .sum()
    }

    /// Get dev VM vCPU count
    pub fn dev_vcpu_count(&self) -> usize {
        self.vms
            .iter()
            .filter(|v| v.workload_type == VmWorkloadType::Dev)
            .map(|v| v.vcpu_pids.len())
            .sum()
    }

    /// Get total VM count
    pub fn vm_count(&self) -> usize {
        self.vms.len()
    }

    /// Get passthrough GPU count
    #[allow(dead_code)]
    pub fn passthrough_gpu_count(&self) -> usize {
        self.passthrough_gpus.len()
    }

    /// Get IOMMU summary
    pub fn iommu_summary(&self) -> String {
        let gpu_groups: Vec<_> = self.iommu_groups.iter().filter(|g| g.has_gpu).collect();

        if gpu_groups.is_empty() {
            return "no GPU groups".to_string();
        }

        let isolated = gpu_groups.iter().filter(|g| g.is_isolated).count();
        format!("{} GPU groups ({} isolated)", gpu_groups.len(), isolated)
    }

    /// Check if IOMMU is available
    pub fn has_iommu(&self) -> bool {
        !self.iommu_groups.is_empty()
    }
}

impl Default for VmMonitor {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            vms: Vec::new(),
            iommu_groups: Vec::new(),
            passthrough_gpus: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_iommu_groups() {
        let _groups = detect_iommu_groups();
    }

    #[test]
    fn test_scan_vms() {
        let result = scan_vms();
        assert!(result.is_ok());
    }
}
