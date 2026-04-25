%global crate scx_ghostbrew

Name:           ghostbrew
Version:        0.3.1
Release:        1%{?dist}
Summary:        sched-ext BPF scheduler optimized for AMD Zen5/X3D processors

License:        MIT
URL:            https://github.com/ghostkellz/ghostbrew
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

ExclusiveArch:  x86_64

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  clang
BuildRequires:  llvm
BuildRequires:  libbpf-devel
BuildRequires:  bpftool
BuildRequires:  systemd-rpm-macros

Requires:       libbpf
Requires:       kernel >= 6.12

# Kernel must have CONFIG_SCHED_CLASS_EXT=y
Recommends:     kernel-cachyos
Suggests:       kernel-ghost

%description
GhostBrew (scx_ghostbrew) is a custom sched-ext BPF scheduler designed
specifically for AMD Zen5 and X3D processors. It combines BORE-inspired
burst detection with hardware-aware scheduling to deliver optimal
performance for gaming and desktop workloads.

Features:
- Per-CCD dispatch queues with topology-aware scheduling
- X3D V-Cache CCD detection and gaming task routing
- BORE-inspired burst detection for interactive prioritization
- AMD Prefcore integration for preferred core selection
- NVIDIA GPU detection with ReBAR awareness
- KVM/QEMU VM and container workload detection

%prep
%autosetup -n ghostbrew-%{version}

%build
export RUSTFLAGS="-C opt-level=3"
cargo build --release --bin ghostbrew --bin scx_ghostbrew --target x86_64-unknown-linux-gnu

%install
install -Dpm 755 target/x86_64-unknown-linux-gnu/release/ghostbrew %{buildroot}%{_bindir}/ghostbrew
install -Dpm 755 target/x86_64-unknown-linux-gnu/release/scx_ghostbrew %{buildroot}%{_bindir}/scx_ghostbrew
install -Dpm 644 scx-ghostbrew.service %{buildroot}%{_unitdir}/scx-ghostbrew.service
install -Dpm 644 man/ghostbrew.1 %{buildroot}%{_mandir}/man1/ghostbrew.1
install -Dpm 644 man/scx_ghostbrew.1 %{buildroot}%{_mandir}/man1/scx_ghostbrew.1
install -Dpm 644 assets/icons/ghostbrew-icon.png %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/ghostbrew.png
install -d %{buildroot}%{_datadir}/bash-completion/completions
install -d %{buildroot}%{_datadir}/zsh/site-functions
install -d %{buildroot}%{_datadir}/fish/vendor_completions.d
target/x86_64-unknown-linux-gnu/release/ghostbrew completions bash > %{buildroot}%{_datadir}/bash-completion/completions/ghostbrew
target/x86_64-unknown-linux-gnu/release/ghostbrew completions zsh > %{buildroot}%{_datadir}/zsh/site-functions/_ghostbrew
target/x86_64-unknown-linux-gnu/release/ghostbrew completions fish > %{buildroot}%{_datadir}/fish/vendor_completions.d/ghostbrew.fish
target/x86_64-unknown-linux-gnu/release/scx_ghostbrew --completions bash > %{buildroot}%{_datadir}/bash-completion/completions/scx_ghostbrew
target/x86_64-unknown-linux-gnu/release/scx_ghostbrew --completions zsh > %{buildroot}%{_datadir}/zsh/site-functions/_scx_ghostbrew
target/x86_64-unknown-linux-gnu/release/scx_ghostbrew --completions fish > %{buildroot}%{_datadir}/fish/vendor_completions.d/scx_ghostbrew.fish

# Documentation
install -Dpm 644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -Dpm 644 docs/README.md %{buildroot}%{_docdir}/%{name}/docs/README.md
install -Dpm 644 docs/architecture/overview.md %{buildroot}%{_docdir}/%{name}/docs/architecture/overview.md
install -Dpm 644 docs/guides/tuning.md %{buildroot}%{_docdir}/%{name}/docs/guides/tuning.md
install -Dpm 644 docs/benchmarks.md %{buildroot}%{_docdir}/%{name}/benchmarks.md
install -Dpm 644 docs/benchmarks/README.md %{buildroot}%{_docdir}/%{name}/docs/benchmarks/README.md
install -Dpm 644 docs/benchmarks/9950x3d-dev-report.txt %{buildroot}%{_docdir}/%{name}/docs/benchmarks/9950x3d-dev-report.txt
install -Dpm 644 docs/benchmarks/9950x3d-dev-report.json %{buildroot}%{_docdir}/%{name}/docs/benchmarks/9950x3d-dev-report.json
install -Dpm 644 docs/guides/troubleshooting.md %{buildroot}%{_docdir}/%{name}/docs/guides/troubleshooting.md
install -Dpm 644 docs/features/dl-server.md %{buildroot}%{_docdir}/%{name}/docs/features/dl-server.md
install -Dpm 644 docs/features/support-bundle.md %{buildroot}%{_docdir}/%{name}/docs/features/support-bundle.md
install -Dpm 644 CHANGELOG.md %{buildroot}%{_docdir}/%{name}/CHANGELOG.md
install -Dpm 755 release/install-system.sh %{buildroot}%{_docdir}/%{name}/install-system.sh

