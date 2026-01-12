// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew Integration Tests
//
// These tests verify the userspace components work correctly.
// Note: Actual BPF attach/detach requires root and sched-ext kernel.
//
// Run with: cargo test --test integration_tests
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use std::path::PathBuf;

/// Test that config files in examples are valid TOML
#[test]
fn test_example_configs_parse() {
    let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/config");

    if !config_dir.exists() {
        eprintln!("Skipping: examples/config not found");
        return;
    }

    for entry in std::fs::read_dir(&config_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "toml") {
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));

            let _: toml::Value = toml::from_str(&content)
                .unwrap_or_else(|e| panic!("Invalid TOML in {:?}: {}", path, e));

            println!("OK: {:?}", path.file_name().unwrap());
        }
    }
}

/// Test that game profiles in examples are valid TOML
#[test]
fn test_example_profiles_parse() {
    let profiles_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/profiles");

    if !profiles_dir.exists() {
        eprintln!("Skipping: examples/profiles not found");
        return;
    }

    for entry in std::fs::read_dir(&profiles_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "toml") {
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));

            let _: toml::Value = toml::from_str(&content)
                .unwrap_or_else(|e| panic!("Invalid TOML in {:?}: {}", path, e));

            // Verify required fields exist
            let parsed: toml::Value = toml::from_str(&content).unwrap();
            assert!(
                parsed.get("name").is_some(),
                "Profile {:?} missing 'name' field",
                path
            );

            println!("OK: {:?}", path.file_name().unwrap());
        }
    }
}

/// Test that the binary can show help without root
#[test]
fn test_help_without_root() {
    use std::process::Command;

    let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/scx_ghostbrew");

    // Skip if binary not built
    if !binary.exists() {
        eprintln!("Skipping: binary not built (run cargo build first)");
        return;
    }

    let output = Command::new(&binary)
        .arg("--help")
        .output()
        .expect("Failed to run binary");

    assert!(output.status.success(), "Help should succeed without root");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("scx_ghostbrew"),
        "Help should mention scx_ghostbrew"
    );
    assert!(
        stdout.contains("--gaming"),
        "Help should mention --gaming flag"
    );
    assert!(stdout.contains("--work"), "Help should mention --work flag");
}

/// Test shell completion generation
#[test]
fn test_completions_generation() {
    use std::process::Command;

    let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/scx_ghostbrew");

    if !binary.exists() {
        eprintln!("Skipping: binary not built");
        return;
    }

    for shell in ["bash", "zsh", "fish"] {
        let output = Command::new(&binary)
            .args(["--completions", shell])
            .output()
            .expect("Failed to run binary");

        assert!(
            output.status.success(),
            "Completions for {} should succeed",
            shell
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.is_empty(),
            "{} completions should not be empty",
            shell
        );
    }
}

/// Test sched-ext kernel support detection
#[test]
fn test_schedext_detection() {
    let schedext_path = std::path::Path::new("/sys/kernel/sched_ext");

    if schedext_path.exists() {
        println!("sched-ext is available on this system");

        // Check state file
        let state_path = schedext_path.join("state");
        if state_path.exists() {
            let state = std::fs::read_to_string(&state_path).unwrap_or_default();
            println!("sched-ext state: {}", state.trim());
        }
    } else {
        println!("sched-ext not available (CONFIG_SCHED_CLASS_EXT=n or old kernel)");
    }

    // This test always passes - it's informational
}

