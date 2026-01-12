# GhostBrew Tuning Guide

This guide covers performance tuning for scx_ghostbrew across different workloads.

## Command Line Options

```
scx_ghostbrew [OPTIONS]

Mode Selection:
    -g, --gaming          Gaming mode - prefer V-Cache CCD for latency-sensitive tasks
    -w, --work            Work mode - prefer frequency CCD for higher boost
    -a, --auto            Auto-detect workload and adjust (default)

Tuning:
    --burst-threshold     Burst detection threshold in nanoseconds (default: 2000000)
    --slice-ns            Time slice in nanoseconds (default: 3000000)
    --ecore-offload       E-core offload mode for Intel: disabled, conservative, aggressive

Output:
    -s, --stats           Print scheduler statistics periodically
    --stats-interval      Statistics interval in seconds (default: 2)
    -b, --benchmark       Benchmark mode - export stats to MangoHud-compatible CSV
    -v, --verbose         Enable verbose logging
    -d, --debug           Enable debug logging (very verbose)

Other:
    --completions         Generate shell completions (bash, zsh, fish, powershell)
```

## Runtime Tuning

GhostBrew supports runtime tuning without restarting the scheduler.

### Control File Interface

Write commands to `/run/ghostbrew/control` to update tunables at runtime:

```bash
# Set burst threshold (nanoseconds)
echo "burst_threshold_ns=1500000" > /run/ghostbrew/control

# Set time slice (nanoseconds)
echo "slice_ns=2500000" > /run/ghostbrew/control

# Enable/disable gaming mode
echo "gaming_mode=true" > /run/ghostbrew/control

# Enable/disable work mode
echo "work_mode=true" > /run/ghostbrew/control
```

Multiple commands can be written at once:

```bash
cat > /run/ghostbrew/control << 'EOF'
burst_threshold_ns=1000000
slice_ns=2500000
gaming_mode=true
EOF
```

### Game Profiles

Game-specific tunables are automatically applied when a game is detected:

```bash
# Create a custom profile
mkdir -p ~/.config/ghostbrew/profiles
cat > ~/.config/ghostbrew/profiles/mygame.toml << 'EOF'
name = "My Game"
exe_name = "mygame.exe"

[tunables]
burst_threshold_ns = 1500000
slice_ns = 2500000
EOF
```

When ghostbrew detects `mygame.exe` running, it will automatically apply these tunables.

### V-Cache Integration

