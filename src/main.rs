// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - sched-ext BPF scheduler for AMD Zen4/Zen5 X3D processors
//
// Copyright (C) 2025 ghostkellz <ckelley@ghostkellz.sh>

mod bpf_skel;
mod cgroup;
mod container;
mod gaming;
mod gpu;
mod pbo;
mod topology;
mod vm;

use anyhow::{Context, Result, bail};
use clap::Parser;
use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use libbpf_rs::MapCore;
use log::{info, warn, debug};
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::thread;

use bpf_skel::*;
use topology::CpuTopology;

const SCHEDULER_NAME: &str = "ghostbrew";

/// GhostBrew - AMD Zen4/Zen5 X3D optimized sched-ext scheduler
#[derive(Parser, Debug)]
#[command(name = "scx_ghostbrew")]
#[command(author = "ghostkellz <ckelley@ghostkellz.sh>")]
#[command(version = "0.1.0")]
#[command(about = "sched-ext BPF scheduler optimized for AMD Zen4/Zen5 X3D processors")]
struct Args {
    /// Gaming mode - prefer V-Cache CCD for latency-sensitive tasks
    #[arg(short, long)]
    gaming: bool,

    /// Productivity mode - prefer frequency CCD
    #[arg(short, long)]
    productivity: bool,

    /// Auto-detect workload and adjust (default)
    #[arg(short, long, default_value_t = true)]
    auto_mode: bool,

    /// Burst detection threshold in nanoseconds
    #[arg(long, default_value_t = 2_000_000)]
    burst_threshold: u64,

    /// Time slice in nanoseconds
    #[arg(long, default_value_t = 3_000_000)]
    slice_ns: u64,

    /// Print scheduler statistics periodically
    #[arg(short, long)]
    stats: bool,

    /// Statistics interval in seconds
    #[arg(long, default_value_t = 2)]
    stats_interval: u64,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Debug logging (very verbose)
    #[arg(short, long)]
    debug: bool,
}

/// CPU context structure matching BPF side
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct CpuCtx {
    ccd: u32,
    ccx: u32,
    node: u32,
    smt_sibling: i32,  // SMT sibling CPU (-1 if none)
    is_vcache: u8,     // bool in BPF is 1 byte
    _pad: [u8; 3],     // padding for alignment
}

/// Scheduler state
struct Scheduler<'a> {
    skel: GhostbrewSkel<'a>,
    struct_ops: Option<libbpf_rs::Link>,
    args: Args,
    topology: CpuTopology,
    gaming_detector: gaming::GamingDetector,
    prefcore: pbo::PrefcoreInfo,
    gpu_monitor: gpu::GpuMonitor,
    epp_manager: pbo::EppManager,
    vm_monitor: vm::VmMonitor,
    container_monitor: container::ContainerMonitor,
    cgroup_monitor: cgroup::CgroupMonitor,
}

impl<'a> Scheduler<'a> {
    fn init(args: Args, open_object: &'a mut MaybeUninit<libbpf_rs::OpenObject>) -> Result<Self> {
        // Set rlimit for BPF
        let rlim = libc::rlimit {
            rlim_cur: libc::RLIM_INFINITY,
            rlim_max: libc::RLIM_INFINITY,
        };
        unsafe {
            if libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) != 0 {
                warn!("Failed to set RLIMIT_MEMLOCK - BPF loading may fail");
            }
        }

        // Detect CPU topology
        let topology = topology::detect_topology()?;
        info!("Detected {} CPUs, {} CCDs", topology.nr_cpus, topology.nr_ccds);
        if let Some(vcache) = topology.vcache_ccd {
            info!("X3D processor detected - V-Cache on CCD {}", vcache);
        }

        // Log per-CCD CPU distribution
        for ccd in 0..topology.nr_ccds {
            let cpus_in_ccd: Vec<u32> = topology.cpu_to_ccd
                .iter()
                .enumerate()
                .filter(|&(_, c)| *c == ccd)
                .map(|(i, _)| i as u32)
                .collect();
            debug!("CCD {}: CPUs {:?}", ccd, cpus_in_ccd);
        }

        // Determine gaming mode
        let gaming_mode = if args.gaming {
            info!("Mode: Gaming (V-Cache CCD preferred)");
            true
        } else if args.productivity {
            info!("Mode: Productivity (Frequency CCD preferred)");
            false
        } else {
            info!("Mode: Auto-detect");
            topology.is_x3d
        };

