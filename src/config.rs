// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Configuration Management
//
// Loads and manages scheduler configuration from TOML files.
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::{Context, Result};
use log::{debug, info, warn};
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

    /// Force gaming mode on or off. Absent means "let auto-detection decide",
    /// which is why this is an Option rather than a defaulted bool.
    #[serde(default)]
    pub gaming_mode: Option<bool>,

    /// Statistics interval in seconds
    #[serde(default = "default_stats_interval")]
    pub stats_interval: u64,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            burst_threshold_ns: default_burst_threshold(),
            slice_ns: default_slice_ns(),
            gaming_mode: None,
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

/// E-core offload strategy for Intel hybrid CPUs.
///
/// The discriminants are the wire format for the BPF `ecore_offload_mode`
/// rodata, so they must stay in sync with `ghostbrew.bpf.c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcoreOffload {
    Disabled,
    Conservative,
    Aggressive,
}

impl EcoreOffload {
    /// Parse a mode name, accepting the aliases the CLI has always allowed.
    /// Returns `None` for unrecognized input so the caller can warn.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "disabled" | "off" | "0" => Some(Self::Disabled),
            "conservative" | "1" => Some(Self::Conservative),
            "aggressive" | "2" => Some(Self::Aggressive),
            _ => None,
        }
    }

    pub fn as_bpf_mode(self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::Conservative => 1,
            Self::Aggressive => 2,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Conservative => "conservative",
            Self::Aggressive => "aggressive",
        }
    }
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

    /// Layer CLI overrides on top of the config file, which in turn sits on top
    /// of the built-in defaults. A `None` override means the flag was absent.
    pub fn resolve(&self, cli: &CliOverrides) -> ResolvedSettings {
        let ecore_offload = cli
            .ecore_offload
            .as_deref()
            .map(|s| (s, "--ecore-offload"))
            .or(Some((
                self.intel.ecore_offload.as_str(),
                "config intel.ecore_offload",
            )))
            .and_then(|(raw, origin)| match EcoreOffload::parse(raw) {
                Some(mode) => Some(mode),
                None => {
                    warn!(
                        "Unknown e-core offload mode '{}' from {}, using conservative",
                        raw, origin
                    );
                    None
                }
            })
            .unwrap_or(EcoreOffload::Conservative);

        ResolvedSettings {
            burst_threshold_ns: cli
                .burst_threshold_ns
                .unwrap_or(self.defaults.burst_threshold_ns),
            slice_ns: cli.slice_ns.unwrap_or(self.defaults.slice_ns),
            stats_interval: cli.stats_interval.unwrap_or(self.defaults.stats_interval),
            ecore_offload,
            gaming_mode: self.defaults.gaming_mode,
            prefer_vcache: self.amd.prefer_vcache,
            prefcore_enabled: self.amd.prefcore_enabled,
            prefer_pcores: self.intel.prefer_pcores,
        }
    }
}

/// Command-line overrides. `None` means the flag was not passed, in which case
/// the config file (and then the built-in default) supplies the value.
#[derive(Debug, Default, Clone)]
pub struct CliOverrides {
    pub burst_threshold_ns: Option<u64>,
    pub slice_ns: Option<u64>,
    pub stats_interval: Option<u64>,
    pub ecore_offload: Option<String>,
}

