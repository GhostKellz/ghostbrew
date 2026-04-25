#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
version="${1:-$(grep '^version = ' "$repo_root/Cargo.toml" | head -n1 | cut -d'"' -f2)}"
target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
release_dir="$target_dir/x86_64-unknown-linux-gnu/release"
bundle_root="$target_dir/release-bundle/ghostbrew-${version}-linux-x86_64"

rm -rf "$bundle_root"
mkdir -p \
  "$bundle_root/assets/icons/hicolor/256x256/apps" \
  "$bundle_root/completions/bash" \
  "$bundle_root/completions/zsh" \
  "$bundle_root/completions/fish" \
  "$bundle_root/man" \
  "$bundle_root/systemd"

cargo build --release --bin ghostbrew --bin scx_ghostbrew --target x86_64-unknown-linux-gnu

install -m755 "$release_dir/ghostbrew" "$bundle_root/ghostbrew"
install -m755 "$release_dir/scx_ghostbrew" "$bundle_root/scx_ghostbrew"
install -m644 "$repo_root/man/ghostbrew.1" "$bundle_root/man/ghostbrew.1"
install -m644 "$repo_root/man/scx_ghostbrew.1" "$bundle_root/man/scx_ghostbrew.1"
install -m644 "$repo_root/scx-ghostbrew.service" "$bundle_root/systemd/scx-ghostbrew.service"
install -m644 "$repo_root/assets/icons/ghostbrew-icon.png" "$bundle_root/assets/icons/hicolor/256x256/apps/ghostbrew.png"

"$release_dir/ghostbrew" completions bash > "$bundle_root/completions/bash/ghostbrew"
"$release_dir/ghostbrew" completions zsh > "$bundle_root/completions/zsh/_ghostbrew"
"$release_dir/ghostbrew" completions fish > "$bundle_root/completions/fish/ghostbrew.fish"
"$release_dir/scx_ghostbrew" --completions bash > "$bundle_root/completions/bash/scx_ghostbrew"
"$release_dir/scx_ghostbrew" --completions zsh > "$bundle_root/completions/zsh/_scx_ghostbrew"
"$release_dir/scx_ghostbrew" --completions fish > "$bundle_root/completions/fish/scx_ghostbrew.fish"

tar -C "$target_dir/release-bundle" -czf "$target_dir/release-bundle/ghostbrew-${version}-linux-x86_64.tar.gz" "ghostbrew-${version}-linux-x86_64"

printf 'Built release bundle: %s\n' "$target_dir/release-bundle/ghostbrew-${version}-linux-x86_64.tar.gz"
