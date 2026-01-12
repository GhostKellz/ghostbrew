// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Runtime Control Interface
//
// Provides a simple file-based interface for runtime tuning.
// Users can write commands to /run/ghostbrew/control to update tunables.
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::fs;
use std::path::PathBuf;

/// Control file commands
#[derive(Debug, Clone)]
pub enum ControlCommand {
    /// Set burst threshold in nanoseconds
    SetBurstThreshold(u64),
    /// Set time slice in nanoseconds
    SetSlice(u64),
    /// Enable gaming mode
    GamingMode(bool),
    /// Enable work mode
    WorkMode(bool),
}

/// Control interface manager
pub struct ControlInterface {
    control_dir: PathBuf,
    control_file: PathBuf,
    last_modified: Option<std::time::SystemTime>,
}

impl ControlInterface {
    /// Create a new control interface
    pub fn new() -> Self {
        let control_dir = PathBuf::from("/run/ghostbrew");
        let control_file = control_dir.join("control");

        Self {
            control_dir,
            control_file,
            last_modified: None,
        }
    }

    /// Initialize the control interface
    pub fn init(&mut self) -> Result<()> {
        // Create control directory if it doesn't exist
        if !self.control_dir.exists() {
            fs::create_dir_all(&self.control_dir).context("Failed to create control directory")?;
        }

        // Create control file with usage instructions
        let usage = r#"# GhostBrew Runtime Control
# Write commands to this file to update scheduler tunables at runtime.
#
# Commands:
#   burst_threshold_ns=<value>  - Set burst threshold (nanoseconds)
#   slice_ns=<value>            - Set time slice (nanoseconds)
#   gaming_mode=<true|false>    - Enable/disable gaming mode
#   work_mode=<true|false>      - Enable/disable work mode
#
# Example:
#   echo "burst_threshold_ns=1500000" > /run/ghostbrew/control
#   echo "gaming_mode=true" >> /run/ghostbrew/control
#
# Multiple commands can be on separate lines.
"#;
        fs::write(&self.control_file, usage).context("Failed to create control file")?;

        // Set permissions (world-writable for easy access)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o666);
            fs::set_permissions(&self.control_file, perms).ok();
        }

        info!("Control interface: {:?}", self.control_file);
        Ok(())
    }

    /// Check for and parse control commands
    pub fn poll_commands(&mut self) -> Vec<ControlCommand> {
        let mut commands = Vec::new();

        // Check if file was modified
        let metadata = match fs::metadata(&self.control_file) {
            Ok(m) => m,
            Err(_) => return commands,
        };

        let modified = metadata.modified().ok();
        if modified == self.last_modified {
            return commands; // No changes
        }
        self.last_modified = modified;

        // Read and parse commands
        let content = match fs::read_to_string(&self.control_file) {
            Ok(c) => c,
            Err(_) => return commands,
        };

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(cmd) = Self::parse_command(line) {
                debug!("Control command: {:?}", cmd);
                commands.push(cmd);
            }
        }

        commands
    }

    /// Parse a single command line
    fn parse_command(line: &str) -> Option<ControlCommand> {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() != 2 {
            return None;
        }

        let key = parts[0].trim().to_lowercase();
        let value = parts[1].trim();

        match key.as_str() {
            "burst_threshold_ns" => value
                .parse::<u64>()
                .ok()
                .map(ControlCommand::SetBurstThreshold),
            "slice_ns" => value.parse::<u64>().ok().map(ControlCommand::SetSlice),
            "gaming_mode" => Self::parse_bool(value).map(ControlCommand::GamingMode),
            "work_mode" => Self::parse_bool(value).map(ControlCommand::WorkMode),
            _ => {
                warn!("Unknown control command: {}", key);
                None
            }
        }
    }

    /// Parse boolean value
    fn parse_bool(s: &str) -> Option<bool> {
        match s.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        }
    }

    /// Get the control file path
    #[allow(dead_code)]
    pub fn control_path(&self) -> &PathBuf {
        &self.control_file
    }
}

impl Default for ControlInterface {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        assert!(matches!(
            ControlInterface::parse_command("burst_threshold_ns=1500000"),
            Some(ControlCommand::SetBurstThreshold(1500000))
        ));

        assert!(matches!(
            ControlInterface::parse_command("gaming_mode=true"),
            Some(ControlCommand::GamingMode(true))
        ));

        assert!(matches!(
            ControlInterface::parse_command("work_mode=false"),
            Some(ControlCommand::WorkMode(false))
        ));

        assert!(ControlInterface::parse_command("# comment").is_none());
        assert!(ControlInterface::parse_command("invalid").is_none());
    }
}