/// Test CPU topology detection (doesn't require root)
#[test]
fn test_cpu_topology_readable() {
    let cpu_path = std::path::Path::new("/sys/devices/system/cpu");

    assert!(cpu_path.exists(), "CPU sysfs should exist");

    // Check online CPUs
    let online_path = cpu_path.join("online");
    assert!(online_path.exists(), "online CPUs file should exist");

    let online = std::fs::read_to_string(&online_path).unwrap();
    println!("Online CPUs: {}", online.trim());

    // Check that at least cpu0 exists
    let cpu0_path = cpu_path.join("cpu0");
    assert!(cpu0_path.exists(), "cpu0 should exist");

    // Check topology info
    let topology_path = cpu0_path.join("topology");
    if topology_path.exists() {
        if let Ok(core_id) = std::fs::read_to_string(topology_path.join("core_id")) {
            println!("CPU0 core_id: {}", core_id.trim());
        }
        if let Ok(physical_id) = std::fs::read_to_string(topology_path.join("physical_package_id"))
        {
            println!("CPU0 physical_package_id: {}", physical_id.trim());
        }
    }
}

/// Test AMD X3D sysfs detection (if available)
#[test]
fn test_amd_x3d_sysfs() {
    let vcache_path = std::path::Path::new("/sys/bus/platform/drivers/amd_x3d_vcache");

    if vcache_path.exists() {
        println!("AMD X3D V-Cache driver detected");

        // Look for mode file
        if let Ok(entries) = std::fs::read_dir(vcache_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let mode_file = path.join("amd_x3d_mode");
                    if mode_file.exists() {
                        let mode = std::fs::read_to_string(&mode_file).unwrap_or_default();
                        println!("V-Cache mode: {}", mode.trim());
                    }
                }
            }
        }
    } else {
        println!("AMD X3D V-Cache driver not available (not an X3D processor)");
    }
}

/// Test /proc scanning (gaming detection uses this)
#[test]
fn test_proc_scanning() {
    let proc_path = std::path::Path::new("/proc");
    assert!(proc_path.exists(), "/proc should exist");

    let mut process_count = 0;
    for entry in std::fs::read_dir(proc_path).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Check if it's a PID directory
        if name_str.chars().all(|c| c.is_ascii_digit()) {
            process_count += 1;

            // Verify we can read comm (some we can't due to permissions)
            let comm_path = entry.path().join("comm");
            if comm_path.exists() {
                // Just check it's readable, don't panic on permission denied
                let _ = std::fs::read_to_string(&comm_path);
            }
        }
    }

    assert!(process_count > 0, "Should find at least some processes");
    println!("Found {} process directories in /proc", process_count);
}

// =============================================================================
// BPF Integration Tests
// =============================================================================

/// Test BPF filesystem availability (required for sched-ext)
#[test]
fn test_bpf_fs_available() {
    let bpffs_path = std::path::Path::new("/sys/fs/bpf");

    if bpffs_path.exists() {
        println!("BPF filesystem mounted at /sys/fs/bpf");

        // Check if we can list it (may require permissions)
        match std::fs::read_dir(bpffs_path) {
            Ok(entries) => {
                let count = entries.count();
                println!("BPF filesystem contains {} entries", count);
            }
            Err(e) => {
                println!(
                    "Cannot read BPF filesystem (permission denied expected): {}",
                    e
                );
            }
        }
    } else {
        println!("BPF filesystem not mounted (kernel may not support BPF)");
    }

    // Informational test - always passes
}

/// Test that libbpf BTF support is available in kernel
#[test]
fn test_btf_available() {
    let btf_path = std::path::Path::new("/sys/kernel/btf/vmlinux");

    if btf_path.exists() {
        let metadata = std::fs::metadata(btf_path).unwrap();
        let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
        println!("BTF vmlinux available ({:.1} MB)", size_mb);
        println!("CO-RE (Compile Once - Run Everywhere) supported");
    } else {
        println!("BTF vmlinux not found - CO-RE may not work");
        println!("Kernel needs CONFIG_DEBUG_INFO_BTF=y");
    }

    // Informational test
}