        // Detect AMD prefcore rankings
        let prefcore = pbo::detect_prefcore(topology.nr_cpus)?;
        if prefcore.enabled {
            info!("AMD Prefcore: enabled (max ranking: {})", prefcore.max_ranking);
        }

        // Detect NVIDIA GPUs
        let gpu_monitor = gpu::GpuMonitor::default();
        if gpu_monitor.gpu_count() > 0 {
            info!("GPU: {}", gpu_monitor.summary());
            if let Some(primary) = gpu_monitor.primary_gpu() {
                info!("  Primary: {} ({} {})",
                      primary.model, primary.pcie_speed, primary.pcie_width);
            }
        }

        // Initialize EPP manager for frequency hints
        let mut epp_manager = pbo::EppManager::new(topology.nr_cpus);
        epp_manager.save_original(topology.nr_cpus);

        // Detect VMs
        let vm_monitor = vm::VmMonitor::default();
        if vm_monitor.vm_count() > 0 {
            info!("VMs: {} detected ({} gaming vCPUs, {} dev vCPUs)",
                  vm_monitor.vm_count(),
                  vm_monitor.gaming_vcpu_count(),
                  vm_monitor.dev_vcpu_count());
        }
        if vm_monitor.has_iommu() {
            info!("IOMMU: {}", vm_monitor.iommu_summary());
        }

        // Detect containers
        let container_monitor = container::ContainerMonitor::default();
        if container_monitor.container_count() > 0 {
            info!("Containers: {} detected ({} AI, {} GPU)",
                  container_monitor.container_count(),
                  container_monitor.ai_container_count(),
                  container_monitor.gpu_container_count());
        }
        if container_monitor.ollama_count() > 0 {
            info!("Ollama: {} processes", container_monitor.ollama_count());
        }

        // Classify cgroups
        let cgroup_monitor = cgroup::CgroupMonitor::default();
        if cgroup_monitor.classified_count() > 0 {
            info!("Cgroup classification: {} cgroups ({} gaming)",
                  cgroup_monitor.classified_count(),
                  cgroup_monitor.gaming_count());
        }

        // Build BPF skeleton
        let skel_builder = GhostbrewSkelBuilder::default();
        debug!("Opening BPF skeleton...");

        let mut open_skel = skel_builder.open(open_object)
            .context("Failed to open BPF skeleton")?;

        // Configure tunables via rodata
        {
            let rodata = &mut open_skel.maps.rodata_data;
            rodata.nr_cpus_possible = topology.nr_cpus;
            rodata.nr_ccds = topology.nr_ccds;
            rodata.vcache_ccd = topology.vcache_ccd.unwrap_or(0);
            rodata.gaming_mode = gaming_mode;
            rodata.smt_enabled = topology.smt_enabled;
            rodata.burst_threshold_ns = args.burst_threshold;
            rodata.slice_ns = args.slice_ns;
            rodata.debug_mode = args.debug;
        }

        // Load BPF program
        debug!("Loading BPF program...");
        let mut skel = open_skel.load()
            .context("Failed to load BPF program")?;

        // Populate cpu_ctxs map with topology info
        debug!("Populating CPU context map...");
        Self::init_cpu_contexts(&mut skel, &topology)?;

        // Populate prefcore rankings map
        if prefcore.enabled {
            debug!("Populating prefcore rankings map...");
            Self::init_prefcore_rankings(&mut skel, &prefcore)?;
        }

        // Attach struct_ops scheduler
        debug!("Attaching scheduler...");
        let struct_ops = skel.maps.ghostbrew_ops.attach_struct_ops()
            .context("Failed to attach struct_ops scheduler")?;

        info!("GhostBrew scheduler attached successfully");
        info!("  Per-CCD DSQs: {} (IDs 1-{})", topology.nr_ccds, topology.nr_ccds);
        info!("  V-Cache CCD: {}", topology.vcache_ccd.unwrap_or(0));
        if prefcore.enabled {
            info!("  Prefcore: {} preferred CPUs", prefcore.preferred_cpus.len());
        }

