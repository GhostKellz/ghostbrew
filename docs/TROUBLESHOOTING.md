# GhostBrew Troubleshooting Guide

This guide covers common issues and solutions when running the GhostBrew scheduler.

## Table of Contents

1. [Scheduler Fails to Load](#scheduler-fails-to-load)
2. [V-Cache Not Detected](#v-cache-not-detected)
3. [Intel Cores Not Differentiated](#intel-cores-not-differentiated)
4. [Gaming Process Not Detected](#gaming-process-not-detected)
5. [High Latency or Stuttering](#high-latency-or-stuttering)
6. [Diagnostic Commands](#diagnostic-commands)

---

## Scheduler Fails to Load

### Symptom
```
Error: sched-ext not supported - ensure CONFIG_SCHED_CLASS_EXT=y in kernel
```

### Solutions

1. **Check kernel configuration**
   ```bash
   zcat /proc/config.gz 2>/dev/null | grep SCHED_CLASS_EXT || \
   grep SCHED_CLASS_EXT /boot/config-$(uname -r)
   ```

   Expected output: `CONFIG_SCHED_CLASS_EXT=y`

2. **Verify sched-ext sysfs exists**
   ```bash
   ls /sys/kernel/sched_ext
   ```

   If missing, you need a kernel with sched-ext support (6.12+ or patched kernel).

3. **Check for another scheduler**
   ```bash
   cat /sys/kernel/sched_ext/root/ops 2>/dev/null
   ```

   If output shows another scheduler name, unload it first:
   ```bash
   # The other scheduler should exit gracefully when stopped
   sudo killall scx_rusty  # or whichever scheduler is running
   ```

### Symptom
```
Error: Failed to attach struct_ops scheduler
```

### Solutions

1. **Ensure running as root**
   ```bash
   sudo scx_ghostbrew
   ```

2. **Check BPF permissions**
   ```bash
   # Verify RLIMIT_MEMLOCK is unlimited
   ulimit -l

   # If limited, GhostBrew sets this automatically, but check:
   cat /proc/sys/kernel/unprivileged_bpf_disabled
   ```

   If unprivileged BPF is disabled, root access is required.

3. **Check dmesg for BPF errors**
   ```bash
   sudo dmesg | tail -50 | grep -i bpf
   ```

---

## V-Cache Not Detected

### Symptom
```
AMD Zen processor detected
```
(instead of "AMD X3D processor detected")

### Solutions

1. **Verify processor model**
   ```bash
   grep "model name" /proc/cpuinfo | head -1
   ```

   Should contain "X3D" (e.g., "AMD Ryzen 9 7950X3D").

2. **Check BIOS settings**

   Some BIOS settings can affect X3D detection:
   - Ensure latest BIOS/AGESA version
   - Check for "Preferred Core" or "V-Cache" settings
   - "Game Mode" / "Creator Mode" BIOS switches may affect behavior

3. **Verify V-Cache sysfs (if using ghost-vcache)**
   ```bash
   ls /sys/bus/platform/drivers/amd_x3d_vcache/
   ```

   If this path doesn't exist, the kernel module isn't loaded. See linux-ghost documentation.

4. **Check CCD topology**
   ```bash
   # View CCD mapping
   sudo scx_ghostbrew -v 2>&1 | grep -E "CCD|X3D"
   ```

### Known X3D Models
- AMD Ryzen 7 7800X3D (single CCD, all V-Cache)
- AMD Ryzen 9 7900X3D (CCD0 has V-Cache)
- AMD Ryzen 9 7950X3D (CCD0 has V-Cache)
- AMD Ryzen 7 9800X3D (single CCD, all V-Cache)
- AMD Ryzen 9 9900X3D (CCD0 has V-Cache)
- AMD Ryzen 9 9950X3D (CCD0 has V-Cache)

---

## Intel Cores Not Differentiated

### Symptom
```
Generic x86-64 processor detected
```
(instead of "Intel 12th/13th/14th gen hybrid detected")

### Solutions

1. **Verify processor model**
   ```bash
   grep "model name" /proc/cpuinfo | head -1
   ```

   Should contain 12th/13th/14th gen Intel Core (e.g., "Intel(R) Core(TM) i9-14900K").

2. **Check cpu_capacity sysfs**
   ```bash
   # P-cores should have capacity 1024, E-cores ~768
   for cpu in /sys/devices/system/cpu/cpu*/cpu_capacity; do
     echo "$cpu: $(cat $cpu 2>/dev/null || echo 'N/A')"
   done | head -20
   ```

   If all values are the same (or missing), the kernel may not support cpu_capacity.

3. **Verify kernel version**
   ```bash
   uname -r
   ```

   Intel hybrid awareness requires kernel 5.18+ with Intel Thread Director support.

4. **Check cluster topology**
   ```bash
   for cpu in /sys/devices/system/cpu/cpu*/topology/cluster_id; do
     echo "$cpu: $(cat $cpu 2>/dev/null || echo 'N/A')"
   done | head -20
   ```

5. **Check Intel ITD (Thread Director)**
   ```bash
   dmesg | grep -i "intel_thread_director\|ITD"
   ```

---

## Gaming Process Not Detected

### Symptom
- Gaming PIDs counter stays at 0
- Games don't get priority scheduling

### Solutions

1. **Check gaming PID detection**
   ```bash
   # Run with verbose logging
   sudo scx_ghostbrew -v --stats

   # Look for "Gaming PIDs:" in output
   ```

2. **Verify process detection patterns**

   GhostBrew detects games by:
   - Known game process names (e.g., `Cyberpunk2077.exe`, `wine-preloader`)
   - Steam game processes
   - Proton/Wine processes
   - GPU-intensive processes

3. **Create a custom profile**

   Create `/etc/ghostbrew/profiles/mygame.toml`:
   ```toml
   name = "My Game"
   exe_name = "mygame.exe"

   [tunables]
   burst_threshold_ns = 1500000

   vcache_preference = "cache"
   smt_preference = "prefer_idle"
   ```

4. **Check if process is running under Wine/Proton**
   ```bash
   ps aux | grep -E "wine|proton|steam"
   ```

5. **Check cgroup classification**
   ```bash
   # Games under Steam are often in a specific cgroup
   cat /proc/$(pgrep -f YourGame)/cgroup
   ```

---

## High Latency or Stuttering

### Symptoms
- Games feel laggy despite scheduler running
- Mouse/keyboard input feels delayed
- Frame times are inconsistent

### Solutions

1. **Check burst threshold setting**
   ```bash
   # Default is 2ms, try lower for more responsiveness
   sudo scx_ghostbrew --burst-threshold 1500000  # 1.5ms
   ```

2. **Check time slice setting**
   ```bash
   # Default is 3ms, try lower for lower latency
   sudo scx_ghostbrew --slice-ns 2000000  # 2ms
   ```

3. **Enable gaming mode explicitly**
   ```bash
   sudo scx_ghostbrew --gaming
   ```

4. **Check for CPU frequency throttling**
   ```bash
   # View current frequencies
   grep MHz /proc/cpuinfo | head -8

   # Check governor
   cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
   ```

   For gaming, use `performance` governor:
   ```bash
   sudo cpupower frequency-set -g performance
   ```

5. **Check for thermal throttling**
   ```bash
   # AMD
   sensors | grep -i tctl

   # Intel
   sensors | grep -i "Package id"
   ```

6. **Verify SMT sibling awareness**
   ```bash
   sudo scx_ghostbrew -v 2>&1 | grep "SMT"
   ```

7. **For AMD X3D: Verify V-Cache mode**
   ```bash
   # If using ghost-vcache
   cat /sys/bus/platform/drivers/amd_x3d_vcache/*/amd_x3d_mode
   ```

   Should show `cache` for gaming, `frequency` for productivity.

---

## Diagnostic Commands

### Quick Health Check
```bash
# All-in-one diagnostic
sudo scx_ghostbrew -v --stats 2>&1 | head -50
```

### BPF Program Status
```bash
# List loaded BPF programs
sudo bpftool prog list | grep ghostbrew

# View BPF maps
sudo bpftool map list | grep ghostbrew
```

### CPU Topology
```bash
# View detected topology
lscpu --extended

# CCD/CCX layout
for cpu in /sys/devices/system/cpu/cpu*/topology/; do
  die=$(cat ${cpu}die_id 2>/dev/null || echo "?")
  cluster=$(cat ${cpu}cluster_id 2>/dev/null || echo "?")
  core=$(cat ${cpu}core_id 2>/dev/null || echo "?")
  echo "CPU $(basename $(dirname $cpu)): die=$die cluster=$cluster core=$core"
done | head -32
```

### AMD Prefcore Rankings
```bash
# View energy performance preference hints
for cpu in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do
  echo "$cpu: $(cat $cpu 2>/dev/null)"
done | head -16

# View AMD pstate status
cat /sys/devices/system/cpu/amd_pstate/status
```

### Intel Hybrid Info
```bash
# View P-core vs E-core capacity
paste <(ls /sys/devices/system/cpu/cpu*/cpu_capacity | sort -V) \
      <(cat /sys/devices/system/cpu/cpu*/cpu_capacity 2>/dev/null) | head -32
```

### Scheduler Statistics
```bash
# View scheduler stats while running
sudo scx_ghostbrew --stats --stats-interval 1
```

### Kernel Messages
```bash
# Recent sched-ext messages
sudo dmesg | grep -iE "sched_ext|scx_|ghostbrew" | tail -20

# BPF verifier errors (if loading failed)
sudo dmesg | grep -iE "bpf|verifier" | tail -30
```

### Process Scheduling
```bash
# View scheduling class of a process
ps -o pid,cls,pri,ni,comm -p $(pgrep -f YourGame)

# View which CPU a process is running on
ps -o pid,psr,comm -p $(pgrep -f YourGame)
```

---

## Getting Help

If you're still experiencing issues:

1. **Collect diagnostic info**
   ```bash
   # Create a diagnostic report
   {
     echo "=== System Info ==="
     uname -a
     echo
     echo "=== CPU Info ==="
     grep "model name" /proc/cpuinfo | head -1
     lscpu | grep -E "CPU\(s\)|Thread|Core|Socket|Model name"
     echo
     echo "=== Kernel Config ==="
     zcat /proc/config.gz 2>/dev/null | grep -E "SCHED_CLASS_EXT|BPF" || \
       grep -E "SCHED_CLASS_EXT|BPF" /boot/config-$(uname -r)
     echo
     echo "=== GhostBrew Output ==="
     sudo timeout 5 scx_ghostbrew -v 2>&1 || echo "Failed to start"
   } > ghostbrew-diagnostic.txt
   ```

2. **File an issue** at https://github.com/ghostkellz/ghostbrew/issues

3. **Join discussions** in the sched-ext community

---

## Common Error Messages

| Error | Cause | Solution |
|-------|-------|----------|
| `sched-ext not supported` | Missing kernel feature | Upgrade to 6.12+ kernel with sched-ext |
| `Failed to load BPF program` | BPF verifier rejection | Check dmesg for details, may need kernel update |
| `Failed to attach struct_ops` | Another scheduler running | Stop other scx_* schedulers first |
| `Permission denied` | Not running as root | Use `sudo` |
| `RLIMIT_MEMLOCK` | Memory limit too low | Usually auto-fixed; check ulimits if persists |
