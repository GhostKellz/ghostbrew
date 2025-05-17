# 👻 ghostbrew

[![Go Build](https://github.com/ghostkellz/ghostbrew/actions/workflows/go.yml/badge.svg)](https://github.com/ghostkellz/ghostbrew/actions)
[![Arch Linux](https://img.shields.io/badge/platform-Arch%20Linux-1793d1?logo=arch-linux\&logoColor=white)](https://archlinux.org)
[![Made with Go](https://img.shields.io/badge/made%20with-Go-00ADD8?logo=go\&logoColor=white)](https://golang.org)
[![Repo Status](https://img.shields.io/badge/status-active-success?style=flat-square)](https://github.com/ghostkellz/ghostbrew)

---

**ghostbrew** is a fast, minimal, and haunted AUR helper for Arch Linux.
Inspired by `yay` and `paru`, but tailored to the GhostKellz ecosystem and terminal-native tooling like `ghostctl`.

> 🧪 Install AUR packages
> 📦 Build from source with safety checks
> ♻️ Upgrade & manage system packages
> 💀 Future support for private AUR overlays

---

## ✨ Features

* 🔍 `ghostbrew search <pkg>` — AUR package search
* 📦 `ghostbrew install <pkg>` — Clone & build from AUR
* ♻️ `ghostbrew upgrade` — Sync packages
* 🔐 Sane defaults, secure build logic
* ⚙️ CLI & future TUI support (Bubbletea)
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

* [x] AUR search (JSON API)
* [x] Install via `makepkg`
* [ ] TUI support via Bubbletea
* [ ] Private repo tap support (`ghostbrew tap`)
* [ ] Parallel build queueing
* [ ] Self-updating via GhostCTL
* [ ] GPG signature verification

---

## 📂 Directory Structure

```
ghostbrew/
├── cmd/            # CLI commands (install, search, upgrade)
├── internal/       # AUR, git, makepkg helpers
├── config/         # YAML config parser
├── main.go
├── README.md
└── go.mod
```

---

## 🧙‍♂️ Philosophy

GhostBrew isn’t just another AUR helper — it's a haunted tool built with intention:
Minimal, inspectable, and extensible by design. A brewing ground for dev-ops, Arch, and mesh-native environments.

---

## 📝 License

MIT © [ghostkellz](https://github.com/ghostkellz)
# 👻 ghostbrew

[![Go Build](https://github.com/ghostkellz/ghostbrew/actions/workflows/go.yml/badge.svg)](https://github.com/ghostkellz/ghostbrew/actions)
[![Arch Linux](https://img.shields.io/badge/platform-Arch%20Linux-1793d1?logo=arch-linux\&logoColor=white)](https://archlinux.org)
[![Made with Go](https://img.shields.io/badge/made%20with-Go-00ADD8?logo=go\&logoColor=white)](https://golang.org)
[![Repo Status](https://img.shields.io/badge/status-active-success?style=flat-square)](https://github.com/ghostkellz/ghostbrew)

---

**ghostbrew** is a fast, minimal, and haunted AUR helper for Arch Linux.
Inspired by `yay` and `paru`, but tailored to the GhostKellz ecosystem and terminal-native tooling like `ghostctl`.

> 🧪 Install AUR packages
> 📦 Build from source with safety checks
> ♻️ Upgrade & manage system packages
> 💀 Future support for private AUR overlays

---

## ✨ Features

* 🔍 `ghostbrew search <pkg>` — AUR package search
* 📦 `ghostbrew install <pkg>` — Clone & build from AUR
* ♻️ `ghostbrew upgrade` — Sync packages
* 🔐 Sane defaults, secure build logic
* ⚙️ CLI & future TUI support (Bubbletea)
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

* [x] AUR search (JSON API)
* [x] Install via `makepkg`
* [ ] TUI support via Bubbletea
* [ ] Private repo tap support (`ghostbrew tap`)
* [ ] Parallel build queueing
* [ ] Self-updating via GhostCTL
* [ ] GPG signature verification

---

## 📂 Directory Structure

```
ghostbrew/
├── cmd/            # CLI commands (install, search, upgrade)
├── internal/       # AUR, git, makepkg helpers
├── config/         # YAML config parser
├── main.go
├── README.md
└── go.mod
```

---

## 🧙‍♂️ Philosophy

GhostBrew isn’t just another AUR helper — it's a haunted tool built with intention:
Minimal, inspectable, and extensible by design. A brewing ground for dev-ops, Arch, and mesh-native environments.

---

## 📝 License

MIT © [ghostkellz](https://github.com/ghostkellz)

