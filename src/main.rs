// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - sched-ext BPF scheduler for AMD Zen4/Zen5 X3D and Intel Hybrid processors
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

mod bpf_skel;
mod cgroup;
mod config;
mod container;
mod control;
mod events;
mod gaming;
mod gpu;
mod intel;
mod mangohud;
mod pbo;
mod profiles;
mod topology;
mod vcache;
mod vm;

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use libbpf_rs::MapCore;
use libbpf_rs::skel::{OpenSkel, SkelBuilder};
use log::{debug, info, warn};
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use bpf_skel::*;
use topology::CpuTopology;

const SCHEDULER_NAME: &str = "ghostbrew";

/// GhostBrew - AMD Zen4/Zen5 X3D and Intel Hybrid optimized sched-ext scheduler
#[derive(Parser, Debug)]
#[command(name = "scx_ghostbrew")]
#[command(author = "ghostkellz <ckelley@ghostkellz.sh>")]
#[command(version = "0.1.0")]
#[command(
    about = "sched-ext BPF scheduler optimized for AMD Zen4/Zen5 X3D and Intel Hybrid processors"
)]
struct Args {
    /// Gaming mode - prefer V-Cache CCD (AMD) or P-cores (Intel) for latency-sensitive tasks
    #[arg(short = 'g', long)]
    gaming: bool,

    /// Work mode - prefer frequency CCD (AMD) for higher boost on non-gaming tasks
    #[arg(short = 'w', long)]
    work: bool,

    /// Auto-detect workload and adjust (default)
    #[arg(short = 'a', long, default_value_t = true)]
    auto_mode: bool,

    /// Burst detection threshold in nanoseconds
    #[arg(long, default_value_t = 2_000_000)]
    burst_threshold: u64,

    /// Time slice in nanoseconds
    #[arg(long, default_value_t = 3_000_000)]
    slice_ns: u64,

    /// E-core offload mode for Intel hybrid CPUs: disabled, conservative, aggressive
    #[arg(long, default_value = "conservative")]
    ecore_offload: String,

    /// Print scheduler statistics periodically
    #[arg(short, long)]
    stats: bool,

    /// Statistics interval in seconds
    #[arg(long, default_value_t = 2)]
    stats_interval: u64,

    /// Benchmark mode - export stats to MangoHud-compatible CSV
    #[arg(short = 'b', long)]
    benchmark: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Debug logging (very verbose)
    #[arg(short, long)]
    debug: bool,

    /// Generate shell completions (bash, zsh, fish, powershell)
    #[arg(long, value_name = "SHELL")]
    completions: Option<clap_complete::Shell>,

    /// Analyze MangoHud frame time log (show stats without running scheduler)
    #[arg(long)]
    analyze_frametime: Option<Option<std::path::PathBuf>>,
}

