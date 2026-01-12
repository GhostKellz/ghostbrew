// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - V-Cache Integration (ghost-vcache coordination)
//
// Monitors and coordinates with the ghost-vcache tool for V-Cache mode switching
// on AMD X3D processors.
//
// Sysfs interface: /sys/bus/platform/drivers/amd_x3d_vcache/*/amd_x3d_mode
// Modes: "cache" (gaming) vs "frequency" (productivity)
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::{Context, Result};
use log::{debug, info};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// V-Cache operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VCacheMode {
    /// Optimized for cache (gaming) - prefer V-Cache CCD
    Cache,
    /// Optimized for frequency (productivity) - prefer high-frequency CCD
    Frequency,
    /// Unknown or unsupported
    Unknown,
}

impl VCacheMode {
    /// Parse mode from sysfs string
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "cache" => VCacheMode::Cache,
            "frequency" | "freq" => VCacheMode::Frequency,
            _ => VCacheMode::Unknown,
        }
    }

    /// Convert to sysfs string
    pub fn to_sysfs_str(self) -> &'static str {
        match self {
            VCacheMode::Cache => "cache",
            VCacheMode::Frequency => "frequency",
            VCacheMode::Unknown => "cache", // Default to cache
        }
    }

    /// Convert to gaming_mode bool for BPF
    pub fn to_gaming_mode(self) -> bool {
        match self {
            VCacheMode::Cache => true,
            VCacheMode::Frequency => false,
            VCacheMode::Unknown => true, // Default to gaming
        }
    }
}

impl std::fmt::Display for VCacheMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VCacheMode::Cache => write!(f, "cache"),
            VCacheMode::Frequency => write!(f, "frequency"),
            VCacheMode::Unknown => write!(f, "unknown"),
        }
    }
}

/// V-Cache switching strategy
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SwitchingStrategy {
    /// User controls via ghost-vcache CLI
    Manual,
    /// GhostBrew decides based on workload
    Automatic {
        gaming_threshold: u32,
        batch_threshold: u32,
    },
    /// React to ghost-vcache changes only
    #[default]
    FollowGhostVcache,
}

/// V-Cache controller for ghost-vcache integration
pub struct VCacheController {
    /// Path to the amd_x3d_mode sysfs file
    sysfs_path: Option<PathBuf>,
    /// Current mode (cached)
    current_mode: VCacheMode,
    /// Switching strategy
    strategy: SwitchingStrategy,
    /// Last time we checked the sysfs file
    last_check: Instant,
    /// Minimum interval between checks (avoid excessive sysfs reads)
    check_interval: Duration,
    /// Hysteresis: require mode to be stable for this duration before switching
    hysteresis_duration: Duration,
    /// When did the target mode become stable
    stable_since: Option<Instant>,
    /// Target mode for hysteresis
    pending_mode: Option<VCacheMode>,
}

impl VCacheController {
    /// Create a new V-Cache controller, auto-detecting sysfs path
    pub fn new() -> Result<Self> {
        let sysfs_path = find_vcache_sysfs();

        let current_mode = if let Some(ref path) = sysfs_path {
            read_vcache_mode(path).unwrap_or(VCacheMode::Unknown)
        } else {
            VCacheMode::Unknown
        };

        if sysfs_path.is_some() {
            info!(
                "V-Cache controller initialized, current mode: {}",
                current_mode
            );
        } else {
            debug!("V-Cache sysfs interface not found (not an X3D processor?)");
        }

        Ok(Self {
            sysfs_path,
            current_mode,
            strategy: SwitchingStrategy::default(),
            last_check: Instant::now(),
            check_interval: Duration::from_millis(500),
            hysteresis_duration: Duration::from_secs(5),
            stable_since: None,
            pending_mode: None,
        })
    }

    /// Check if V-Cache switching is available
    pub fn is_available(&self) -> bool {
        self.sysfs_path.is_some()
    }

    /// Get current V-Cache mode
    pub fn current_mode(&self) -> VCacheMode {
        self.current_mode
    }

    /// Set the switching strategy
    pub fn set_strategy(&mut self, strategy: SwitchingStrategy) {
        self.strategy = strategy;
    }

    /// Poll for mode changes from ghost-vcache
    ///
    /// Returns Some(new_mode) if the mode changed, None otherwise.
    pub fn poll_changes(&mut self) -> Option<VCacheMode> {
        if !self.is_available() {
            return None;
        }

        // Rate limit sysfs reads
        if self.last_check.elapsed() < self.check_interval {
            return None;
        }
        self.last_check = Instant::now();

        let path = self.sysfs_path.as_ref()?;
        let new_mode = read_vcache_mode(path).unwrap_or(VCacheMode::Unknown);

        if new_mode != self.current_mode {
            let old_mode = self.current_mode;
            self.current_mode = new_mode;
            info!("V-Cache mode changed: {} -> {}", old_mode, new_mode);
            return Some(new_mode);
        }

        None
    }

