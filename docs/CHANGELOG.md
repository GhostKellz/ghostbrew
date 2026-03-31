# Changelog

All notable changes to GhostBrew will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- 25+ pre-configured profiles

#### V-Cache Coordination
- Integration with ghost-vcache tool from linux-ghost
- Automatic mode detection from sysfs
- Switching strategies: manual, automatic, follow_ghost_vcache

#### Runtime Control Interface
- File-based control at `/run/ghostbrew/control`
- Runtime tunable updates without restart

#### MangoHud Integration
- Scheduler stats export to MangoHud-compatible CSV
- `--benchmark` flag for benchmark mode
- `--analyze-frametime` to analyze MangoHud logs post-benchmark

#### Event Streaming
- BPF ringbuf for real-time scheduler events
- Event types: gaming detected, V-Cache migration, preempt kick, high latency

### Changed

- Improved BPF task classification chain
- Enhanced topology detection for Zen5 processors
- Better burst detection with configurable thresholds

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
- X3D V-Cache CCD identification

#### Gaming Detection
- Wine/Proton process detection via `/proc` scanning
- Comm name pattern matching
- Parent process chain walking
- Gaming task routing to V-Cache CCD

#### Virtualization & Container Support
- KVM/QEMU VM detection
- Docker/Podman/containerd container detection
- Ollama/AI workload detection

### Technical Details

- Written in Rust (2024 edition) with BPF
- Uses libbpf-rs for BPF program loading
- Requires kernel with CONFIG_SCHED_CLASS_EXT=y
