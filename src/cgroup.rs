// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Cgroup-based Workload Classification
//
// Copyright (C) 2025 ghostkellz <ckelley@ghostkellz.sh>
//
// Classifies workloads by cgroup path patterns:
// - gaming.slice, steam, proton -> GAMING
// - docker, libpod, containerd -> CONTAINER
// - machine-qemu -> VM
// - system.slice -> BATCH

use anyhow::Result;
use log::{debug, info};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Workload classes matching BPF definitions
pub const WORKLOAD_GAMING: u32 = 1;
#[allow(dead_code)]
pub const WORKLOAD_INTERACTIVE: u32 = 2;
pub const WORKLOAD_BATCH: u32 = 3;
pub const WORKLOAD_AI: u32 = 4;
pub const WORKLOAD_CONTAINER: u32 = 7;

/// Gaming cgroup patterns (path contains these)
const GAMING_PATTERNS: &[&str] = &[
    "gaming.slice",
    "gaming-",
    "steam",
    "proton",
    "lutris",
    "heroic",
    "gamescope",
    "wine",
];

/// Container cgroup patterns
const CONTAINER_PATTERNS: &[&str] = &["docker", "libpod", "podman", "containerd", "cri-o", "lxc"];

/// AI/ML cgroup patterns
const AI_PATTERNS: &[&str] = &["ollama", "pytorch", "tensorflow", "cuda"];

/// VM cgroup patterns (for QEMU/libvirt)
const VM_PATTERNS: &[&str] = &["machine-qemu", "machine.slice", "libvirt"];

/// Batch/system cgroup patterns (low priority)
const BATCH_PATTERNS: &[&str] = &["system.slice", "background.slice"];

/// Cgroup information with classification
#[derive(Debug, Clone)]
pub struct CgroupInfo {
    /// Full path to cgroup
    pub path: String,
    /// Cgroup ID (inode number of cgroup directory)
    pub id: u64,
    /// Classified workload type
    pub workload_class: u32,
}

/// Get cgroup ID from path (uses inode number as cgroup ID)
/// This matches how the kernel identifies cgroups via kn->id
fn get_cgroup_id(path: &Path) -> Option<u64> {
    // Try reading cgroup.id file first (cgroup v2)
    let id_path = path.join("cgroup.id");
    if let Ok(content) = fs::read_to_string(&id_path)
        && let Ok(id) = content.trim().parse::<u64>()
    {
        return Some(id);
    }

    // Fallback: use inode number of the directory
    // Note: This may not exactly match kernel's kn->id
    if let Ok(metadata) = fs::metadata(path) {
        return Some(metadata.ino());
    }

    None
}

/// Classify cgroup by its path
fn classify_cgroup_path(path: &str) -> u32 {
    let path_lower = path.to_lowercase();

    // Gaming patterns (highest priority for latency)
    for pattern in GAMING_PATTERNS {
        if path_lower.contains(pattern) {
            return WORKLOAD_GAMING;
        }
    }

    // AI/ML patterns
    for pattern in AI_PATTERNS {
        if path_lower.contains(pattern) {
            return WORKLOAD_AI;
        }
    }

    // Container patterns
    for pattern in CONTAINER_PATTERNS {
        if path_lower.contains(pattern) {
            return WORKLOAD_CONTAINER;
        }
    }

    // VM patterns (treat as batch by default)
    for pattern in VM_PATTERNS {
        if path_lower.contains(pattern) {
            return WORKLOAD_BATCH;
        }
    }

    // Batch/system patterns
    for pattern in BATCH_PATTERNS {
        if path_lower.contains(pattern) {
            return WORKLOAD_BATCH;
        }
    }

    // Default: no classification (let other detection methods handle it)
    0
}

/// Scan cgroup hierarchy and classify cgroups
pub fn scan_cgroups() -> Result<Vec<CgroupInfo>> {
    let mut cgroups = Vec::new();
    let cgroup_root = Path::new("/sys/fs/cgroup");

    if !cgroup_root.exists() {
        debug!("Cgroup filesystem not mounted at /sys/fs/cgroup");
        return Ok(cgroups);
    }

    scan_cgroup_dir(cgroup_root, "", &mut cgroups)?;

    Ok(cgroups)
}

/// Recursively scan cgroup directory
fn scan_cgroup_dir(dir: &Path, relative_path: &str, cgroups: &mut Vec<CgroupInfo>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    // Get cgroup ID for this directory
    if let Some(id) = get_cgroup_id(dir) {
        let workload_class = classify_cgroup_path(relative_path);

        // Only add if we have a classification
        if workload_class > 0 {
            cgroups.push(CgroupInfo {
                path: relative_path.to_string(),
                id,
                workload_class,
            });
        }
    }

    // Recurse into subdirectories
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip pseudo-files and controllers
                if name.starts_with("cgroup.")
                    || name.starts_with("cpu.")
                    || name.starts_with("memory.")
                    || name.starts_with("io.")
                    || name.starts_with("pids.")
                {
                    continue;
                }

                let new_relative = if relative_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", relative_path, name)
                };

                scan_cgroup_dir(&path, &new_relative, cgroups)?;
            }
        }
    }

    Ok(())
}

