// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Configuration Management
//
// Loads and manages scheduler configuration from TOML files.
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::{Context, Result};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GhostBrewConfig {
    /// Global default settings
    #[serde(default)]
    pub defaults: DefaultConfig,

    /// AMD-specific settings
    #[serde(default)]
    pub amd: AmdConfig,

    /// Intel-specific settings
    #[serde(default)]
    pub intel: IntelConfig,

    /// Path to game profiles directory
    #[serde(default)]
    pub profiles_dir: Option<PathBuf>,
}

/// Default scheduling parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultConfig {
    /// Burst detection threshold in nanoseconds
    #[serde(default = "default_burst_threshold")]
    pub burst_threshold_ns: u64,

    /// Time slice in nanoseconds
    #[serde(default = "default_slice_ns")]
    pub slice_ns: u64,

    /// Enable gaming mode by default
    #[serde(default = "default_gaming_mode")]
    pub gaming_mode: bool,

    /// Statistics interval in seconds
    #[serde(default = "default_stats_interval")]
    pub stats_interval: u64,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            burst_threshold_ns: default_burst_threshold(),
            slice_ns: default_slice_ns(),
            gaming_mode: default_gaming_mode(),
            stats_interval: default_stats_interval(),
        }
    }
}

fn default_burst_threshold() -> u64 {
    2_000_000 // 2ms
}

fn default_slice_ns() -> u64 {
    3_000_000 // 3ms
}

fn default_gaming_mode() -> bool {
    true
}

fn default_stats_interval() -> u64 {
    2
}

/// AMD-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmdConfig {
    /// Prefer V-Cache CCD for gaming tasks
    #[serde(default = "default_true")]
    pub prefer_vcache: bool,

    /// Enable AMD Prefcore integration
    #[serde(default = "default_true")]
    pub prefcore_enabled: bool,

    /// V-Cache switching strategy: "manual", "automatic", "follow_ghost_vcache"
    #[serde(default = "default_vcache_strategy")]
    pub vcache_switching: String,
}

impl Default for AmdConfig {
    fn default() -> Self {
        Self {
            prefer_vcache: true,
            prefcore_enabled: true,
            vcache_switching: default_vcache_strategy(),
        }
    }
}

fn default_vcache_strategy() -> String {
    "follow_ghost_vcache".to_string()
}

/// Intel-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelConfig {
    /// Prefer P-cores for gaming/interactive tasks
    #[serde(default = "default_true")]
    pub prefer_pcores: bool,

    /// E-core offload mode: "disabled", "conservative", "aggressive"
    #[serde(default = "default_ecore_offload")]
    pub ecore_offload: String,
}

impl Default for IntelConfig {
    fn default() -> Self {
        Self {
            prefer_pcores: true,
            ecore_offload: default_ecore_offload(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_ecore_offload() -> String {
    "conservative".to_string()
}

/// Standard config file locations (in priority order)
const CONFIG_PATHS: &[&str] = &[
    "/etc/ghostbrew/config.toml",
    "~/.config/ghostbrew/config.toml",
];

impl GhostBrewConfig {
    /// Load configuration from standard paths
    pub fn load() -> Result<Self> {
        for path in CONFIG_PATHS {
            let expanded = shellexpand::tilde(path);
            let path = PathBuf::from(expanded.as_ref());

            if path.exists() {
                return Self::load_from_path(&path);
            }
        }

        debug!("No config file found, using defaults");
        Ok(Self::default())
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let config: GhostBrewConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", path))?;

        info!("Loaded config from {:?}", path);
        debug!("Config: {:?}", config);

        Ok(config)
    }

    /// Check if V-Cache auto-switching is enabled
    pub fn is_vcache_auto_switching(&self) -> bool {
        self.amd.vcache_switching.to_lowercase() == "automatic"
    }

    /// Check if we should follow ghost-vcache mode changes
    pub fn should_follow_ghost_vcache(&self) -> bool {
        self.amd.vcache_switching.to_lowercase() == "follow_ghost_vcache"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GhostBrewConfig::default();
        assert_eq!(config.defaults.burst_threshold_ns, 2_000_000);
        assert_eq!(config.defaults.slice_ns, 3_000_000);
        assert!(config.defaults.gaming_mode);
        assert!(config.amd.prefer_vcache);
        assert!(config.intel.prefer_pcores);
    }

    #[test]
    fn test_parse_config() {
        let toml_str = r#"
[defaults]
burst_threshold_ns = 1500000
gaming_mode = false

[intel]
ecore_offload = "aggressive"
"#;
        let config: GhostBrewConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.defaults.burst_threshold_ns, 1500000);
        assert!(!config.defaults.gaming_mode);
        assert_eq!(config.intel.ecore_offload, "aggressive");
    }
}
