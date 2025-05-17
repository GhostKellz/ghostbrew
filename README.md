# 👻 ghostbrew

[![Go Build](https://github.com/ghostkellz/ghostbrew/actions/workflows/go.yml/badge.svg)](https://github.com/ghostkellz/ghostbrew/actions)
[![Arch Linux](https://img.shields.io/badge/platform-Arch%20Linux-1793d1?logo=arch-linux&logoColor=white)](https://archlinux.org)
[![Made with Go](https://img.shields.io/badge/made%20with-Go-00ADD8?logo=go&logoColor=white)](https://golang.org)
[![Repo Status](https://img.shields.io/badge/status-active-success?style=flat-square)](https://github.com/ghostkellz/ghostbrew)

---

**ghostbrew** is a fast, minimal, and security-focused AUR helper for Arch Linux.
Inspired by `yay` and `paru`, but designed for modern, parallel, and auditable package management.

> 🧪 Interactive AUR search & install (fzf-style)
> ⚡ Parallel, dependency-aware builds
> 🔒 PKGBUILD security audit before install
> 🔑 GPG key check & (soon) auto-import
> 🏴 Private repo/tap support
> 🐚 Shell completions for bash/zsh/fish
> 💾 Configurable via YAML

---

## ✨ Features

* 🔍 `ghostbrew search <pkg>` — Interactive AUR search & install
* 📦 Parallel install with dependency resolution
* 🕵️ PKGBUILD inspector: highlights risky commands before build
* 🔑 GPG key check (auto-import coming soon)
* ♻️ `ghostbrew upgrade` — Sync official, Chaotic-AUR, and AUR packages
* 🏴 `ghostbrew tap <repo>` — Add private PKGBUILD repos
* 🐚 `ghostbrew completion <shell>` — Shell completions
* 💪 Configurable via `~/.config/ghostbrew/config.yml`

---

## 🚀 Getting Started

```bash
git clone git@github.com:ghostkellz/ghostbrew.git
cd ghostbrew
go build -o ghostbrew
./ghostbrew --help
```

### 🔧 Requirements

* Arch Linux / Arch-based distro
* `git`, `go`, `makepkg`, `sudo`
* Internet (for AUR access)

---

## 🔮 Roadmap

* [x] Interactive AUR search (promptui)
* [x] Parallel, dependency-aware builds
* [x] PKGBUILD security audit
* [x] Shell completions
* [x] Config file support
* [x] Tap/private repo support
* [ ] GPG key auto-import
* [ ] Advanced upgrade logic (AUR + official)
* [ ] TUI support via Bubbletea
* [ ] Self-updating via GhostCTL
* [ ] Plugin/hook system

---

## 📂 Directory Structure

```
ghostbrew/
├── cmd/            # CLI commands (search, install, upgrade, tap, etc)
├── internal/       # AUR, git, makepkg helpers
├── config/         # YAML config parser
├── main.go
├── README.md
└── go.mod
```

---

## 🧙‍♂️ Philosophy

GhostBrew isn’t just another AUR helper — it's a haunted tool built for speed, security, and extensibility. Minimal, inspectable, and ready for dev-ops and mesh-native environments.

---

## 📝 License

MIT © [ghostkellz](https://github.com/ghostkellz)

