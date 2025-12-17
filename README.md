<p align="center">
  <img src="assets/logo/ghostbrew.png" alt="GhostBrew" width="400">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Status-In%20Development-F97316?style=for-the-badge" alt="In Development">
  <a href="#sched-ext"><img src="https://img.shields.io/badge/sched--ext-BPF-3B82F6?style=for-the-badge" alt="sched-ext BPF"></a>
  <a href="#rust"><img src="https://img.shields.io/badge/Rust-Edition%202024-E44D26?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"></a>
</p>

<p align="center">
  <a href="#amd-zen5x3d"><img src="https://img.shields.io/badge/AMD-Zen5%2FX3D-ED1C24?style=for-the-badge&logo=amd&logoColor=white" alt="AMD Zen5/X3D"></a>
  <a href="#v-cache-aware"><img src="https://img.shields.io/badge/V--Cache-Aware-8B5CF6?style=for-the-badge" alt="V-Cache Aware"></a>
  <a href="#numa-topology"><img src="https://img.shields.io/badge/NUMA-Topology-10B981?style=for-the-badge" alt="NUMA Topology"></a>
</p>

<p align="center">
  <a href="#gaming-optimized"><img src="https://img.shields.io/badge/Gaming-Optimized-EC4899?style=for-the-badge&logo=steam&logoColor=white" alt="Gaming Optimized"></a>
  <a href="#burst-detection"><img src="https://img.shields.io/badge/Burst-Detection-06B6D4?style=for-the-badge" alt="Burst Detection"></a>
  <a href="#linux-ghost"><img src="https://img.shields.io/badge/linux--ghost-Integration-FCD34D?style=for-the-badge&logo=linux&logoColor=black" alt="linux-ghost Integration"></a>
</p>

---

## What is GhostBrew?

**GhostBrew** (`scx_ghostbrew`) is a custom sched-ext BPF scheduler designed specifically for AMD Zen5 and X3D processors. It combines BORE-inspired burst detection with hardware-aware scheduling to deliver optimal performance for gaming and desktop workloads.

Built in Rust with BPF, GhostBrew runs as a userspace scheduler that can be loaded and unloaded at runtime without kernel rebuilds.

> **Warning:** GhostBrew is currently an experimental proof-of-concept/MVP. It is under active development and not yet recommended for production use. Use at your own risk.

### Why GhostBrew?

| Feature | scx_lavd | scx_bpfland | scx_ghostbrew |
|:--------|:--------:|:-----------:|:-------------:|
| Gaming Optimized | Yes | Partial | **Yes** |
| X3D V-Cache Aware | No | No | **Yes** |
| Zen5 Topology Aware | Partial | Partial | **Yes** |
| BORE-style Burst Detection | No | No | **Yes** |
| CCD/CCX Affinity | No | No | **Yes** |
| Integrated V-Cache Switching | No | No | **Yes** |

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    scx_ghostbrew                         │
│                   (Userspace Rust)                       │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │  Topology   │  │   Burst     │  │    V-Cache      │  │
│  │  Detection  │  │  Tracking   │  │   Coordinator   │  │
│  │  (Zen5/X3D) │  │  (BORE-ish) │  │   (ghost-vcache)│  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
├─────────────────────────────────────────────────────────┤
│                ghostbrew.bpf.c (BPF)                     │
│         Actual scheduling decisions in kernel            │
├─────────────────────────────────────────────────────────┤
│                 EEVDF (Kernel Base)                      │
│              Fallback when scx not active                │
└─────────────────────────────────────────────────────────┘
```

---

## Features

### AMD Zen5/X3D

- **V-Cache CCD Detection** — Identifies which CCD has 3D V-Cache on X3D chips
- **CCD Affinity** — Pins latency-sensitive tasks to V-Cache CCD, productivity to frequency CCD
- **CCX-Local Scheduling** — Minimizes L3 cache misses by keeping tasks on same CCX
- **Preferred Core Awareness** — Integrates with AMD P-State preferred core ranking
- **NUMA Optimization** — Chiplet-aware memory locality

### Gaming Optimized

- **Burst Detection** — BORE-inspired algorithm favoring interactive over CPU-hog tasks
- **Wine/Proton Detection** — Identifies game processes by comm name patterns
- **Futex-Aware** — Prioritizes tasks waiting on game sync primitives
- **Low-Latency Wakeup** — Fast scheduling for audio and input threads
- **Frame-Pacing Hints** — Detects vsync patterns for smoother gameplay

### NUMA Topology

- **Per-CCD Scheduling Domains** — Separate queues per CCD
- **SMT Sibling Awareness** — Smart hyperthreading utilization
- **Memory Locality** — Prefers CPUs near task's memory allocations
- **Cross-CCD Migration Cost** — Penalizes unnecessary CCD hops

### Integration

- **ghost-vcache Coordination** — Syncs with V-Cache mode switching tool
- **linux-ghost Kernel** — Designed to work with GHOST scheduler kernel patch
- **Runtime Tunable** — Adjust parameters via sysfs without restart
- **Graceful Fallback** — If GhostBrew crashes, EEVDF takes over seamlessly

---

## Quick Start

### Prerequisites

```bash
# Arch Linux
sudo pacman -S rust clang llvm libbpf bpf-linker

# Kernel with sched-ext support (linux-ghost, linux-cachyos, 6.12+)
zcat /proc/config.gz | grep SCHED_CLASS_EXT
# Should show: CONFIG_SCHED_CLASS_EXT=y
```

### Build

```bash
git clone https://github.com/ghostkellz/ghostbrew.git
cd ghostbrew
cargo build --release
```

### Run

```bash
# Start GhostBrew scheduler
sudo ./target/release/scx_ghostbrew