/// CPU context structure matching BPF side
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct CpuCtx {
    ccd: u32,
    ccx: u32,
    node: u32,
    smt_sibling: i32, // SMT sibling CPU (-1 if none)
    is_vcache: u8,    // bool in BPF is 1 byte (AMD X3D)
    is_pcore: u8,     // Intel hybrid P-core flag
    is_turbo: u8,     // Highest-performing core (prefcore or HWP)
    _pad: [u8; 1],    // padding for alignment
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
    // Config and profiles
    config: config::GhostBrewConfig,
    profile_manager: profiles::ProfileManager,
    vcache_controller: vcache::VCacheController,
    /// Active game profiles (PID -> profile name)
    active_profiles: std::collections::HashMap<u32, String>,
    /// MangoHud stats exporter
    mangohud_exporter: Option<mangohud::MangoHudExporter>,
    /// Runtime control interface
    control_interface: control::ControlInterface,
    /// Event handler for ringbuf events
    event_handler: Arc<events::EventHandler>,
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
        info!(
            "Detected {} CPUs, {} CCDs/clusters",
            topology.nr_cpus, topology.nr_ccds
        );

        // Log architecture-specific info
        match &topology.arch {
            topology::CpuArch::AmdZen { is_x3d, generation } => {
                if *is_x3d {
                    info!(
                        "AMD Zen {} X3D processor detected - V-Cache on CCD {}",
                        generation,
                        topology.vcache_ccd.unwrap_or(0)
                    );
                } else {
                    info!("AMD Zen {} processor detected", generation);
                }
            }
            topology::CpuArch::IntelHybrid { generation } => {
                info!(
                    "Intel {}th gen hybrid detected - {} P-cores, {} E-cores",
                    generation,
                    topology.pcore_cpus.len(),
                    topology.ecore_cpus.len()
                );
                debug!("P-cores: {:?}", topology.pcore_cpus);
                debug!("E-cores: {:?}", topology.ecore_cpus);
            }
            topology::CpuArch::Generic => {
                info!("Generic x86-64 processor detected");
            }
        }

        // Log per-CCD/cluster CPU distribution
        for ccd in 0..topology.nr_ccds {
            let cpus_in_ccd: Vec<u32> = topology
                .cpu_to_ccd
                .iter()
                .enumerate()
                .filter(|&(_, c)| *c == ccd)
                .map(|(i, _)| i as u32)
                .collect();
            debug!("CCD/Cluster {}: CPUs {:?}", ccd, cpus_in_ccd);
        }

        // Parse E-core offload mode for Intel
        let ecore_offload_mode: u32 = match args.ecore_offload.to_lowercase().as_str() {
            "disabled" | "off" | "0" => 0,
            "conservative" | "1" => 1,
            "aggressive" | "2" => 2,
            _ => {
                warn!(
                    "Unknown ecore_offload mode '{}', using conservative",
                    args.ecore_offload
                );
                1
            }
        };

        // Determine gaming mode and work mode
        let (gaming_mode, work_mode) = if args.gaming {
            if topology.is_intel_hybrid {
                info!("Mode: Gaming (P-cores preferred)");
            } else {
                info!("Mode: Gaming (V-Cache CCD preferred)");
            }
            (true, false)
        } else if args.work {
            if topology.is_intel_hybrid {
                info!("Mode: Work (balanced E-core offload)");
            } else {
                info!("Mode: Work (Frequency CCD preferred for higher boost)");
            }
            (false, true)
        } else {
            info!("Mode: Auto-detect");
            (topology.is_x3d || topology.is_intel_hybrid, false)
        };

        // Load configuration
        let config = config::GhostBrewConfig::load().unwrap_or_else(|e| {
            warn!("Failed to load config: {}, using defaults", e);
            config::GhostBrewConfig::default()
        });

        // Load game profiles
        let mut profile_manager = profiles::ProfileManager::new();
        if let Some(ref profiles_dir) = config.profiles_dir {
            let _ = profile_manager.load_from_directory(profiles_dir);
        }
        let profile_count = profile_manager.load_standard_paths().unwrap_or(0);
        if profile_count > 0 {
            info!("Loaded {} game profiles", profile_count);
            for profile in profile_manager.all_profiles() {
                debug!("  Profile: {} (exe: {:?})", profile.name, profile.exe_name);
            }
        }

        // Initialize V-Cache controller (ghost-vcache integration)
        let mut vcache_controller = vcache::VCacheController::default();
        if vcache_controller.is_available() {
            // Set strategy from config
            if config.is_vcache_auto_switching() {
                vcache_controller.set_strategy(vcache::SwitchingStrategy::Automatic {
                    gaming_threshold: 1,
                    batch_threshold: 4,
                });
                info!("V-Cache: automatic switching enabled");
            } else if config.should_follow_ghost_vcache() {
                vcache_controller.set_strategy(vcache::SwitchingStrategy::FollowGhostVcache);
                info!("V-Cache: following ghost-vcache mode changes");
            } else {
                vcache_controller.set_strategy(vcache::SwitchingStrategy::Manual);
                info!("V-Cache: manual control");
            }
        }

        // Detect AMD prefcore rankings
        let prefcore = pbo::detect_prefcore(topology.nr_cpus)?;
        if prefcore.enabled {
            info!(
                "AMD Prefcore: enabled (max ranking: {})",
                prefcore.max_ranking
            );
        }

        // Detect NVIDIA GPUs
        let gpu_monitor = gpu::GpuMonitor::default();
        if gpu_monitor.gpu_count() > 0 {
            info!("GPU: {}", gpu_monitor.summary());
            if let Some(primary) = gpu_monitor.primary_gpu() {
                info!(
                    "  Primary: {} ({} {})",
                    primary.model, primary.pcie_speed, primary.pcie_width
                );
            }
        }

        // Initialize EPP manager for frequency hints
        let mut epp_manager = pbo::EppManager::new(topology.nr_cpus);
        epp_manager.save_original(topology.nr_cpus);

        // Detect VMs
        let vm_monitor = vm::VmMonitor::default();
        if vm_monitor.vm_count() > 0 {
            info!(
                "VMs: {} detected ({} gaming vCPUs, {} dev vCPUs)",
                vm_monitor.vm_count(),
                vm_monitor.gaming_vcpu_count(),
                vm_monitor.dev_vcpu_count()
            );
        }
        if vm_monitor.has_iommu() {
            info!("IOMMU: {}", vm_monitor.iommu_summary());
        }

        // Detect containers
        let container_monitor = container::ContainerMonitor::default();
        if container_monitor.container_count() > 0 {
            info!(
                "Containers: {} detected ({} AI, {} GPU)",
                container_monitor.container_count(),
                container_monitor.ai_container_count(),
                container_monitor.gpu_container_count()
            );
        }
        if container_monitor.ollama_count() > 0 {
            info!("Ollama: {} processes", container_monitor.ollama_count());
        }

        // Classify cgroups
        let cgroup_monitor = cgroup::CgroupMonitor::default();
        if cgroup_monitor.classified_count() > 0 {
            info!(
                "Cgroup classification: {} cgroups ({} gaming)",
                cgroup_monitor.classified_count(),
                cgroup_monitor.gaming_count()
            );
        }

        // Build BPF skeleton
        let skel_builder = GhostbrewSkelBuilder::default();
        debug!("Opening BPF skeleton...");

        let mut open_skel = skel_builder
            .open(open_object)
            .context("Failed to open BPF skeleton")?;

        // Configure tunables via rodata
        {
            let rodata = &mut open_skel.maps.rodata_data;
            // Topology config (static, set before load)
            rodata.nr_cpus_possible = topology.nr_cpus;
            rodata.nr_ccds = topology.nr_ccds;
            rodata.vcache_ccd = topology.vcache_ccd.unwrap_or(0);
            rodata.smt_enabled = topology.smt_enabled;
            rodata.debug_mode = args.debug;
            // Intel hybrid support
            rodata.is_intel_hybrid = topology.is_intel_hybrid;
            rodata.nr_pcores = topology.pcore_cpus.len() as u32;
            rodata.nr_ecores = topology.ecore_cpus.len() as u32;
            rodata.ecore_offload_mode = ecore_offload_mode;
            // Zen 5 specific support
            rodata.zen_generation = topology.zen_generation.unwrap_or(0);
            rodata.freq_ccd = topology.freq_ccd.unwrap_or(0);
            rodata.asymmetric_ccd_boost = topology.asymmetric_ccd_boost;
            rodata.vcache_l3_mb = topology.vcache_l3_mb.unwrap_or(0);
            // Default tunables (will be overwritten by runtime_tunables map after load)
            rodata.default_burst_threshold_ns = args.burst_threshold;
            rodata.default_slice_ns = args.slice_ns;
        }

        // Load BPF program
        debug!("Loading BPF program...");
        let mut skel = open_skel.load().context("Failed to load BPF program")?;

        // Populate cpu_ctxs map with topology info
        debug!("Populating CPU context map...");
        Self::init_cpu_contexts(&mut skel, &topology)?;

        // Populate prefcore rankings map
        if prefcore.enabled {
            debug!("Populating prefcore rankings map...");
            Self::init_prefcore_rankings(&mut skel, &prefcore)?;
        }

        // Initialize runtime tunables map
        debug!("Initializing runtime tunables...");
        Self::init_runtime_tunables(&mut skel, &args, gaming_mode, work_mode)?;

        // Attach struct_ops scheduler
        debug!("Attaching scheduler...");
        let struct_ops = skel
            .maps
            .ghostbrew_ops
            .attach_struct_ops()
            .context("Failed to attach struct_ops scheduler")?;

        info!("GhostBrew scheduler attached successfully");
        info!(
            "  Per-CCD/Cluster DSQs: {} (IDs 1-{})",
            topology.nr_ccds, topology.nr_ccds
        );
        if topology.is_intel_hybrid {
            info!(
                "  Intel Hybrid: {} P-cores, {} E-cores",
                topology.pcore_cpus.len(),
                topology.ecore_cpus.len()
            );
            let mode_str = match ecore_offload_mode {
                0 => "disabled",
                1 => "conservative",
                2 => "aggressive",
                _ => "unknown",
            };
            info!("  E-core offload: {}", mode_str);
        } else {
            info!("  V-Cache CCD: {}", topology.vcache_ccd.unwrap_or(0));
            // Zen 5 specific info
            if topology.asymmetric_ccd_boost {
                info!(
                    "  Zen {} X3D asymmetric boost: freq CCD {}, L3 {}MB",
                    topology.zen_generation.unwrap_or(5),
                    topology.freq_ccd.unwrap_or(1),
                    topology.vcache_l3_mb.unwrap_or(64)
                );
            } else if let Some(zen_gen) = topology.zen_generation {
                info!("  Zen {} architecture", zen_gen);
            }
        }
        if prefcore.enabled {
            info!(
                "  Prefcore: {} preferred CPUs",
                prefcore.preferred_cpus.len()
            );
        }

        // Initialize MangoHud exporter if MangoHud is detected or benchmark mode
        let mangohud_exporter = if mangohud::is_mangohud_running() || args.benchmark {
            let mut exporter = mangohud::MangoHudExporter::new();
            if let Err(e) = exporter.init() {
                warn!("Failed to initialize MangoHud exporter: {}", e);
                None
            } else {
                info!("MangoHud stats export: {:?}", exporter.output_path());
                Some(exporter)
            }
        } else {
            None
        };

        // Initialize runtime control interface
        let mut control_interface = control::ControlInterface::new();
        if let Err(e) = control_interface.init() {
            warn!("Failed to initialize control interface: {}", e);
        }

        // Initialize event handler for ringbuf
        let event_handler = Arc::new(events::EventHandler::new(args.verbose || args.debug));

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
            config,
            profile_manager,
            vcache_controller,
            active_profiles: std::collections::HashMap::new(),
            mangohud_exporter,
            control_interface,
            event_handler,
        })
    }

    /// Initialize per-CPU context in BPF map
    fn init_cpu_contexts(skel: &mut GhostbrewSkel, topology: &CpuTopology) -> Result<()> {
        let vcache_ccd = topology.vcache_ccd.unwrap_or(u32::MAX);

        // Find max turbo ranking for determining "turbo" cores
        let max_turbo = topology.turbo_rankings.iter().max().copied().unwrap_or(0);
        let turbo_threshold = max_turbo * 95 / 100; // Top 5% are "turbo" cores

        for cpu in 0..topology.nr_cpus {
            let cpu_idx = cpu as usize;
            let ccd = topology.cpu_to_ccd.get(cpu_idx).copied().unwrap_or(0);
            let ccx = topology.cpu_to_ccx.get(cpu_idx).copied().unwrap_or(0);
            let node = topology.cpu_to_node.get(cpu_idx).copied().unwrap_or(0);
            let smt_sibling = topology.cpu_to_sibling.get(cpu_idx).copied().unwrap_or(-1);

            // Determine if this is a P-core (Intel hybrid)
            let is_pcore = if topology.is_intel_hybrid {
                topology.pcore_cpus.contains(&cpu)
            } else {
                false
            };

            // Determine if this is a high-turbo core (top performers)
            let turbo_ranking = topology.turbo_rankings.get(cpu_idx).copied().unwrap_or(0);
            let is_turbo = turbo_ranking >= turbo_threshold && turbo_threshold > 0;

            let ctx = CpuCtx {
                ccd,
                ccx,
                node,
                smt_sibling,
                is_vcache: if ccd == vcache_ccd { 1 } else { 0 },
                is_pcore: if is_pcore { 1 } else { 0 },
                is_turbo: if is_turbo { 1 } else { 0 },
                _pad: [0; 1],
            };

            let key = cpu.to_ne_bytes();
            let value = unsafe {
                std::slice::from_raw_parts(
                    &ctx as *const CpuCtx as *const u8,
                    std::mem::size_of::<CpuCtx>(),
                )
            };

            skel.maps
                .cpu_ctxs
                .update(&key, value, libbpf_rs::MapFlags::ANY)
                .with_context(|| format!("Failed to update cpu_ctxs for CPU {}", cpu))?;

            debug!(
                "CPU {}: CCD={}, CCX={}, node={}, vcache={}, pcore={}, turbo={}, smt_sibling={}",
                cpu, ccd, ccx, node, ctx.is_vcache, ctx.is_pcore, ctx.is_turbo, smt_sibling
            );
        }

        Ok(())
    }

    /// Initialize prefcore rankings in BPF map
    fn init_prefcore_rankings(
        skel: &mut GhostbrewSkel,
        prefcore: &pbo::PrefcoreInfo,
    ) -> Result<()> {
        for (cpu, &ranking) in prefcore.rankings.iter().enumerate() {
            let key = (cpu as u32).to_ne_bytes();
            let value = ranking.to_ne_bytes();

            skel.maps
                .prefcore_rankings
                .update(&key, &value, libbpf_rs::MapFlags::ANY)
                .with_context(|| format!("Failed to update prefcore_rankings for CPU {}", cpu))?;
        }

        debug!(
            "Populated prefcore rankings for {} CPUs",
            prefcore.rankings.len()
        );
        Ok(())
    }

    /// Initialize runtime tunables in BPF map
    fn init_runtime_tunables(
        skel: &mut GhostbrewSkel,
        args: &Args,
        gaming_mode: bool,
        work_mode: bool,
    ) -> Result<()> {
        // Struct layout must match BPF runtime_tunables:
        // u64 burst_threshold_ns, u64 slice_ns, u8 gaming_mode, u8 work_mode, u8[6] pad
        let mut value = [0u8; 24];
        value[0..8].copy_from_slice(&args.burst_threshold.to_ne_bytes());
        value[8..16].copy_from_slice(&args.slice_ns.to_ne_bytes());
        value[16] = if gaming_mode { 1 } else { 0 };
        value[17] = if work_mode { 1 } else { 0 };
        // Padding bytes 18-23 are already 0

        let key = 0u32.to_ne_bytes();
        skel.maps
            .runtime_tunables
            .update(&key, &value, libbpf_rs::MapFlags::ANY)
            .context("Failed to initialize runtime_tunables map")?;

        debug!(
            "Runtime tunables: burst={}ns, slice={}ns, gaming={}, work={}",
            args.burst_threshold, args.slice_ns, gaming_mode, work_mode
        );
        Ok(())
    }

    /// Update runtime tunables at runtime (for profile switching, etc.)
    fn update_runtime_tunables(
        &mut self,
        burst_threshold_ns: Option<u64>,
        slice_ns: Option<u64>,
        gaming_mode: Option<bool>,
        work_mode: Option<bool>,
    ) -> Result<()> {
        // Read current values
        let key = 0u32.to_ne_bytes();
        let current = self
            .skel
            .maps
            .runtime_tunables
            .lookup(&key, libbpf_rs::MapFlags::ANY)?
            .ok_or_else(|| anyhow::anyhow!("runtime_tunables map empty"))?;

        // Parse current values
        let mut burst = u64::from_ne_bytes(current[0..8].try_into().unwrap_or([0; 8]));
        let mut slice = u64::from_ne_bytes(current[8..16].try_into().unwrap_or([0; 8]));
        let mut gaming = current[16] != 0;
        let mut work = current[17] != 0;

        // Apply updates
        if let Some(v) = burst_threshold_ns {
            burst = v;
        }
        if let Some(v) = slice_ns {
            slice = v;
        }
        if let Some(v) = gaming_mode {
            gaming = v;
        }
        if let Some(v) = work_mode {
            work = v;
        }

        // Write back
        let mut value = [0u8; 24];
        value[0..8].copy_from_slice(&burst.to_ne_bytes());
        value[8..16].copy_from_slice(&slice.to_ne_bytes());
        value[16] = if gaming { 1 } else { 0 };
        value[17] = if work { 1 } else { 0 };

        self.skel
            .maps
            .runtime_tunables
            .update(&key, &value, libbpf_rs::MapFlags::ANY)
            .context("Failed to update runtime_tunables map")?;

        debug!(
            "Updated runtime tunables: burst={}ns, slice={}ns, gaming={}, work={}",
            burst, slice, gaming, work
        );
        Ok(())
    }

    /// Apply profile-specific tunables to BPF
    fn apply_profile_tunables_direct(
        &mut self,
        profile_name: &str,
        tunables: &profiles::ProfileTunables,
    ) {
        // Only update if profile specifies custom values
        if tunables.burst_threshold_ns.is_none() && tunables.slice_ns.is_none() {
            return;
        }

        if let Err(e) = self.update_runtime_tunables(
            tunables.burst_threshold_ns,
            tunables.slice_ns,
            None, // Don't change gaming_mode
            None, // Don't change work_mode
        ) {
            warn!(
                "Failed to apply profile tunables for '{}': {}",
                profile_name, e
            );
        } else {
            info!(
                "Applied profile '{}' tunables: burst={:?}ns, slice={:?}ns",
                profile_name, tunables.burst_threshold_ns, tunables.slice_ns
            );
        }
    }

    /// Revert to default tunables when no profiled games are active
    fn revert_to_default_tunables(&mut self) {
        info!("Reverting to default tunables (no active profiles)");

        if let Err(e) = self.update_runtime_tunables(
            Some(self.args.burst_threshold),
            Some(self.args.slice_ns),
            None, // Keep current gaming_mode
            None, // Keep current work_mode
        ) {
            warn!("Failed to revert to default tunables: {}", e);
        }
    }

    fn run(&mut self, shutdown: Arc<AtomicBool>) -> Result<()> {
        info!("GhostBrew v{} running...", env!("CARGO_PKG_VERSION"));
        info!("Burst threshold: {} ns", self.args.burst_threshold);
        info!("Time slice: {} ns", self.args.slice_ns);
        info!(
            "SMT: {}",
            if self.topology.smt_enabled {
                "enabled"
            } else {
                "disabled"
            }
        );

        info!("Event streaming enabled (ringbuf)");

        // Initial gaming PID scan
        self.update_gaming_pids();

        // Initial cgroup classification population
        self.update_cgroup_classes();

        // Main loop
        let stats_interval = Duration::from_secs(self.args.stats_interval);
        let poll_interval = Duration::from_millis(100);
        let mut last_stats = Instant::now();

        while !shutdown.load(Ordering::Relaxed) {
            // Poll ringbuf for events (100ms timeout, non-blocking)
            // Build ringbuf in each iteration to avoid lifetime issues
            {
                if let Ok(ringbuf) =
                    events::build_ringbuf(&self.skel.maps.events, self.event_handler.clone())
                    && let Err(e) = events::poll_events(&ringbuf, poll_interval)
                {
                    debug!("Ringbuf poll error: {}", e);
                }
            }

            // Check if it's time for periodic tasks
            if last_stats.elapsed() < stats_interval {
                continue;
            }
            last_stats = Instant::now();

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

            // Poll V-Cache controller for mode changes (ghost-vcache integration)
            self.poll_vcache_mode();

            // Poll control interface for runtime tuning commands
            self.poll_control_interface();

            // Print stats if requested
            if self.args.stats {
                self.print_stats();
            }

            // Export to MangoHud CSV if enabled
            self.export_mangohud_stats();
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
                for (pid, class) in &new_pids {
                    let key = pid.to_ne_bytes();
                    let value = class.to_ne_bytes();
                    if let Err(e) =
                        self.skel
                            .maps
                            .gaming_pids
                            .update(&key, &value, libbpf_rs::MapFlags::ANY)
                    {
                        debug!("Failed to add gaming PID {}: {}", pid, e);
                    }

                    // Check for profile match
                    if let Some(exe_name) = gaming::get_exe_name(*pid) {
                        // Clone profile data to avoid borrow conflict
                        let profile_data = self
                            .profile_manager
                            .match_process(&exe_name, None)
                            .map(|p| (p.name.clone(), p.tunables.clone()));

                        if let Some((profile_name, tunables)) = profile_data {
                            info!(
                                "Matched profile '{}' for {} (PID {})",
                                profile_name, exe_name, pid
                            );
                            self.active_profiles.insert(*pid, profile_name.clone());

                            // Apply profile tunables to BPF
                            self.apply_profile_tunables_direct(&profile_name, &tunables);
                        }
                    }
                }

                // Remove old PIDs
                for pid in &removed_pids {
                    let key = pid.to_ne_bytes();
                    let _ = self.skel.maps.gaming_pids.delete(&key);
                    // Clean up active profiles
                    if let Some(profile_name) = self.active_profiles.remove(pid) {
                        debug!("Removed profile '{}' for PID {}", profile_name, pid);

                        // If no more profiled games, revert to default tunables
                        if self.active_profiles.is_empty() {
                            self.revert_to_default_tunables();
                        }
                    }
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
                    info!(
                        "New VM detected: {} ({}) with {} vCPUs",
                        vm.name,
                        vm.workload_type,
                        vm.vcpu_pids.len()
                    );
                }

                // Update BPF map with all vCPU workloads
                let workloads = self.vm_monitor.get_vcpu_workloads();
                for (pid, workload_type) in workloads {
                    let key = pid.to_ne_bytes();
                    let class = match workload_type {
                        vm::VmWorkloadType::Gaming => 6u32,  // WORKLOAD_VM_GAMING
                        vm::VmWorkloadType::Dev => 5u32,     // WORKLOAD_VM_DEV
                        vm::VmWorkloadType::Ai => 4u32,      // WORKLOAD_AI
                        vm::VmWorkloadType::Unknown => 5u32, // Default to dev
                    };
                    let value = class.to_ne_bytes();
                    let _ =
                        self.skel
                            .maps
                            .vm_vcpu_pids
                            .update(&key, &value, libbpf_rs::MapFlags::ANY);
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
                    info!(
                        "New container detected: {} ({}) with {} PIDs, GPU: {}",
                        container.id,
                        container.workload_type,
                        container.pids.len(),
                        container.has_gpu
                    );
                }

                // Update BPF map with all container PIDs
                let pids = self.container_monitor.all_pids();
                for (pid, workload_type) in pids {
                    let key = pid.to_ne_bytes();
                    let class = match workload_type {
                        container::ContainerWorkloadType::Ai => 4u32, // WORKLOAD_AI
                        container::ContainerWorkloadType::Gaming => 1u32, // WORKLOAD_GAMING
                        container::ContainerWorkloadType::Compute => 3u32, // WORKLOAD_BATCH
                        container::ContainerWorkloadType::General => 7u32, // WORKLOAD_CONTAINER
                    };
                    let value = class.to_ne_bytes();
                    let _ = self.skel.maps.container_pids.update(
                        &key,
                        &value,
                        libbpf_rs::MapFlags::ANY,
                    );
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
                for cg in new_cgroups
                    .iter()
                    .filter(|c| c.workload_class == cgroup::WORKLOAD_GAMING)
                {
                    info!("Gaming cgroup detected: {}", cg.path);
                }

                // Update BPF map with all classifications
                let classifications = self.cgroup_monitor.get_classifications();
                for (&cgroup_id, &workload_class) in classifications {
                    let key = cgroup_id.to_ne_bytes();
                    let value = workload_class.to_ne_bytes();
                    let _ = self.skel.maps.cgroup_classes.update(
                        &key,
                        &value,
                        libbpf_rs::MapFlags::ANY,
                    );
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

    /// Poll V-Cache controller for mode changes (ghost-vcache integration)
    fn poll_vcache_mode(&mut self) {
        if !self.vcache_controller.is_available() {
            return;
        }

        // Check for mode changes from ghost-vcache
        if let Some(new_mode) = self.vcache_controller.poll_changes() {
            let gaming_mode = new_mode.to_gaming_mode();
            info!(
                "V-Cache mode changed to {} (gaming_mode={})",
                new_mode, gaming_mode
            );

            // Update runtime tunables to reflect new mode
            if let Err(e) = self.update_runtime_tunables(None, None, Some(gaming_mode), None) {
                warn!("Failed to update gaming_mode from V-Cache: {}", e);
            }
        }

        // For automatic switching strategy, evaluate based on workload metrics
        if self.config.is_vcache_auto_switching() {
            let bss = &self.skel.maps.bss_data;
            let nr_gaming = bss.nr_gaming_tasks;
            let nr_batch = bss.nr_enqueued.saturating_sub(nr_gaming); // Rough batch estimate

            if let Some(target_mode) = self.vcache_controller.evaluate_switch(nr_gaming, nr_batch)
                && let Err(e) = self.vcache_controller.request_mode(target_mode)
            {
                warn!("Failed to switch V-Cache mode: {}", e);
            }
        }
    }

    /// Poll control interface for runtime tuning commands
    fn poll_control_interface(&mut self) {
        let commands = self.control_interface.poll_commands();

        for cmd in commands {
            match cmd {
                control::ControlCommand::SetBurstThreshold(ns) => {
                    info!("Control: Setting burst_threshold_ns={}", ns);
                    if let Err(e) = self.update_runtime_tunables(Some(ns), None, None, None) {
                        warn!("Failed to apply burst threshold: {}", e);
                    }
                }
                control::ControlCommand::SetSlice(ns) => {
                    info!("Control: Setting slice_ns={}", ns);
                    if let Err(e) = self.update_runtime_tunables(None, Some(ns), None, None) {
                        warn!("Failed to apply slice: {}", e);
                    }
                }
                control::ControlCommand::GamingMode(enabled) => {
                    info!("Control: Setting gaming_mode={}", enabled);
                    if let Err(e) = self.update_runtime_tunables(None, None, Some(enabled), None) {
                        warn!("Failed to apply gaming mode: {}", e);
                    }
                }
                control::ControlCommand::WorkMode(enabled) => {
                    info!("Control: Setting work_mode={}", enabled);
                    if let Err(e) = self.update_runtime_tunables(None, None, None, Some(enabled)) {
                        warn!("Failed to apply work mode: {}", e);
                    }
                }
            }
        }
    }

    fn print_stats(&self) {
        let bss = &self.skel.maps.bss_data;
        println!("--- GhostBrew Stats ---");
        println!("  Enqueued: {}", bss.nr_enqueued);
        println!(
            "  Dispatched: {} (direct: {})",
            bss.nr_dispatched, bss.nr_direct_dispatched
        );
        println!("  Gaming tasks: {}", bss.nr_gaming_tasks);
        println!("  Interactive tasks: {}", bss.nr_interactive_tasks);
        println!("  V-Cache migrations: {}", bss.nr_vcache_migrations);
        println!(
            "  CCD local: {} | cross: {}",
            bss.nr_ccd_local, bss.nr_ccd_cross
        );
        println!("  SMT idle picks: {}", bss.nr_smt_idle_picks);
        println!("  Compaction overflows: {}", bss.nr_compaction_overflows);
        println!("  Preempt kicks: {}", bss.nr_preempt_kicks);
        // Scheduling latency stats
        if bss.latency_count > 0 {
            let avg_latency_us = bss.latency_sum_ns / bss.latency_count / 1000;
            let min_latency_us = bss.latency_min_ns / 1000;
            let max_latency_us = bss.latency_max_ns / 1000;
            println!(
                "  Sched latency: avg {}us, min {}us, max {}us",
                avg_latency_us, min_latency_us, max_latency_us
            );
            // Gaming latency and frame pacing if we have gaming tasks
            if bss.gaming_latency_count > 0 {
                let gaming_avg_us = bss.gaming_latency_sum_ns / bss.gaming_latency_count / 1000;
                // Calculate jitter (standard deviation) from sum of squares
                // Variance = E[X^2] - E[X]^2 = (sum_sq/n) - (sum/n)^2
                let mean_us = bss.gaming_latency_sum_ns / bss.gaming_latency_count / 1000;
                let mean_sq = bss.gaming_latency_sum_sq / bss.gaming_latency_count;
                let variance = mean_sq.saturating_sub(mean_us * mean_us);
                let jitter_us = (variance as f64).sqrt() as u64;
                println!(
                    "  Gaming latency: avg {}us, jitter {}us ({} samples)",
                    gaming_avg_us, jitter_us, bss.gaming_latency_count
                );
                // Frame pacing stats
                let late_pct = if bss.gaming_latency_count > 0 {
                    (bss.gaming_late_frames * 100) / bss.gaming_latency_count
                } else {
                    0
                };
                if bss.gaming_late_frames > 0 || bss.gaming_preempted > 0 {
                    println!(
                        "  Frame pacing: {}% late (>1ms), {} preemptions",
                        late_pct, bss.gaming_preempted
                    );
                }
            }
        }
        // Phase 4a stats
        println!("  Proton tasks: {}", bss.nr_proton_tasks);
        println!("  Parent chain detects: {}", bss.nr_parent_chain_detects);
        println!(
            "  Userspace hint detects: {}",
            bss.nr_userspace_hint_detects
        );
        println!("  Prefcore placements: {}", bss.nr_prefcore_placements);
        if self.topology.is_x3d {
            println!("  V-Cache CCD: {}", self.topology.vcache_ccd.unwrap_or(0));
            if self.vcache_controller.is_available() {
                println!("  V-Cache mode: {}", self.vcache_controller.current_mode());
            }
            // Zen 5 asymmetric boost stats
            if self.topology.asymmetric_ccd_boost {
                println!("  Freq CCD placements: {}", bss.nr_freq_ccd_placements);
            }
            // Per-CCD load display
            self.print_ccd_loads();
        }
        // Game profiles loaded
        if self.profile_manager.count() > 0 {
            println!("  Game profiles: {}", self.profile_manager.count());
        }
        // Active profiles (matched running games)
        if !self.active_profiles.is_empty() {
            let profiles: Vec<&str> = self.active_profiles.values().map(|s| s.as_str()).collect();
            println!("  Active profiles: {}", profiles.join(", "));
        }
        // Phase 4b stats - GPU
        println!("  GPU feeder tasks: {}", bss.nr_gpu_feeder_tasks);
        if self.gpu_monitor.gpu_count() > 0 {
            println!(
                "  GPU: {} ({})",
                self.gpu_monitor.summary(),
                if self.gpu_monitor.any_gpu_active() {
                    "active"
                } else {
                    "idle"
                }
            );
        }
        // Phase 4c stats - VM/Container
        if bss.nr_vm_vcpu_tasks > 0 || self.vm_monitor.vm_count() > 0 {
            println!(
                "  VM vCPU tasks: {} (gaming: {}, dev: {})",
                bss.nr_vm_vcpu_tasks, bss.nr_gaming_vm_vcpus, bss.nr_dev_vm_vcpus
            );
        }
        if bss.nr_container_tasks > 0 || self.container_monitor.container_count() > 0 {
            println!(
                "  Container tasks: {} (AI: {})",
                bss.nr_container_tasks, bss.nr_ai_container_tasks
            );
        }
        if self.container_monitor.ollama_count() > 0 {
            println!(
                "  Ollama processes: {}",
                self.container_monitor.ollama_count()
            );
        }
        // Phase 4d stats - Cgroup classification
        if bss.nr_cgroup_classifications > 0 || self.cgroup_monitor.classified_count() > 0 {
            println!(
                "  Cgroup classifications: {} (gaming: {})",
                bss.nr_cgroup_classifications, bss.nr_cgroup_gaming
            );
            println!(
                "  Cgroups monitored: {} ({} gaming)",
                self.cgroup_monitor.classified_count(),
                self.cgroup_monitor.gaming_count()
            );
        }
        // Event streaming stats
        println!("  {}", self.event_handler.counters.summary());
        println!("---");
    }

    /// Print per-CCD load statistics
    fn print_ccd_loads(&self) {
        let vcache_ccd = self.topology.vcache_ccd.unwrap_or(0);
        let freq_ccd = self.topology.freq_ccd;

        for ccd in 0..self.topology.nr_ccds {
            let key = ccd.to_ne_bytes();
            if let Ok(Some(value)) = self
                .skel
                .maps
                .ccd_loads
                .lookup(&key, libbpf_rs::MapFlags::ANY)
                && value.len() >= 16
            {
                let nr_gaming = u64::from_ne_bytes(value[0..8].try_into().unwrap_or([0; 8]));
                let nr_tasks = u64::from_ne_bytes(value[8..16].try_into().unwrap_or([0; 8]));

                let label = if ccd == vcache_ccd {
                    "V-Cache"
                } else if freq_ccd == Some(ccd) {
                    "Freq"
                } else {
                    "CCD"
                };

                println!(
                    "  {} CCD{}: {} tasks ({} gaming)",
                    label, ccd, nr_tasks, nr_gaming
                );
            }
        }
    }

    /// Export stats to MangoHud-compatible CSV
    fn export_mangohud_stats(&mut self) {
        if self.mangohud_exporter.is_none() {
            return;
        }

        // Get per-CCD task counts first (before mutable borrow)
        let (ccd0_tasks, ccd1_tasks) = self.get_ccd_task_counts();

        let bss = &self.skel.maps.bss_data;

        // Calculate jitter from sum of squares
        let (jitter_us, late_pct) = if bss.gaming_latency_count > 0 {
            let mean_us = bss.gaming_latency_sum_ns / bss.gaming_latency_count / 1000;
            let mean_sq = bss.gaming_latency_sum_sq / bss.gaming_latency_count;
            let variance = mean_sq.saturating_sub(mean_us * mean_us);
            let jitter = (variance as f64).sqrt() as u64;
            let late = (bss.gaming_late_frames * 100) / bss.gaming_latency_count;
            (jitter, late)
        } else {
            (0, 0)
        };

        let stats = mangohud::SchedulerStats {
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            gaming_tasks: bss.nr_gaming_tasks,
            latency_avg_us: if bss.latency_count > 0 {
                bss.latency_sum_ns / bss.latency_count / 1000
            } else {
                0
            },
            latency_max_us: bss.latency_max_ns / 1000,
            jitter_us,
            late_pct,
            preemptions: bss.gaming_preempted,
            ccd0_tasks,
            ccd1_tasks,
        };

        if let Some(ref mut exporter) = self.mangohud_exporter
            && let Err(e) = exporter.write_sample(&stats)
        {
            debug!("Failed to write MangoHud stats: {}", e);
        }
    }

    /// Get task counts per CCD from BPF map
    fn get_ccd_task_counts(&self) -> (u64, u64) {
        let mut ccd0 = 0u64;
        let mut ccd1 = 0u64;

        for ccd in 0..self.topology.nr_ccds.min(2) {
            let key = ccd.to_ne_bytes();
            if let Ok(Some(value)) = self
                .skel
                .maps
                .ccd_loads
                .lookup(&key, libbpf_rs::MapFlags::ANY)
                && value.len() >= 16
            {
                let nr_tasks = u64::from_ne_bytes(value[8..16].try_into().unwrap_or([0; 8]));
                if ccd == 0 {
                    ccd0 = nr_tasks;
                } else {
                    ccd1 = nr_tasks;
                }
            }
        }

        (ccd0, ccd1)
    }
}

/// Analyze MangoHud frame time log and print statistics
fn analyze_frametime_log(path: Option<std::path::PathBuf>) -> Result<()> {
    use mangohud::MangoHudLogReader;

    let reader = MangoHudLogReader::new();

    let log_path = match path {
        Some(p) => p,
        None => reader
            .find_latest_log()
            .context("No MangoHud log found. Run a game with MangoHud first, or specify a path.")?,
    };

    println!("Analyzing: {}", log_path.display());
    println!();

    let frame_times = reader.read_frame_times(&log_path)?;

    if frame_times.is_empty() {
        bail!("No frame times found in log file");
    }

    let stats = MangoHudLogReader::analyze_frame_times(&frame_times);

    println!("╔══════════════════════════════════════════╗");
    println!("║      MangoHud Frame Time Analysis        ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("  Frames analyzed:  {}", stats.count);
    println!();
    println!("  Frame Time (ms):");
    println!("    Average:        {:.2}", stats.avg_ms);
    println!("    Min:            {:.2}", stats.min_ms);
    println!("    Max:            {:.2}", stats.max_ms);
    println!("    Std Dev:        {:.2} (jitter)", stats.std_dev_ms);
    println!();
    println!("  FPS:");
    println!("    Average:        {:.1}", stats.fps_avg);
    println!("    1% Low:         {:.1}", stats.fps_1_low);
    println!("    0.1% Low:       {:.1}", stats.fps_01_low);
    println!();

    // Quality assessment
    let quality = if stats.std_dev_ms < 1.0 {
        "Excellent (very consistent)"
    } else if stats.std_dev_ms < 2.0 {
        "Good (minor variance)"
    } else if stats.std_dev_ms < 5.0 {
        "Fair (noticeable hitches)"
    } else {
        "Poor (significant stuttering)"
    };
    println!("  Frame Pacing:     {}", quality);
    println!();

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle shell completions
    if let Some(shell) = args.completions {
        let mut cmd = Args::command();
        generate(shell, &mut cmd, "scx_ghostbrew", &mut std::io::stdout());
        return Ok(());
    }

    // Handle frame time analysis (doesn't require root or scheduler)
    if let Some(path_option) = args.analyze_frametime {
        return analyze_frametime_log(path_option);
    }

    // Initialize logging
    let log_level = if args.debug {
        "debug"
    } else if args.verbose {
        "info"
    } else {
        "warn"
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    info!("scx_{} v{}", SCHEDULER_NAME, env!("CARGO_PKG_VERSION"));

    // Check for root
    if !nix::unistd::Uid::effective().is_root() {
        bail!("scx_ghostbrew must be run as root");
    }

    // Check for sched-ext support
    if !std::path::Path::new("/sys/kernel/sched_ext").exists() {
        bail!("sched-ext not supported - ensure CONFIG_SCHED_CLASS_EXT=y in kernel");
    }

    // Log kernel info
    if let Ok(release) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        let release = release.trim();
        let kernel_type = if release.contains("cachyos") {
            if release.contains("lto") {
                "CachyOS LTO"
            } else {
                "CachyOS"
            }
        } else if release.contains("ghost") {
            "linux-ghost"
        } else if release.contains("zen") {
            "linux-zen"
        } else {
            "Linux"
        };
        info!("Kernel: {} ({})", release, kernel_type);
    }

    // Set up shutdown signal
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        info!("Received shutdown signal");
        shutdown_clone.store(true, Ordering::Relaxed);
    })
    .context("Failed to set signal handler")?;

    // Initialize and run scheduler
    let mut open_object = MaybeUninit::uninit();
    let mut scheduler = Scheduler::init(args, &mut open_object)?;
    scheduler.run(shutdown)
}
