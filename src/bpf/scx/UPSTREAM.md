# Vendored sched_ext headers — provenance

`src/bpf/scx/*.h` are verbatim copies of `tools/sched_ext/include/scx/` from the
Linux tree. `src/bpf/vmlinux.h` is generated from a running kernel's BTF. Neither
is edited by hand — local changes are lost on the next sync.

## Current snapshot

| | |
|---|---|
| `scx/*.h` source | torvalds/linux `v7.3-rc1` (`cee9395acd8043be0644b25c34bfa86623f2b935`) |
| Commit date | 2026-08-30 |
| `vmlinux.h` source | BTF of `7.2.0-1-cachyos-lto` |

The files were initially synced from merge-window commit `66498c75b4f8`. They
are byte-identical at `v7.3-rc1`, which includes the kfunc compatibility fixes
from `56bbc91219c0` and `62f3d531e41a`.

Resync again at 7.3 final.

## Resync

```sh
# headers — from a Linux checkout at the tag/SHA you want to pin
cp "$LINUX/tools/sched_ext/include/scx/"*.h src/bpf/scx/
git -C "$LINUX" rev-parse HEAD      # record above

# vmlinux.h — from the BTF of the kernel you are targeting
bpftool btf dump file /sys/kernel/btf/vmlinux format c > src/bpf/vmlinux.h
uname -r                            # record above
```

Then update the table, rebuild, and confirm every kfunc the object references
still resolves against the target kernel's BTF. `build.rs` goes through
`libbpf_cargo::SkeletonBuilder`, which does not leave the object on disk, so
compile a standalone copy to inspect:

```sh
mkdir -p .scratch
clang -target bpf -D__TARGET_ARCH_x86 -I src/bpf -I src/bpf/scx -g -O2 \
      -c src/bpf/ghostbrew.bpf.c -o .scratch/gb-verify.bpf.o

bpftool btf dump file /sys/kernel/btf/vmlinux format raw \
  | grep -oP "FUNC '\K[a-zA-Z0-9_]+" | sort -u > .scratch/btf-funcs.txt

llvm-readelf --symbols .scratch/gb-verify.bpf.o \
  | awk '$4=="NOTYPE" && $7=="UND" {print $5, $8}' | sort -u \
  | while read bind sym; do
      ess=$(echo "$sym" | sed -E 's/^(.*)___[^_].*$/\1/')
      grep -qx "$ess" .scratch/btf-funcs.txt || echo "MISS[$bind] $sym -> $ess"
    done

rm -f .scratch/gb-verify.bpf.o .scratch/btf-funcs.txt
```

Do not filter on `GLOBAL` — that hides the `WEAK` symbols, which are the
compat-guarded ones you actually want to see.

`___suffix` is stripped because libbpf matches on the *rightmost* `___`
(`bpf_core_essential_name_len()`), so `scx_bpf_dsq_insert___v2___compat`
resolves to the kernel symbol `scx_bpf_dsq_insert___v2`.

A `WEAK` miss is expected and fine — those are guarded by `bpf_ksym_exists()`
and pruned by the verifier. Against 7.2 the expected misses are the pre-6.13
legacy names `scx_bpf_consume`, `scx_bpf_dispatch`, `scx_bpf_dispatch_vtime`.
A `GLOBAL` miss is a real breakage.

## Version gates

`compat.h` records which ops field arrived when. Relevant to us:

- **v7.1** — `ops.sub_attach()`, `ops.sub_detach()`, `ops.sub_cgroup_id`
- **v7.3** — `ops.rescue_bandwidth_ppt`, `ops.rescue_quantum_us`

The 7.2 kernel we target has the sub-scheduler fields but not the rescue ones.