If using [ghost-vcache](https://github.com/ghostkellz/ghost-vcache), mode changes are automatically reflected in the scheduler:

```bash
# Switch to cache mode (for gaming)
ghost-vcache cache

# Switch to frequency mode (for productivity)
ghost-vcache frequency
```

## Workload Profiles

### Gaming

For optimal gaming performance:

```bash
sudo scx_ghostbrew -g -s
```

What this does:
- Prioritizes gaming tasks (Wine/Proton) above all others
- Routes gaming tasks to V-Cache CCD on X3D processors
- Reduces scheduling latency for game threads
- Shows real-time stats

### Mixed Gaming + Streaming

When gaming while streaming or recording:

```bash
sudo scx_ghostbrew -g
```

This balances:
- Gaming tasks get V-Cache CCD
- OBS/streaming can use frequency CCD for encoding
- Audio (PipeWire) gets high priority on any CCD

### Development/Compilation

For compilation and development workloads:

```bash
sudo scx_ghostbrew
```

Default mode provides:
- Good parallelism across all CCDs
- Interactive tasks (IDE, terminal) stay responsive
- Compilers spread across available cores

### AI/ML Workloads

For AI inference (Ollama, etc.):

```bash
sudo scx_ghostbrew -v
```

GhostBrew automatically:
- Detects Ollama and AI containers
- Classifies as batch workload (throughput-oriented)
- Avoids interfering with interactive tasks

### Virtual Machines

For VM workloads (QEMU/KVM):

```bash
sudo scx_ghostbrew -v
```

GhostBrew automatically:
- Detects vCPU threads
- Classifies VMs by type (gaming, dev, AI)
- Routes gaming VM vCPUs to V-Cache CCD
- Respects libvirt CPU pinning when detected

## AMD X3D Optimization

### V-Cache CCD Selection

On X3D processors, GhostBrew identifies the V-Cache CCD:

| Processor | V-Cache CCD | Frequency CCD |
|-----------|-------------|---------------|
| 9950X3D | CCD0 | CCD1 |
| 9900X3D | CCD0 | CCD1 |
| 7950X3D | CCD0 | CCD1 |
| 7900X3D | CCD0 | CCD1 |
| 7800X3D | CCD0 (only) | - |

### Task Routing

| Task Type | Target CCD | Reason |
|-----------|------------|--------|
| Gaming | V-Cache | Cache-sensitive workload |
| Interactive | V-Cache | Low latency |
| Compilation | Frequency | Clock speed benefits |
| Streaming | Frequency | Encoding benefits |
| Background | Any | Load balancing |

### Verifying V-Cache Detection

```bash
# Check if V-Cache CCD detected
sudo scx_ghostbrew -v 2>&1 | grep -i vcache

# Example output:
# X3D V-Cache CCD detected: 0
# V-Cache CPUs: 0-15
```

## AMD Prefcore Integration

GhostBrew reads AMD Prefcore rankings when available:

```bash
# Check prefcore status
cat /sys/devices/system/cpu/amd_pstate/prefcore

# View per-CPU rankings
for i in /sys/devices/system/cpu/cpufreq/policy*/amd_pstate_prefcore_ranking; do
    echo "$i: $(cat $i)"
done
```

Higher rankings indicate preferred cores (better silicon). GhostBrew uses these rankings to select optimal CPUs for latency-sensitive tasks.

## Container Workloads

### Docker/Podman

GhostBrew automatically detects containers via cgroup patterns:

```bash
# Containers are detected from:
# - /sys/fs/cgroup/docker/
# - /sys/fs/cgroup/libpod/
# - /sys/fs/cgroup/containerd/
```

### Ollama

Ollama processes are specifically detected and classified as AI workloads:

```bash
# Check Ollama detection
sudo scx_ghostbrew -v 2>&1 | grep -i ollama
```

### NVIDIA Container Runtime

If nvidia-container-runtime is present, GPU containers receive appropriate scheduling.

## Virtual Machine Workloads

### Gaming VMs

VMs with GPU passthrough for gaming are detected via:
- VM name containing "gaming", "windows", "game"
- VFIO-bound GPU detected
- Looking Glass client running

Gaming VMs get:
- vCPU threads routed to V-Cache CCD
- Priority boost similar to native gaming

### Development VMs

General-purpose VMs get batch priority to avoid interfering with host interactive tasks.

### Checking VM Detection

```bash
sudo scx_ghostbrew -v 2>&1 | grep -i "vm\|qemu\|vcpu"
```

## Monitoring Performance

### Built-in Statistics

```bash
# Run with stats output
sudo scx_ghostbrew -v

# Example output every 2 seconds:
# === GhostBrew Statistics ===
# Scheduled: 15234 | Gaming: 842 | Interactive: 3421
# V-Cache placements: 1203 | Cross-CCD migrations: 45
# VM vCPUs: 4 (gaming: 0, dev: 4)
# Cgroups: 78 (gaming: 0, container: 4)
```

### Key Metrics

| Metric | Good Value | Concern |
|--------|------------|---------|
| Gaming tasks | >0 when gaming | 0 during gameplay |
| V-Cache placements | High during gaming | Low = detection issue |
| Cross-CCD migrations | Low (<5% of scheduled) | High = thrashing |
| Preemptions | Moderate | Very high = contention |

### External Monitoring

```bash
# CPU usage per core
htop

# Per-CCD temperature (for X3D)
watch -n1 "sensors | grep Tctl"

# sched-ext status
cat /sys/kernel/sched_ext/state
```

## Troubleshooting

### Gaming Not Detected

If gaming tasks aren't being prioritized:

1. Check if the game process is detected:
```bash
sudo scx_ghostbrew -v 2>&1 | grep -i gaming
```

2. Verify Wine/Proton process names:
```bash
ps aux | grep -E "wine|proton|\.exe"
```

3. Check cgroup classification:
```bash
cat /proc/$(pgrep -f "your-game")/cgroup
```

### High Cross-CCD Migration

If you see excessive migrations:

1. Check task affinity settings
2. Verify topology detection:
```bash
sudo scx_ghostbrew -v 2>&1 | head -50
```

### Scheduler Not Loading

If GhostBrew fails to load:

1. Check kernel support:
```bash
zcat /proc/config.gz | grep SCHED_CLASS_EXT
```

2. Check for conflicting scheduler:
```bash
cat /sys/kernel/sched_ext/state
```

3. Check BPF verifier errors:
```bash
sudo scx_ghostbrew -v 2>&1 | head -100
```

### System Instability

If the system becomes unstable:

1. Stop GhostBrew: `sudo pkill scx_ghostbrew`
2. System automatically falls back to EEVDF
3. Check dmesg for errors: `dmesg | tail -50`

## Performance Comparison

### Expected Results vs EEVDF

| Workload | EEVDF | GhostBrew | Difference |
|----------|-------|-----------|------------|
| Game FPS (avg) | 100% | ~same | Neutral |
| Game 1% lows | 100% | 105-115% | Better |
| Frame times | Variable | More consistent | Better |
| Compile time | 100% | ~same | Neutral |
| Desktop feel | Good | Better | Improved |

### When to Use GhostBrew

GhostBrew excels when:
- Gaming on X3D processors
- Mixed gaming + background tasks
- Running VMs alongside desktop work
- AI inference with interactive use

Consider alternatives when:
- Pure server workloads (use default scheduler)
- Realtime audio production (consider scx_lavd)
- Unknown/exotic hardware (test carefully)
