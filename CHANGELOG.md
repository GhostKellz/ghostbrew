# Changelog

All notable changes to GhostBrew will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-01-11

### Added

#### AMD Ryzen 9950X3D (Zen5) Support
- Full support for Zen5 X3D processors with 128MB V-Cache
- Verified on linux-ghost-tkg 6.18 with znver5 optimizations
- Baseline benchmarks documented in BENCHMARKS.md
- Prefcore ranking integration validated

#### Intel Hybrid CPU Support
- Full P-core vs E-core differentiation for 12th-14th gen Intel CPUs
- E-core offload modes: disabled, conservative, aggressive
- Intel Thread Director (ITD) capacity-based detection
- Turbo ranking integration for optimal core selection

#### Per-Game Profiles
- TOML-based game profile system
- Auto-detection by executable name or Steam App ID
- Per-game tunables (burst threshold, slice duration)
- V-Cache and SMT preference per profile
- 25+ pre-configured profiles including:
  - **Games**: Cyberpunk 2077, Baldur's Gate 3, Elden Ring, Counter-Strike 2,
    Path of Exile 2, Satisfactory, Marvel Rivals
  - **Streaming**: OBS Studio, Sunshine (game streaming host)
  - **VM Gaming**: Looking Glass (GPU passthrough)
  - **Productivity**: DaVinci Resolve, Kdenlive, Blender, HandBrake
  - **Development**: Compilation workloads
- 5 example configurations for different use cases:
  - Gaming, Productivity, Streaming, VM Gaming, AI/ML Workloads

#### V-Cache Coordination
- Integration with ghost-vcache tool from linux-ghost
- Automatic mode detection from sysfs
- Switching strategies: manual, automatic, follow_ghost_vcache
- Hysteresis to prevent rapid mode switching

#### Runtime Control Interface
- File-based control at `/run/ghostbrew/control`
- Runtime tunable updates without restart
- Commands: burst_threshold_ns, slice_ns, gaming_mode, work_mode

#### MangoHud Integration
- Scheduler stats export to MangoHud-compatible CSV
- `--benchmark` flag for benchmark mode
- `--analyze-frametime` to analyze MangoHud logs post-benchmark
- Frame time statistics: avg, min, max, std dev, 1%/0.1% lows
- Frame pacing quality assessment

#### Event Streaming
- BPF ringbuf for real-time scheduler events
- Event types: gaming detected, V-Cache migration, preempt kick, high latency
- Event counters for summary statistics
- Verbose event logging with `--verbose`

#### Configuration System
- TOML configuration file support
- Locations: `/etc/ghostbrew/config.toml`, `~/.config/ghostbrew/config.toml`
- All CLI options configurable via file
- Per-profile tunables override

#### Build & CI Improvements
- Hardware matrix CI with self-hosted AMD X3D and Intel hybrid runners
- Criterion micro-benchmarks for scheduler hot paths
- System benchmark script (`benches/system_bench.sh`)
- 14 integration tests covering BPF, sysfs, cgroups, and CLI
- Comprehensive troubleshooting guide

### Changed

- Improved BPF task classification chain (+744 lines)
- Enhanced topology detection for Zen5 processors
- Better burst detection with configurable thresholds
- Optimized CCD locality tracking

### Fixed

- All clippy warnings resolved
- Collapsed nested if statements for cleaner code
- Proper use of Rust idioms (let-chains, derive macros)

### Documentation

- BENCHMARKS.md with methodology and results framework
- TROUBLESHOOTING.md for common issues
- Expanded TUNING.md with profile examples
- Man page (scx_ghostbrew.1)

---

## [0.1.0] - 2025-12-17

### Added

#### Core Scheduler
- Initial sched-ext BPF scheduler implementation
- Per-CCD dispatch queues (DSQ_GAMING, DSQ_INTERACTIVE, DSQ_DEFAULT, DSQ_BATCH)
- Multi-level task classification chain
- BORE-inspired burst detection for interactive task prioritization
- Graceful fallback to EEVDF on exit or crash

#### AMD Topology Support
- Zen5/X3D CPU topology detection
- CCD (Core Complex Die) and CCX (Core Complex) mapping
- SMT sibling awareness
- NUMA node detection
- X3D V-Cache CCD identification (7800X3D, 7900X3D, 7950X3D, 9900X3D, 9950X3D)
- AMD Prefcore ranking integration

#### Gaming Detection
- Wine/Proton process detection via `/proc` scanning
- Comm name pattern matching (wine, proton, .exe, etc.)
- Parent process chain walking (up to 8 levels)
- Environment variable detection (WINEPREFIX, STEAM_COMPAT_DATA_PATH)
- Gaming task routing to V-Cache CCD

#### GPU & Hardware Integration
- NVIDIA GPU detection via `/proc/driver/nvidia/`
- Resizable BAR (ReBAR) detection
- GPU power state monitoring (D0/D3)
- AMD PBO/Prefcore integration
- Preferred core scheduling

#### Virtualization & Container Support
- KVM/QEMU VM detection
- vCPU thread identification and classification
- VM type detection (gaming, dev, AI) via command line parsing
- GPU passthrough detection via VFIO/IOMMU
- Docker/Podman/containerd container detection
- NVIDIA container runtime support
- Ollama/AI workload detection

#### Cgroup Classification
- Cgroup-based workload classification
- Pattern matching for gaming.slice, docker, podman, etc.
- Cgroup ID to workload class BPF map
- Periodic cgroup rescanning

#### Build & Release
- GitHub Actions CI workflow (build, fmt, clippy)
- GitHub Actions release workflow with Arch package
- Arch Linux PKGBUILD
- Fedora RPM spec

#### Statistics & Monitoring
- Core scheduling statistics (nr_scheduled, nr_migrations, nr_preemptions)
- Gaming statistics (nr_gaming_tasks, nr_vcache_placements, nr_proton_tasks)
- VM statistics (nr_vm_vcpu_tasks, gaming/dev breakdown)
- Container statistics (nr_container_tasks, nr_ai_tasks)
- Cgroup statistics (nr_cgroup_classifications)

### Technical Details

- Written in Rust (2024 edition) with BPF
- Uses libbpf-rs for BPF program loading
- Requires kernel with CONFIG_SCHED_CLASS_EXT=y
- Tested on Linux 6.18+ with CachyOS kernels

### Known Limitations

- No GUI configuration tool yet
- Limited telemetry/analytics integration
