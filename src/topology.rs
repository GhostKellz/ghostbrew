// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - CPU Topology Detection for AMD Zen and Intel Hybrid processors
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use crate::intel::{self, IntelHybridInfo};
use anyhow::{Context, Result};
use log::debug;
use std::fs;
use std::path::Path;

/// CPU architecture type
#[derive(Debug, Clone, PartialEq)]
pub enum CpuArch {
    /// AMD Zen architecture (optionally with X3D V-Cache)
    AmdZen {
        is_x3d: bool,
        /// Zen generation: 4 = Zen 4 (family 25), 5 = Zen 5 (family 26)
        generation: u32,
    },
    /// Intel hybrid architecture (12th/13th/14th gen with P-cores and E-cores)
    IntelHybrid { generation: u32 },
    /// Generic x86-64 (no special optimizations)
    Generic,
}

/// CPU topology information
#[allow(dead_code)]
pub struct CpuTopology {
    pub nr_cpus: u32,
    pub nr_ccds: u32,
    pub vcache_ccd: Option<u32>,
    pub cpu_to_ccd: Vec<u32>,
    pub cpu_to_ccx: Vec<u32>,
    pub cpu_to_node: Vec<u32>,
    pub cpu_to_sibling: Vec<i32>, // SMT sibling CPU (-1 if none)
    pub smt_enabled: bool,
    pub is_x3d: bool,
    pub model_name: String,
    // Intel hybrid support
    pub arch: CpuArch,
    pub is_intel_hybrid: bool,
    pub pcore_cpus: Vec<u32>,
    pub ecore_cpus: Vec<u32>,
    pub turbo_rankings: Vec<u32>,
    // Zen 5 specific
    /// AMD Zen generation (4 = Zen 4, 5 = Zen 5), None for non-AMD
    pub zen_generation: Option<u32>,
    /// Non-V-Cache CCD for frequency-bound tasks (Zen 5 X3D asymmetric boost)
    pub freq_ccd: Option<u32>,
    /// L3 cache size in MB for V-Cache CCD (96MB: 32MB base + 64MB stacked)
    pub vcache_l3_mb: Option<u32>,
    /// Whether this CPU has asymmetric CCD boost (Zen 5 X3D)
    pub asymmetric_ccd_boost: bool,
}

/// Known X3D processor models
const X3D_MODELS: &[&str] = &[
    "7800X3D", "7900X3D", "7950X3D", "9800X3D", "9900X3D", "9950X3D",
];

