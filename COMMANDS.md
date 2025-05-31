# ghostbrew Commands & Keybindings

## CLI Commands

- `ghostbrew search <pkg>` — Unified search (AUR, ChaoticAUR, Pacman, Flatpak)
- `ghostbrew install <pkg>` — Install with source priority, PKGBUILD diff/audit, GPG check, Lua hooks
- `ghostbrew upgrade` — Upgrade all packages (AUR, ChaoticAUR, Flatpak, Pacman), with backup/rollback and parallel jobs
- `ghostbrew tap <repo>` — Add private PKGBUILD repo
- `ghostbrew completion <shell>` — Shell completions
- `ghostbrew tui` — Launch interactive TUI
- `ghostbrew rollback <pkg>` — Rollback a package to previous backup

## TUI Keybindings

- `/` — New search
- `Up/Down` — Navigate results
- `Space` — Select/deselect for batch install
- `Enter` — Install selected (or highlighted)
- `d` — Toggle PKGBUILD/dependency details
- `l` — Toggle log pane
- `h` — Toggle help popup
- `q` — Quit

## TUI Features

- Unified results from all sources, with color-coded labels
- Batch install (multi-select)
- PKGBUILD preview and dependency tree for AUR
- Status bar for progress/errors
- Log pane for install/build output
- Respects Lua config for priorities, ignored packages, and hooks
- PKGBUILD diff/audit before install/upgrade
- Rollback support for failed upgrades/installs
- AUR metadata (votes, popularity, maintainer) in details
- Flatpak sandbox info in details

---

See README.md for more details and configuration options.