/// Effective settings after resolving CLI over config over defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSettings {
    pub burst_threshold_ns: u64,
    pub slice_ns: u64,
    pub stats_interval: u64,
    pub ecore_offload: EcoreOffload,
    /// `None` leaves the gaming/work decision to auto-detection.
    pub gaming_mode: Option<bool>,
    pub prefer_vcache: bool,
    pub prefcore_enabled: bool,
    pub prefer_pcores: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GhostBrewConfig::default();
        assert_eq!(config.defaults.burst_threshold_ns, 2_000_000);
        assert_eq!(config.defaults.slice_ns, 3_000_000);
        assert_eq!(config.defaults.gaming_mode, None);
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
        assert_eq!(config.defaults.gaming_mode, Some(false));
        assert_eq!(config.intel.ecore_offload, "aggressive");
    }

    #[test]
    fn test_ecore_offload_modes() {
        assert_eq!(
            EcoreOffload::parse("disabled"),
            Some(EcoreOffload::Disabled)
        );
        assert_eq!(EcoreOffload::parse("off"), Some(EcoreOffload::Disabled));
        assert_eq!(EcoreOffload::parse("0"), Some(EcoreOffload::Disabled));
        assert_eq!(
            EcoreOffload::parse("CONSERVATIVE"),
            Some(EcoreOffload::Conservative)
        );
        assert_eq!(
            EcoreOffload::parse("aggressive"),
            Some(EcoreOffload::Aggressive)
        );
        assert_eq!(EcoreOffload::parse("nonsense"), None);
    }

    #[test]
    fn test_ecore_offload_bpf_encoding() {
        assert_eq!(EcoreOffload::Disabled.as_bpf_mode(), 0);
        assert_eq!(EcoreOffload::Conservative.as_bpf_mode(), 1);
        assert_eq!(EcoreOffload::Aggressive.as_bpf_mode(), 2);
    }

    #[test]
    fn test_vcache_strategy_accessors() {
        let mut config = GhostBrewConfig::default();
        // Default strategy is follow_ghost_vcache.
        assert!(!config.is_vcache_auto_switching());
        assert!(config.should_follow_ghost_vcache());

        config.amd.vcache_switching = "Automatic".to_string();
        assert!(config.is_vcache_auto_switching());
        assert!(!config.should_follow_ghost_vcache());

        config.amd.vcache_switching = "manual".to_string();
        assert!(!config.is_vcache_auto_switching());
        assert!(!config.should_follow_ghost_vcache());
    }

    /// The shipped example configs must only use keys `GhostBrewConfig`
    /// actually reads. Before v0.3.4 several of them advertised knobs the
    /// scheduler silently ignored; this is the guard against that recurring.
    #[test]
    fn test_example_configs_only_use_known_keys() {
        // Derive the key set from a fully-populated config rather than
        // restating it here, where it would drift. `Value::try_from` is used
        // instead of `to_string` because the latter rejects the root-level
        // `profiles_dir` appearing after the `[defaults]`/`[amd]`/`[intel]`
        // tables.
        let schema_src = GhostBrewConfig {
            defaults: DefaultConfig {
                gaming_mode: Some(true),
                ..Default::default()
            },
            profiles_dir: Some(PathBuf::from("/etc/ghostbrew/profiles")),
            ..Default::default()
        };
        let schema = toml::Value::try_from(&schema_src)
            .expect("config should serialize")
            .as_table()
            .expect("config serializes to a table")
            .clone();

        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/config");
        let mut checked = 0;

        for entry in fs::read_dir(&dir).expect("examples/config should exist") {
            let path = entry.expect("readable dir entry").path();
            if path.extension().is_none_or(|e| e != "toml") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let raw = fs::read_to_string(&path).expect("readable example");
            let parsed: toml::Table =
                toml::from_str(&raw).unwrap_or_else(|e| panic!("{}: invalid TOML: {}", name, e));

            for (key, value) in &parsed {
                let schema_value = schema
                    .get(key)
                    .unwrap_or_else(|| panic!("{}: unknown top-level key '{}'", name, key));
                if let (Some(section), Some(schema_section)) =
                    (value.as_table(), schema_value.as_table())
                {
                    for inner in section.keys() {
                        assert!(
                            schema_section.contains_key(inner),
                            "{}: [{}] sets '{}', which GhostBrewConfig does not read",
                            name,
                            key,
                            inner
                        );
                    }
                }
            }

            // Types must line up as well, not just key names.
            toml::from_str::<GhostBrewConfig>(&raw)
                .unwrap_or_else(|e| panic!("{}: does not deserialize: {}", name, e));
            checked += 1;
        }

        assert!(checked > 0, "no example configs were checked");
    }

    #[test]
    fn test_resolve_precedence() {
        let toml_str = r#"
[defaults]
burst_threshold_ns = 1000000
slice_ns = 4000000
stats_interval = 5

[intel]
ecore_offload = "aggressive"
"#;
        let config: GhostBrewConfig = toml::from_str(toml_str).unwrap();

        // No CLI flags: the config file wins over the built-in defaults.
        let from_config = config.resolve(&CliOverrides::default());
        assert_eq!(from_config.burst_threshold_ns, 1_000_000);
        assert_eq!(from_config.slice_ns, 4_000_000);
        assert_eq!(from_config.stats_interval, 5);
        assert_eq!(from_config.ecore_offload, EcoreOffload::Aggressive);

        // CLI flags win over the config file.
        let from_cli = config.resolve(&CliOverrides {
            burst_threshold_ns: Some(2_500_000),
            slice_ns: Some(1_000_000),
            stats_interval: Some(1),
            ecore_offload: Some("disabled".to_string()),
        });
        assert_eq!(from_cli.burst_threshold_ns, 2_500_000);
        assert_eq!(from_cli.slice_ns, 1_000_000);
        assert_eq!(from_cli.stats_interval, 1);
        assert_eq!(from_cli.ecore_offload, EcoreOffload::Disabled);

        // Built-in defaults apply when neither source specifies anything.
        let bare = GhostBrewConfig::default().resolve(&CliOverrides::default());
        assert_eq!(bare.burst_threshold_ns, 2_000_000);
        assert_eq!(bare.slice_ns, 3_000_000);
        assert_eq!(bare.stats_interval, 2);
        assert_eq!(bare.ecore_offload, EcoreOffload::Conservative);
        assert_eq!(bare.gaming_mode, None);
    }

    #[test]
    fn test_resolve_bad_ecore_mode_falls_back() {
        let mut config = GhostBrewConfig::default();
        config.intel.ecore_offload = "nonsense".to_string();
        assert_eq!(
            config.resolve(&CliOverrides::default()).ecore_offload,
            EcoreOffload::Conservative
        );

        // A bad CLI value must not silently fall through to the config value.
        config.intel.ecore_offload = "aggressive".to_string();
        let resolved = config.resolve(&CliOverrides {
            ecore_offload: Some("bogus".to_string()),
            ..Default::default()
        });
        assert_eq!(resolved.ecore_offload, EcoreOffload::Conservative);
    }

    #[test]
    fn test_resolve_passes_through_platform_flags() {
        let toml_str = r#"
[defaults]
gaming_mode = true

[amd]
prefer_vcache = false
prefcore_enabled = false

[intel]
prefer_pcores = false
"#;
        let config: GhostBrewConfig = toml::from_str(toml_str).unwrap();
        let resolved = config.resolve(&CliOverrides::default());
        assert_eq!(resolved.gaming_mode, Some(true));
        assert!(!resolved.prefer_vcache);
        assert!(!resolved.prefcore_enabled);
        assert!(!resolved.prefer_pcores);
    }

    #[test]
    fn test_amd_intel_sections_parse() {
        let toml_str = r#"
[amd]
prefer_vcache = false
prefcore_enabled = false

[intel]
prefer_pcores = false
"#;
        let config: GhostBrewConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.amd.prefer_vcache);
        assert!(!config.amd.prefcore_enabled);
        assert!(!config.intel.prefer_pcores);
        // Unspecified keys still fall back to their defaults.
        assert_eq!(config.amd.vcache_switching, "follow_ghost_vcache");
        assert_eq!(config.intel.ecore_offload, "conservative");
    }
}
