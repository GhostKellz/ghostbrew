#!/bin/bash
set -e

echo "🔧 Building ghostbrew..."
cargo build --release

echo "📦 Installing to /usr/local/bin/ghostbrew..."
sudo install -Dm755 target/release/ghostbrew /usr/local/bin/ghostbrew

echo "📄 Copying README and LICENSE..."
sudo install -Dm644 README.md /usr/local/share/doc/ghostbrew/README.md
sudo install -Dm644 LICENSE /usr/local/share/licenses/ghostbrew/LICENSE

echo "✅ Done. Run with: ghostbrew"

