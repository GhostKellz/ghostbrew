// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Container Runtime Support
//
// Copyright (C) 2025 ghostkellz <ckelley@ghostkellz.sh>
//
// Supports:
// - Docker/containerd
// - Podman
// - NVIDIA Container Runtime/Toolkit
// - AI workloads (Ollama, PyTorch, TensorFlow)

use anyhow::Result;
use log::{debug, info};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Container workload classification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContainerWorkloadType {
    Ai,       // AI/ML workload (Ollama, PyTorch, etc.) - batch priority, GPU affinity
    Gaming,   // Gaming container (rare but possible) - gaming priority
    Compute,  // GPU compute workload - batch priority
    General,  // General container - batch priority
}

impl std::fmt::Display for ContainerWorkloadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerWorkloadType::Ai => write!(f, "AI"),
            ContainerWorkloadType::Gaming => write!(f, "gaming"),
            ContainerWorkloadType::Compute => write!(f, "compute"),
            ContainerWorkloadType::General => write!(f, "general"),
        }
    }
}

/// Information about a detected container
#[derive(Debug)]
#[allow(dead_code)]
pub struct ContainerInfo {
    /// Container ID (short form)
    pub id: String,
    /// Container name (if available)
    pub name: String,
    /// Runtime (docker, podman, containerd)
    pub runtime: String,
    /// Workload type
    pub workload_type: ContainerWorkloadType,
    /// Main process PIDs in the container
    pub pids: Vec<u32>,
    /// Whether container has GPU access
    pub has_gpu: bool,
    /// Cgroup path
    pub cgroup_path: String,
}

/// AI/ML process patterns
const AI_PATTERNS: &[&str] = &[
    "ollama",
    "ollama_llama_server",
    "llama",
    "pytorch",
    "torch",
    "tensorflow",
    "tf_",
    "transformers",
    "huggingface",
    "vllm",
    "triton",
    "onnx",
    "cuda",
];

/// Gaming patterns (for containerized gaming - rare)
const GAMING_PATTERNS: &[&str] = &[
    "steam",
    "proton",
    "wine",
    "gamescope",
];

/// GPU compute patterns
const COMPUTE_PATTERNS: &[&str] = &[
    "nvidia-smi",
    "cuda-",
    "nvcc",
    "nccl",
    "cudnn",
];

/// Detect if NVIDIA Container Runtime is available
pub fn nvidia_runtime_available() -> bool {
    // Check for nvidia-container-runtime
    Path::new("/usr/bin/nvidia-container-runtime").exists() ||
    Path::new("/usr/bin/nvidia-container-toolkit").exists() ||
    // Check Docker config for nvidia runtime
    Path::new("/etc/docker/daemon.json").exists() && {
        if let Ok(content) = fs::read_to_string("/etc/docker/daemon.json") {
            content.contains("nvidia")
        } else {
            false
        }
    }
}

/// Scan for containers via cgroups (runtime-agnostic)
pub fn scan_containers() -> Result<Vec<ContainerInfo>> {
    let mut containers = Vec::new();

    // Scan cgroup v2 hierarchy
    let cgroup_base = Path::new("/sys/fs/cgroup");
    if cgroup_base.exists() {
        scan_cgroup_dir(cgroup_base, &mut containers)?;
    }

    Ok(containers)
}

/// Recursively scan cgroup directory for containers
fn scan_cgroup_dir(dir: &Path, containers: &mut Vec<ContainerInfo>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let dir_name = dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Check if this looks like a container cgroup
    if is_container_cgroup(&dir_name) {
        if let Some(container) = parse_container_cgroup(dir)? {
            containers.push(container);
        }
        return Ok(()); // Don't recurse into container cgroups
    }

    // Recurse into subdirectories
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip system cgroups we don't care about
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "system.slice" || name == "user.slice" || name == "init.scope" {
                    continue;
                }
                scan_cgroup_dir(&path, containers)?;
            }
        }
    }

    Ok(())
}

/// Check if a cgroup directory name looks like a container
fn is_container_cgroup(name: &str) -> bool {
    // Docker format: docker-<id>.scope or <id> under docker/
    if name.starts_with("docker-") && name.ends_with(".scope") {
        return true;
    }

    // Podman format: libpod-<id>.scope
    if name.starts_with("libpod-") && name.ends_with(".scope") {
        return true;
    }

    // containerd format: varies, but often has container ID pattern
    if name.len() == 64 && name.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    // Short container IDs (12 chars)
    if name.len() == 12 && name.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    false
}

