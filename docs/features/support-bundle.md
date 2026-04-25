# GhostBrew Support Bundle

GhostBrew includes a lightweight support-bundle workflow for troubleshooting scheduler, kernel, and topology issues.

## Generate A Bundle

```bash
ghostbrew support
```

By default this writes a dated bundle under `~/.local/state/ghostbrew/support/`.

To also write a machine-readable JSON companion:

```bash
ghostbrew support --json --output ~/.local/state/ghostbrew/support/my-support.txt
```

## What It Captures

- distribution, kernel, architecture, and timestamp
- current `sched_ext` state
- `/run/ghostbrew/control` ownership and mode
- CPU model and `lscpu` summary
- AMD X3D runtime details when available
- active GhostBrew config and profile directories
- latest local benchmark reports from `~/.local/state/ghostbrew/benchmarks/`
- checked-in benchmark examples from `docs/benchmarks/` when present
- recent `dmesg` lines relevant to `sched_ext`, `ghostbrew`, `bpf`, and verifier output

## Recommended Workflow

```bash
ghostbrew benchmark --workload "cargo check -q"
ghostbrew support --json
```

Attach the generated bundle and mention whether the issue occurred in `cache` mode or `frequency` mode.
