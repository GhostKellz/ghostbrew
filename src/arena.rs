// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - BPF Arena Support (Placeholder)
//
// BPF Arena provides shared memory regions between BPF programs and userspace
// for complex data structures like per-task history buffers.
//
// Requirements:
// - Kernel 6.18+ with BPF_MAP_TYPE_ARENA support
// - libbpf 1.5+
// - scx_arena crate (when available)
//
// This module provides detection and future scaffolding.
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

#![allow(dead_code)] // Scaffolding for future Arena implementation

use log::debug;
use std::fs;
use std::path::Path;

/// Check if BPF Arena is supported by the running kernel
pub fn is_arena_supported() -> bool {
    // BPF Arena requires BPF_MAP_TYPE_ARENA which was added in kernel 6.18
    // We can detect this by checking for the BTF type or kernel version

    // Method 1: Check kernel version
    if let Ok(release) = fs::read_to_string("/proc/sys/kernel/osrelease")
        && let Some(version) = parse_kernel_version(release.trim())
        // Arena requires 6.18+
        && (version.0 > 6 || (version.0 == 6 && version.1 >= 18))
    {
        debug!("Kernel version {} supports BPF Arena", release.trim());
        return true;
    }

    // Method 2: Check for arena-related BTF types in vmlinux
    if Path::new("/sys/kernel/btf/vmlinux").exists() {
        // Could probe BTF for BPF_MAP_TYPE_ARENA support
        // For now, rely on kernel version check
    }

    debug!("BPF Arena not supported (requires kernel 6.18+)");
    false
}

/// Parse kernel version string like "6.18.0-cachyos" into (major, minor, patch)
fn parse_kernel_version(release: &str) -> Option<(u32, u32, u32)> {
    let version_part = release.split('-').next()?;
    let mut parts = version_part.split('.');

    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    Some((major, minor, patch))
}

/// Arena feature status information
#[derive(Debug, Clone)]
pub struct ArenaStatus {
    /// Whether Arena is supported by the kernel
    pub supported: bool,
    /// Whether Arena is enabled in this build (feature-gated)
    pub enabled: bool,
    /// Reason if not available
    pub reason: String,
}

impl ArenaStatus {
    /// Check Arena availability
    pub fn check() -> Self {
        let supported = is_arena_supported();

        Self {
            supported,
            enabled: false, // Not yet implemented
            reason: if supported {
                "Arena support available but not yet implemented in this build".to_string()
            } else {
                "Requires kernel 6.18+ with BPF_MAP_TYPE_ARENA".to_string()
            },
        }
    }
}

impl Default for ArenaStatus {
    fn default() -> Self {
        Self::check()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kernel_version() {
        assert_eq!(parse_kernel_version("6.18.0-cachyos"), Some((6, 18, 0)));
        assert_eq!(parse_kernel_version("6.12.5"), Some((6, 12, 5)));
        assert_eq!(parse_kernel_version("7.0.0"), Some((7, 0, 0)));
    }

    #[test]
    fn test_arena_status() {
        let status = ArenaStatus::check();
        // Just verify it runs without panicking
        assert!(!status.reason.is_empty());
    }
}
