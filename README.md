# 👻 ghostbrew

[![Arch Linux](https://img.shields.io/badge/platform-Arch%20Linux-1793d1?logo=arch-linux&logoColor=white)](https://archlinux.org)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Repo Status](https://img.shields.io/badge/status-active-success?style=flat-square)](https://github.com/ghostkellz/ghostbrew)
[![Build](https://img.shields.io/github/actions/workflow/status/ghostkellz/ghostbrew/main.yml?branch=main)](https://github.com/ghostkellz/ghostbrew/actions)
![Built with Clap](https://img.shields.io/badge/built%20with-clap-orange)
![License](https://img.shields.io/github/license/ghostkellz/ghostbrew)
---

## Overview
Ghostbrew is a fast, minimal, Rust-powered AUR helper for Arch Linux. It supports:
- Unified search across AUR, Pacman, and Flatpak
- PKGBUILD audit and rollback
- GPG key management
- Batch operations via TUI

---

## Installation
### From AUR
```bash
yay -S ghostbrew
```

### Manual Build
```bash
git clone https://github.com/ghostkellz/ghostbrew.git
cd ghostbrew
makepkg -si
```

---

## Usage
### CLI
```bash
ghostbrew search <query>
ghostbrew install <package>
ghostbrew upgrade
ghostbrew rollback <package>
```

### TUI
Run `ghostbrew tui` for an interactive interface.

---

## Configuration
Ghostbrew supports Lua-based configuration via `~/.config/ghostbrew/brew.lua`. Example:
```lua
ignored_packages = {"package1", "package2"}
parallel = 4
```

---

## License
Ghostbrew is licensed under the MIT License.