/// Parse a container cgroup and extract info
fn parse_container_cgroup(cgroup_path: &Path) -> Result<Option<ContainerInfo>> {
    let cgroup_name = cgroup_path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Extract container ID
    let id = extract_container_id(&cgroup_name);

    // Get PIDs in this cgroup
    let procs_path = cgroup_path.join("cgroup.procs");
    let pids = if procs_path.exists() {
        fs::read_to_string(&procs_path)?
            .lines()
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    } else {
        Vec::new()
    };

    if pids.is_empty() {
        return Ok(None);
    }

    // Determine runtime from path
    let path_str = cgroup_path.to_string_lossy();
    let runtime = if path_str.contains("docker") {
        "docker"
    } else if path_str.contains("libpod") || path_str.contains("podman") {
        "podman"
    } else if path_str.contains("containerd") {
        "containerd"
    } else {
        "unknown"
    }.to_string();

    // Classify workload and check for GPU
    let (workload_type, has_gpu) = classify_container_workload(&pids, cgroup_path);

    // Try to get container name (from Docker/Podman)
    let name = get_container_name(&id, &runtime);

    debug!("Container {}: {} PIDs, type: {}, GPU: {}",
           id, pids.len(), workload_type, has_gpu);

    Ok(Some(ContainerInfo {
        id,
        name,
        runtime,
        workload_type,
        pids,
        has_gpu,
        cgroup_path: cgroup_path.to_string_lossy().to_string(),
    }))
}

/// Extract container ID from cgroup name
fn extract_container_id(name: &str) -> String {
    // docker-<id>.scope -> <id>
    if let Some(id) = name.strip_prefix("docker-")
        && let Some(id) = id.strip_suffix(".scope") {
            return id[..12.min(id.len())].to_string();
        }

    // libpod-<id>.scope -> <id>
    if let Some(id) = name.strip_prefix("libpod-")
        && let Some(id) = id.strip_suffix(".scope") {
            return id[..12.min(id.len())].to_string();
        }

    // Already looks like an ID
    if name.len() >= 12 && name.chars().take(12).all(|c| c.is_ascii_hexdigit()) {
        return name[..12].to_string();
    }

    name.to_string()
}

/// Classify container workload based on processes and environment
fn classify_container_workload(pids: &[u32], cgroup_path: &Path) -> (ContainerWorkloadType, bool) {
    let mut has_gpu = false;
    let mut workload_type = ContainerWorkloadType::General;

    // Check each process in the container
    for &pid in pids {
        // Check process comm
        let comm_path = format!("/proc/{}/comm", pid);
        if let Ok(comm) = fs::read_to_string(&comm_path) {
            let comm_lower = comm.trim().to_lowercase();

            // AI patterns
            for pattern in AI_PATTERNS {
                if comm_lower.contains(pattern) {
                    workload_type = ContainerWorkloadType::Ai;
                    break;
                }
            }

            // Gaming patterns
            for pattern in GAMING_PATTERNS {
                if comm_lower.contains(pattern) {
                    workload_type = ContainerWorkloadType::Gaming;
                    break;
                }
            }

            // Compute patterns
            for pattern in COMPUTE_PATTERNS {
                if comm_lower.contains(pattern) {
                    if workload_type == ContainerWorkloadType::General {
                        workload_type = ContainerWorkloadType::Compute;
                    }
                    break;
                }
            }
        }

        // Check environment for NVIDIA/CUDA
        let environ_path = format!("/proc/{}/environ", pid);
        if let Ok(environ) = fs::read_to_string(&environ_path) {
            if environ.contains("NVIDIA") || environ.contains("CUDA") {
                has_gpu = true;
            }
            if environ.contains("OLLAMA") {
                workload_type = ContainerWorkloadType::Ai;
            }
        }
    }

    // Check if cgroup has NVIDIA device access
    let devices_path = cgroup_path.join("devices.list");
    if devices_path.exists()
        && let Ok(devices) = fs::read_to_string(&devices_path) {
            // NVIDIA devices are typically c 195:* (nvidia) or c 235:* (nvidia-uvm)
            if devices.contains("195:") || devices.contains("235:") {
                has_gpu = true;
            }
        }

    (workload_type, has_gpu)
}

/// Try to get container name from runtime
fn get_container_name(id: &str, runtime: &str) -> String {
    match runtime {
        "docker" => {
            // Try docker inspect (if docker CLI available)
            // For now, just use ID
            id.to_string()
        }
        "podman" => {
            id.to_string()
        }
        _ => id.to_string(),
    }
}

