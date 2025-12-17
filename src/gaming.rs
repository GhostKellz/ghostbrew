// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Gaming Process Detection
//
// Copyright (C) 2025 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::Result;
use log::{debug, info};
use std::collections::HashSet;
use std::fs;

/// Workload classification types (matches BPF side)
pub const WORKLOAD_GAMING: u32 = 1;
pub const WORKLOAD_AI: u32 = 4;

/// Gaming process patterns in executable paths
const GAMING_EXE_PATTERNS: &[&str] = &[
    "wine",
    "proton",
    "steam",
    "lutris",
    "heroic",
    "gamescope",
    "pressure-vessel",
];

/// Gaming-related environment variables
const GAMING_ENV_VARS: &[&str] = &[
    "WINEPREFIX",
    "STEAM_COMPAT_DATA_PATH",
    "STEAM_COMPAT_CLIENT_INSTALL_PATH",
    "PROTON_LOG",
    "DXVK_",
    "VKD3D_",
    "WINE_",
];

/// AI/ML process patterns
const AI_EXE_PATTERNS: &[&str] = &[
    "ollama", "llama", "pytorch", "python", // Many AI workloads run under python
];

/// AI-related environment variables
const AI_ENV_VARS: &[&str] = &["OLLAMA_", "CUDA_VISIBLE_DEVICES", "PYTORCH_", "TF_"];

/// Scan /proc for gaming and AI processes
/// Returns a map of PID -> workload class
pub fn scan_gaming_pids() -> Result<Vec<(u32, u32)>> {
    let mut gaming_pids = Vec::new();

    // Read /proc directory
    let proc_dir = match fs::read_dir("/proc") {
        Ok(dir) => dir,
        Err(e) => {
            debug!("Failed to read /proc: {}", e);
            return Ok(gaming_pids);
        }
    };

    for entry in proc_dir.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Skip non-numeric entries
        let pid: u32 = match name.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Check if this is a gaming or AI process
        if let Some(workload_class) = classify_process(pid) {
            gaming_pids.push((pid, workload_class));
        }
    }

    debug!("Found {} gaming/AI processes", gaming_pids.len());
    Ok(gaming_pids)
}

/// Classify a single process by PID
fn classify_process(pid: u32) -> Option<u32> {
    // Check executable path first (most reliable)
    if let Some(class) = check_exe_path(pid) {
        return Some(class);
    }

    // Check environment variables
    if let Some(class) = check_environ(pid) {
        return Some(class);
    }

    None
}

/// Check /proc/[pid]/exe for gaming patterns
fn check_exe_path(pid: u32) -> Option<u32> {
    let exe_path = format!("/proc/{}/exe", pid);

    let exe = match fs::read_link(&exe_path) {
        Ok(p) => p,
        Err(_) => return None,
    };

    let exe_str = exe.to_string_lossy().to_lowercase();

    // Check for gaming patterns
    for pattern in GAMING_EXE_PATTERNS {
        if exe_str.contains(pattern) {
            debug!("PID {} detected as gaming via exe: {}", pid, exe_str);
            return Some(WORKLOAD_GAMING);
        }
    }

    // Check for .exe suffix (Wine/Proton)
    if exe_str.ends_with(".exe") {
        debug!("PID {} detected as gaming via .exe: {}", pid, exe_str);
        return Some(WORKLOAD_GAMING);
    }

    // Check for AI patterns
    for pattern in AI_EXE_PATTERNS {
        if exe_str.contains(pattern) {
            // Additional check: python needs environ confirmation
            if *pattern == "python" {
                return None; // Let environ check handle python
            }
            debug!("PID {} detected as AI via exe: {}", pid, exe_str);
            return Some(WORKLOAD_AI);
        }
    }

    None
}

/// Check /proc/[pid]/environ for gaming environment variables
fn check_environ(pid: u32) -> Option<u32> {
    let environ_path = format!("/proc/{}/environ", pid);

    let environ = match fs::read_to_string(&environ_path) {
        Ok(e) => e,
        Err(_) => return None,
    };

    // Check for gaming environment variables
    for var in GAMING_ENV_VARS {
        if environ.contains(var) {
            debug!("PID {} detected as gaming via env: {}", pid, var);
            return Some(WORKLOAD_GAMING);
        }
    }

    // Check for AI environment variables
    for var in AI_ENV_VARS {
        if environ.contains(var) {
            debug!("PID {} detected as AI via env: {}", pid, var);
            return Some(WORKLOAD_AI);
        }
    }

    None
}

/// Get all child PIDs of a process (for marking entire process trees)
#[allow(dead_code)]
pub fn get_child_pids(pid: u32) -> Vec<u32> {
    let mut children = Vec::new();
    let children_path = format!("/proc/{}/task/{}/children", pid, pid);

    if let Ok(content) = fs::read_to_string(&children_path) {
        for child_str in content.split_whitespace() {
            if let Ok(child_pid) = child_str.parse::<u32>() {
                children.push(child_pid);
                // Recursively get grandchildren
                children.extend(get_child_pids(child_pid));
            }
        }
    }

    children
}

/// Gaming detector state for incremental updates
pub struct GamingDetector {
    known_gaming_pids: HashSet<u32>,
    known_ai_pids: HashSet<u32>,
}

impl GamingDetector {
    pub fn new() -> Self {
        Self {
            known_gaming_pids: HashSet::new(),
            known_ai_pids: HashSet::new(),
        }
    }

    /// Scan and return only changed PIDs (new or removed)
    #[allow(clippy::type_complexity)]
    pub fn scan_changes(&mut self) -> Result<(Vec<(u32, u32)>, Vec<u32>)> {
        let current_scan = scan_gaming_pids()?;

        let mut current_gaming: HashSet<u32> = HashSet::new();
        let mut current_ai: HashSet<u32> = HashSet::new();

        for (pid, class) in &current_scan {
            match *class {
                WORKLOAD_GAMING => {
                    current_gaming.insert(*pid);
                }
                WORKLOAD_AI => {
                    current_ai.insert(*pid);
                }
                _ => {}
            }
        }

        // Find new PIDs
        let mut new_pids: Vec<(u32, u32)> = Vec::new();
        for pid in current_gaming.difference(&self.known_gaming_pids) {
            new_pids.push((*pid, WORKLOAD_GAMING));
        }
        for pid in current_ai.difference(&self.known_ai_pids) {
            new_pids.push((*pid, WORKLOAD_AI));
        }

        // Find removed PIDs
        let mut removed_pids: Vec<u32> = Vec::new();
        for pid in self.known_gaming_pids.difference(&current_gaming) {
            removed_pids.push(*pid);
        }
        for pid in self.known_ai_pids.difference(&current_ai) {
            removed_pids.push(*pid);
        }

        // Update state
        self.known_gaming_pids = current_gaming;
        self.known_ai_pids = current_ai;

        if !new_pids.is_empty() || !removed_pids.is_empty() {
            info!(
                "Gaming detector: {} new, {} removed",
                new_pids.len(),
                removed_pids.len()
            );
        }

        Ok((new_pids, removed_pids))
    }

    /// Get counts for logging
    pub fn counts(&self) -> (usize, usize) {
        (self.known_gaming_pids.len(), self.known_ai_pids.len())
    }
}

impl Default for GamingDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_gaming_pids() {
        // This test just verifies the function runs without panicking
        let result = scan_gaming_pids();
        assert!(result.is_ok());
    }
}
