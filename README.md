# 👻 ghostbrew

[![Arch Linux](https://img.shields.io/badge/platform-Arch%20Linux-1793d1?logo=arch-linux&logoColor=white)](https://archlinux.org)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Repo Status](https://img.shields.io/badge/status-active-success?style=flat-square)](https://github.com/ghostkellz/ghostbrew)

---

**ghostbrew** is a fast, minimal, and security-focused AUR helper for Arch Linux, now rewritten in Rust.  
Inspired by `yay` and `paru`, built for modern, parallel, and auditable package management with future GUI integration via `ghostview`.

> 🧪 Interactive AUR search & install (fzf-style)  
> ⚡ Parallel, dependency-aware builds  
> 🔒 PKGBUILD security audit before install  
> 🔑 GPG key check & auto-import  
> 🏴 Private repo/tap support  
> 🐚 Shell completions for bash/zsh/fish  
> 💾 Configurable via YAML

---

## ✨ Features

* 🔍 `ghostbrew search <pkg>` — Interactive AUR search & install
* 📦 Parallel install with full dependency resolution
* 🕵️ PKGBUILD inspector: highlights dangerous or suspicious commands before build
* 🔑 GPG key verification and (soon) auto-import
* ♻️ `ghostbrew upgrade` — Sync and upgrade AUR, Chaotic-AUR, and official packages
* 🏴 `ghostbrew tap <repo>` — Add private PKGBUILD repos
* 🐚 `ghostbrew completion <shell>` — Shell completions (bash/zsh/fish)
* 💪 Configurable via `~/.config/ghostbrew/config.yml`

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
- [ ] Build pipeline (makepkg wrapper)
- [ ] PKGBUILD static analysis (audit)
- [ ] GPG key auto-import
- [ ] Dependency graph resolution
- [ ] Upgrade command (`upgrade`)
- [ ] Tap support for custom repos
- [ ] GUI frontend via GhostView (egui or Tauri)
- [ ] GhostCTL integration for auto-updating
- [ ] Plugin/hook system (e.g. `ghostbrew hook prebuild`)
- [ ] TUI interface (bubbletea or similar)

---

## 📂 Directory Structure

```
ghostbrew/
├── src/
│   ├── commands/         # CLI subcommands
│   ├── aur/              # AUR fetching/parsing logic
│   ├── config.rs         # YAML config loader
│   ├── gpg.rs            # GPG key handling
│   └── main.rs           # CLI entry point
├── archive/
│   └── go-v0.1/          # Legacy Go prototype
├── Cargo.toml
├── README.md
└── LICENSE
```

---

## 🧙‍♂️ Philosophy

Ghostbrew isn’t just another AUR helper — it’s a haunted, extensible tool built for speed, auditability, and developer-first workflows. Minimal and inspectable by design, made for mesh-native environments and security-conscious users.

---

## 📝 License

MIT © [ghostkellz](https://github.com/ghostkellz)

