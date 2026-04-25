#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

output="${1:-$HOME/.local/state/ghostbrew/support/ghostbrew-support.txt}"
mkdir -p "$(dirname "$output")"

mode_file=""
for candidate in /sys/bus/platform/drivers/amd_x3d_vcache/*/amd_x3d_mode; do
  if [[ -f "$candidate" ]]; then
    mode_file="$candidate"
    break
  fi
done

{
  echo "ghostbrew support bundle"
  echo "======================="
  echo
  echo "[system]"
  echo "timestamp=$(date --iso-8601=seconds)"
  echo "hostname=$(hostname)"
  echo "distribution=$(grep '^PRETTY_NAME=' /etc/os-release | cut -d= -f2- | tr -d '"' || true)"
  echo "kernel=$(uname -r)"
  echo "arch=$(uname -m)"
  echo
  echo "[ghostbrew]"
  echo "version=$(cargo pkgid 2>/dev/null | awk -F# '{print $2}' || echo unknown)"
  echo "sched_ext_state=$(cat /sys/kernel/sched_ext/state 2>/dev/null || echo unavailable)"
  echo "control_file=$(ls -l /run/ghostbrew/control 2>/dev/null || echo missing)"
  echo
  echo "[cpu]"
  grep -m1 '^model name' /proc/cpuinfo || true
  echo "lscpu_summary=$(lscpu | tr '\n' ';' | sed 's/;/ | /g')"
  echo
  echo "[amd_x3d]"
  echo "prefcore=$(cat /sys/devices/system/cpu/amd_pstate/prefcore 2>/dev/null || echo unavailable)"
  echo "pstate_status=$(cat /sys/devices/system/cpu/amd_pstate/status 2>/dev/null || echo unavailable)"
  echo "vcache_mode=$(if [[ -n "$mode_file" ]]; then cat "$mode_file"; else echo unavailable; fi)"
  echo "die_cpus_list_cpu0=$(cat /sys/devices/system/cpu/cpu0/topology/die_cpus_list 2>/dev/null || echo unavailable)"
  echo "die_cpus_list_cpu8=$(cat /sys/devices/system/cpu/cpu8/topology/die_cpus_list 2>/dev/null || echo unavailable)"
  echo "cluster_id_cpu0=$(cat /sys/devices/system/cpu/cpu0/topology/cluster_id 2>/dev/null || echo unavailable)"
  echo "cluster_id_cpu8=$(cat /sys/devices/system/cpu/cpu8/topology/cluster_id 2>/dev/null || echo unavailable)"
  echo
  echo "[config]"
  for path in /etc/ghostbrew/config.toml "$HOME/.config/ghostbrew/config.toml"; do
    if [[ -f "$path" ]]; then
      echo "config_path=$path"
      sed 's/^/  /' "$path"
    fi
  done
  echo
  echo "[profiles]"
  for dir in /etc/ghostbrew/profiles "$HOME/.config/ghostbrew/profiles"; do
    if [[ -d "$dir" ]]; then
      echo "profiles_dir=$dir"
      ls "$dir"
    fi
  done
  echo
  echo "[benchmark_reports]"
  for file in docs/benchmarks/9950x3d-dev-report.txt docs/benchmarks/9950x3d-dev-report.json; do
    if [[ -f "$file" ]]; then
      echo "file=$file"
      sed 's/^/  /' "$file"
    fi
  done
  echo
  echo "[recent_dmesg]"
  sudo dmesg | rg 'sched_ext|scx_|ghostbrew|bpf|verifier' | tail -n 80 || true
} > "$output"

echo "Wrote support bundle to $output"
