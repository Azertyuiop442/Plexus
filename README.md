<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/header/grid.svg?title=PLEXUS&subtitle=Modular+Terminal+Multiplexer+%26+Live+Action+Dock&logo=rust&theme=cyan&mode=dark" />
    <img alt="PLEXUS" src="https://shieldcn.dev/header/grid.svg?title=PLEXUS&subtitle=Modular+Terminal+Multiplexer+%26+Live+Action+Dock&logo=rust&theme=cyan&mode=light" />
  </picture>
</p>

<p align="center">
  <b>A modular, lightweight terminal multiplexer and live action dock in Rust - fully customizable with your own mods and agent feeds.</b>
</p>

<p align="center">
  <a href="https://github.com/Azertyuiop442/Plexus/releases"><img src="https://shieldcn.dev/badge/plexus-v0.1.5-24837b.svg?logo=rust&variant=outline" alt="Plexus v0.1.5"/></a>
  <a href="https://github.com/Azertyuiop442/Plexus"><img src="https://shieldcn.dev/github/stars/Azertyuiop442/Plexus.svg?variant=outline" alt="GitHub Stars"/></a>
  <a href="LICENSE.md"><img src="https://shieldcn.dev/badge/license-Fair_%26_Community.svg?variant=outline" alt="License"/></a>
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

## 02. Features

<details>
<summary><b>Click to expand technical features list</b></summary>
<br/>

- **Decoupled Multiplexing**: Native Rust PTY terminal multiplexer with persistent tabs, background process isolation, and hot reload (<kbd>Ctrl</kbd>+<kbd>R</kbd>).
- **Data-Driven Mod Bridge**: Any process can push live widgets, metrics, and interactive modals via JSON IPC (`/tmp/cc-sidebar/`). Fully language-agnostic.
- **Autonomous Agent Skills**:
  - Install skill bundles directly from any GitHub repository URL.
  - Live remote git tracking: automatically checks upstream commit freshness and flags updates.
  - One-click in-app background updater with progress indicators and automatic context injection.
- **Configurable Sound Alerts**:
  - Audio cues on task completion and user intervention / permission prompts.
  - Native, zero-latency playback on macOS (`afplay`), Windows (`PowerShell` / SystemSounds), and Linux (`paplay` / `pw-play`).
  - Anti-duplication run latch and global debounce cooldown to eliminate audio overlap across multiple open terminals.
  - Dedicated configuration modal with live sound preview and test triggers.
- **Transient Error Recovery & Auto-Retry**:
  - Real-time classifier for provider rate limits (429), server outages (5xx), and network drops.
  - Configurable exponential backoff, jitter, and interrupt safety.
- **Live Usage Telemetry**:
  - Interactive 1-line ASCII gauge tracking usage limits (5-hour, weekly, and monthly quotas).
- **Self-Pulling Update Engine**:
  - In-app version detection with automatic fast-forward updates.

</details>

---

## 03. Installation

### One-Command Installer

- **macOS / Linux**:
```bash
curl -fsSL https://raw.githubusercontent.com/Azertyuiop442/Plexus/public/install.sh | bash
```

- **Windows (PowerShell)**:
```powershell
irm https://raw.githubusercontent.com/Azertyuiop442/Plexus/public/install.ps1 | iex
```

### Upgrading from v0.1.3

To upgrade to v0.1.4 and activate the self-pulling update engine, re-run the one-command installer above or run:
```bash
git -C ~/.commandcode/mods/cc-dashboard pull --ff-only origin public && bash ~/.commandcode/mods/cc-dashboard/install.sh
```

### Manual Build
```bash
git clone https://github.com/Azertyuiop442/Plexus.git ~/.commandcode/mods/cc-dashboard
cd ~/.commandcode/mods/cc-dashboard
bash install.sh
```

---

## 04. Keybindings

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

## 05. Ecosystem &amp; Mod Hub

Discover and install community extensions on the **[Plexus Community Mods Hub](https://github.com/Azertyuiop442/plexus-community-mods)**.

---

## 06. License

Source-available under the [Fair &amp; Community License](LICENSE.md). Free for personal, academic, and open-source use.

---

<h2 align="center">Reviews &amp; Ratings</h2>

<p align="center">
  <a href="https://peership.dev/apps/abd3bcce-ceb3-4b11-a3d8-0781d98e43dc"><img src="https://peership.dev/api/badge/abd3bcce-ceb3-4b11-a3d8-0781d98e43dc" alt="Plexus on PeerShip"/></a>
  <br/><br/>
  Tested Plexus? Share your rating and review on PeerShip! <br/>
  👉 <b><a href="https://peership.dev/apps/abd3bcce-ceb3-4b11-a3d8-0781d98e43dc">Leave your review for Plexus on PeerShip</a></b>
</p>
