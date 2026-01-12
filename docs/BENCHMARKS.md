# GhostBrew Benchmarks

This document describes how to run benchmarks, understand metrics, and compare GhostBrew against other schedulers.

## Table of Contents

1. [Running Benchmarks](#running-benchmarks)
2. [Metrics Collected](#metrics-collected)
3. [Comparison Methodology](#comparison-methodology)
4. [Hardware-Specific Results](#hardware-specific-results)

---

## Running Benchmarks

### Criterion Micro-Benchmarks

GhostBrew includes Criterion-based micro-benchmarks for measuring internal decision latencies.

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench -- task_classification
cargo bench -- cpu_selection
cargo bench -- intel_hybrid

# Generate HTML reports (in target/criterion/)
cargo bench -- --verbose
```

### Benchmark Groups

| Group | Description |
|-------|-------------|
| `task_classification` | Burst-based workload classification |
| `cpu_selection` | V-Cache/P-core CPU selection logic |
| `intel_hybrid` | Intel P-core/E-core selection |
| `dsq_dispatch` | Dispatch queue task selection |
| `gaming_pid_lookup` | Gaming process detection |
| `ccd_locality` | CCD/cluster locality calculations |

### Runtime Statistics

While running, GhostBrew can output live statistics:

```bash
# Print stats every 2 seconds
sudo scx_ghostbrew --stats --stats-interval 2

# Verbose mode with debug info
sudo scx_ghostbrew --stats -v
```

### Collecting Metrics

Use the metrics collector for long-running analysis:

```rust
use ghostbrew::bench::metrics::{MetricsCollector, SchedulerMetrics};
use std::time::Duration;

let mut collector = MetricsCollector::new(Duration::from_secs(1));

// Collect snapshots periodically
collector.record(metrics_from_bpf());

// Get aggregate statistics
if let Some(avg) = collector.average() {
    println!("Dispatch rate: {:.0}/s", avg.dispatch_rate);
    println!("Locality: {:.1}%", avg.locality_ratio * 100.0);
}
```

---

## Metrics Collected

### Core Scheduling Metrics

| Metric | Description | Ideal Value |
|--------|-------------|-------------|
| `nr_enqueued` | Tasks entering scheduler | N/A (workload dependent) |
| `nr_dispatched` | Tasks dispatched to CPUs | Close to nr_enqueued |
| `nr_direct_dispatched` | Fast-path dispatches | Higher is better |
| `dispatch_rate` | Tasks dispatched per second | Higher is better |

### Locality Metrics

| Metric | Description | Ideal Value |
|--------|-------------|-------------|
| `nr_ccd_local` | Dispatches within same CCD/cluster | Higher is better |
| `nr_ccd_cross` | Cross-CCD/cluster dispatches | Lower is better |
| `locality_ratio` | CCD-local / total dispatches | > 0.90 (90%) |

### Gaming/Interactive Metrics

| Metric | Description | Ideal Value |
|--------|-------------|-------------|
| `nr_gaming_tasks` | Gaming processes scheduled | Workload dependent |
| `nr_interactive_tasks` | Low-burst tasks (< threshold) | Higher for gaming |
| `nr_vcache_migrations` | Tasks moved to V-Cache CCD | Context dependent |
| `nr_pcore_placements` | Tasks placed on Intel P-cores | Higher for gaming |
| `nr_ecore_offloads` | Tasks offloaded to E-cores | Batch workloads |

### SMT Metrics

| Metric | Description | Ideal Value |
|--------|-------------|-------------|
| `nr_smt_idle_picks` | Chose CPU with idle SMT sibling | Higher is better |
| `smt_utilization_ratio` | Both siblings busy / total | Lower for latency |

### Preemption Metrics

| Metric | Description | Ideal Value |
|--------|-------------|-------------|
| `nr_preempt_kicks` | Times a task was preempted | Lower is better (less churn) |
| `nr_compaction_overflows` | DSQ compaction events | Lower is better |

---

## Comparison Methodology

### Schedulers to Compare

- **EEVDF** (kernel default): Linux 6.6+ default scheduler
- **scx_lavd**: Latency-aware virtual deadline scheduler
- **scx_rusty**: Rust-based general purpose scheduler
- **BORE** (if available): Burst-Oriented Response Enhancer

### Test Workloads

#### Gaming Benchmark
```bash
# Frame time consistency test
gamescope -f -r 120 -- ./your_game

# Measure while scheduler runs:
# - Average frame time
# - 99th percentile frame time
# - Frame time variance
# - Input latency (if measurable)
```

#### Compilation Benchmark
```bash
# Linux kernel compile (batch workload)
time make -j$(nproc) bzImage

# Compare total time and system time
```

#### Mixed Workload
```bash
# Run game + background compile simultaneously
# Measure gaming frame times during compile
```

### Measurement Tools

```bash
# Frame time analysis (MangoHud)
MANGOHUD=1 ./game
# Review ~/.local/share/MangoHud/game_name.csv

# CPU utilization per core
htop
# or
mpstat -P ALL 1

# Scheduler latency
sudo perf sched latency
sudo perf sched record ./workload
sudo perf sched latency -s max

# BPF-based latency
sudo bpftrace -e 'kprobe:pick_next_task_scx { @start[tid] = nsecs; }
                  kretprobe:pick_next_task_scx /@start[tid]/ {
                    @latency = hist(nsecs - @start[tid]);
                    delete(@start[tid]);
                  }'
```

### Benchmark Protocol

1. **Baseline**: Run workload with EEVDF (boot without sched-ext)
2. **GhostBrew**: `sudo scx_ghostbrew --gaming`
3. **Alternative**: `sudo scx_lavd` or `sudo scx_rusty`

For each:
- 3 runs minimum
- System idle for 30s before each run
- Same background processes
- Record: CPU governor, frequencies, thermals

### Expected Results

#### AMD X3D Systems

| Metric | EEVDF | GhostBrew | Improvement |
|--------|-------|-----------|-------------|
| Gaming 99%ile frame time | baseline | TBD | TBD |
| V-Cache utilization | ~50% | ~95% | +45% |
| CCD locality | ~60% | ~95% | +35% |

#### Intel Hybrid Systems

| Metric | EEVDF | GhostBrew | Improvement |
|--------|-------|-----------|-------------|
| Gaming P-core utilization | ~70% | ~95% | +25% |
| E-core batch offload | ~30% | ~80% | +50% |
| Frame time variance | baseline | TBD | TBD |

---

## Hardware-Specific Results

### AMD Ryzen 9 7950X3D (Zen4)

**Configuration:**
- Architecture: Zen4
- 16 cores / 32 threads
- CCD0: 8 cores with 96MB V-Cache
- CCD1: 8 cores standard
- GPU: NVIDIA RTX 5090 (PCIe 5.0 x16, 32GB BAR1, ReBAR enabled)
- Kernel: linux-cachyos-lto 6.18

**Live Test Results (December 2025):**

```
System: AMD Ryzen 9 7950X3D + RTX 5090
Mode: Gaming (--gaming flag)
V-Cache: cache mode via ghost-vcache sysfs
Background: Ollama, 4 VMs (10 dev vCPUs), 82 cgroups

Scheduler Statistics (10s sample):
  Enqueued:           69,624
  Dispatched:        265,956 (direct: 196,333)
  Gaming tasks:       17,424
  Interactive tasks:  51,102
  V-Cache migrations: 24,797
  CCD local:          24,871 (79.3%)
  CCD cross:           6,502 (20.7%)
  SMT idle picks:    127,376
  Prefcore placements:139,366
  GPU feeder tasks:       20
```

**Key Metrics:**

| Metric | Value | Notes |
|--------|-------|-------|
| CCD Locality | 79.3% | Local dispatches within same CCD |
| Direct Dispatch | 73.8% | Fast-path (no DSQ queuing) |
| SMT Awareness | 47.9% | Chose CPU with idle sibling |
| Prefcore Usage | 52.4% | Used AMD's preferred cores |
| V-Cache Migrations | 9.3% | Tasks moved to V-Cache CCD |

**Prefcore Rankings:**
- Best cores: CPUs 9, 13, 25, 29 (ranking 236)
- V-Cache CCD0: CPUs 0-15
- Frequency CCD1: CPUs 16-31

**Key Findings:**
- Gaming workloads benefit from V-Cache CCD placement
- Compile workloads benefit from frequency CCD
- Mixed workloads require dynamic switching
- Prefcore integration improves single-thread performance
- SMT-aware scheduling reduces contention

### AMD Ryzen 7 9800X3D

**Configuration:**
- 8 cores / 16 threads
- Single CCD with 96MB V-Cache

**Key Findings:**
- Simpler topology (no CCD selection needed)
- Focus on SMT-aware scheduling
- Prefcore rankings still valuable

### AMD Ryzen 9 9950X3D (Zen5)

**Configuration:**
- Architecture: Zen5
- 16 cores / 32 threads
- CCD0: 8 cores with 128MB V-Cache
- CCD1: 8 cores standard
- L3 Cache: 144 MiB total (16MB base + 128MB V-Cache)
- Max boost: 5998 MHz
- Kernel: linux-ghost-tkg 6.18 (with znver5 optimizations)

**Live Test Results (January 2026):**

```
System: AMD Ryzen 9 9950X3D
Mode: Gaming (--gaming flag)
V-Cache: frequency mode (gaming testing via amd_x3d_mode sysfs)
Background: System idle baseline

Baseline (EEVDF - no sched-ext):
  sysbench 1-thread:  2466.70 events/sec
  sysbench 16-thread: 37627.64 events/sec
```

**Prefcore Rankings:**
```
CCD0 (V-Cache):
  CPU 0:  ranking 186
  CPU 4:  ranking 196
  CPU 8:  ranking 236 (best)
  CPU 12: ranking 211

CCD1 (Frequency):
  CPU 16: ranking 186
  CPU 20: ranking 196
  CPU 24: ranking 236 (best)
  CPU 28: ranking 211
```

**Key Findings:**
- Same CCD topology as 7950X3D (dual CCD, V-Cache on CCD0)
- Zen5 architecture with higher boost clocks (up to 6GHz)
- V-Cache sysfs interface: AMDI0101:00
- Prefcore rankings distributed evenly across CCDs
- Best prefcore cores: CPU 8 (CCD0) and CPU 24 (CCD1)

### Intel Core i9-14900K

**Configuration:**
- 8 P-cores (16 threads) + 16 E-cores
- Total: 24 cores / 32 threads

**Key Findings:**
- P-cores essential for gaming frame time consistency
- E-cores valuable for background compilation
- E-core offload mode significantly impacts mixed workloads

---

## Benchmark Scripts

### Quick Comparison Script

```bash
#!/bin/bash
# compare_schedulers.sh

WORKLOAD="make -j$(nproc) vmlinux"
RESULTS_DIR="./benchmark_results"
mkdir -p "$RESULTS_DIR"

echo "=== Baseline (EEVDF) ==="
# Ensure no scx scheduler running
sudo killall scx_ghostbrew scx_lavd scx_rusty 2>/dev/null
sleep 5

/usr/bin/time -v $WORKLOAD 2>&1 | tee "$RESULTS_DIR/eevdf.txt"
make clean >/dev/null 2>&1

echo "=== GhostBrew ==="
sudo scx_ghostbrew &
SCX_PID=$!
sleep 2

/usr/bin/time -v $WORKLOAD 2>&1 | tee "$RESULTS_DIR/ghostbrew.txt"
make clean >/dev/null 2>&1

sudo kill $SCX_PID 2>/dev/null

echo "=== Results ==="
echo "EEVDF:"
grep "Elapsed" "$RESULTS_DIR/eevdf.txt"
echo "GhostBrew:"
grep "Elapsed" "$RESULTS_DIR/ghostbrew.txt"
```

### Gaming Frame Time Logger

```bash
#!/bin/bash
# log_frametimes.sh

# Requires MangoHud
export MANGOHUD=1
export MANGOHUD_CONFIG="log_duration=60,output_folder=$HOME/frametimes"

# Start scheduler
sudo scx_ghostbrew --gaming &
SCX_PID=$!
sleep 2

echo "Running game for 60 seconds..."
timeout 60 gamescope -r 120 -- ./your_game

sudo kill $SCX_PID 2>/dev/null

echo "Results in $HOME/frametimes/"
```

---

## Interpreting Results

### Good Signs
- Locality ratio > 90%
- Direct dispatch ratio > 70%
- SMT idle picks > 50% of dispatches
- Gaming 99%ile frame time improved
- Batch workload throughput maintained

### Warning Signs
- Locality ratio < 70% (excessive cross-CCD)
- High preempt kicks (scheduler thrashing)
- Compaction overflows increasing
- Frame time variance higher than baseline

### Tuning Based on Results

| Issue | Adjustment |
|-------|------------|
| High frame time variance | Lower `--burst-threshold` |
| Batch jobs too slow | Increase `--slice-ns` |
| V-Cache underutilized | Enable `--gaming` mode |
| P-cores overloaded | Set `--ecore-offload aggressive` |
| Too many E-core offloads | Set `--ecore-offload conservative` |

---

## Future Benchmarks

Planned additions:
- [ ] Automated frame time capture via MangoHud
- [ ] Steam game profile benchmarks
- [ ] Power consumption comparison
- [ ] Memory bandwidth utilization
- [ ] Cache miss rates via perf
