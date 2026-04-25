# Contributing to GhostBrew

Thank you for your interest in contributing to GhostBrew.

## Development Setup

### Prerequisites

- Rust 1.90+ (2024 edition)
- Clang/LLVM with BPF target
- libbpf and libbpf-dev
- bpftool
- Linux kernel 6.12+ with `CONFIG_SCHED_CLASS_EXT=y`

### Building

```bash
cargo build --release
```

### Running Tests

```bash
cargo test --release
```

### Code Quality

Before submitting changes, ensure:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --release
```

## Code Organization

- `src/` - Userspace Rust code
- `src/bpf/` - BPF C code and headers
- `src/bpf/scx/` - sched-ext compatibility headers
- `docs/` - Documentation
- `examples/` - Configuration examples
- `release/` - Distribution packaging

## Submitting Changes

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run the test suite
5. Submit a pull request

## Commit Messages

Use clear, descriptive commit messages:

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Test additions/changes
- `chore:` - Build/tooling changes

## License

- Userspace code is licensed under MIT
- BPF code is licensed under GPL-2.0 (required for kernel loading)

By contributing, you agree that your contributions will be licensed under the project's license terms.

## Questions

Open an issue on GitHub for questions or discussion.
