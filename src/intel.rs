// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Intel Hybrid (P-core/E-core) Detection
//
// Supports Intel 12th, 13th, and 14th generation processors with
// heterogeneous core architectures (Performance + Efficiency cores).
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::{Context, Result};
use log::debug;
use std::fs;

/// Intel hybrid processor information
#[derive(Debug, Clone, Default)]
pub struct IntelHybridInfo {
    /// Whether this is an Intel hybrid processor
    pub is_hybrid: bool,
    /// Processor generation (12, 13, or 14)
    pub generation: u32,
    /// CPUs that are Performance cores
    pub pcore_cpus: Vec<u32>,
    /// CPUs that are Efficiency cores
    pub ecore_cpus: Vec<u32>,
    /// Turbo rankings from HWP (higher = better boost capability)
    pub turbo_rankings: Vec<u32>,
}

/// Known Intel hybrid processor models (12th, 13th, 14th gen)
const INTEL_HYBRID_PATTERNS: &[(&str, u32)] = &[
    // 14th Gen (Raptor Lake Refresh)
    ("14900", 14),
    ("14700", 14),
    ("14600", 14),
    ("14500", 14),
    ("14400", 14),
    // 13th Gen (Raptor Lake)
    ("13900", 13),
    ("13700", 13),
    ("13600", 13),
    ("13500", 13),
    ("13400", 13),
    // 12th Gen (Alder Lake)
    ("12900", 12),
    ("12700", 12),
    ("12600", 12),
    ("12500", 12),
    ("12400", 12),
];

/// P-core capacity threshold (P-cores report 1024, E-cores ~768)
const PCORE_CAPACITY_THRESHOLD: u32 = 900;

/// Check if the model name indicates an Intel hybrid processor
pub fn is_intel_hybrid_model(model_name: &str) -> Option<u32> {
    if !model_name.contains("Intel") {
        return None;
    }

    for (pattern, generation) in INTEL_HYBRID_PATTERNS {
        if model_name.contains(pattern) {
            return Some(*generation);
        }
    }

    None
}

/// Detect Intel hybrid processor topology
pub fn detect_intel_hybrid(nr_cpus: u32, model_name: &str) -> Result<IntelHybridInfo> {
    let generation = match is_intel_hybrid_model(model_name) {
        Some(g) => g,
        None => {
            debug!("Not an Intel hybrid processor");
            return Ok(IntelHybridInfo::default());
        }
    };

    debug!(
        "Detected Intel {}th gen hybrid processor: {}",
        generation, model_name
    );

    let mut pcore_cpus = Vec::new();
    let mut ecore_cpus = Vec::new();
    let mut turbo_rankings = vec![0u32; nr_cpus as usize];

    // Detect P-core vs E-core using cpu_capacity sysfs
    // P-cores: capacity 1024 (max), E-cores: ~768
    for cpu in 0..nr_cpus {
        let capacity = read_cpu_capacity(cpu).unwrap_or(1024);

        if capacity >= PCORE_CAPACITY_THRESHOLD {
            pcore_cpus.push(cpu);
            // P-cores get higher turbo ranking
            turbo_rankings[cpu as usize] = capacity;
        } else {
            ecore_cpus.push(cpu);
            // E-cores get lower turbo ranking
            turbo_rankings[cpu as usize] = capacity;
        }
    }

    // Try to refine with base frequency if capacity isn't available
    if pcore_cpus.is_empty() && ecore_cpus.is_empty() {
        debug!("cpu_capacity not available, falling back to frequency detection");
        detect_by_frequency(
            nr_cpus,
            &mut pcore_cpus,
            &mut ecore_cpus,
            &mut turbo_rankings,
        )?;
    }

    debug!(
        "Intel hybrid: {} P-cores, {} E-cores",
        pcore_cpus.len(),
        ecore_cpus.len()
    );
    debug!("P-cores: {:?}", pcore_cpus);
    debug!("E-cores: {:?}", ecore_cpus);

    Ok(IntelHybridInfo {
        is_hybrid: true,
        generation,
        pcore_cpus,
        ecore_cpus,
        turbo_rankings,
    })
}

/// Read CPU capacity from sysfs
fn read_cpu_capacity(cpu: u32) -> Result<u32> {
    let path = format!("/sys/devices/system/cpu/cpu{}/cpu_capacity", cpu);
    let content = fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path))?;
    content
        .trim()
        .parse()
        .with_context(|| format!("Failed to parse {}", path))
}

/// Fallback detection using base frequency
fn detect_by_frequency(
    nr_cpus: u32,
    pcore_cpus: &mut Vec<u32>,
    ecore_cpus: &mut Vec<u32>,
    turbo_rankings: &mut [u32],
) -> Result<()> {
    let mut frequencies: Vec<(u32, u32)> = Vec::new();

    for cpu in 0..nr_cpus {
        let freq = read_base_frequency(cpu).unwrap_or(0);
        frequencies.push((cpu, freq));
    }

    if frequencies.is_empty() {
        return Ok(());
    }

    // Find the max frequency to determine P-core threshold
    let max_freq = frequencies.iter().map(|(_, f)| *f).max().unwrap_or(0);
    // P-cores typically have base freq >= 80% of max
    let pcore_threshold = max_freq * 80 / 100;

    for (cpu, freq) in frequencies {
        if freq >= pcore_threshold {
            pcore_cpus.push(cpu);
            turbo_rankings[cpu as usize] = freq / 1000; // Normalize to MHz
        } else {
            ecore_cpus.push(cpu);
            turbo_rankings[cpu as usize] = freq / 1000;
        }
    }

    Ok(())
}

/// Read base frequency from cpufreq
fn read_base_frequency(cpu: u32) -> Result<u32> {
    // Try base_frequency first (preferred)
    let base_path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/base_frequency", cpu);
    if let Some(freq) = fs::read_to_string(&base_path)
        .ok()
        .and_then(|c| c.trim().parse::<u32>().ok())
    {
        return Ok(freq);
    }

    // Fall back to cpuinfo_max_freq
    let max_path = format!(
        "/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_max_freq",
        cpu
    );
    let content =
        fs::read_to_string(&max_path).with_context(|| format!("Failed to read {}", max_path))?;
    content
        .trim()
        .parse()
        .with_context(|| format!("Failed to parse {}", max_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel_hybrid_detection() {
        // 14th Gen
        assert_eq!(is_intel_hybrid_model("Intel Core i9-14900K"), Some(14));
        assert_eq!(is_intel_hybrid_model("Intel Core i7-14700K"), Some(14));

        // 13th Gen
        assert_eq!(is_intel_hybrid_model("Intel Core i9-13900K"), Some(13));
        assert_eq!(is_intel_hybrid_model("Intel Core i5-13600K"), Some(13));

        // 12th Gen
        assert_eq!(is_intel_hybrid_model("Intel Core i9-12900K"), Some(12));

        // Non-hybrid Intel
        assert_eq!(is_intel_hybrid_model("Intel Core i7-10700K"), None);
        assert_eq!(is_intel_hybrid_model("Intel Core i5-10400"), None);

        // AMD (not Intel)
        assert_eq!(is_intel_hybrid_model("AMD Ryzen 9 7950X3D"), None);
        assert_eq!(is_intel_hybrid_model("AMD Ryzen 7 7800X3D"), None);
    }

    #[test]
    fn test_pcore_threshold() {
        // P-cores should be >= 900 capacity
        assert!(1024 >= PCORE_CAPACITY_THRESHOLD);
        // E-cores are typically ~768
        assert!(768 < PCORE_CAPACITY_THRESHOLD);
    }
}
