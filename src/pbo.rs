// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - AMD PBO/Prefcore Integration
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::fs;
use std::path::Path;

/// AMD pstate prefcore information
pub struct PrefcoreInfo {
    /// Whether prefcore is enabled system-wide
    pub enabled: bool,
    /// Per-CPU prefcore rankings (0-255, higher = preferred)
    pub rankings: Vec<u32>,
    /// Highest ranking value found
    pub max_ranking: u32,
    /// CPUs with the highest ranking (best for boosting)
    pub preferred_cpus: Vec<u32>,
}

impl PrefcoreInfo {
    pub fn new(nr_cpus: u32) -> Self {
        Self {
            enabled: false,
            rankings: vec![0; nr_cpus as usize],
            max_ranking: 0,
            preferred_cpus: Vec::new(),
        }
    }
}

/// Detect AMD pstate prefcore rankings
pub fn detect_prefcore(nr_cpus: u32) -> Result<PrefcoreInfo> {
    let mut info = PrefcoreInfo::new(nr_cpus);

    // Check if prefcore is enabled
    let prefcore_path = "/sys/devices/system/cpu/amd_pstate/prefcore";
    if Path::new(prefcore_path).exists()
        && let Ok(content) = fs::read_to_string(prefcore_path)
    {
        info.enabled = content.trim() == "enabled";
    }

    if !info.enabled {
        debug!("AMD prefcore not enabled");
        return Ok(info);
    }

    info!("AMD prefcore enabled - reading CPU rankings");

    // Read per-CPU prefcore rankings
    for cpu in 0..nr_cpus {
        let ranking_path = format!(
            "/sys/devices/system/cpu/cpufreq/policy{}/amd_pstate_prefcore_ranking",
            cpu
        );

        if let Ok(content) = fs::read_to_string(&ranking_path)
            && let Ok(ranking) = content.trim().parse::<u32>()
        {
            info.rankings[cpu as usize] = ranking;
            if ranking > info.max_ranking {
                info.max_ranking = ranking;
            }
        }
    }

    // Find CPUs with the highest ranking
    for (cpu, &ranking) in info.rankings.iter().enumerate() {
        if ranking == info.max_ranking && info.max_ranking > 0 {
            info.preferred_cpus.push(cpu as u32);
        }
    }

    // Log summary
    if !info.preferred_cpus.is_empty() {
        info!(
            "Prefcore: max ranking {} on CPUs {:?}",
            info.max_ranking, info.preferred_cpus
        );
    }

    // Log per-CCD rankings if debug enabled
    for cpu in 0..nr_cpus {
        debug!(
            "CPU {}: prefcore ranking {}",
            cpu, info.rankings[cpu as usize]
        );
    }

    Ok(info)
}

/// Get the current EPP (Energy Performance Preference) for a CPU
pub fn get_cpu_epp(cpu: u32) -> Result<String> {
    let path = format!(
        "/sys/devices/system/cpu/cpufreq/policy{}/energy_performance_preference",
        cpu
    );
    fs::read_to_string(&path)
        .map(|s| s.trim().to_string())
        .with_context(|| format!("Failed to read EPP for CPU {}", cpu))
}

/// Set the EPP for a CPU
pub fn set_cpu_epp(cpu: u32, epp: &str) -> Result<()> {
    let path = format!(
        "/sys/devices/system/cpu/cpufreq/policy{}/energy_performance_preference",
        cpu
    );
    fs::write(&path, epp).with_context(|| format!("Failed to set EPP {} for CPU {}", epp, cpu))
}

/// Get available EPP values for a CPU
#[allow(dead_code)]
pub fn get_available_epps(cpu: u32) -> Result<Vec<String>> {
    let path = format!(
        "/sys/devices/system/cpu/cpufreq/policy{}/energy_performance_available_preferences",
        cpu
    );
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read available EPPs for CPU {}", cpu))?;
    Ok(content.split_whitespace().map(String::from).collect())
}

/// AMD pstate driver mode
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum PstateMode {
    Active,  // amd-pstate-epp
    Passive, // amd-pstate
    Guided,  // amd-pstate with guided autonomous mode
    Unknown,
}

/// Detect the current amd_pstate driver mode
#[allow(dead_code)]
pub fn detect_pstate_mode() -> PstateMode {
    let status_path = "/sys/devices/system/cpu/amd_pstate/status";

    if let Ok(content) = fs::read_to_string(status_path) {
        match content.trim() {
            "active" => PstateMode::Active,
            "passive" => PstateMode::Passive,
            "guided" => PstateMode::Guided,
            _ => PstateMode::Unknown,
        }
    } else {
        // Check if amd_pstate is even loaded
        let driver_path = "/sys/devices/system/cpu/cpufreq/policy0/scaling_driver";
        if let Ok(driver) = fs::read_to_string(driver_path)
            && driver.trim().starts_with("amd")
        {
            return PstateMode::Unknown;
        }
        PstateMode::Unknown
    }
}

/// EPP state manager for tracking and restoring EPP values
pub struct EppManager {
    /// Original EPP values per CPU
    original_epp: Vec<Option<String>>,
    /// Current EPP values per CPU
    current_epp: Vec<Option<String>>,
    /// Whether we've modified EPP
    modified: bool,
}

impl EppManager {
    pub fn new(nr_cpus: u32) -> Self {
        Self {
            original_epp: vec![None; nr_cpus as usize],
            current_epp: vec![None; nr_cpus as usize],
            modified: false,
        }
    }

    /// Save original EPP values for all CPUs
    pub fn save_original(&mut self, nr_cpus: u32) {
        for cpu in 0..nr_cpus {
            if let Ok(epp) = get_cpu_epp(cpu) {
                self.original_epp[cpu as usize] = Some(epp.clone());
                self.current_epp[cpu as usize] = Some(epp);
            }
        }
    }

    /// Set EPP for a CPU (tracks changes)
    pub fn set_epp(&mut self, cpu: u32, epp: &str) -> Result<()> {
        let cpu_idx = cpu as usize;

        // Check if already set
        if let Some(current) = &self.current_epp[cpu_idx]
            && current == epp
        {
            return Ok(());
        }

        set_cpu_epp(cpu, epp)?;
        self.current_epp[cpu_idx] = Some(epp.to_string());
        self.modified = true;
        debug!("Set CPU {} EPP to {}", cpu, epp);

        Ok(())
    }

    /// Restore original EPP values
    pub fn restore_original(&mut self) {
        if !self.modified {
            return;
        }

        for (cpu, original) in self.original_epp.iter().enumerate() {
            if let Some(epp) = original
                && let Err(e) = set_cpu_epp(cpu as u32, epp)
            {
                warn!("Failed to restore EPP for CPU {}: {}", cpu, e);
            }
        }

        info!("Restored original EPP values");
    }
}

impl Drop for EppManager {
    fn drop(&mut self) {
        self.restore_original();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_pstate_mode() {
        // This just tests that the function doesn't panic
        let _mode = detect_pstate_mode();
    }

    #[test]
    fn test_detect_prefcore() {
        // This just tests basic functionality
        let result = detect_prefcore(32);
        assert!(result.is_ok());
    }
}
