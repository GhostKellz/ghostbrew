# Changelog

All notable changes to GhostBrew will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3] - 2026-06-15

### Added

- Linux 7.1 listed as a supported/recommended kernel. GhostBrew uses the built-in
  `scx_bpf_select_cpu_dfl()` path, so it inherits 7.1's idle SMT-sibling preference
  in default CPU selection automatically (no scheduler changes required)
- Dependabot configuration for `cargo` and `github-actions` ecosystems (weekly,
  grouped minor/patch PRs)
- `Security Audit` workflow running `cargo audit --deny warnings` on dependency
  changes, on a weekly schedule, and on demand

### Changed

- Refreshed crate lockfile to latest in-semver versions (bitflags, cc, chrono, log,
  regex, time, zerocopy, and related)

### Removed

- Dropped the unused `notify` dependency, which also removes the unmaintained
  `instant` advisory (RUSTSEC-2024-0384) and 10 other transitive crates

### Security

- Added `.cargo/audit.toml` allowlisting the unmaintained transitive `paste`
  advisory (RUSTSEC-2024-0436, pulled in via `scx_utils`) so audits stay green

---

## [0.3.2] - 2026-05-26

### Changed

- Upgraded scx_utils and scx_stats from 1.0.x to 1.1.x (fixes API incompatibility between scx_utils 1.0.8 and scx_stats 1.0.22)
- Updated packaging for Arch, Fedora, and Debian
- CLI version now derives from `Cargo.toml` (`CARGO_PKG_VERSION`) instead of a hardcoded string

### Security

- PKGBUILD source checksum is now pinned at release time via `updpkgsums` (CI) instead of `SKIP`, so `makepkg` verifies the downloaded source tarball

---

## [0.3.1] - 2026-04-24

### Added

- `ghostbrew` front-end CLI with `run`, `support`, `benchmark`, and `completions` subcommands
- Support bundle workflow with text and JSON output under `~/.local/state/ghostbrew/support/`
- Benchmark workflow with text and JSON output under `~/.local/state/ghostbrew/benchmarks/`
- Release/install scripts for hosted installs and reproducible Linux release bundles
- Shell completion packaging for `ghostbrew` and `scx_ghostbrew` across bash, zsh, and fish
- GitHub issue templates for bug reports, feature requests, and support intake
- SECURITY.md with vulnerability reporting guidelines and security considerations

### Fixed

- Version string alignment across Cargo.toml, CLI, BPF, and packaging files
- Clippy warnings for manual checked division patterns (use `checked_div()`)
- Zen 5 X3D mode sync so ghost-vcache `frequency` mode enables GhostBrew `work_mode`
- AMD CCD/CCX topology detection to prefer `die_cpus_list` / `die_id` and ignore invalid `cluster_id` values
- Linux packaging/install drift across Arch, Fedora, Debian, man pages, completions, and release assets

### Changed

- Updated dependencies to latest compatible versions
- Pinned scx_stats to 1.0.x for scx_utils 1.0.8 compatibility
- Added regression coverage for topology helpers and Zen 5 X3D mode behavior
- Expanded dev/build workload classification and profile tunables for 9950X3D frequency-CCD steering
- Reorganized `docs/` into a cleaner foldered layout with `docs/README.md` as the root index
- Moved checked-in benchmark example reports under `docs/benchmarks/`
- Updated packaging files (PKGBUILD, spec, Debian packaging, installer, release bundle) for `v0.3.1`

---

## [0.3.0] - 2026-03-31

### Added

#### Wakeup Frequency Tracking
- Tracks inter-wakeup intervals using EWMA for accurate frequency detection
- Differentiates legitimate latency-sensitive apps from busyloop offenders
- Applies vtime penalty to high-frequency wakers (>50Hz) that aren't gaming tasks
- New stats: `nr_high_wakeup_tasks`, `nr_wakeup_penalties`

