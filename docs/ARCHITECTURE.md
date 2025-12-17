# GhostBrew Architecture

This document describes the internal architecture of scx_ghostbrew, a sched-ext BPF scheduler optimized for AMD Zen5/X3D processors.

## Overview

GhostBrew is a hybrid userspace/BPF scheduler that uses the Linux sched-ext framework. The architecture splits responsibilities:

- **BPF (kernel)**: Real-time scheduling decisions, task classification, CPU selection
- **Userspace (Rust)**: Topology detection, workload monitoring, BPF map population, statistics

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Userspace (Rust)                            │
├─────────────────────────────────────────────────────────────────────┤
│  main.rs          │ Entry point, CLI, main loop                     │
│  topology.rs      │ CPU/CCD/CCX/NUMA detection                      │
│  gaming.rs        │ Gaming process detection (Wine/Proton)          │
│  cgroup.rs        │ Cgroup-based workload classification            │
│  container.rs     │ Docker/Podman/Ollama detection                  │
│  vm.rs            │ KVM/QEMU VM detection and classification        │
│  gpu.rs           │ NVIDIA GPU detection, ReBAR awareness           │
│  pbo.rs           │ AMD PBO/Prefcore integration                    │
├─────────────────────────────────────────────────────────────────────┤
│                            BPF Maps                                  │
│  (Shared state between userspace and BPF)                           │
├─────────────────────────────────────────────────────────────────────┤
│                      ghostbrew.bpf.c (BPF)                          │
│  select_cpu()     │ CPU selection with CCD/topology awareness       │
│  enqueue()        │ Task enqueue with priority adjustment           │
│  dispatch()       │ Per-CPU dispatch from DSQs                      │
│  running()        │ Track task execution, burst time                │
│  stopping()       │ Update burst score on task stop                 │
│  is_gaming_task() │ Multi-level gaming detection                    │
├─────────────────────────────────────────────────────────────────────┤
│                    Linux Kernel (EEVDF base)                        │
└─────────────────────────────────────────────────────────────────────┘
```

## BPF Scheduler Core

### Task Classification Chain

GhostBrew uses a multi-level detection chain for task classification:

```
Level 1: PID Maps (userspace-populated)
    │
    ├── gaming_pids      → GAMING priority
    ├── vm_vcpu_pids     → VM workload type
    └── container_pids   → Container workload type
    │
Level 2: Comm Name Patterns (BPF string matching)
    │
    ├── "wine", "proton", ".exe"  → GAMING
    ├── "CPU N/KVM"               → VM vCPU
    └── "pipewire", "pulseaudio"  → INTERACTIVE
    │
Level 3: Cgroup Classification (userspace-populated)
    │
    ├── gaming.slice, steam       → GAMING
    ├── docker, podman            → CONTAINER
    └── system.slice              → BATCH
    │
Level 4: Parent Process Walking (BPF)
    │
    └── Walk up to 8 ancestors looking for gaming patterns
    │
Level 5: Burst Detection (BORE-inspired)
    │
    └── Runtime analysis for interactive vs CPU-bound
```

### Workload Types

```c
#define WORKLOAD_GAMING      1   // Gaming tasks - lowest latency
#define WORKLOAD_INTERACTIVE 2   // Interactive - low latency
#define WORKLOAD_BATCH       3   // Background - throughput
#define WORKLOAD_AI          4   // AI/ML - throughput, GPU affinity
#define WORKLOAD_CONTAINER   7   // Containerized workloads
```

### Dispatch Queues (DSQs)

GhostBrew uses multiple dispatch queues for workload isolation:

| DSQ | Purpose | Priority |
|-----|---------|----------|
| `DSQ_GAMING` | Gaming/latency-critical tasks | Highest |
| `DSQ_INTERACTIVE` | Desktop, UI, audio | High |
| `DSQ_DEFAULT` | General tasks | Normal |
| `DSQ_BATCH` | Background, system tasks | Low |

### CPU Selection Algorithm

```c
select_cpu(task):
    1. Check if task has preferred CPU (prev_cpu valid?)
    2. Check if prev_cpu is idle → use it (cache hot)
    3. If gaming task:
       - Prefer V-Cache CCD (X3D) or preferred cores
       - Search within CCD for idle CPU
    4. Search same CCX for idle CPU (L3 locality)
    5. Search same CCD for idle CPU
    6. Fall back to any idle CPU
    7. Return prev_cpu if nothing found (let dispatch handle it)