# With gaming mode (V-Cache CCD preferred)
sudo ./target/release/scx_ghostbrew --gaming

# With verbose output
sudo ./target/release/scx_ghostbrew -v
```

### Systemd Service

```bash
# Install service
sudo cp scx-ghostbrew.service /etc/systemd/system/

# Start
sudo systemctl start scx-ghostbrew

# Enable on boot
sudo systemctl enable scx-ghostbrew

# Check status
systemctl status scx-ghostbrew
```

---

## Configuration

### Command Line Options

```
scx_ghostbrew [OPTIONS]

OPTIONS:
    -g, --gaming          Gaming mode (prefer V-Cache CCD)
    -p, --productivity    Productivity mode (prefer frequency CCD)
    -a, --auto            Auto-detect workload (default)
    -v, --verbose         Verbose logging
    -s, --stats           Print scheduler statistics
    --burst-threshold     Burst detection threshold (default: 2000000ns)
    --migrate-cost        Cross-CCD migration cost (default: 500000ns)
```

### Runtime Tuning

```bash
# View current settings
cat /sys/kernel/sched_ext/ghostbrew/*

# Adjust burst threshold
echo 1500000 | sudo tee /sys/kernel/sched_ext/ghostbrew/burst_threshold_ns

# Toggle gaming mode
echo 1 | sudo tee /sys/kernel/sched_ext/ghostbrew/gaming_mode
```

---

## Project Structure

```
ghostbrew/
├── Cargo.toml               # Rust project config
├── src/
│   ├── main.rs              # Entry point, CLI handling
│   ├── scheduler.rs         # Core scheduler coordination
│   ├── topology.rs          # Zen5/X3D topology detection
│   ├── burst.rs             # Burst tracking (BORE-inspired)
│   ├── vcache.rs            # V-Cache CCD coordination
│   └── bpf/
│       ├── ghostbrew.bpf.c  # BPF scheduler logic
│       └── vmlinux.h        # Kernel type definitions
├── scx-ghostbrew.service    # Systemd unit
├── docs/
│   ├── ARCHITECTURE.md      # Deep dive on design
│   ├── TUNING.md            # Performance tuning guide
│   └── BENCHMARKS.md        # Benchmark results
└── archive/                 # Reference material (untracked)
```

---

## How It Works

### Task Classification

GhostBrew classifies tasks into categories:

| Category | Detection | Scheduling |
|----------|-----------|------------|
| **Gaming** | comm contains wine, proton, game names | V-Cache CCD, highest priority |
| **Interactive** | Low burst time, frequent wakeups | V-Cache CCD, high priority |
| **Audio/Input** | comm contains pipewire, pulseaudio | Any CCD, realtime priority |
| **Productivity** | High burst time, CPU-bound | Frequency CCD, normal priority |
| **Background** | Nice > 0, low priority hints | Any available CPU, lowest priority |

### Burst Detection

Inspired by BORE scheduler:

```
burst_score = runtime_since_last_sleep

if burst_score < threshold:
    task = interactive (favor)
else:
    task = cpu_hog (deprioritize)
```

### V-Cache Affinity

For X3D processors (7950X3D, 9950X3D):

```
CCD0 (V-Cache): Gaming, latency-sensitive
CCD1 (Frequency): Compiling, rendering, background
```

GhostBrew automatically detects which CCD has V-Cache and routes tasks accordingly.

---

## Benchmarks

> **Coming Soon** — Benchmarks comparing GhostBrew vs scx_lavd vs BORE

Planned tests:
- Game frame times (1% lows, 0.1% lows)
- Input latency
- Compile times (parallel builds)
- Mixed workload (gaming + streaming)

---

## Compatibility

### Supported CPUs

| CPU | V-Cache Aware | Tested |
|-----|:-------------:|:------:|
| AMD Ryzen 9 9950X3D | Yes | WIP |
| AMD Ryzen 9 9900X3D | Yes | WIP |
| AMD Ryzen 9 7950X3D | Yes | WIP |
| AMD Ryzen 9 7900X3D | Yes | WIP |
| AMD Ryzen 7 7800X3D | Yes | WIP |
| AMD Ryzen 9 9950X | No (Zen5 opts only) | WIP |
| AMD Ryzen 9 9900X | No (Zen5 opts only) | WIP |
| Other Zen4/Zen5 | No (generic sched-ext) | WIP |

### Supported Kernels

- **linux-ghost** (recommended)
- linux-cachyos (6.12+)
- linux-zen (6.12+)
- Mainline Linux (6.12+)

Requires `CONFIG_SCHED_CLASS_EXT=y`

---

## Related Projects

- [linux-ghost](https://github.com/ghostkellz/linux-ghost) — Custom kernel with GHOST scheduler patch
- [scx](https://github.com/sched-ext/scx) — sched-ext schedulers (scx_lavd, scx_bpfland, etc.)
- [BORE](https://github.com/firelzrd/bore-scheduler) — Burst-Oriented Response Enhancer (inspiration)

---

## Contributing

GhostBrew is in early development. Contributions welcome!

- **Testing** — Run on your Zen5/X3D system, report issues
- **Benchmarks** — Help establish performance baselines
- **Code** — Check [docs/](docs/) for architecture and implementation details

---

<p align="center">
  <i>Brewing the perfect schedule for AMD Zen.</i>
</p>