#### SMT Contention Avoidance
- Detects when SMT sibling is running compute-heavy workloads
- Migrates latency-sensitive tasks away from contended SMT pairs
- New stat: `nr_smt_contention_avoids`

#### Futex-Aware Scheduling
- fexit hooks for `__futex_wait` and `futex_wake` system calls
- Priority boost (2x) for tasks holding futex locks
- Reduces lock holder preemption for better gaming thread synchronization
- New stat: `nr_futex_boosts`

#### Core Compaction / Power Mode
- Consolidates tasks onto fewer cores during low system utilization
- Three modes: `off`, `balanced` (50% compaction), `aggressive` (25% compaction)
- Gaming/interactive tasks always bypass compaction
- CLI flag: `--power-save [off|balanced|aggressive]`
- New stat: `nr_power_compactions`

#### Tickless Mode
- Grants infinite time slices when no contention exists
- Reduces timer interrupt overhead during single-task periods
- Automatic starvation prevention in ops.tick
- CLI flag: `--tickless`

#### Per-Game Latency Histograms
- 16-bucket logarithmic histogram for gaming latency distribution
- P50/P95/P99 percentile calculation exported to MangoHud CSV
- Extended CSV format with `latency_p50_us`, `latency_p95_us`, `latency_p99_us` columns

#### GPU Scheduler Coordination
- GPU utilization monitoring via AMD sysfs and nvidia-smi
- GpuBottleneck detection: `GpuBound`, `CpuBound`, `Balanced`
- Foundation for CPU/GPU coordination in future releases

#### DL Server Integration
- Automatic detection of DL server support (kernel 7.0+)
- Documentation of RT starvation protection benefits
- New docs: `docs/features/dl-server.md`

#### NUMA-Aware Game Profiles
- Per-profile NUMA node preference: `auto`, `gpu_local`, `node0`, `node1`, `spread`
- Explicit CPU affinity lists in game profiles
- BPF map `numa_hints` for userspace-driven NUMA placement

#### BPF Arena Placeholder
- Detection scaffolding for BPF Arena (kernel 6.18+)
- `arena.rs` module with version detection

### Changed

- Extended `task_ctx` with wakeup tracking, flags, and futex state
- Extended `runtime_tunables` with power_save_mode, tickless, gpu_bound_mode
- Enhanced `SchedulerStats` with percentile fields

---

## [0.2.2] - 2026-02-19

### Added

#### Linux 7.0 Kernel Compatibility
- Synced sched-ext headers from kernel 7.0-rc (`compat.bpf.h`, `common.bpf.h`, etc.)
- Regenerated `vmlinux.h` from running kernel BTF with conflict filtering
- Full compatibility with 6.19 DL server changes (RT starvation fix)

### Changed

#### Dependencies
- Bumped libbpf-rs and libbpf-cargo from 0.24 to 0.26
- Updated BSS/rodata access patterns for new Option-wrapped API

#### Build System
- vmlinux.h now filters conflicting extern declarations that clash with compat inline wrappers
- Improved forward compatibility with kernel API evolution

### Fixed

- BSS data access now properly handles Option wrapper (`bss_data.as_ref()`)
- Rodata access uses explicit unwrap with context (`rodata_data.as_mut().expect(...)`)

### Documentation

- Updated CODE_REVIEW.md with v7 compatibility checklist and discovered watch-outs
- Documented sched-ext API changes between 6.19 and 7.0

---

## [0.2.1] - 2026-01-15

### Fixed

- Resolved all clippy warnings for Rust 1.92
- Minor code quality improvements

---

## [0.2.0] - 2026-01-11

### Added

#### AMD Ryzen 9950X3D (Zen5) Support
- Full support for Zen5 X3D processors with 128MB V-Cache
- Verified on linux-ghost-tkg 6.18 with znver5 optimizations
- Baseline benchmarks documented in `docs/benchmarks.md`
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

- `docs/benchmarks.md` with methodology and results framework
- `docs/guides/troubleshooting.md` for common issues
- Expanded `docs/guides/tuning.md` with profile examples
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
