%global crate scx_ghostbrew

Name:           ghostbrew
Version:        0.1.0
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
cargo build --release

%install
install -Dpm 755 target/release/scx_ghostbrew %{buildroot}%{_bindir}/scx_ghostbrew
install -Dpm 644 scx-ghostbrew.service %{buildroot}%{_unitdir}/scx-ghostbrew.service

# Documentation
install -Dpm 644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -Dpm 644 docs/ARCHITECTURE.md %{buildroot}%{_docdir}/%{name}/ARCHITECTURE.md
install -Dpm 644 docs/TUNING.md %{buildroot}%{_docdir}/%{name}/TUNING.md
install -Dpm 644 docs/CHANGELOG.md %{buildroot}%{_docdir}/%{name}/CHANGELOG.md

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
%doc README.md docs/ARCHITECTURE.md docs/TUNING.md docs/CHANGELOG.md
%{_bindir}/scx_ghostbrew
%{_unitdir}/scx-ghostbrew.service

%changelog
* Tue Dec 17 2024 ghostkellz <ckelley@ghostkellz.sh> - 0.1.0-1
- Initial release
- sched-ext BPF scheduler for AMD Zen5/X3D
- BORE-inspired burst detection
- V-Cache CCD routing for gaming
- VM, container, and AI workload support
