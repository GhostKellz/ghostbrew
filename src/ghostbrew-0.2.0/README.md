# 👻 ghostbrew

[![Arch Linux](https://img.shields.io/badge/platform-Arch%20Linux-1793d1?logo=arch-linux&logoColor=white)](https://archlinux.org)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Repo Status](https://img.shields.io/badge/status-active-success?style=flat-square)](https://github.com/ghostkellz/ghostbrew)

---

**ghostbrew** is a fast, minimal, and security-focused AUR helper for Arch Linux, written in Rust.
Inspired by `yay` and `paru`, but designed for modern, parallel, and auditable package management with a native TUI and Lua config.

> 🧪 Interactive AUR, Pacman, and Flatpak search & install (Rust TUI)
> ⚡ Parallel, dependency-aware builds
> 🔒 PKGBUILD security audit before install
> 🔑 GPG key check & auto-import
> 🏴 Private repo/tap support
> 🐚 Shell completions for bash/zsh/fish
> 💾 Configurable via Lua

---

## ✨ Features

* 🔍 `ghostbrew search <pkg>` — Unified AUR/Pacman/Flatpak search & install
* 📦 Parallel install with dependency resolution
* 🕵️ PKGBUILD inspector: highlights risky commands before build
* 🔑 GPG key check and auto-import
* ♻️ `ghostbrew upgrade` — Sync official, Chaotic-AUR, and AUR packages
* 🏴 `ghostbrew tap <repo>` — Add private PKGBUILD repos
* 🐚 `ghostbrew completion <shell>` — Shell completions
* 💪 Configurable via `~/.config/ghostbrew/brew.lua`
* 🖥️ Native TUI (ratatui/crossterm)

---

## 🚀 Getting Started

```bash
git clone https://github.com/ghostkellz/ghostbrew.git
cd ghostbrew
makepkg -si
```

### 🔧 Requirements

* Arch Linux / Arch-based distro
* `git`, `rust`, `cargo`, `makepkg`, `sudo`
* Internet (for AUR access)

---

## 🔮 Roadmap

* [x] Interactive AUR search (Rust TUI)
* [x] Parallel, dependency-aware builds
* [x] PKGBUILD security audit
* [x] Shell completions
* [x] Lua config file support
* [x] Tap/private repo support
* [x] GPG key auto-import
* [x] Flatpak integration
* [ ] Advanced upgrade logic (AUR + official)
* [ ] Self-updating via GhostCTL
* [ ] Plugin/hook system
* [ ] GUI frontend

---

## 📂 Directory Structure

```
ghostbrew/
├── src/            # Rust source code (core, tui, aur, pacman, etc)
├── brew.lua        # Example Lua config
├── PKGBUILD        # Arch packaging
├── README.md
├── Cargo.toml
```

---

## 🧙‍♂️ Philosophy

Ghostbrew isn’t just another AUR helper — it's a haunted tool built for speed, security, and extensibility. Minimal, inspectable, and ready for dev-ops and mesh-native environments.

---

## 📝 License

MIT © [ghostkellz](https://github.com/ghostkellz)