/// Detect CPU topology
pub fn detect_topology() -> Result<CpuTopology> {
    let nr_cpus = detect_nr_cpus()?;
    let model_name = detect_model_name()?;
    let is_x3d = is_x3d_processor(&model_name);
    let cpu_family = detect_cpu_family();

    debug!("Detected CPU: {}", model_name);
    debug!("Is X3D: {}, CPU family: {}", is_x3d, cpu_family);

    // Detect Intel hybrid architecture
    let intel_info = intel::detect_intel_hybrid(nr_cpus, &model_name)?;
    let is_intel_hybrid = intel_info.is_hybrid;

    // Determine Zen generation from CPU family
    // Family 25 = Zen 3/4, Family 26 = Zen 5
    let zen_generation = if model_name.contains("AMD") {
        match cpu_family {
            26 => Some(5), // Zen 5
            25 => Some(4), // Zen 4 (and Zen 3)
            23 => Some(2), // Zen 2
            _ => None,
        }
    } else {
        None
    };

    // Determine architecture type
    let arch = if is_intel_hybrid {
        debug!(
            "Intel {}th gen hybrid: {} P-cores, {} E-cores",
            intel_info.generation,
            intel_info.pcore_cpus.len(),
            intel_info.ecore_cpus.len()
        );
        CpuArch::IntelHybrid {
            generation: intel_info.generation,
        }
    } else if model_name.contains("AMD") {
        let zen_gen = zen_generation.unwrap_or(4);
        debug!("AMD Zen {} architecture, X3D: {}", zen_gen, is_x3d);
        CpuArch::AmdZen {
            is_x3d,
            generation: zen_gen,
        }
    } else {
        debug!("Generic x86-64 architecture");
        CpuArch::Generic
    };

    // Detect CCD/CCX mapping from sysfs topology
    // For Intel hybrid, we use cluster_id to group P-cores and E-cores
    let (cpu_to_ccd, cpu_to_ccx, cpu_to_node) = if is_intel_hybrid {
        detect_intel_topology(nr_cpus, &intel_info)?
    } else {
        detect_cpu_topology(nr_cpus)?
    };

    // Count unique CCDs (or clusters for Intel)
    let nr_ccds = cpu_to_ccd.iter().max().map(|&m| m + 1).unwrap_or(1);

    // Determine V-Cache CCD for X3D processors
    let vcache_ccd = if is_x3d {
        detect_vcache_ccd(&model_name, nr_ccds)
    } else {
        None
    };

    // Detect SMT siblings
    let (cpu_to_sibling, smt_enabled) = detect_smt_siblings(nr_cpus)?;
    debug!("SMT enabled: {}", smt_enabled);

    // Zen 5 X3D specific: asymmetric CCD boost
    // Non-V-Cache CCD can boost higher, use for frequency-bound tasks
    let is_zen5_x3d = is_x3d && zen_generation == Some(5);
    let asymmetric_ccd_boost = is_zen5_x3d && nr_ccds >= 2;

    // Determine frequency CCD (non-V-Cache CCD for Zen 5 X3D)
    let freq_ccd = if asymmetric_ccd_boost && let Some(vc) = vcache_ccd {
        // Use the other CCD for frequency-bound tasks
        Some(if vc == 0 { 1 } else { 0 })
    } else {
        None
    };

    // V-Cache L3 size per CCD: All X3D V-Cache CCDs have 96MB (32MB base + 64MB stacked)
    // - Single-CCD (7800X3D, 9800X3D): 96MB total
    // - Multi-CCD (7900X3D, 7950X3D, 9900X3D, 9950X3D): 96MB on V-Cache CCD, 32MB on regular CCD
    let vcache_l3_mb = if is_x3d { Some(96) } else { None };

    if asymmetric_ccd_boost {
        debug!(
            "Zen 5 X3D asymmetric boost: V-Cache CCD {:?}, Freq CCD {:?}, L3 {:?}MB",
            vcache_ccd, freq_ccd, vcache_l3_mb
        );
    } else if is_x3d && nr_ccds == 1 {
        debug!(
            "Single-CCD X3D: all {} cores have V-Cache, L3 {:?}MB",
            nr_cpus, vcache_l3_mb
        );
    }

    Ok(CpuTopology {
        nr_cpus,
        nr_ccds,
        vcache_ccd,
        cpu_to_ccd,
        cpu_to_ccx,
        cpu_to_node,
        cpu_to_sibling,
        smt_enabled,
        is_x3d,
        model_name,
        arch,
        is_intel_hybrid,
        pcore_cpus: intel_info.pcore_cpus,
        ecore_cpus: intel_info.ecore_cpus,
        turbo_rankings: intel_info.turbo_rankings,
        zen_generation,
        freq_ccd,
        vcache_l3_mb,
        asymmetric_ccd_boost,
    })
}

/// Detect CPU family from /proc/cpuinfo
/// Family 25 = Zen 3/4, Family 26 = Zen 5
fn detect_cpu_family() -> u32 {
    let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") else {
        return 0;
    };
    for line in cpuinfo.lines() {
        if line.starts_with("cpu family")
            && let Some((_, value)) = line.split_once(':')
            && let Ok(family) = value.trim().parse::<u32>()
        {
            return family;
        }
    }
    0 // Unknown
}

/// Get number of online CPUs
fn detect_nr_cpus() -> Result<u32> {
    let online = fs::read_to_string("/sys/devices/system/cpu/online")
        .context("Failed to read online CPUs")?;

    // Parse CPU range like "0-31"
    let online = online.trim();
    if let Some((_, end)) = online.split_once('-') {
        let end: u32 = end.parse().context("Failed to parse CPU count")?;
        return Ok(end + 1);
    }

    // Single CPU or comma-separated
    Ok(online.split(',').count() as u32)
}

/// Get CPU model name
fn detect_model_name() -> Result<String> {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").context("Failed to read /proc/cpuinfo")?;

    for line in cpuinfo.lines() {
        if line.starts_with("model name")
            && let Some((_, name)) = line.split_once(':')
        {
            return Ok(name.trim().to_string());
        }
    }

    Ok("Unknown".to_string())
}

/// Check if this is an X3D processor
fn is_x3d_processor(model_name: &str) -> bool {
    X3D_MODELS.iter().any(|&model| model_name.contains(model))
}

/// Detect which CCD has V-Cache
fn detect_vcache_ccd(model_name: &str, nr_ccds: u32) -> Option<u32> {
    // For current X3D processors:
    // - 7800X3D: Single CCD, all cores have V-Cache
    // - 7900X3D, 7950X3D: CCD0 has V-Cache
    // - 9900X3D, 9950X3D: CCD0 has V-Cache (assumed same as Zen4)

    if model_name.contains("7800X3D") || model_name.contains("9800X3D") {
        // Single CCD, all V-Cache
        return Some(0);
    }

    if nr_ccds >= 2 {
        // Multi-CCD X3D: CCD0 typically has V-Cache
        return Some(0);
    }

    Some(0) // Default assumption
}

