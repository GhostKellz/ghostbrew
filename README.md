# 👻 ghostbrew

[![Arch Linux](https://img.shields.io/badge/platform-Arch%20Linux-1793d1?logo=arch-linux&logoColor=white)](https://archlinux.org)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Repo Status](https://img.shields.io/badge/status-active-success?style=flat-square)](https://github.com/ghostkellz/ghostbrew)
[![Build](https://img.shields.io/github/actions/workflow/status/ghostkellz/ghostbrew/main.yml?branch=main)](https://github.com/ghostkellz/ghostbrew/actions)
![Built with Clap](https://img.shields.io/badge/built%20with-clap-orange)
![License](https://img.shields.io/github/license/ghostkellz/ghostbrew)
---

**ghostbrew** is a fast, minimal, and security-focused AUR helper for Arch Linux, now rewritten in Rust.  
Inspired by `yay` and `paru`, built for modern, parallel, and auditable package management with future GUI integration via `ghostview` (already working as a Rust frontend).

> 🧪 Interactive AUR search & install (TUI and CLI)  
> ⚡ Parallel, dependency-aware builds  
> 🔒 PKGBUILD security audit before install  
> 🔑 GPG key check & auto-import  
> 🏴 Private repo/tap support  
> 🐚 Shell completions for bash/zsh/fish  
> 💾 Configurable via Lua 

---

## ✨ Features

* 🔍 `ghostbrew search <pkg>` — Interactive AUR search & install
* 📦 Parallel install with full dependency resolution
* 🕵️ PKGBUILD inspector: highlights dangerous or suspicious commands before build
* 🔑 GPG key verification and (soon) auto-import
* ♻️ `ghostbrew upgrade` — Sync and upgrade AUR, Chaotic-AUR, and official packages
* 🏴 `ghostbrew tap <repo>` — Add private PKGBUILD repos
* 🐚 `ghostbrew completion <shell>` — Shell completions (bash/zsh/fish)
* 💪 Configurable via `~/.config/ghostbrew/brew.lua`
* 🖥️ GUI frontend via GhostView (Rust, egui)

---

## 🚀 Getting Started

```bash
git clone https://github.com/ghostkellz/ghostbrew.git
cd ghostbrew
cargo build --release
./target/release/ghostbrew --help
```

---

### 🔧 Requirements

* Arch Linux / Arch-based distro
* `git`, `makepkg`, `sudo`, `base-devel`
* Rust toolchain (`rustup`, `cargo`)
* Internet (for AUR access)

---

## 🔮 Roadmap

> 🚧 Status: Rust-based `v0.2` in active development

- [x] Initialize Rust CLI with Clap
- [x] Search AUR via API (or Git)
- [x] Basic PKGBUILD downloader/parser
- [x] Build pipeline (makepkg wrapper)
- [x] PKGBUILD static analysis (audit)
- [x] GPG key check (auto-import in progress)
- [x] Dependency graph resolution & parallel install
- [x] Upgrade command (`upgrade`)
- [x] Tap support for custom repos
- [x] GUI frontend via GhostView (egui)
- [x] Plugin/hook system (Lua-powered, pre/post install)
- [x] TUI interface (ratatui-based, batch install, PKGBUILD preview, dependency tree)
- [x] Advanced Lua config (priorities, ignored packages, hooks)
- [x] Log pane and error reporting in TUI
- [x] Help popup and keybinding hints in TUI
- [x] PKGBUILD diff/audit before upgrade/install
- [x] Rollback to previous package versions
- [x] AUR comments/votes/changelog in TUI
- [x] Flatpak/AppImage sandbox info in TUI
- [x] Async Rust for all network/disk IO
- [ ] "ghostbrew doctor" for self-diagnosis
- [ ] Plugin marketplace/user scripts

---

## 📂 Directory Structure

```
ghostbrew/
├── src/
│   ├── aur.rs            # AUR fetching/parsing logic
│   ├── config.rs         # Lua config loader
│   ├── core.rs           # Unified search/install logic
│   ├── flatpak.rs        # Flatpak integration
│   ├── gpg.rs            # GPG key handling
│   ├── hooks.rs          # Plugin/hook system
│   ├── pacman.rs         # Pacman repo logic
│   ├── tui.rs            # TUI (ratatui-based)
│   ├── utils.rs          # Shared helpers
│   └── main.rs           # CLI entry point
├── brew.lua              # Example Lua config
├── COMMANDS.md           # CLI/TUI commands & keybindings
├── Cargo.toml
├── README.md
└── LICENSE
```

---

## 🧙‍♂️ Philosophy

Ghostbrew isn’t just another AUR helper — it’s a haunted, extensible tool built for speed, auditability, and developer-first workflows. Minimal and inspectable by design, made for mesh-native environments and security-conscious users.

---

## 📝 Example Lua Config

Create `~/.config/ghostbrew/brew.lua`:

```lua
-- Example ghostbrew Lua config
ignored_packages = { "linux", "nvidia" }
parallel = 4
priorities = { "chaotic-aur", "aur", "pacman", "flatpak" }

function pre_install(pkg)
  print("[hook] About to install " .. pkg)
end

function post_install(pkg)
  print("[hook] Finished installing " .. pkg)
end
```

---

## 📖 Documentation

- [COMMANDS.md](COMMANDS.md): CLI and TUI commands, keybindings, and features
- [brew.lua](brew.lua): Example Lua config (priorities, hooks, ignored packages)
- [README.md](README.md): Features, philosophy, roadmap, and getting started

---

## 📖 Commands

See [COMMANDS.md](COMMANDS.md) for a full list of CLI and TUI commands, options, and keybindings.

---

## 📝 License

MIT © [ghostkellz](https://github.com/ghostkellz)

