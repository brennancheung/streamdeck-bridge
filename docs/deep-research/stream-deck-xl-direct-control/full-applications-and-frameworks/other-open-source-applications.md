# Other Open-Source Applications

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Full Applications & Frameworks

---

## Key Findings

- **OpenDeck** (Rust/Svelte/Tauri, 1.7k stars) is the most feature-complete cross-platform alternative, with backward-compatible support for official Elgato Stream Deck SDK plugins via WebSocket and Wine integration
- **StreamController** (Python/GTK4, 1k stars) is the leading Linux-native option with a built-in plugin store, but is still in beta with known memory issues
- **Boatswain** (C/GTK4, GNOME ecosystem) is the most polished GNOME-native app but has deliberately limited its plugin API to in-tree only
- **Deckmaster** (Go, 294 stars) takes a unique widget-based approach with TOML configuration, well-suited for power users who prefer config files over GUIs
- Pure libraries (python-elgato-streamdeck, node-elgato-stream-deck, elgato-streamdeck Rust crate, StreamDeckSharp .NET) are the real building blocks -- all support the XL model and are better starting points than forking a full application
- No existing open-source application exposes a general-purpose REST or WebSocket API for external programmatic control of the device; this is a clear gap in the ecosystem
- For a build-vs-reuse decision: use a **library** (Python, Node, Rust, or .NET) as the foundation and build a custom control layer on top, rather than trying to adapt an existing end-user application

---

## 1. Full Applications (End-User Software)

### 1.1 OpenDeck

