# ghostbrew Commands & Keybindings

## CLI Commands

- `ghostbrew search <pkg>` — Unified search (AUR, ChaoticAUR, Pacman, Flatpak)
- `ghostbrew install <pkg>` — Install with source priority
- `ghostbrew upgrade` — Upgrade all packages (AUR, ChaoticAUR, Flatpak)
- `ghostbrew tap <repo>` — Add private PKGBUILD repo
- `ghostbrew completion <shell>` — Shell completions
- `ghostbrew tui` — Launch interactive TUI

## TUI Keybindings

- `/` — New search
- `Up/Down` — Navigate results
- `Space` — Select/deselect for batch install
- `Enter` — Install selected (or highlighted)
- `d` — Toggle PKGBUILD/dependency details
- `q` — Quit

## TUI Features

- Unified results from all sources, with color-coded labels
- Batch install (multi-select)
- PKGBUILD preview and dependency tree for AUR
- Status bar for progress/errors
- Respects Lua config for priorities, ignored packages, and hooks

---

See README.md for more details and configuration options.