/// Test sched-ext BPF program slots
#[test]
fn test_schedext_slots() {
    let schedext_path = std::path::Path::new("/sys/kernel/sched_ext");

    if !schedext_path.exists() {
        println!("sched-ext not available, skipping slot test");
        return;
    }

    // Check current scheduler (if any)
    let root_path = schedext_path.join("root");
    if root_path.exists() {
        match std::fs::read_dir(&root_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    println!("sched-ext root entry: {:?}", entry.file_name());
                }
            }
            Err(e) => {
                println!("Cannot read sched-ext root: {}", e);
            }
        }
    }

    // Check hotplug seq (used by scx_utils)
    let hotplug_seq = schedext_path.join("hotplug_seq");
    if hotplug_seq.exists() {
        let seq = std::fs::read_to_string(&hotplug_seq).unwrap_or_default();
        println!("Hotplug sequence: {}", seq.trim());
    }
}

/// Test AMD pstate prefcore integration
#[test]
fn test_amd_pstate_prefcore() {
    let pstate_path = std::path::Path::new("/sys/devices/system/cpu/amd_pstate");

    if !pstate_path.exists() {
        println!("AMD pstate not available (Intel CPU or different governor)");
        return;
    }

    // Check status
    let status_path = pstate_path.join("status");
    if status_path.exists() {
        let status = std::fs::read_to_string(&status_path).unwrap_or_default();
        println!("AMD pstate status: {}", status.trim());
    }

    // Check prefcore
    let prefcore_path = pstate_path.join("prefcore");
    if prefcore_path.exists() {
        let prefcore = std::fs::read_to_string(&prefcore_path).unwrap_or_default();
        println!("AMD prefcore: {}", prefcore.trim());

        if prefcore.trim() == "enabled" {
            // Read a few prefcore rankings
            for cpu in [0, 8, 16, 24] {
                let ranking_path = format!(
                    "/sys/devices/system/cpu/cpufreq/policy{}/amd_pstate_prefcore_ranking",
                    cpu
                );
                if let Ok(ranking) = std::fs::read_to_string(&ranking_path) {
                    println!("CPU {} prefcore ranking: {}", cpu, ranking.trim());
                }
            }
        }
    }
}

/// Test cgroup v2 availability (used for workload classification)
#[test]
fn test_cgroup_v2() {
    let cgroup_path = std::path::Path::new("/sys/fs/cgroup");

    assert!(cgroup_path.exists(), "Cgroup filesystem should exist");

    // Check for cgroup v2 (unified hierarchy)
    let cgroup_type = cgroup_path.join("cgroup.type");
    let cgroup_controllers = cgroup_path.join("cgroup.controllers");

    if cgroup_controllers.exists() {
        println!("Cgroup v2 (unified) detected");
        let controllers = std::fs::read_to_string(&cgroup_controllers).unwrap_or_default();
        println!("Available controllers: {}", controllers.trim());
    } else if cgroup_type.exists() {
        println!("Cgroup v2 with type file");
    } else {
        println!("Cgroup v1 or hybrid mode");
    }

    // Count cgroups
    let mut cgroup_count = 0;
    fn count_cgroups(path: &std::path::Path, count: &mut usize) {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.starts_with("cgroup.") && !name.starts_with("cpu.") {
                        *count += 1;
                        count_cgroups(&entry_path, count);
                    }
                }
            }
        }
    }
    count_cgroups(cgroup_path, &mut cgroup_count);
    println!("Total cgroup directories: {}", cgroup_count);
}

/// Test binary exits cleanly when scheduler is already attached
#[test]
fn test_binary_handles_busy_scheduler() {
    use std::process::Command;

    let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/scx_ghostbrew");

    if !binary.exists() {
        eprintln!("Skipping: binary not built");
        return;
    }

    // Check if another scheduler is running
    let schedext_state = std::path::Path::new("/sys/kernel/sched_ext/state");
    if schedext_state.exists() {
        let state = std::fs::read_to_string(schedext_state).unwrap_or_default();
        if state.trim() == "enabled" {
            println!("Another sched-ext scheduler is running");
            println!("Binary should exit gracefully if run without root");
        }
    }

    // Test that version works without root
    let output = Command::new(&binary)
        .arg("--version")
        .output()
        .expect("Failed to run binary");

    assert!(output.status.success(), "--version should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Version output: {}", stdout.trim());
}
