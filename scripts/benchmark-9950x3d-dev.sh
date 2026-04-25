#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

binary="target/x86_64-unknown-linux-gnu/release/scx_ghostbrew"
log_file="${TMPDIR:-/tmp}/ghostbrew-9950x3d-dev.log"
pid_file="${TMPDIR:-/tmp}/ghostbrew-9950x3d-dev.pid"
results_dir="docs/benchmarks"
report_file="$results_dir/9950x3d-dev-report.txt"
json_file="$results_dir/9950x3d-dev-report.json"

mkdir -p "$results_dir"

if [[ ! -x "$binary" ]]; then
  cargo build --release >/dev/null
fi

cleanup() {
  if [[ -f "$pid_file" ]]; then
    sudo kill "$(cat "$pid_file")" 2>/dev/null || true
    sudo rm -f "$pid_file"
  fi
}

trap cleanup EXIT

printf 'Current ghost-vcache mode: '
cat /sys/bus/platform/drivers/amd_x3d_vcache/*/amd_x3d_mode

sudo sh -c '"'$binary'" --stats --stats-interval 1 > "'$log_file'" 2>&1 & echo $! > "'$pid_file'"'
sleep 1

echo 'Running dev workload: cargo check -q'
cargo check -q
sleep 1

cleanup
sleep 1

echo
echo 'Last GhostBrew stats sample:'
python - <<'PY'
from pathlib import Path
path = Path("/tmp/ghostbrew-9950x3d-dev.log")
if not path.exists():
    raise SystemExit("missing benchmark log")
print(path.read_text())
PY

python - <<'PY'
import json
import re
from pathlib import Path

log_path = Path('/tmp/ghostbrew-9950x3d-dev.log')
text = log_path.read_text()
samples = [chunk.strip() for chunk in text.split('--- GhostBrew Stats ---') if chunk.strip()]
last = samples[-1] if samples else ''

def extract(name: str):
    m = re.search(rf'{re.escape(name)}:\s+([^\n]+)', last)
    return m.group(1).strip() if m else None

report = {
    'mode': Path('/sys/bus/platform/drivers/amd_x3d_vcache').glob('*/amd_x3d_mode'),
}
mode = 'unknown'
for candidate in Path('/sys/bus/platform/drivers/amd_x3d_vcache').glob('*/amd_x3d_mode'):
    mode = candidate.read_text().strip()
    break

payload = {
    'vcache_mode': mode,
    'enqueued': extract('Enqueued'),
    'interactive_tasks': extract('Interactive tasks'),
    'prefcore_placements': extract('Prefcore placements'),
    'freq_ccd_placements': extract('Freq CCD placements'),
    'log_file': str(log_path),
}

Path('docs/benchmarks/9950x3d-dev-report.json').write_text(json.dumps(payload, indent=2) + '\n')
Path('docs/benchmarks/9950x3d-dev-report.txt').write_text(
    '\n'.join([
        'GhostBrew 9950X3D dev benchmark',
        f"vcache_mode={payload['vcache_mode']}",
        f"enqueued={payload['enqueued']}",
        f"interactive_tasks={payload['interactive_tasks']}",
        f"prefcore_placements={payload['prefcore_placements']}",
        f"freq_ccd_placements={payload['freq_ccd_placements']}",
        f"log_file={payload['log_file']}",
    ]) + '\n'
)
PY

echo
echo "Structured reports written to: $report_file and $json_file"
