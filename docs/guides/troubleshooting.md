# GhostBrew Troubleshooting Guide

This guide covers common issues and solutions when running the GhostBrew scheduler.

## Scheduler Fails To Load

### Symptom

```text
Error: sched-ext not supported - ensure CONFIG_SCHED_CLASS_EXT=y in kernel
```

### Checks

1. Verify kernel configuration:

```bash
zcat /proc/config.gz 2>/dev/null | grep SCHED_CLASS_EXT || \
grep SCHED_CLASS_EXT /boot/config-$(uname -r)
```

2. Verify sched-ext sysfs exists:

```bash
ls /sys/kernel/sched_ext
```

3. Stop another active sched-ext scheduler first:

```bash
cat /sys/kernel/sched_ext/state
sudo systemctl stop scx-ghostbrew
```

## Attach Or BPF Errors

### Symptom

```text
Error: Failed to attach struct_ops scheduler
```

### Checks

1. Ensure you are running as root:

```bash
sudo scx_ghostbrew
```

2. Check BPF-related diagnostics:

```bash
ulimit -l
cat /proc/sys/kernel/unprivileged_bpf_disabled
sudo dmesg | tail -50 | grep -i bpf
```

## V-Cache Not Detected

### Checks

1. Confirm the processor is an X3D model:

```bash
grep "model name" /proc/cpuinfo | head -1
```

2. Check the ghost-vcache sysfs interface if applicable:

```bash
ls /sys/bus/platform/drivers/amd_x3d_vcache/
```

3. Inspect GhostBrew topology logging:

```bash
sudo scx_ghostbrew -v 2>&1 | grep -E "CCD|X3D"
```

## Intel Hybrid Not Detected

### Checks

```bash
grep "model name" /proc/cpuinfo | head -1
uname -r
for cpu in /sys/devices/system/cpu/cpu*/cpu_capacity; do
  echo "$cpu: $(cat $cpu 2>/dev/null || echo 'N/A')"
done | head -20
```

## Gaming Process Not Detected

### Checks

```bash
sudo scx_ghostbrew -v -s
ps aux | grep -E "wine|proton|steam"
cat /proc/$(pgrep -f YourGame)/cgroup
```

If needed, add a profile in `/etc/ghostbrew/profiles/` or `~/.config/ghostbrew/profiles/`.

## High Latency Or Stuttering

### Checks

```bash
sudo scx_ghostbrew --burst-threshold 1500000
sudo scx_ghostbrew --slice-ns 2000000
sudo scx_ghostbrew --gaming
grep MHz /proc/cpuinfo | head -8
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
```

## Diagnostics

Preferred support workflow:

```bash
ghostbrew benchmark --workload "cargo check -q"
ghostbrew support --json
```

Attach the generated support bundle when filing an issue.