# License
install -Dpm 644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE

%post
%systemd_post scx-ghostbrew.service

%preun
%systemd_preun scx-ghostbrew.service

%postun
%systemd_postun_with_restart scx-ghostbrew.service

%files
%license LICENSE
%doc README.md docs/README.md docs/architecture/overview.md docs/guides/tuning.md docs/benchmarks.md docs/benchmarks/README.md docs/benchmarks/9950x3d-dev-report.txt docs/benchmarks/9950x3d-dev-report.json docs/guides/troubleshooting.md docs/features/dl-server.md docs/features/support-bundle.md CHANGELOG.md
%{_bindir}/ghostbrew
%{_bindir}/scx_ghostbrew
%{_unitdir}/scx-ghostbrew.service
%{_mandir}/man1/ghostbrew.1*
%{_mandir}/man1/scx_ghostbrew.1*
%{_datadir}/icons/hicolor/256x256/apps/ghostbrew.png
%{_datadir}/bash-completion/completions/ghostbrew
%{_datadir}/bash-completion/completions/scx_ghostbrew
%{_datadir}/zsh/site-functions/_ghostbrew
%{_datadir}/zsh/site-functions/_scx_ghostbrew
%{_datadir}/fish/vendor_completions.d/ghostbrew.fish
%{_datadir}/fish/vendor_completions.d/scx_ghostbrew.fish

%changelog
* Thu Apr 24 2026 ghostkellz <ckelley@ghostkellz.sh> - 0.3.1-1
- Version alignment across Cargo.toml, CLI, BPF, and packaging
- Fixed clippy warnings for checked division patterns
- Added SECURITY.md and CONTRIBUTING.md
- Documentation restructure (lowercase filenames)
- Updated dependencies
- Fixed DL server detection to require sysfs interface

* Mon Mar 31 2026 ghostkellz <ckelley@ghostkellz.sh> - 0.3.0-1
- Wakeup frequency tracking with EWMA
- SMT contention avoidance
- Futex-aware scheduling with priority boost
- Core compaction / power save modes
- Tickless mode for reduced timer overhead
- Per-game latency histograms (P50/P95/P99)
- GPU scheduler coordination
- DL server integration (kernel 7.0+)
- NUMA-aware game profiles

* Wed Feb 19 2026 ghostkellz <ckelley@ghostkellz.sh> - 0.2.2-1
- Linux 7.0 kernel compatibility
- Synced sched-ext headers from kernel 7.0-rc
- Bumped libbpf-rs/libbpf-cargo to 0.26
- Updated BSS/rodata access for new API

* Wed Jan 15 2026 ghostkellz <ckelley@ghostkellz.sh> - 0.2.1-1
- Resolved clippy warnings for Rust 1.92
- Minor code quality improvements

* Sat Jan 11 2026 ghostkellz <ckelley@ghostkellz.sh> - 0.2.0-1
- AMD Ryzen 9950X3D (Zen5) support with 128MB V-Cache
- Intel Hybrid CPU support (P-core/E-core differentiation)
- Per-game profiles (25+ profiles for games, streaming, productivity)
- V-Cache coordination with ghost-vcache sysfs
- MangoHud integration for frame time analysis
- Runtime control interface at /run/ghostbrew/control
- TOML configuration system
- Event streaming via BPF ringbuf
- 43 tests (29 unit + 14 integration)

* Tue Dec 17 2024 ghostkellz <ckelley@ghostkellz.sh> - 0.1.0-1
- Initial release
- sched-ext BPF scheduler for AMD Zen5/X3D
- BORE-inspired burst detection
- V-Cache CCD routing for gaming
- VM, container, and AI workload support