| Attribute | Detail |
|---|---|
| **Repository** | [nekename/OpenDeck](https://github.com/nekename/OpenDeck) |
| **Language/Stack** | Rust (53%) + Svelte (29%) + TypeScript (5%), built with Tauri |
| **Stars** | ~1,700 |
| **Last Release** | v2.12.0 (May 2026) |
| **License** | GPL-3.0 |
| **Platforms** | Linux, Windows, macOS |
| **XL Support** | Yes |

**Key Differentiator:** Backward compatibility with the official Elgato Stream Deck SDK plugin ecosystem. This means hundreds of existing plugins work out of the box. On Linux and macOS, Windows-only plugins run via Wine.

**Architecture:**
- Tauri desktop app with Rust backend and SvelteKit frontend
- Hardware communication via the `elgato-streamdeck` Rust crate
- Plugin communication over WebSocket connections (bidirectional, real-time)
- Three plugin runtime environments: Wine (Windows plugins cross-platform), Node.js (JS/TS plugins), Native (compiled binaries)
- Profiles stored in folder-based organization with automatic window-based switching

**Plugin System:**
- Supports both the legacy Stream Deck SDK protocol and the newer OpenAction API
- OpenAction API is a cross-platform, device-agnostic successor designed to work with any programmable control surface
- Property Inspector UI for per-action configuration
- Plugin management through a graphical interface

**Strengths:** Most complete feature set among open-source options. Cross-platform. Active development. Elgato plugin compatibility is a massive advantage.

**Weaknesses:** GPL-3.0 license constrains derivative work. No exposed API for external programmatic control -- it is an end-user GUI application.

---

### 1.2 StreamController

| Attribute | Detail |
|---|---|
| **Repository** | [StreamController/StreamController](https://github.com/StreamController/StreamController) |
| **Language/Stack** | Python (99%), GTK4/libadwaita |
| **Stars** | ~1,000 |
| **Last Release** | 1.5.0-beta.14 (May 2026) |
| **License** | GPL-3.0 |
| **Platforms** | Linux only |
| **XL Support** | Yes (Original v2, Mini, XL, Pedal, Plus, Neo, Modules) |

**Key Differentiator:** Built-in plugin store for discovering and installing community plugins. Deeply integrated with the GNOME/Linux desktop, supporting automatic page switching based on active window across GNOME, Hyprland, Sway, KDE, and X11.

**Architecture:**
- Python application with GTK4/libadwaita UI
- Distributed via Flatpak (Flathub)
- Plugin tools published as a separate PyPI package (`streamcontroller-plugin-tools`)
- Custom plugin API (not Elgato SDK-compatible)

**Plugin System:**
- Plugins written in Python using the `streamcontroller-plugin-tools` base package
- Built-in marketplace for plugin discovery and updates
- Community plugins for OBS, weather, media control, and more

**Strengths:** Best Linux desktop integration. Active plugin ecosystem. Clean GTK4 UI with wallpaper/video customization and screensaver support.

**Weaknesses:** Linux only. Still in beta with documented memory usage concerns. No Elgato SDK plugin compatibility. GPL-3.0 license.

---

### 1.3 Boatswain

| Attribute | Detail |
|---|---|
| **Repository** | [World/boatswain](https://gitlab.gnome.org/World/boatswain) (GitLab) |
| **Language/Stack** | C, GTK4/libadwaita |
| **Stars** | N/A (GNOME GitLab; 911 commits) |
| **Last Release** | v6.0 |
| **License** | GPL-3.0 |
| **Platforms** | Linux only (GNOME) |
| **XL Support** | Yes (uses reverse-engineered HID protocol) |

**Key Differentiator:** Official GNOME ecosystem application. The developer contributed udev rules upstream to systemd so Stream Deck devices are automatically accessible without manual configuration.

**Architecture:**
- Native C application with GTK4/libadwaita
- Internal plugin architecture, but all plugins are deliberately in-tree (no public plugin API yet)
- Direct HID communication with Stream Deck hardware
- Distributed exclusively via Flathub

**Features:**
- OBS Studio integration (via obs-websocket)
- Network request actions
- Gaming score tracker
- File/application launcher
- Folders, profiles, and multi-actions

**Strengths:** Native performance (C). Clean GNOME integration. Upstream-first philosophy (systemd udev rules contribution).

**Weaknesses:** No public plugin API. GNOME-specific. Limited action types compared to OpenDeck or StreamController. GPL-3.0 license.

---

### 1.4 Deckmaster

| Attribute | Detail |
|---|---|
| **Repository** | [muesli/deckmaster](https://github.com/muesli/deckmaster) |
| **Language/Stack** | Go (100%) |
| **Stars** | 294 |
| **Last Release** | v0.9.0 (March 2023) |
| **License** | MIT |
| **Platforms** | Linux |
| **XL Support** | Yes (explicit USB PID `0x006c` support) |

**Key Differentiator:** Widget-based architecture with TOML configuration files. No GUI for configuration -- everything is defined in `.deck` files. Ideal for power users and automation workflows.

**Architecture:**
- Built on the `muesli/streamdeck` Go library
- Configuration via TOML `.deck` files
- Modular widget system (each widget type is a Go module)
- Supports multiple pages with deck navigation
- Short press vs. long press differentiation

**Widget System:**
- **Button** -- static icons and labels
- **Time** -- clock/date display with custom formatting
- **Top** -- live CPU and memory utilization bars
- **Command** -- renders output of shell commands on the button
- **Weather** -- temperature and conditions with themed icons
- **Recent Window** -- shows recently focused X11 windows

**Action System:** Deck switching, shell command execution, keyboard emulation (xdotool), clipboard paste, D-Bus method calls.

**Strengths:** MIT license. Clean Go codebase. TOML config is version-controllable. Widget system is easy to extend.

**Weaknesses:** Development appears stalled (last release March 2023, 21 open PRs). X11-dependent features will not work on Wayland. No GUI configuration.

---

### 1.5 streamdeck-linux-gui

| Attribute | Detail |
|---|---|
| **Repository** | [streamdeck-linux-gui/streamdeck-linux-gui](https://github.com/streamdeck-linux-gui/streamdeck-linux-gui) |
| **Language/Stack** | Python (97%), Qt-based UI |
| **Stars** | 416 |
| **Last Release** | v4.1.4 (February 2026) |
| **License** | MIT |
| **Platforms** | Linux |
| **XL Support** | Yes (Original, MK2, Mini, XL, Pedal) |

**Key Differentiator:** The simplest graphical Linux Stream Deck tool. A maintained fork of the abandoned `streamdeck-ui` project. The maintainers themselves recommend StreamController for new users seeking more features.

**Architecture:**
- Built on the `python-elgato-streamdeck` library
- Uses `evdev` with `uinput` for key press simulation
- Runs as a systemd background service
- Configuration stored in JSON, supports import/export

**Features:**
- Multi-device support, multi-page layouts
- Drag-and-drop button arrangement
- Animated GIF icons
- Brightness control and auto-dim
- Auto-reconnect on device unplug/replug
- Command execution, hotkeys, text input

**Strengths:** MIT license. Simple and stable. Good for basic use cases. Active maintenance for bug fixes.

**Weaknesses:** Maintenance mode only -- no new features accepted. Limited action types. No plugin system.

---

### 1.6 deckxstream

| Attribute | Detail |
|---|---|
| **Repository** | [SecretAgentKen/deckxstream](https://github.com/SecretAgentKen/deckxstream) |
| **Language/Stack** | JavaScript (93%) + C++ (7%), Node.js |
| **Stars** | 20 |
| **Last Update** | No longer actively maintained (security updates only) |
| **License** | MIT |
| **Platforms** | Linux |
| **XL Support** | Yes (USB PID `0x006c`) |

**Key Differentiator:** Dynamic buttons -- any button's icon, text, or command can be updated in real-time from the output of a running process. Also supports dynamic pages generated from command output.

**Architecture:**
- Built on the `elgato-stream-deck` npm library
- JSON configuration file (`.deckxstream.json` in HOME)
- Uses `libxdo` bindings for hotkey/text simulation
- Child process spawning for command execution

**Features:**
- PNG, SVG, and animated GIF support
- Dynamic buttons updated by external process output
- Dynamic pages generated from commands
- Sticky buttons that persist across pages
- Base64 data URI icon support
- Screensaver animations

**Strengths:** MIT license. Dynamic button concept is powerful for monitoring/dashboard use cases.

**Weaknesses:** Unmaintained. Small community. Linux only. Requires `libxdo-dev`.

---

### 1.7 Freedeck

| Attribute | Detail |
|---|---|
| **Repository** | [Freedeck/Freedeck](https://github.com/Freedeck/Freedeck) |
| **Language/Stack** | JavaScript (65%) + HTML (24%) + CSS (12%), Webpack |
| **Stars** | 18 |
| **Last Release** | v6.0.0-ob3 (May 2024) |
| **License** | AGPL-3.0 |
| **Platforms** | Windows (primary), cross-platform planned |
| **XL Support** | Phone/tablet-based (not hardware Stream Deck) |

**Key Differentiator:** Turns a phone/tablet into a Stream Deck via a web UI. Not designed for Elgato hardware -- it is a software-only macro pad.

**Architecture:**
- Web-based UI served from a desktop companion app
- Plugin system with a built-in Marketplace
- One-click installer

**Strengths:** Simple plugin system. Active solo development over 3+ years.

**Weaknesses:** AGPL-3.0 license. Not for Elgato hardware. Small community. Windows-centric.

---

### 1.8 Other Notable Mentions

| Project | Stack | Stars | Notes |
|---|---|---|---|
| **ODeck** ([willianrod/ODeck](https://github.com/willianrod/ODeck)) | TypeScript/React/Electron + React Native | 436 | Phone-to-desktop control via Socket.io. MIT license. Not for Elgato hardware. |
| **WebDeck** ([Lenochxd/WebDeck](https://github.com/Lenochxd/WebDeck)) | Python/Flask/Jinja2 | 892 | Browser-based control via QR code pairing. GPL-3.0. Windows only. Not for Elgato hardware. |
| **Stream-Pi** ([Stream-Pi](https://github.com/Stream-Pi)) | Java/JavaFX | 253 (server) | Client-server architecture. Runs on Raspberry Pi, phones, or any device. GPL-3.0. Has an Action API for Java plugins. Development stalled (last release July 2021). |
| **SnakeDeck** ([jpetazzo/snakedeck](https://github.com/jpetazzo/snakedeck)) | Python | 24 | Alpha-quality but used in production. Supports inline Python eval on key press. YAML config. Multicast sync between multiple Stream Decks. |
| **Macro Deck** ([Macro-Deck-App](https://github.com/Macro-Deck-App)) | C#/.NET | varies | Phone-to-desktop macro pad with a separate Stream Deck Connector (WebSocket bridge to USB). Windows-focused. |

---

## 2. Libraries and Frameworks (Building Blocks)

These are not end-user applications -- they provide the low-level device communication layer that applications are built on top of. All support the Stream Deck XL.

### 2.1 python-elgato-streamdeck

| Attribute | Detail |
|---|---|
| **Repository** | [abcminiuser/python-elgato-streamdeck](https://github.com/abcminiuser/python-elgato-streamdeck) |
| **Stars** | ~1,100 |
| **License** | MIT |
| **Install** | `pip install streamdeck` |
| **XL Support** | Yes (32 keys, 8x4 grid, 96x96 JPEG per key) |

The foundational Python library. Used by StreamController, streamdeck-linux-gui, SnakeDeck, and many others. Supports all known Stream Deck models including the Studio. Provides device enumeration, brightness control, key image setting, and button state callbacks.

### 2.2 node-elgato-stream-deck

| Attribute | Detail |
|---|---|
| **Repository** | [Julusian/node-elgato-stream-deck](https://github.com/Julusian/node-elgato-stream-deck) |
| **Stars** | 197 |
| **License** | MIT |
| **Install** | `npm install @elgato-stream-deck/node` |
| **XL Support** | Yes |

TypeScript monorepo providing multiple packages: `@elgato-stream-deck/core` (platform-agnostic), `@elgato-stream-deck/node` (Node.js HID), `@elgato-stream-deck/webhid` (browser via Chromium WebHID), and `@elgato-stream-deck/tcp`. Used by Bitfocus Companion. The WebHID variant is particularly interesting for browser-based control panels.

### 2.3 elgato-streamdeck (Rust)

| Attribute | Detail |
|---|---|
| **Repository** | [OpenActionAPI/rust-elgato-streamdeck](https://github.com/OpenActionAPI/rust-elgato-streamdeck) |
| **Stars** | ~60 |
| **License** | MIT |
| **Install** | `cargo add elgato-streamdeck` |
| **XL Support** | Yes |

The Rust crate used by OpenDeck. Based on `hidapi`. Supports all current Stream Deck models. Created as a better-designed successor to `ryankurte/rust-streamdeck`.

### 2.4 StreamDeckSharp (.NET)

| Attribute | Detail |
|---|---|
| **Repository** | [OpenMacroBoard/StreamDeckSharp](https://github.com/OpenMacroBoard/StreamDeckSharp) |
| **Stars** | 395 |
| **License** | MIT |
| **Install** | NuGet `StreamDeckSharp` |
| **XL Support** | Yes |

Unofficial .NET wrapper. Simple API: `var deck = StreamDeck.OpenDevice()`. Supports key image bitmaps, brightness, and key state events. Part of the OpenMacroBoard project which also provides a higher-level SDK abstraction.

### 2.5 Go Libraries

| Library | Stars | Notes |
|---|---|---|
| [muesli/streamdeck](https://github.com/muesli/streamdeck) | ~80 | Foundation for Deckmaster. Linux-focused. |
| [dh1tw/streamdeck](https://github.com/dh1tw/streamdeck) | ~40 | MIT license. Cross-platform aspirations. |
| [magicmonkey/go-streamdeck](https://github.com/magicmonkey/go-streamdeck) | ~30 | Ubuntu-focused. Go 1.13+. |

### 2.6 @fnando/streamdeck (Archived)

| Attribute | Detail |
|---|---|
| **Repository** | [fnando/streamdeck](https://github.com/fnando/streamdeck) (Archived Dec 2025) |
| **Stars** | 36 |
| **License** | MIT |

A lean TypeScript framework for developing Elgato Stream Deck **plugins** (not device control). Provided scaffolding, bundling, and release automation for the official Stream Deck plugin format. Archived but worth studying for its plugin development workflow.

---

## 3. Architectural Patterns Comparison

### 3.1 Plugin Systems

| Application | Plugin Approach | Elgato SDK Compatible | Language |
|---|---|---|---|
| **OpenDeck** | WebSocket protocol, multiple runtimes | Yes (full) | Any (via Wine/Node/Native) |
| **StreamController** | Python packages via PyPI | No | Python only |
| **Boatswain** | In-tree only (no public API) | No | C |
| **Deckmaster** | Widget modules (compiled in) | No | Go |
| **Freedeck** | JS plugins with Marketplace | No | JavaScript |
| **Stream-Pi** | Action API | No | Java |

### 3.2 Configuration Formats

| Application | Format | Notes |
|---|---|---|
| **OpenDeck** | Internal (Elgato-compatible profiles) | Folder-based with auto-switching |
| **StreamController** | Internal (Flatpak sandbox) | GUI-driven, no user-editable files |
| **Deckmaster** | TOML (`.deck` files) | Version-controllable, git-friendly |
| **deckxstream** | JSON (`.deckxstream.json`) | Single file, dynamic content support |
| **streamdeck-linux-gui** | JSON | Import/export support |
| **SnakeDeck** | YAML | Hot-reloading on file change |

### 3.3 Communication Architectures

| Pattern | Used By |
|---|---|
| **Direct HID/USB** | All hardware-controlling apps (via underlying library) |
| **WebSocket (plugin protocol)** | OpenDeck, Macro Deck Connector, StreamDeckWS |
| **Socket.io** | ODeck (phone-to-desktop) |
| **Flask HTTP** | WebDeck (browser-based) |
| **D-Bus** | Deckmaster (Linux IPC) |
| **systemd service** | streamdeck-linux-gui, SnakeDeck |

---

## 4. API and Remote Control Capabilities

This is a notable gap in the ecosystem. None of the surveyed applications expose a general-purpose REST or WebSocket API designed for external programmatic control.

| Project | External API | Notes |
|---|---|---|
| **OpenDeck** | No | WebSocket is internal (plugin protocol), not for external consumers |
| **StreamController** | No | No documented API |
| **Boatswain** | No | No API |
| **Deckmaster** | No | D-Bus actions are outbound only (calling other services) |
| **StreamDeckWS** | Yes (WebSocket) | Elgato plugin that proxies Stream Deck events to/from a WebSocket server (e.g., Node-RED). Requires official Elgato software. |
| **Macro Deck** | Partial | WebSocket between Connector and Macro Deck app, but not a public API |
| **Bitfocus Companion** | Yes | HTTP API + WebSocket, but Companion is a separate research topic |

**Implication for build-vs-reuse:** If external programmatic control is a requirement, you will need to build this layer yourself on top of a library. No existing open-source Stream Deck application provides it.

---

## 5. XL Model Support Matrix

| Project | XL Supported | How |
|---|---|---|
| **OpenDeck** | Yes | Via `elgato-streamdeck` Rust crate |
| **StreamController** | Yes | Via `python-elgato-streamdeck` |
| **Boatswain** | Yes | Direct HID (C) |
| **Deckmaster** | Yes | Via `muesli/streamdeck` Go lib (USB PID `0x006c`) |
| **streamdeck-linux-gui** | Yes | Via `python-elgato-streamdeck` |
| **deckxstream** | Yes | Via `elgato-stream-deck` npm (USB PID `0x006c`) |
| **Freedeck** | No | Phone/tablet only, no hardware support |
| **ODeck** | No | Phone/tablet only |
| **WebDeck** | No | Browser-based, no hardware support |
| **Stream-Pi** | No | Raspberry Pi / phone client, no Elgato HID |
| **SnakeDeck** | Yes | Via `python-elgato-streamdeck` |

---

## 6. Build vs. Reuse Recommendations

### If you want to build a custom Stream Deck XL controller:

**Use a library, not a fork of an application.**

The applications above are designed as end-user tools with opinionated UIs and plugin systems. Forking one to build something different will create ongoing merge pain. Instead:

1. **Python path:** Use `python-elgato-streamdeck` (1.1k stars, MIT, actively maintained). Best documentation, broadest device support, simplest API. Good for prototyping and scripts.

2. **Node.js/TypeScript path:** Use `@elgato-stream-deck/node` (197 stars, MIT). Monorepo with clean architecture. The `@elgato-stream-deck/tcp` package hints at network control possibilities. The WebHID variant enables browser-based interfaces.

3. **Rust path:** Use `elgato-streamdeck` crate (MIT). Best performance. Used in production by OpenDeck. Natural fit if building a Tauri app.

4. **Go path:** Use `muesli/streamdeck` (MIT). Deckmaster proves it works. Go's single-binary deployment is convenient for server/service scenarios.

5. **.NET path:** Use `StreamDeckSharp` (395 stars, MIT). Mature, simple API. Good for Windows-centric tools.

### What to steal from existing projects:

- **OpenDeck's OpenAction API spec** -- if you want to build a plugin system, study this rather than inventing a new protocol
- **Deckmaster's TOML config approach** -- for a config-driven rather than GUI-driven tool
- **deckxstream's dynamic button concept** -- buttons whose content updates from process output
- **SnakeDeck's inline Python eval** -- for rapid prototyping of button behaviors
- **Boatswain's systemd udev contribution** -- ensures devices are accessible without manual setup

---

## Sources

1. [OpenDeck - GitHub](https://github.com/nekename/OpenDeck)
2. [OpenDeck - DeepWiki Architecture](https://deepwiki.com/nekename/OpenDeck)
3. [StreamController - GitHub](https://github.com/StreamController/StreamController)
4. [StreamController - Official Site](https://streamcontroller.core447.com/)
5. [Boatswain - GNOME GitLab](https://gitlab.gnome.org/World/boatswain)
6. [Boatswain - Developer Blog](https://feaneron.com/2022/03/17/boatswain-your-stream-deck-app-for-linux/)
7. [Boatswain - GNOME Apps](https://apps.gnome.org/Boatswain/)
8. [Deckmaster - GitHub](https://github.com/muesli/deckmaster)
9. [streamdeck-linux-gui - GitHub](https://github.com/streamdeck-linux-gui/streamdeck-linux-gui)
10. [deckxstream - GitHub](https://github.com/SecretAgentKen/deckxstream)
11. [Freedeck - GitHub](https://github.com/Freedeck/Freedeck)
12. [ODeck - GitHub](https://github.com/willianrod/ODeck)
13. [WebDeck - GitHub](https://github.com/Lenochxd/WebDeck)
14. [Stream-Pi Server - GitHub](https://github.com/Stream-Pi/Server)
15. [SnakeDeck - GitHub](https://github.com/jpetazzo/snakedeck)
16. [Macro Deck Stream Deck Connector - GitHub](https://github.com/Macro-Deck-App/Macro-Deck-Stream-Deck-Connector)
17. [python-elgato-streamdeck - GitHub](https://github.com/abcminiuser/python-elgato-streamdeck)
18. [node-elgato-stream-deck - GitHub](https://github.com/Julusian/node-elgato-stream-deck)
19. [elgato-streamdeck Rust crate - GitHub](https://github.com/OpenActionAPI/rust-elgato-streamdeck)
20. [StreamDeckSharp - GitHub](https://github.com/OpenMacroBoard/StreamDeckSharp)
21. [fnando/streamdeck framework - GitHub](https://github.com/fnando/streamdeck)
22. [OpenAction API - Introduction](https://openaction.amankhanna.me/)
23. [StreamDeckWS - GitHub](https://github.com/ybizeul/StreamDeckWS)
24. [OpenDeck on Polimetro](https://www.polimetro.com/en/What-is-OpenDeck/)
25. [AlternativeTo - Stream Deck Alternatives](https://alternativeto.net/software/streamdeck/)