    /// Request a mode change (write to sysfs)
    ///
    /// Note: This requires appropriate permissions (typically root).
    pub fn request_mode(&mut self, mode: VCacheMode) -> Result<()> {
        let path = self.sysfs_path.as_ref().context("V-Cache not available")?;

        let mode_str = mode.to_sysfs_str();
        fs::write(path, mode_str).with_context(|| {
            format!("Failed to write V-Cache mode '{}' to {:?}", mode_str, path)
        })?;

        self.current_mode = mode;
        info!("V-Cache mode set to: {}", mode);

        Ok(())
    }

    /// Evaluate whether a mode switch is needed based on workload metrics
    ///
    /// For automatic strategy, decides based on gaming task count.
    pub fn evaluate_switch(
        &mut self,
        nr_gaming_tasks: u64,
        nr_batch_tasks: u64,
    ) -> Option<VCacheMode> {
        match &self.strategy {
            SwitchingStrategy::Manual | SwitchingStrategy::FollowGhostVcache => None,

            SwitchingStrategy::Automatic {
                gaming_threshold,
                batch_threshold,
            } => {
                let target = if nr_gaming_tasks >= *gaming_threshold as u64 {
                    VCacheMode::Cache
                } else if nr_batch_tasks >= *batch_threshold as u64 && nr_gaming_tasks == 0 {
                    VCacheMode::Frequency
                } else {
                    return None; // No clear signal
                };

                // Apply hysteresis
                self.apply_hysteresis(target)
            }
        }
    }

    /// Apply hysteresis to avoid rapid mode switching
    fn apply_hysteresis(&mut self, target: VCacheMode) -> Option<VCacheMode> {
        if target == self.current_mode {
            // Already in target mode, clear pending
            self.pending_mode = None;
            self.stable_since = None;
            return None;
        }

        if self.pending_mode == Some(target)
            && let Some(since) = self.stable_since
            && since.elapsed() >= self.hysteresis_duration
        {
            // Stable long enough, switch
            self.pending_mode = None;
            self.stable_since = None;
            return Some(target);
        } else if self.pending_mode != Some(target) {
            // New target, start hysteresis timer
            self.pending_mode = Some(target);
            self.stable_since = Some(Instant::now());
        }

        None
    }
}

impl Default for VCacheController {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            sysfs_path: None,
            current_mode: VCacheMode::Unknown,
            strategy: SwitchingStrategy::default(),
            last_check: Instant::now(),
            check_interval: Duration::from_millis(500),
            hysteresis_duration: Duration::from_secs(5),
            stable_since: None,
            pending_mode: None,
        })
    }
}

/// Find the sysfs path for amd_x3d_vcache mode file
fn find_vcache_sysfs() -> Option<PathBuf> {
    let base = PathBuf::from("/sys/bus/platform/drivers/amd_x3d_vcache");

    if !base.exists() {
        return None;
    }

    // Look for device directories (AMDI*)
    if let Ok(entries) = fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let mode_file = path.join("amd_x3d_mode");
                if mode_file.exists() {
                    debug!("Found V-Cache sysfs at: {:?}", mode_file);
                    return Some(mode_file);
                }
            }
        }
    }

    // Fallback: check for wildcard pattern
    let glob_pattern = base.join("AMDI*/amd_x3d_mode");
    if let Some(pattern_str) = glob_pattern.to_str()
        && let Ok(paths) = glob::glob(pattern_str)
    {
        for path in paths.flatten() {
            if path.exists() {
                debug!("Found V-Cache sysfs at: {:?}", path);
                return Some(path);
            }
        }
    }

    None
}

/// Read current V-Cache mode from sysfs
fn read_vcache_mode(path: &PathBuf) -> Result<VCacheMode> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read V-Cache mode from {:?}", path))?;

    Ok(VCacheMode::from_str(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcache_mode_parse() {
        assert_eq!(VCacheMode::from_str("cache"), VCacheMode::Cache);
        assert_eq!(VCacheMode::from_str("Cache"), VCacheMode::Cache);
        assert_eq!(VCacheMode::from_str("frequency"), VCacheMode::Frequency);
        assert_eq!(VCacheMode::from_str("freq"), VCacheMode::Frequency);
        assert_eq!(VCacheMode::from_str("unknown"), VCacheMode::Unknown);
        assert_eq!(VCacheMode::from_str("  cache\n"), VCacheMode::Cache);
    }

    #[test]
    fn test_vcache_mode_to_gaming() {
        assert!(VCacheMode::Cache.to_gaming_mode());
        assert!(!VCacheMode::Frequency.to_gaming_mode());
        assert!(VCacheMode::Unknown.to_gaming_mode());
    }

    #[test]
    fn test_switching_strategy_default() {
        let strategy = SwitchingStrategy::default();
        assert_eq!(strategy, SwitchingStrategy::FollowGhostVcache);
    }
}
