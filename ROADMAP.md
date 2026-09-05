# Plexus Roadmap & Architecture Explorations

This document outlines upcoming architectural capabilities, planned features, and research tracks for **Plexus**.

---

## 1. Cross-Session Multi-Agent Communication (Inter-Terminal Mesh)

### Overview
Modern development workflows often involve multiple concurrent agent sessions working in the same workspace or across related repositories (e.g. Frontend in Tab 1, Backend API in Tab 2, Test Suite in Tab 3). Currently, each terminal session runs in total isolation. 

Plexus acts as the host multiplexer and process supervisor, placing it in the ideal position to serve as the **local communication bus and peer discovery orchestrator**.

---

### Architecture & Mechanisms

```
+-----------------------------------------------------------------------+
|                              PLEXUS MUX                               |
|                                                                       |
|   +-------------------+  +-------------------+  +-------------------+  |
|   |    Terminal 1     |  |    Terminal 2     |  |    Terminal 3     |  |
|   |   (Backend API)   |  |    (Frontend UI)  |  |    (QA / Tests)   |  |
|   +---------+---------+  +---------+---------+  +---------+---------+  |
|             |                      |                      |           |
+-------------|----------------------|----------------------|-----------+
              |                      |                      |
              v                      v                      v
+-----------------------------------------------------------------------+
|                    PLEXUS LOCAL IPC / MESSAGE BUS                     |
|                                                                       |
|  * Peer Registry (`/tmp/cc-sidebar/peers.json`)                       |
|    - Lists active panes, PID, workspace CWD, agent state & model     |
|                                                                       |
|  * Inter-Agent Messaging (P2P Message Routing)                        |
|    - `SendMessage(target_terminal, message)` via MCP tool / Mod Hook  |
|    - Non-blocking inbound queue with configurable delivery pacing    |
|                                                                       |
|  * File Locking & Anti-Collision Tracker                              |
|    - Tracks active files being edited per pane to avoid git conflicts |
|                                                                       |
|  * Ambient Workspace Feed (Passive Context Injection)                 |
|    - Single-line status banner injected into prompt context           |
|      `[Workspace: Terminal 1 editing auth.rs | Terminal 2 idle]`      |
+-----------------------------------------------------------------------+
```

---

### Key Capabilities

1. **Peer Discovery (`list_agents` / `peers`)**:
   - Allows an agent in any pane to query which other agents/terminals are running in the current workspace.
   - Provides live metadata: terminal label, working directory, current status (`Working`, `Idle`, `Blocked`), and active model.

2. **Cross-Session Message Routing (`SendMessage`)**:
   - Direct message delivery between panes.
   - Enables handoffs (e.g. Backend agent notifies Frontend agent: *"Auth endpoint updated to v2 with payload schema {...}"*).
   - Inbound controls: allow agents to accept messages immediately, queue them until the next prompt turn, or hold them.

3. **Workspace File Locking & Anti-Collision**:
   - Agents register files currently being drafted or edited.
   - Other agents checking the registry avoid concurrent edits on the same files, eliminating git merge collisions.

4. **Task Delegation & Agent Teams**:
   - Coordinator tab acts as team lead, decomposing a feature into sub-tasks and delegating to specialized worker tabs.

---

## 2. Planned Roadmap Tiers

| Milestone | Target | Focus Areas |
| :--- | :--- | :--- |
| **v0.1.3** | *Shipped* | Multi-tab Error Recovery, Real-Time Auto-Retry, macOS Process Immunity |
| **v0.1.4** | *Shipped* | Agent Skills Manager, Live Remote Git Sync, One-Click GitHub Bundle Install & Autonomous Auto-Update Engine Fix |
| **v0.1.5** | *Current Release* | Cross-Platform Sound Alerts & Notifications, Audio Preview Modal, Task Completion & Blocked State Triggers |
| **v0.2.0** | *Upcoming* | Peer Registry Daemon, `peers.json` IPC Bridge, Ambient Workspace Context |
| **v0.2.5** | *Planned* | `SendMessage` Inter-Agent Tool (MCP), Inbound Delivery Queue & Controls |
| **v0.3.0** | *Planned* | Workspace File Locking, Multi-Agent Git Conflict Prevention, Team Handoffs |