/// Detect Intel hybrid topology (cluster mapping for P-core/E-core grouping)
fn detect_intel_topology(
    nr_cpus: u32,
    intel_info: &IntelHybridInfo,
) -> Result<(Vec<u32>, Vec<u32>, Vec<u32>)> {
    let mut cpu_to_ccd = vec![0u32; nr_cpus as usize];
    let mut cpu_to_ccx = vec![0u32; nr_cpus as usize];
    let mut cpu_to_node = vec![0u32; nr_cpus as usize];

    // For Intel hybrid, use cluster_id to group CPUs
    // P-cores and E-cores are typically in different clusters
    for cpu in 0..nr_cpus {
        let base = format!("/sys/devices/system/cpu/cpu{}/topology", cpu);

        // Read cluster ID (groups of cores)
        let cluster_id = read_topology_file(&format!("{}/cluster_id", base)).unwrap_or(0);

        // Use cluster as CCD equivalent
        cpu_to_ccd[cpu as usize] = cluster_id;
        cpu_to_ccx[cpu as usize] = cluster_id;

        // NUMA node
        let node = detect_cpu_node(cpu).unwrap_or(0);
        cpu_to_node[cpu as usize] = node;

        let core_type = if intel_info.pcore_cpus.contains(&cpu) {
            "P-core"
        } else {
            "E-core"
        };

        debug!(
            "CPU {}: {} cluster={}, node={}",
            cpu, core_type, cluster_id, node
        );
    }

    Ok((cpu_to_ccd, cpu_to_ccx, cpu_to_node))
}

/// Detect per-CPU topology (CCD, CCX, NUMA node)
fn detect_cpu_topology(nr_cpus: u32) -> Result<(Vec<u32>, Vec<u32>, Vec<u32>)> {
    let mut cpu_to_ccd = vec![0u32; nr_cpus as usize];
    let mut cpu_to_ccx = vec![0u32; nr_cpus as usize];
    let mut cpu_to_node = vec![0u32; nr_cpus as usize];

    for cpu in 0..nr_cpus {
        let base = format!("/sys/devices/system/cpu/cpu{}/topology", cpu);

        // Read physical package ID (socket/die)
        let _die_id = read_topology_file(&format!("{}/die_id", base))
            .or_else(|_| read_topology_file(&format!("{}/physical_package_id", base)))
            .unwrap_or(0);

        // Read cluster ID (CCX on Zen)
        let cluster_id = read_topology_file(&format!("{}/cluster_id", base)).unwrap_or(0);

        // For AMD Zen, we can approximate CCD from core_id ranges
        // Typically: CCD0 = cores 0-7, CCD1 = cores 8-15 (for 16-core)
        let core_id = read_topology_file(&format!("{}/core_id", base)).unwrap_or(cpu);

        // Heuristic: cores 0-7 = CCD0, 8-15 = CCD1, etc.
        // This works for most Zen4/Zen5 layouts
        let ccd = core_id / 8;

        cpu_to_ccd[cpu as usize] = ccd;
        cpu_to_ccx[cpu as usize] = cluster_id;

        // NUMA node
        let node = detect_cpu_node(cpu).unwrap_or(0);
        cpu_to_node[cpu as usize] = node;

        debug!(
            "CPU {}: CCD={}, CCX={}, Node={}",
            cpu, ccd, cluster_id, node
        );
    }

    Ok((cpu_to_ccd, cpu_to_ccx, cpu_to_node))
}

/// Read a topology file and parse as u32
fn read_topology_file(path: &str) -> Result<u32> {
    let content = fs::read_to_string(path).with_context(|| format!("Failed to read {}", path))?;
    content
        .trim()
        .parse()
        .with_context(|| format!("Failed to parse {}", path))
}

/// Detect SMT siblings for all CPUs
fn detect_smt_siblings(nr_cpus: u32) -> Result<(Vec<i32>, bool)> {
    let mut cpu_to_sibling = vec![-1i32; nr_cpus as usize];
    let mut smt_enabled = false;

    for cpu in 0..nr_cpus {
        let path = format!(
            "/sys/devices/system/cpu/cpu{}/topology/thread_siblings_list",
            cpu
        );

        if let Ok(siblings_str) = fs::read_to_string(&path) {
            let siblings: Vec<u32> = parse_cpu_list(&siblings_str);

            // Find the sibling that isn't this CPU
            for &sibling in &siblings {
                if sibling != cpu && sibling < nr_cpus {
                    cpu_to_sibling[cpu as usize] = sibling as i32;
                    smt_enabled = true;
                    break;
                }
            }
        }

        debug!(
            "CPU {}: SMT sibling = {}",
            cpu, cpu_to_sibling[cpu as usize]
        );
    }

    Ok((cpu_to_sibling, smt_enabled))
}