        Ok(Self {
            skel,
            struct_ops: Some(struct_ops),
            args,
            topology,
            gaming_detector: gaming::GamingDetector::new(),
            prefcore,
            gpu_monitor,
            epp_manager,
            vm_monitor,
            container_monitor,
            cgroup_monitor,
        })
    }

    /// Initialize per-CPU context in BPF map
    fn init_cpu_contexts(skel: &mut GhostbrewSkel, topology: &CpuTopology) -> Result<()> {
        let vcache_ccd = topology.vcache_ccd.unwrap_or(u32::MAX);

        for cpu in 0..topology.nr_cpus {
            let cpu_idx = cpu as usize;
            let ccd = topology.cpu_to_ccd.get(cpu_idx).copied().unwrap_or(0);
            let ccx = topology.cpu_to_ccx.get(cpu_idx).copied().unwrap_or(0);
            let node = topology.cpu_to_node.get(cpu_idx).copied().unwrap_or(0);
            let smt_sibling = topology.cpu_to_sibling.get(cpu_idx).copied().unwrap_or(-1);

            let ctx = CpuCtx {
                ccd,
                ccx,
                node,
                smt_sibling,
                is_vcache: if ccd == vcache_ccd { 1 } else { 0 },
                _pad: [0; 3],
            };

            let key = cpu.to_ne_bytes();
            let value = unsafe {
                std::slice::from_raw_parts(
                    &ctx as *const CpuCtx as *const u8,
                    std::mem::size_of::<CpuCtx>()
                )
            };

            skel.maps.cpu_ctxs.update(&key, value, libbpf_rs::MapFlags::ANY)
                .with_context(|| format!("Failed to update cpu_ctxs for CPU {}", cpu))?;

            debug!("CPU {}: CCD={}, CCX={}, node={}, vcache={}, smt_sibling={}",
                   cpu, ccd, ccx, node, ctx.is_vcache, smt_sibling);
        }

        Ok(())
    }

    /// Initialize prefcore rankings in BPF map
    fn init_prefcore_rankings(skel: &mut GhostbrewSkel, prefcore: &pbo::PrefcoreInfo) -> Result<()> {
        for (cpu, &ranking) in prefcore.rankings.iter().enumerate() {
            let key = (cpu as u32).to_ne_bytes();
            let value = ranking.to_ne_bytes();

            skel.maps.prefcore_rankings.update(&key, &value, libbpf_rs::MapFlags::ANY)
                .with_context(|| format!("Failed to update prefcore_rankings for CPU {}", cpu))?;
        }

        debug!("Populated prefcore rankings for {} CPUs", prefcore.rankings.len());
        Ok(())
    }

    fn run(&mut self, shutdown: Arc<AtomicBool>) -> Result<()> {
        info!("GhostBrew v{} running...", env!("CARGO_PKG_VERSION"));
        info!("Burst threshold: {} ns", self.args.burst_threshold);
        info!("Time slice: {} ns", self.args.slice_ns);
        info!("SMT: {}", if self.topology.smt_enabled { "enabled" } else { "disabled" });

        // Initial gaming PID scan
        self.update_gaming_pids();

        // Initial cgroup classification population
        self.update_cgroup_classes();

        // Main loop
        while !shutdown.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(self.args.stats_interval));

            // Scan for gaming PIDs periodically
            self.update_gaming_pids();

            // Update GPU power states
            if self.gpu_monitor.update_power_states() {
                debug!("GPU power state changed");
            }

            // Scan for VMs and update BPF map
            self.update_vm_pids();

            // Scan for containers and update BPF map
            self.update_container_pids();

            // Update cgroup classifications
            self.update_cgroup_classes();

            // Apply EPP hints based on workload
            self.update_epp_hints();

            // Print stats if requested
            if self.args.stats {
                self.print_stats();
            }
        }

        info!("GhostBrew shutting down...");

        // Restore original EPP values
        self.epp_manager.restore_original();

        // Detach scheduler
        self.struct_ops.take();

        Ok(())
    }

    /// Update the gaming_pids BPF map with detected gaming processes
    fn update_gaming_pids(&mut self) {
        match self.gaming_detector.scan_changes() {
            Ok((new_pids, removed_pids)) => {
                // Add new gaming PIDs
                for (pid, class) in new_pids {
                    let key = pid.to_ne_bytes();
                    let value = class.to_ne_bytes();
                    if let Err(e) = self.skel.maps.gaming_pids.update(&key, &value, libbpf_rs::MapFlags::ANY) {
                        debug!("Failed to add gaming PID {}: {}", pid, e);
                    }
                }

                // Remove old PIDs
                for pid in removed_pids {
                    let key = pid.to_ne_bytes();
                    let _ = self.skel.maps.gaming_pids.delete(&key);
                }

                let (gaming, ai) = self.gaming_detector.counts();
                if gaming > 0 || ai > 0 {
                    debug!("Gaming PIDs: {}, AI PIDs: {}", gaming, ai);
                }
            }
            Err(e) => {
                debug!("Gaming PID scan failed: {}", e);
            }
        }
    }

    /// Update VM vCPU PIDs in BPF map
    fn update_vm_pids(&mut self) {
        match self.vm_monitor.rescan() {
            Ok((new_vms, removed_pids)) => {
                // Log new VMs
                for vm in &new_vms {
                    info!("New VM detected: {} ({}) with {} vCPUs",
                          vm.name, vm.workload_type, vm.vcpu_pids.len());
                }

                // Update BPF map with all vCPU workloads
                let workloads = self.vm_monitor.get_vcpu_workloads();
                for (pid, workload_type) in workloads {
                    let key = pid.to_ne_bytes();
                    let class = match workload_type {
                        vm::VmWorkloadType::Gaming => 6u32, // WORKLOAD_VM_GAMING
                        vm::VmWorkloadType::Dev => 5u32,    // WORKLOAD_VM_DEV
                        vm::VmWorkloadType::Ai => 4u32,     // WORKLOAD_AI
                        vm::VmWorkloadType::Unknown => 5u32, // Default to dev
                    };
                    let value = class.to_ne_bytes();
                    let _ = self.skel.maps.vm_vcpu_pids.update(&key, &value, libbpf_rs::MapFlags::ANY);
                }

                // Remove old PIDs
                for pid in removed_pids {
                    let key = pid.to_ne_bytes();
                    let _ = self.skel.maps.vm_vcpu_pids.delete(&key);
                }
            }
            Err(e) => {
                debug!("VM scan failed: {}", e);
            }
        }
    }

    /// Update container PIDs in BPF map
    fn update_container_pids(&mut self) {
        match self.container_monitor.rescan() {
            Ok((new_containers, removed_ids)) => {
                // Log new containers
                for container in &new_containers {
                    info!("New container detected: {} ({}) with {} PIDs, GPU: {}",
                          container.id, container.workload_type,
                          container.pids.len(), container.has_gpu);
                }

                // Update BPF map with all container PIDs
                let pids = self.container_monitor.all_pids();
                for (pid, workload_type) in pids {
                    let key = pid.to_ne_bytes();
                    let class = match workload_type {
                        container::ContainerWorkloadType::Ai => 4u32,      // WORKLOAD_AI
                        container::ContainerWorkloadType::Gaming => 1u32,  // WORKLOAD_GAMING
                        container::ContainerWorkloadType::Compute => 3u32, // WORKLOAD_BATCH
                        container::ContainerWorkloadType::General => 7u32, // WORKLOAD_CONTAINER
                    };
                    let value = class.to_ne_bytes();
                    let _ = self.skel.maps.container_pids.update(&key, &value, libbpf_rs::MapFlags::ANY);
                }

                // Log removed containers
                for id in &removed_ids {
                    debug!("Container removed: {}", id);
                }
            }
            Err(e) => {
                debug!("Container scan failed: {}", e);
            }
        }
    }

    /// Update cgroup classifications in BPF map
    fn update_cgroup_classes(&mut self) {
        match self.cgroup_monitor.rescan() {
            Ok((new_cgroups, removed_ids)) => {
                // Log new gaming cgroups
                for cg in new_cgroups.iter().filter(|c| c.workload_class == cgroup::WORKLOAD_GAMING) {
                    info!("Gaming cgroup detected: {}", cg.path);
                }

                // Update BPF map with all classifications
                let classifications = self.cgroup_monitor.get_classifications();
                for (&cgroup_id, &workload_class) in classifications {
                    let key = cgroup_id.to_ne_bytes();
                    let value = workload_class.to_ne_bytes();
                    let _ = self.skel.maps.cgroup_classes.update(&key, &value, libbpf_rs::MapFlags::ANY);
                }

                // Remove old cgroups from map
                for cgroup_id in removed_ids {
                    let key = cgroup_id.to_ne_bytes();
                    let _ = self.skel.maps.cgroup_classes.delete(&key);
                }
            }
            Err(e) => {
                debug!("Cgroup scan failed: {}", e);
            }
        }
    }

    /// Update EPP hints based on active workloads
    fn update_epp_hints(&mut self) {
        let (gaming_count, _ai_count) = self.gaming_detector.counts();
        let gpu_active = self.gpu_monitor.any_gpu_active();

        // When gaming is active and GPU is in D0, boost preferred cores
        if gaming_count > 0 && gpu_active {
            // Set performance EPP on preferred cores (highest prefcore ranking)
            for &cpu in &self.prefcore.preferred_cpus {
                if let Err(e) = self.epp_manager.set_epp(cpu, "performance") {
                    debug!("Failed to set EPP for CPU {}: {}", cpu, e);
                }
            }
        }
        // Note: EPP is automatically restored on shutdown via EppManager::drop
    }

    fn print_stats(&self) {
        let bss = &self.skel.maps.bss_data;
        println!("--- GhostBrew Stats ---");
        println!("  Enqueued: {}", bss.nr_enqueued);
        println!("  Dispatched: {} (direct: {})",
                 bss.nr_dispatched, bss.nr_direct_dispatched);
        println!("  Gaming tasks: {}", bss.nr_gaming_tasks);
        println!("  Interactive tasks: {}", bss.nr_interactive_tasks);
        println!("  V-Cache migrations: {}", bss.nr_vcache_migrations);
        println!("  CCD local: {} | cross: {}",
                 bss.nr_ccd_local, bss.nr_ccd_cross);
        println!("  SMT idle picks: {}", bss.nr_smt_idle_picks);
        println!("  Compaction overflows: {}", bss.nr_compaction_overflows);
        println!("  Preempt kicks: {}", bss.nr_preempt_kicks);
        // Phase 4a stats
        println!("  Proton tasks: {}", bss.nr_proton_tasks);
        println!("  Parent chain detects: {}", bss.nr_parent_chain_detects);
        println!("  Userspace hint detects: {}", bss.nr_userspace_hint_detects);
        println!("  Prefcore placements: {}", bss.nr_prefcore_placements);
        if self.topology.is_x3d {
            println!("  V-Cache CCD: {}", self.topology.vcache_ccd.unwrap_or(0));
        }
        // Phase 4b stats - GPU
        println!("  GPU feeder tasks: {}", bss.nr_gpu_feeder_tasks);
        if self.gpu_monitor.gpu_count() > 0 {
            println!("  GPU: {} ({})",
                     self.gpu_monitor.summary(),
                     if self.gpu_monitor.any_gpu_active() { "active" } else { "idle" });
        }
        // Phase 4c stats - VM/Container
        if bss.nr_vm_vcpu_tasks > 0 || self.vm_monitor.vm_count() > 0 {
            println!("  VM vCPU tasks: {} (gaming: {}, dev: {})",
                     bss.nr_vm_vcpu_tasks, bss.nr_gaming_vm_vcpus, bss.nr_dev_vm_vcpus);
        }
        if bss.nr_container_tasks > 0 || self.container_monitor.container_count() > 0 {
            println!("  Container tasks: {} (AI: {})",
                     bss.nr_container_tasks, bss.nr_ai_container_tasks);
        }
        if self.container_monitor.ollama_count() > 0 {
            println!("  Ollama processes: {}", self.container_monitor.ollama_count());
        }
        // Phase 4d stats - Cgroup classification
        if bss.nr_cgroup_classifications > 0 || self.cgroup_monitor.classified_count() > 0 {
            println!("  Cgroup classifications: {} (gaming: {})",
                     bss.nr_cgroup_classifications, bss.nr_cgroup_gaming);
            println!("  Cgroups monitored: {} ({} gaming)",
                     self.cgroup_monitor.classified_count(),
                     self.cgroup_monitor.gaming_count());
        }
        println!("---");
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug {
        "debug"
    } else if args.verbose {
        "info"
    } else {
        "warn"
    };

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(log_level)
    ).init();

    info!("scx_{} v{}", SCHEDULER_NAME, env!("CARGO_PKG_VERSION"));

    // Check for root
    if !nix::unistd::Uid::effective().is_root() {
        bail!("scx_ghostbrew must be run as root");
    }

    // Check for sched-ext support
    if !std::path::Path::new("/sys/kernel/sched_ext").exists() {
        bail!("sched-ext not supported - ensure CONFIG_SCHED_CLASS_EXT=y in kernel");
    }

    // Set up shutdown signal
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        info!("Received shutdown signal");
        shutdown_clone.store(true, Ordering::Relaxed);
    }).context("Failed to set signal handler")?;

    // Initialize and run scheduler
    let mut open_object = MaybeUninit::uninit();
    let mut scheduler = Scheduler::init(args, &mut open_object)?;
    scheduler.run(shutdown)
}
