#!/bin/bash
# pkgbuild.sh - Helper script for ghostbrew AUR releases
set -e

PKGNAME=ghostbrew
VERSION=$(grep '^version =' Cargo.toml | head -1 | cut -d '"' -f2)
TARBALL="$PKGNAME-$VERSION.tar.gz"
TAG="v$VERSION"

# 1. Check ring version
if ! grep -q 'ring = "=0.16.20"' Cargo.toml; then
  echo "[ERROR] ring must be pinned to 0.16.20 in Cargo.toml for AUR builds!" >&2
  exit 1
fi
if grep -A1 'name = "ring"' Cargo.lock | grep -v 'version = "0.16.20"' | grep -q 'version'; then
  echo "[ERROR] ring version in Cargo.lock is not 0.16.20! Run: cargo update -p ring" >&2
  exit 1
fi

# 2. Ensure Cargo.lock is present
if [ ! -f Cargo.lock ]; then
  echo "[ERROR] Cargo.lock missing! Run: cargo generate-lockfile" >&2
  exit 1
fi

# 3. Create source tarball
rm -f "$TARBALL"
git archive --format=tar.gz --output="$TARBALL" "$TAG"

echo "[INFO] Created $TARBALL for release."

echo "[INFO] To test build:"
echo "  tar -xzf $TARBALL && cd $PKGNAME-$VERSION && makepkg -si"
