# DL Server Integration

This document describes the Deadline (DL) Server integration in GhostBrew for RT starvation protection.

## Overview

The DL Server is a Linux kernel feature (available in kernel 7.0+) that provides guaranteed runtime for sched_ext tasks even under heavy real-time (SCHED_FIFO/SCHED_RR) workloads. Without this protection, RT tasks can completely starve SCHED_EXT scheduled tasks.

## The Problem

Real-time tasks in Linux have strict priority over normal tasks. When a system has many RT tasks (common with audio production, real-time control systems, or certain gaming setups), SCHED_EXT tasks may receive no CPU time, leading to:

- Stalled gaming threads
- Dropped frames
- System unresponsiveness
- Application timeouts

## How DL Server Works

The DL Server reserves a portion of CPU bandwidth for SCHED_EXT tasks using the deadline scheduling class. This creates a guaranteed execution window that RT tasks cannot preempt.

```
┌─────────────────────────────────────────────────────────────────┐
│                        CPU Timeline                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Without DL Server:                                             │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ RT │ RT │ RT │ RT │ RT │ RT │ RT │ RT │ RT │ RT │ NORMAL   ││
│  └─────────────────────────────────────────────────────────────┘│
│                         (RT tasks dominate)                     │
│                                                                 │
│  With DL Server (5% reserved):                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ RT │ DL │ RT │ RT │ DL │ RT │ RT │ DL │ RT │ RT │ DL │ RT ││
│  └─────────────────────────────────────────────────────────────┘│
│           ↑             ↑             ↑             ↑           │
│        (DL server windows for SCHED_EXT tasks)                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Requirements

- **Kernel 7.0+**: The ext_server/DL server interface
- **sched_ext enabled**: `CONFIG_SCHED_CLASS_EXT=y`
- **No user configuration needed**: Automatic when available

## Detection in GhostBrew

GhostBrew automatically detects DL server availability at startup:

```
[INFO] DL server available (kernel 7.0.1) - RT starvation protection enabled
```

Or on older kernels:

```
[DEBUG] DL server not available (requires kernel 7.0+, current: 6.12.5)
```

## Performance Characteristics

When DL server is active:

| Metric | Impact |
|--------|--------|
| RT starvation | Eliminated (~5% guaranteed runtime) |
| RT task latency | Minimal impact (<1%) |
| Gaming frame pacing | Stable under RT pressure |
| Audio workloads | Compatible (JACK, PipeWire RT) |

## Use Cases

The DL server is particularly beneficial for:

1. **Gaming + Audio Production**: Running games while Ardour/REAPER uses RT priority
2. **Streaming + Gaming**: OBS encoder threads with RT priority
3. **VR Applications**: SteamVR compositor RT threads alongside game threads
4. **Professional Audio**: JACK with RT priority and gaming

## Kernel Configuration

For kernel 7.0+, the DL server is enabled by default when sched_ext is active. No manual configuration is required.

If building a custom kernel, ensure these options:

```
CONFIG_SCHED_CLASS_EXT=y
CONFIG_EXT_GROUP_SCHED=y  # For cgroup support
```

## Related Documentation

- [Linux sched_ext documentation](https://www.kernel.org/doc/html/latest/scheduler/sched-ext.html)
- [GhostBrew ARCHITECTURE.md](./ARCHITECTURE.md)
