// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Per-Game Profile Management
//
// Loads and manages game-specific scheduling profiles from TOML files.
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::{Context, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Game profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    /// Profile name (e.g., "Cyberpunk 2077")
    pub name: String,

    /// Process matching patterns
    #[serde(default)]
    pub exe_name: Option<String>,

    /// Steam App ID for matching
    #[serde(default)]
    pub steam_appid: Option<u32>,

    /// Comm name pattern (regex-like matching)
    #[serde(default)]
    pub comm_pattern: Option<String>,

    /// Scheduling tunables
    #[serde(default)]
    pub tunables: ProfileTunables,

    /// V-Cache preference for AMD X3D
    #[serde(default)]
    pub vcache_preference: VCachePreference,

    /// SMT behavior preference
    #[serde(default)]
    pub smt_preference: SmtPreference,
}

/// Per-profile scheduling tunables
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileTunables {
    /// Burst detection threshold in nanoseconds
    #[serde(default)]
    pub burst_threshold_ns: Option<u64>,

    /// Time slice in nanoseconds
    #[serde(default)]
    pub slice_ns: Option<u64>,

    /// Priority boost for main game thread
    #[serde(default)]
    pub priority_boost: Option<i32>,
}

/// V-Cache preference for AMD X3D processors
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VCachePreference {
    /// Let scheduler decide based on workload
    #[default]
    Auto,
    /// Prefer V-Cache CCD (gaming mode)
    Cache,
    /// Prefer high-frequency CCD (productivity)
    Frequency,
}

/// SMT (hyperthreading) preference
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SmtPreference {
    /// Let scheduler decide
    #[default]
    Auto,
    /// Prefer CPUs with idle SMT sibling
    PreferIdle,
    /// Allow shared physical cores
    AllowShared,
}

/// Profile manager handles loading and matching game profiles
pub struct ProfileManager {
    profiles: HashMap<String, GameProfile>,
    /// Index by lowercase exe name for fast lookup
    by_exe: HashMap<String, String>,
    /// Index by Steam App ID for fast lookup
    by_appid: HashMap<u32, String>,
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            by_exe: HashMap::new(),
            by_appid: HashMap::new(),
        }
    }

    /// Load profiles from a directory
    pub fn load_from_directory(&mut self, dir: &PathBuf) -> Result<usize> {
        if !dir.exists() {
            debug!("Profiles directory does not exist: {:?}", dir);
            return Ok(0);
        }

        let mut count = 0;

        for entry in fs::read_dir(dir).with_context(|| format!("Failed to read {:?}", dir))? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "toml") {
                match self.load_profile_file(&path) {
                    Ok(profile) => {
                        self.add_profile(profile);
                        count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to load profile {:?}: {}", path, e);
                    }
                }
            }
        }

        info!("Loaded {} game profiles from {:?}", count, dir);
        Ok(count)
    }

    /// Load profiles from standard paths
    pub fn load_standard_paths(&mut self) -> Result<usize> {
        let paths = vec![
            PathBuf::from("/etc/ghostbrew/profiles"),
            dirs::config_dir()
                .map(|p| p.join("ghostbrew/profiles"))
                .unwrap_or_default(),
        ];

        let mut total = 0;
        for path in paths {
            if path.exists() {
                total += self.load_from_directory(&path)?;
            }
        }

        Ok(total)
    }

    /// Load a single profile from a TOML file
    fn load_profile_file(&self, path: &PathBuf) -> Result<GameProfile> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read profile: {:?}", path))?;

        let profile: GameProfile = toml::from_str(&content)
            .with_context(|| format!("Failed to parse profile: {:?}", path))?;

        debug!("Loaded profile: {} from {:?}", profile.name, path);
        Ok(profile)
    }

    /// Add a profile to the manager
    fn add_profile(&mut self, profile: GameProfile) {
        let name = profile.name.clone();

        // Index by exe name (lowercase for case-insensitive matching)
        if let Some(ref exe) = profile.exe_name {
            self.by_exe.insert(exe.to_lowercase(), name.clone());
        }

        // Index by Steam App ID
        if let Some(appid) = profile.steam_appid {
            self.by_appid.insert(appid, name.clone());
        }

        self.profiles.insert(name, profile);
    }

    /// Match a process to a profile by exe name or Steam App ID
    pub fn match_process(&self, exe_name: &str, steam_appid: Option<u32>) -> Option<&GameProfile> {
        // Try Steam App ID first (most specific)
        if let Some(appid) = steam_appid
            && let Some(profile_name) = self.by_appid.get(&appid)
        {
            return self.profiles.get(profile_name);
        }

        // Try exact exe name match (case-insensitive)
        let exe_lower = exe_name.to_lowercase();
        if let Some(profile_name) = self.by_exe.get(&exe_lower) {
            return self.profiles.get(profile_name);
        }

        // Try partial exe name match (e.g., "Cyberpunk2077.exe" contains "cyberpunk")
        for (pattern, profile_name) in &self.by_exe {
            if exe_lower.contains(pattern) || pattern.contains(&exe_lower) {
                return self.profiles.get(profile_name);
            }
        }

        None
    }

    /// Get all profiles (for iteration)
    pub fn all_profiles(&self) -> impl Iterator<Item = &GameProfile> {
        self.profiles.values()
    }

    /// Number of loaded profiles
    pub fn count(&self) -> usize {
        self.profiles.len()
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_parse() {
        let toml_str = r#"
name = "Test Game"
exe_name = "testgame.exe"
steam_appid = 12345
vcache_preference = "cache"
smt_preference = "prefer_idle"

[tunables]
burst_threshold_ns = 1000000
"#;
        let profile: GameProfile = toml::from_str(toml_str).unwrap();
        assert_eq!(profile.name, "Test Game");
        assert_eq!(profile.exe_name, Some("testgame.exe".to_string()));
        assert_eq!(profile.steam_appid, Some(12345));
        assert_eq!(profile.tunables.burst_threshold_ns, Some(1000000));
        assert_eq!(profile.vcache_preference, VCachePreference::Cache);
        assert_eq!(profile.smt_preference, SmtPreference::PreferIdle);
    }
}