/// Cgroup monitor for tracking and classifying cgroups
pub struct CgroupMonitor {
    /// Classified cgroups: cgroup_id -> workload_class
    classifications: HashMap<u64, u32>,
    /// Path to ID mapping for logging
    path_map: HashMap<u64, String>,
}

impl CgroupMonitor {
    pub fn new() -> Result<Self> {
        let cgroups = scan_cgroups()?;
        let mut classifications = HashMap::new();
        let mut path_map = HashMap::new();

        for cg in &cgroups {
            classifications.insert(cg.id, cg.workload_class);
            path_map.insert(cg.id, cg.path.clone());
        }

        let gaming_count = cgroups
            .iter()
            .filter(|c| c.workload_class == WORKLOAD_GAMING)
            .count();
        let container_count = cgroups
            .iter()
            .filter(|c| c.workload_class == WORKLOAD_CONTAINER)
            .count();
        let ai_count = cgroups
            .iter()
            .filter(|c| c.workload_class == WORKLOAD_AI)
            .count();

        if !cgroups.is_empty() {
            info!(
                "Cgroups: {} classified ({} gaming, {} container, {} AI)",
                cgroups.len(),
                gaming_count,
                container_count,
                ai_count
            );

            // Log gaming cgroups specifically
            for cg in cgroups
                .iter()
                .filter(|c| c.workload_class == WORKLOAD_GAMING)
            {
                debug!("  Gaming cgroup: {} (id={})", cg.path, cg.id);
            }
        }

        Ok(Self {
            classifications,
            path_map,
        })
    }

    /// Rescan cgroups and return changes
    pub fn rescan(&mut self) -> Result<(Vec<CgroupInfo>, Vec<u64>)> {
        let current = scan_cgroups()?;

        let current_ids: std::collections::HashSet<u64> = current.iter().map(|c| c.id).collect();
        let old_ids: std::collections::HashSet<u64> =
            self.classifications.keys().copied().collect();

        // Find new cgroups
        let new_cgroups: Vec<CgroupInfo> = current
            .iter()
            .filter(|c| !old_ids.contains(&c.id))
            .cloned()
            .collect();

        // Find removed cgroups
        let removed_ids: Vec<u64> = old_ids.difference(&current_ids).copied().collect();

        // Update internal state
        self.classifications.clear();
        self.path_map.clear();
        for cg in &current {
            self.classifications.insert(cg.id, cg.workload_class);
            self.path_map.insert(cg.id, cg.path.clone());
        }

        // Log changes
        for cg in &new_cgroups {
            debug!(
                "New cgroup classified: {} -> class {}",
                cg.path, cg.workload_class
            );
        }
        for id in &removed_ids {
            if let Some(path) = self.path_map.get(id) {
                debug!("Cgroup removed: {}", path);
            }
        }

        Ok((new_cgroups, removed_ids))
    }

    /// Get all classifications for populating BPF map
    pub fn get_classifications(&self) -> &HashMap<u64, u32> {
        &self.classifications
    }

    /// Get count of classified cgroups
    pub fn classified_count(&self) -> usize {
        self.classifications.len()
    }

    /// Get count of gaming cgroups
    pub fn gaming_count(&self) -> usize {
        self.classifications
            .values()
            .filter(|&&c| c == WORKLOAD_GAMING)
            .count()
    }

    /// Get count of container cgroups
    #[allow(dead_code)]
    pub fn container_count(&self) -> usize {
        self.classifications
            .values()
            .filter(|&&c| c == WORKLOAD_CONTAINER)
            .count()
    }

    /// Get count of AI cgroups
    #[allow(dead_code)]
    pub fn ai_count(&self) -> usize {
        self.classifications
            .values()
            .filter(|&&c| c == WORKLOAD_AI)
            .count()
    }
}

impl Default for CgroupMonitor {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            classifications: HashMap::new(),
            path_map: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_cgroup_path() {
        assert_eq!(
            classify_cgroup_path("user.slice/gaming.slice/steam"),
            WORKLOAD_GAMING
        );
        assert_eq!(classify_cgroup_path("docker/abc123"), WORKLOAD_CONTAINER);
        assert_eq!(
            classify_cgroup_path("system.slice/sshd.service"),
            WORKLOAD_BATCH
        );
        assert_eq!(classify_cgroup_path("user.slice/user-1000.slice"), 0);
    }

    #[test]
    fn test_scan_cgroups() {
        let result = scan_cgroups();
        assert!(result.is_ok());
    }
}
