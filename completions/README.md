# GhostBrew Completions

Generated shell completion assets for `ghostbrew` should be staged here for release packaging.

- `bash/ghostbrew`
- `zsh/_ghostbrew`
- `fish/ghostbrew.fish`

Regenerate them with the built binary:

```bash
cargo build --release --bin ghostbrew --target x86_64-unknown-linux-gnu
target/x86_64-unknown-linux-gnu/release/ghostbrew completions bash > completions/bash/ghostbrew
target/x86_64-unknown-linux-gnu/release/ghostbrew completions zsh > completions/zsh/_ghostbrew
target/x86_64-unknown-linux-gnu/release/ghostbrew completions fish > completions/fish/ghostbrew.fish
```
