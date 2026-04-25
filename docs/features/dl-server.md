# DL Server Integration

This document describes the Deadline (DL) Server integration in GhostBrew for RT starvation protection.

## Overview

The DL Server is a Linux kernel feature available in kernel 7.0+ that provides guaranteed runtime for sched_ext tasks even under heavy real-time workloads.

## Why It Matters

Without DL server protection, RT tasks can starve sched_ext workloads completely, which can cause:

- stalled gaming threads
- dropped frames
- system unresponsiveness
- application timeouts

## How It Works

The DL Server reserves a portion of CPU bandwidth for sched_ext tasks using deadline scheduling windows.

## Requirements

- kernel 7.0+
- `CONFIG_SCHED_CLASS_EXT=y`
- no manual configuration in GhostBrew when the interface is available

## Detection In GhostBrew

GhostBrew logs whether DL server support is available at startup.

## Kernel Configuration

If building a custom kernel, ensure:

```text
CONFIG_SCHED_CLASS_EXT=y
CONFIG_EXT_GROUP_SCHED=y
```

## Related Documentation

- [Linux sched_ext documentation](https://www.kernel.org/doc/html/latest/scheduler/sched-ext.html)
- [GhostBrew architecture overview](../architecture/overview.md)
