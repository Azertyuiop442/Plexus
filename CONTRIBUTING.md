# Contributing to Plexus

Thank you for your interest in contributing to **Plexus**. We welcome bug reports, architectural discussions, and code contributions from the community.

---

## 1. Bug Reports

Before opening a bug report, search existing issues to check if the problem has already been documented.

When filing an issue, please provide:
- **Environment**: OS (macOS / Linux / Windows), architecture (x86_64 / aarch64), and terminal emulator (e.g. Ghostty, iTerm2, Alacritty, Kitty).
- **Version**: Commit hash or output of `plexus --version`.
- **Reproduction**: Minimal, clear sequence of steps to reproduce the behavior.
- **Expected vs. Actual**: What you expected to happen versus what occurred.
- **Logs**: Any relevant terminal output or crash traces.

---

## 2. Feature Proposals

Plexus is designed as a focused, high-performance terminal multiplexer and presentation shell. To maintain code quality and performance:

1. **Open an Issue First**: Discuss significant architectural changes or new commands before submitting a pull request.
2. **Decoupled Architecture**: Features should respect the boundary between the native multiplexer shell and external mods. Complex domain logic belongs in external companion mods communicating via the Mod Bridge (`/tmp/cc-sidebar/`), not hardcoded into the core binary.

---

## 3. Development Workflow

### Prerequisites
- Rust 1.78+ (`rustup`)
- Cargo
- A Nerd Font for icon rendering in terminal

### Building & Testing
```bash
# Clone repository
git clone https://github.com/Azertyuiop442/Plexus.git
cd Plexus

# Run test suite
cargo test

# Build debug binary
cargo build --bin plexus

# Build release binary
cargo build --release --bin plexus
```

### Code Standards
- **Style**: Follow standard Rust conventions (`cargo fmt` / `cargo clippy`).
- **Color Palette**: UI colors must strictly adhere to the Flexoki Dark palette defined in `src/theme.rs`. Do not hardcode raw RGB values outside the theme module.
- **Icons**: Terminal glyphs must use Nerd Font codepoints through the `nf-icons` macro library.
- **Modularity**: Keep components decoupled and focused (< 1,000 lines per file).
- **Tests**: All logic changes and new features must be accompanied by unit tests.

---

## 4. Contributor License Agreement (CLA)

By submitting a pull request or patch to this repository, you agree that your contribution is provided under the terms of the project's [LICENSE.md](LICENSE.md), granting the copyright holder full rights to incorporate, distribute, and license the contribution.
