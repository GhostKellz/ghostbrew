#!/usr/bin/env bash
set -euo pipefail

REPO="GhostKellz/ghostbrew"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"
INSTALL_PREFIX="/usr/local"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

log() {
  printf '\033[1;36m==>\033[0m %s\n' "$1"
}

fail() {
  printf '\033[1;31merror:\033[0m %s\n' "$1" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

need_cmd curl
need_cmd tar
need_cmd install

if command -v python3 >/dev/null 2>&1; then
  PYTHON_BIN="python3"
elif command -v python >/dev/null 2>&1; then
  PYTHON_BIN="python"
else
  fail "required command not found: python3 or python"
fi

if [[ "${EUID}" -ne 0 ]]; then
  fail "run this installer as root (for example: curl -fsSL https://ghostbrew.cktech.sh | sudo bash)"
fi

log "GhostBrew installing"
log "Fetching latest release metadata"

release_json="${TMP_DIR}/release.json"
curl -fsSL "$API_URL" -o "$release_json"

asset_url="$($PYTHON_BIN - <<'PY' "$release_json"
import json, sys
with open(sys.argv[1], 'r', encoding='utf-8') as f:
    data = json.load(f)
assets = {asset['name']: asset['browser_download_url'] for asset in data.get('assets', [])}
preferred = [
    next((name for name in assets if name.startswith('ghostbrew-') and name.endswith('-linux-x86_64.tar.gz')), None),
    next((name for name in assets if name.startswith('scx-ghostbrew-') and name.endswith('-linux-x86_64.tar.gz')), None),
]
for name in preferred:
    if name:
        print(assets[name])
        raise SystemExit(0)
raise SystemExit(1)
PY
)" || fail "could not locate a downloadable Linux tar.gz release asset"

asset_name="$(basename "$asset_url")"
archive_path="${TMP_DIR}/${asset_name}"

log "Downloading ${asset_name}"
curl -fsSL "$asset_url" -o "$archive_path"

log "Extracting archive"
tar -xzf "$archive_path" -C "$TMP_DIR"

extract_root="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -type d -name 'ghostbrew-*' | head -n1)"
[[ -n "$extract_root" ]] || fail "failed to find extracted release directory"

install -d \
  "${INSTALL_PREFIX}/bin" \
  "${INSTALL_PREFIX}/share/icons/hicolor/256x256/apps" \
  "${INSTALL_PREFIX}/share/man/man1" \
  "${INSTALL_PREFIX}/share/bash-completion/completions" \
  "${INSTALL_PREFIX}/share/zsh/site-functions" \
  "${INSTALL_PREFIX}/share/fish/vendor_completions.d" \
  "/etc/systemd/system"

for bin_name in ghostbrew scx_ghostbrew; do
  if [[ -f "${extract_root}/${bin_name}" ]]; then
    install -m755 "${extract_root}/${bin_name}" "${INSTALL_PREFIX}/bin/${bin_name}"
    log "Installed ${bin_name} -> ${INSTALL_PREFIX}/bin/${bin_name}"
  fi
done

if [[ -f "${extract_root}/man/ghostbrew.1" ]]; then
  install -m644 "${extract_root}/man/ghostbrew.1" "${INSTALL_PREFIX}/share/man/man1/ghostbrew.1"
fi
if [[ -f "${extract_root}/man/scx_ghostbrew.1" ]]; then
  install -m644 "${extract_root}/man/scx_ghostbrew.1" "${INSTALL_PREFIX}/share/man/man1/scx_ghostbrew.1"
fi

if [[ -f "${extract_root}/assets/icons/hicolor/256x256/apps/ghostbrew.png" ]]; then
  install -m644 "${extract_root}/assets/icons/hicolor/256x256/apps/ghostbrew.png" "${INSTALL_PREFIX}/share/icons/hicolor/256x256/apps/ghostbrew.png"
fi

if [[ -f "${extract_root}/completions/bash/ghostbrew" ]]; then
  install -m644 "${extract_root}/completions/bash/ghostbrew" "${INSTALL_PREFIX}/share/bash-completion/completions/ghostbrew"
fi
if [[ -f "${extract_root}/completions/bash/scx_ghostbrew" ]]; then
  install -m644 "${extract_root}/completions/bash/scx_ghostbrew" "${INSTALL_PREFIX}/share/bash-completion/completions/scx_ghostbrew"
fi
if [[ -f "${extract_root}/completions/zsh/_ghostbrew" ]]; then
  install -m644 "${extract_root}/completions/zsh/_ghostbrew" "${INSTALL_PREFIX}/share/zsh/site-functions/_ghostbrew"
fi
if [[ -f "${extract_root}/completions/zsh/_scx_ghostbrew" ]]; then
  install -m644 "${extract_root}/completions/zsh/_scx_ghostbrew" "${INSTALL_PREFIX}/share/zsh/site-functions/_scx_ghostbrew"
fi
if [[ -f "${extract_root}/completions/fish/ghostbrew.fish" ]]; then
  install -m644 "${extract_root}/completions/fish/ghostbrew.fish" "${INSTALL_PREFIX}/share/fish/vendor_completions.d/ghostbrew.fish"
fi
if [[ -f "${extract_root}/completions/fish/scx_ghostbrew.fish" ]]; then
  install -m644 "${extract_root}/completions/fish/scx_ghostbrew.fish" "${INSTALL_PREFIX}/share/fish/vendor_completions.d/scx_ghostbrew.fish"
fi
if [[ -f "${extract_root}/systemd/scx-ghostbrew.service" ]]; then
  install -m644 "${extract_root}/systemd/scx-ghostbrew.service" "/etc/systemd/system/scx-ghostbrew.service"
  log "Installed systemd service"
fi
log "Installed shell completions"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q "${INSTALL_PREFIX}/share/icons/hicolor" >/dev/null 2>&1 || true
fi

if command -v mandb >/dev/null 2>&1; then
  mandb >/dev/null 2>&1 || true
fi

log "Done"
printf 'Run: ghostbrew --help\n'
printf 'Run scheduler: sudo ghostbrew run\n'