/// Parse a CPU list string like "0,16" or "0-3,16-19" into a Vec of CPU numbers
fn parse_cpu_list(list: &str) -> Vec<u32> {
    let mut cpus = Vec::new();

    for part in list.trim().split(',') {
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(s), Ok(e)) = (start.parse::<u32>(), end.parse::<u32>()) {
                for cpu in s..=e {
                    cpus.push(cpu);
                }
            }
        } else if let Ok(cpu) = part.parse::<u32>() {
            cpus.push(cpu);
        }
    }

    cpus
}

/// Detect NUMA node for a CPU
fn detect_cpu_node(cpu: u32) -> Result<u32> {
    let node_path = format!("/sys/devices/system/cpu/cpu{}/node0", cpu);
    if Path::new(&node_path).exists() {
        return Ok(0);
    }

    // Check other nodes
    for node in 0..8 {
        let path = format!("/sys/devices/system/node/node{}/cpulist", node);
        if let Ok(cpulist) = fs::read_to_string(&path)
            && cpu_in_list(cpu, &cpulist)
        {
            return Ok(node);
        }
    }

    Ok(0)
}

/// Check if CPU is in a cpulist string like "0-7,16-23"
fn cpu_in_list(cpu: u32, list: &str) -> bool {
    for range in list.trim().split(',') {
        if let Some((start, end)) = range.split_once('-') {
            if let (Ok(s), Ok(e)) = (start.parse::<u32>(), end.parse::<u32>())
                && cpu >= s
                && cpu <= e
            {
                return true;
            }
        } else if let Ok(single) = range.parse::<u32>()
            && cpu == single
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Zen 5 X3D models (family 26)
    const ZEN5_X3D_MODELS: &[&str] = &["9800X3D", "9900X3D", "9950X3D"];

    #[test]
    fn test_is_x3d() {
        assert!(is_x3d_processor("AMD Ryzen 9 7950X3D"));
        assert!(is_x3d_processor("AMD Ryzen 7 7800X3D"));
        assert!(is_x3d_processor("AMD Ryzen 9 9950X3D"));
        assert!(!is_x3d_processor("AMD Ryzen 9 7950X"));
        assert!(!is_x3d_processor("Intel Core i9-14900K"));
    }

    #[test]
    fn test_cpu_in_list() {
        assert!(cpu_in_list(5, "0-7"));
        assert!(cpu_in_list(0, "0-7"));
        assert!(cpu_in_list(7, "0-7"));
        assert!(!cpu_in_list(8, "0-7"));
        assert!(cpu_in_list(16, "0-7,16-23"));
        assert!(cpu_in_list(5, "5"));
    }

    #[test]
    fn test_cpu_arch_detection() {
        // AMD Zen 4 X3D should be detected
        let arch = if is_x3d_processor("AMD Ryzen 9 7950X3D") {
            CpuArch::AmdZen {
                is_x3d: true,
                generation: 4,
            }
        } else {
            CpuArch::Generic
        };
        assert_eq!(
            arch,
            CpuArch::AmdZen {
                is_x3d: true,
                generation: 4
            }
        );

        // AMD Zen 5 X3D should be detected
        let arch_zen5 = if is_x3d_processor("AMD Ryzen 9 9950X3D") {
            CpuArch::AmdZen {
                is_x3d: true,
                generation: 5,
            }
        } else {
            CpuArch::Generic
        };
        assert_eq!(
            arch_zen5,
            CpuArch::AmdZen {
                is_x3d: true,
                generation: 5
            }
        );

        // Intel hybrid should be detected
        if let Some(g) = intel::is_intel_hybrid_model("Intel Core i9-14900K") {
            let arch = CpuArch::IntelHybrid { generation: g };
            assert_eq!(arch, CpuArch::IntelHybrid { generation: 14 });
        }
    }

    #[test]
    fn test_zen5_x3d_detection() {
        // Zen 5 X3D models
        assert!(
            ZEN5_X3D_MODELS
                .iter()
                .any(|m| "AMD Ryzen 9 9950X3D".contains(m))
        );
        assert!(
            ZEN5_X3D_MODELS
                .iter()
                .any(|m| "AMD Ryzen 9 9900X3D".contains(m))
        );
        assert!(
            ZEN5_X3D_MODELS
                .iter()
                .any(|m| "AMD Ryzen 7 9800X3D".contains(m))
        );

        // Zen 4 X3D models should not match Zen 5
        assert!(
            !ZEN5_X3D_MODELS
                .iter()
                .any(|m| "AMD Ryzen 9 7950X3D".contains(m))
        );
    }
}