```

## Userspace Components

### Topology Detection (`topology.rs`)

Reads from `/sys/devices/system/cpu/` to build:

- CPU → Core → CCX → CCD → NUMA node mappings
- SMT sibling relationships
- X3D V-Cache CCD identification
- AMD Prefcore rankings

### Gaming Detection (`gaming.rs`)

Scans `/proc` for gaming processes:

- Wine/Proton processes via `/proc/[pid]/exe` symlinks
- Environment variables: `WINEPREFIX`, `STEAM_COMPAT_DATA_PATH`
- Comm name patterns for known games
- Parent process chain walking

### Cgroup Classification (`cgroup.rs`)

Scans `/sys/fs/cgroup` hierarchy:

- Pattern matching on cgroup paths
- Gaming patterns: `gaming.slice`, `steam`, `proton`
- Container patterns: `docker`, `libpod`, `containerd`
- Returns cgroup ID (inode) → workload class mapping

### VM Detection (`vm.rs`)

Detects and classifies virtual machines:

- QEMU/KVM process detection
- vCPU thread identification (`CPU N/KVM` pattern)
- VM type classification (gaming, dev, AI) via command line parsing
- GPU passthrough detection via VFIO/IOMMU

### Container Detection (`container.rs`)

Monitors containerized workloads:

- Docker, Podman, containerd containers
- NVIDIA container runtime integration
- Ollama/AI workload detection
- GPU container identification

### GPU Integration (`gpu.rs`)

NVIDIA GPU awareness:

- GPU count and capabilities
- ReBAR (Resizable BAR) detection
- Power state monitoring (D0/D3)
- GPU-active signaling to BPF

## BPF Maps

### Core Maps

| Map | Type | Key | Value | Purpose |
|-----|------|-----|-------|---------|
| `cpu_to_ccd` | ARRAY | cpu_id | ccd_id | Topology |
| `cpu_to_ccx` | ARRAY | cpu_id | ccx_id | Topology |
| `vcache_cpus` | ARRAY | index | cpu_mask | V-Cache CPUs |
| `prefcore_rankings` | ARRAY | cpu_id | ranking | AMD Prefcore |

### Task Classification Maps

| Map | Type | Key | Value | Purpose |
|-----|------|-----|-------|---------|
| `gaming_pids` | HASH | pid | gaming_type | Gaming PIDs |
| `vm_vcpu_pids` | HASH | pid | vm_workload | VM vCPU threads |
| `container_pids` | HASH | pid | workload_type | Container PIDs |
| `cgroup_classes` | HASH | cgroup_id | workload_class | Cgroup classification |

### Per-Task Context

```c
struct task_ctx {
    u64 burst_time;      // Runtime since last sleep
    u64 last_run_at;     // Timestamp of last run
    u32 preferred_ccd;   // Preferred CCD for this task
    u32 gaming_detected; // Cached gaming detection result
    u8  workload_class;  // Cached workload classification
};
```

## Statistics

GhostBrew exports statistics via BPF globals:

### Core Statistics
- `nr_scheduled` - Total tasks scheduled
- `nr_migrations` - Cross-CCD migrations
- `nr_preemptions` - Task preemptions

### Gaming Statistics
- `nr_gaming_tasks` - Gaming task dispatches
- `nr_vcache_placements` - Tasks placed on V-Cache CCD
- `nr_proton_tasks` - Wine/Proton tasks detected

### Workload Statistics
- `nr_vm_vcpu_tasks` - VM vCPU tasks scheduled
- `nr_container_tasks` - Container tasks scheduled
- `nr_ai_tasks` - AI/ML tasks scheduled
- `nr_cgroup_classifications` - Cgroup-based classifications

## Error Handling

GhostBrew is designed for graceful degradation:

1. **BPF Verification Failure**: Falls back to EEVDF
2. **Topology Detection Failure**: Uses conservative defaults
3. **Map Population Failure**: Detection continues with reduced accuracy
4. **Runtime Errors**: Logged, scheduler continues

If the scheduler crashes or is killed, the kernel automatically falls back to EEVDF scheduling.

## Build System

The build process:

1. `build.rs` generates BPF skeleton from `ghostbrew.bpf.c`
2. Clang compiles BPF code with kernel headers
3. libbpf-cargo generates Rust bindings
4. Cargo compiles the userspace binary

Required tools:
- Rust (2024 edition)
- Clang/LLVM with BPF target
- libbpf
- bpftool
- Kernel headers with BTF

## Future Directions

- Frame-pacing detection for smoother gaming
- GPU scheduler coordination
- Intel hybrid (P-core/E-core) support
- Power efficiency modes
- Per-game profiles