/// Scan specifically for Ollama processes
pub fn scan_ollama() -> Vec<(u32, String)> {
    let mut ollama_pids = Vec::new();

    if let Ok(proc_dir) = fs::read_dir("/proc") {
        for entry in proc_dir.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            let pid: u32 = match name.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Check comm
            let comm_path = format!("/proc/{}/comm", pid);
            if let Ok(comm) = fs::read_to_string(&comm_path) {
                let comm = comm.trim().to_lowercase();
                if comm.contains("ollama") {
                    ollama_pids.push((pid, comm.to_string()));
                }
            }
        }
    }

    ollama_pids
}

/// Container Monitor for tracking containers and their state
#[allow(dead_code)]
pub struct ContainerMonitor {
    containers: Vec<ContainerInfo>,
    nvidia_available: bool,
    ollama_pids: Vec<(u32, String)>,
}

impl ContainerMonitor {
    pub fn new() -> Result<Self> {
        let nvidia_available = nvidia_runtime_available();
        if nvidia_available {
            info!("NVIDIA Container Runtime: available");
        }

        let containers = scan_containers()?;
        let ollama_pids = scan_ollama();

        if !containers.is_empty() {
            info!("Containers: {} detected", containers.len());
            for c in &containers {
                debug!("  {} ({}): {} PIDs, type: {}, GPU: {}",
                       c.id, c.runtime, c.pids.len(), c.workload_type, c.has_gpu);
            }
        }

        if !ollama_pids.is_empty() {
            info!("Ollama: {} processes detected", ollama_pids.len());
        }

        Ok(Self {
            containers,
            nvidia_available,
            ollama_pids,
        })
    }

    /// Rescan for containers (call periodically)
    pub fn rescan(&mut self) -> Result<(Vec<ContainerInfo>, Vec<String>)> {
        let current = scan_containers()?;
        self.ollama_pids = scan_ollama();

        let current_ids: HashSet<String> = current.iter().map(|c| c.id.clone()).collect();
        let old_ids: HashSet<String> = self.containers.iter().map(|c| c.id.clone()).collect();

        // Find new containers
        let new_containers: Vec<ContainerInfo> = current.into_iter()
            .filter(|c| !old_ids.contains(&c.id))
            .collect();

        // Find removed containers
        let removed_ids: Vec<String> = old_ids.difference(&current_ids).cloned().collect();

        // Update container list
        self.containers = scan_containers()?;

        Ok((new_containers, removed_ids))
    }

    /// Get all container PIDs with their workload type
    #[allow(dead_code)]
    pub fn get_container_workloads(&self) -> HashMap<u32, ContainerWorkloadType> {
        let mut workloads = HashMap::new();

        for container in &self.containers {
            for &pid in &container.pids {
                workloads.insert(pid, container.workload_type);
            }
        }

        workloads
    }

    /// Get AI container count
    pub fn ai_container_count(&self) -> usize {
        self.containers.iter()
            .filter(|c| c.workload_type == ContainerWorkloadType::Ai)
            .count()
    }

    /// Get GPU container count
    pub fn gpu_container_count(&self) -> usize {
        self.containers.iter()
            .filter(|c| c.has_gpu)
            .count()
    }

    /// Get total container count
    pub fn container_count(&self) -> usize {
        self.containers.len()
    }

    /// Check if NVIDIA runtime is available
    #[allow(dead_code)]
    pub fn has_nvidia_runtime(&self) -> bool {
        self.nvidia_available
    }

    /// Get Ollama process count
    pub fn ollama_count(&self) -> usize {
        self.ollama_pids.len()
    }

    /// Get all container PIDs (for BPF map)
    pub fn all_pids(&self) -> Vec<(u32, ContainerWorkloadType)> {
        let mut pids = Vec::new();
        for container in &self.containers {
            for &pid in &container.pids {
                pids.push((pid, container.workload_type));
            }
        }
        pids
    }
}

impl Default for ContainerMonitor {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            containers: Vec::new(),
            nvidia_available: false,
            ollama_pids: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nvidia_runtime_available() {
        let _available = nvidia_runtime_available();
    }

    #[test]
    fn test_scan_containers() {
        let result = scan_containers();
        assert!(result.is_ok());
    }

    #[test]
    fn test_scan_ollama() {
        let _pids = scan_ollama();
    }
}
