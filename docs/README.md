# GhostBrew Documentation

Documentation for scx_ghostbrew, a sched-ext BPF scheduler for AMD Zen4/Zen5 X3D and Intel Hybrid processors.

## Contents

- [architecture/overview.md](architecture/overview.md) - Internal architecture, BPF scheduling logic, and userspace components
- [guides/tuning.md](guides/tuning.md) - Performance tuning guide and configuration options
- [guides/troubleshooting.md](guides/troubleshooting.md) - Common issues and solutions
- [benchmarks.md](benchmarks.md) - Benchmarking methodology and results framework
- [benchmarks/](benchmarks/) - Checked-in benchmark example reports
- [features/dl-server.md](features/dl-server.md) - DL server integration for RT starvation protection (kernel 7.0+)
- [features/support-bundle.md](features/support-bundle.md) - Support bundle workflow and captured diagnostics

## Quick Links

- [Main README](../README.md) - Installation and quick start
- [CHANGELOG](../CHANGELOG.md) - Version history
- [CONTRIBUTING](../CONTRIBUTING.md) - Contribution guidelines
- [SECURITY](../SECURITY.md) - Security policy

## Configuration

Example configurations are available in:
- `examples/config/` - Main configuration files
- `examples/profiles/` - Per-game profile templates
