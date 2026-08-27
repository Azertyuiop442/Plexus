<h1 align="center">PLEXUS</h1>

<p align="center">
  <b>A modular, lightweight terminal multiplexer and live action dock in Rust - fully customizable with your own mods and agent feeds.</b>
</p>

<p align="center">
  <a href="#"><img src="https://img.shields.io/badge/Version-v0.1.0-24837B?style=flat-square&amp;labelColor=1C1B1A" alt="Version"/></a>
  <a href="https://peership.dev"><img src="https://img.shields.io/badge/Feedback-Want_to_leave_a_feedback%3F-8B7EC8?style=flat-square&amp;labelColor=1C1B1A" alt="Want to leave a feedback?"/></a>
  <a href="LICENSE.md"><img src="https://img.shields.io/badge/License-Community_%26_Commercial-DA702C?style=flat-square&amp;labelColor=1C1B1A" alt="License"/></a>
</p>

<p align="center">
  <img src="https://skillicons.dev/icons?i=rust,bash,git,linux,apple,windows" alt="Tech Stack"/>
</p>

<p align="center">
  <img src="assets/screenshot.png" alt="Plexus Terminal Multiplexer Screenshot" width="100%"/>
</p>

---

## 01. Architecture &amp; Mod Bridge

Plexus acts as a presentation shell decoupled from background logic. Companion mods (written in Python, Go, Node.js, Rust, or Bash) communicate non-blockingly via file-IPC in `/tmp/cc-sidebar/`:

- `mods-data/<mod>.json`: Mods push live widgets, metrics, and modals.
- `mod-pickup.json`: Plexus routes user clicks and triggers back to the active mod.

<p align="center">
  <img src="assets/architecture.svg" alt="Plexus Architecture Decoupling" width="100%"/>
</p>

---

## 02. Installation

### One-Command Installer

- **macOS / Linux**:
```bash
curl -fsSL https://raw.githubusercontent.com/Azertyuiop442/Plexus/public/install.sh | bash
```

- **Windows (PowerShell)**:
```powershell
irm https://raw.githubusercontent.com/Azertyuiop442/Plexus/public/install.ps1 | iex
```

### Manual Build
```bash
git clone https://github.com/Azertyuiop442/Plexus.git ~/.commandcode/mods/cc-dashboard
cd ~/.commandcode/mods/cc-dashboard
bash install.sh
```

---

## 03. Keybindings

| Shortcut | Action | Scope |
|:---:|---|:---:|
| <kbd>Ctrl</kbd> + <kbd>P</kbd> | Quick Switcher / Fuzzy Palette | `[Global]` |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Process Tree Inspector | `[Global]` |
| <kbd>Ctrl</kbd> + <kbd>B</kbd> | Toggle Left Sidebar | `[Global]` |
| <kbd>Ctrl</kbd> + <kbd>D</kbd> | Toggle Right Panel Dock | `[Global]` |
| <kbd>Ctrl</kbd> + <kbd>M</kbd> | Maximize / Restore Dock | `[Global]` |
| <kbd>Ctrl</kbd> + <kbd>Space</kbd> | Context Menu | `[Active Pane]` |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> · <kbd>+</kbd> | New Terminal Tab | `[Tab Bar]` |
| <kbd>Ctrl</kbd> + <kbd>W</kbd> · <kbd>x</kbd> | Close Tab | `[Active Tab]` |
| <kbd>Alt</kbd> + <kbd>1</kbd> .. <kbd>9</kbd> | Jump to Tab N | `[Tab Bar]` |
| <kbd>Ctrl</kbd> + <kbd>R</kbd> | Hot Reload Shell | `[Global]` |

---

## 04. Ecosystem &amp; Mod Hub

Discover and install community extensions on the **[Plexus Community Mods Hub](https://github.com/Azertyuiop442/plexus-community-mods)**.

---

## 05. License

Source-available under the [Community &amp; Commercial License](LICENSE.md). Free for personal, academic, and open-source use.

---

<h2 align="center">Feedback &amp; Community</h2>

<p align="center">
  Have feedback, bug reports, or ideas? We're actively co-testing on PeerShip! <br/>
  <b><a href="https://peership.dev">Test Plexus and leave feedback on PeerShip (peership.dev)</a></b>
</p>
