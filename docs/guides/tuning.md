# GhostBrew Tuning Guide

This guide covers performance tuning for scx_ghostbrew across different workloads.

## Command Line Options

```text
scx_ghostbrew [OPTIONS]

Mode Selection:
    -g, --gaming          Gaming mode - prefer V-Cache CCD for latency-sensitive tasks
    -w, --work            Work mode - prefer frequency CCD for higher boost
    -a, --auto-mode       Auto-detect workload and adjust (default)

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
    ghostbrew completions Generate shell completions from the front-end CLI
```

## Runtime Tuning

GhostBrew supports runtime tuning without restarting the scheduler.

### Control File Interface

Write commands to `/run/ghostbrew/control` to update tunables at runtime. This file is created with mode `0600`, so use `sudo` or another privileged context when writing to it:

```bash
# Set burst threshold (nanoseconds)
sudo sh -c 'printf "%s\n" "burst_threshold_ns=1500000" > /run/ghostbrew/control'

# Set time slice (nanoseconds)
sudo sh -c 'printf "%s\n" "slice_ns=2500000" > /run/ghostbrew/control'

# Enable/disable gaming mode
sudo sh -c 'printf "%s\n" "gaming_mode=true" > /run/ghostbrew/control'

# Enable/disable work mode
sudo sh -c 'printf "%s\n" "work_mode=true" > /run/ghostbrew/control'
```

Multiple commands can be written at once:

```bash
sudo tee /run/ghostbrew/control >/dev/null <<'EOF'
burst_threshold_ns=1000000
slice_ns=2500000
gaming_mode=true
EOF
```

## Profiles And Integration

### Game Profiles

Game-specific tunables are automatically applied when a game is detected:

```bash
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
ghost-vcache cache
ghost-vcache frequency
```

## Workload Profiles

### Gaming

```bash
sudo scx_ghostbrew -g -s
```

- Prioritizes gaming tasks above all others
- Routes gaming tasks to the V-Cache CCD on X3D processors
- Reduces scheduling latency for game threads

### Development And Compilation

```bash
sudo scx_ghostbrew
```

- Preserves interactive responsiveness
- Spreads compilers across available cores
- Works with profile-driven `work_mode` overrides for dev/build activity

### AI, Containers, And VMs

GhostBrew automatically detects and classifies:

- Ollama and other AI workloads
- Docker/Podman/containerd workloads
- QEMU/KVM virtual machines and vCPU threads

Use `sudo scx_ghostbrew -v` when you want to inspect those classifications live.

## AMD X3D Optimization

### CCD Routing

| Task Type | Target CCD | Reason |
|-----------|------------|--------|
| Gaming | V-Cache | Cache-sensitive workload |
| Interactive | V-Cache | Low latency |
| Compilation | Frequency | Clock speed benefits |
| Streaming | Frequency | Encoding benefits |
| Background | Any | Load balancing |

### Verifying V-Cache Detection

```bash
sudo scx_ghostbrew -v 2>&1 | grep -i vcache
```

## AMD Prefcore Integration

```bash
cat /sys/devices/system/cpu/amd_pstate/prefcore

for i in /sys/devices/system/cpu/cpufreq/policy*/amd_pstate_prefcore_ranking; do
    echo "$i: $(cat $i)"
done
```

Higher rankings indicate preferred cores. GhostBrew uses these rankings for latency-sensitive placement decisions.
