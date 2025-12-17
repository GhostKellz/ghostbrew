# Changelog

All notable changes to GhostBrew will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

- Intel CPU support is minimal (basic scheduling only)
- No GUI configuration tool yet
- Per-game profiles not yet implemented
