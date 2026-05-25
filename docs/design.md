# Design

This document covers use cases and requirements for the full system (bridge + client). The bridge repo implements the USB Bridge (R-1) and WebSocket Protocol (R-5). Everything else is client-side, documented here for context on what the bridge needs to support.

---

## Use Cases

### UC-1: Mode-Based Button Layouts

A developer presses a "DEV" button. The remaining 31 buttons change to show developer tools: git status, run tests, start/stop dev server, toggle terminal, etc. Pressing "GIT" within dev mode narrows further to git-specific actions: commit, push, pull, branch status, PR creation. Pressing the mode button again (or a "back" button) returns to the parent mode.

**Bridge implication:** Must handle rapid 32-key image updates efficiently. Batch updates (multiple keys in quick succession) should not bottleneck.

### UC-2: Media and Volume Control

A row of buttons controls media playback: play/pause, next, previous, volume up, volume down, mute. The play/pause button shows the current state (play icon when paused, pause icon when playing). A button displays the currently playing track name and album art, updating in real time.

**Bridge implication:** Key down/up events with timestamps enable the client to implement hold-to-repeat logic.

### UC-3: CI/CD Build Status Dashboard

Buttons represent projects, each showing green/yellow/red based on CI build status. Pressing a red button opens the failed build in a browser. Buttons update automatically when builds complete, pushed by a monitoring tool via the WebSocket API.

**Bridge implication:** Multiple WebSocket clients must be able to connect and push images to specific keys.

### UC-4: System Monitoring Dashboard

A cluster of buttons displays real-time system stats: CPU usage (as a gauge), memory usage (as a bar), network throughput (as a sparkline), battery level. These update every 1–2 seconds.

**Bridge implication:** Image cache with dedup — if the client sends the same image bytes for a key, the bridge skips the USB transfer.

### UC-5: Chord Combinations

Hold a modifier button and press another button to trigger a different action (e.g., modifier + button 12 = deploy to staging). Without the modifier, button 12 does something else (e.g., run tests). Button labels change while the modifier is held to show chord actions.

**Bridge implication:** Key events must include accurate timestamps so the client can implement chord detection windows.

### UC-6: Context-Aware Adaptation

When the user switches between applications (e.g., VS Code to Figma), specific button regions adapt — a cluster of buttons changes to show relevant actions while persistent buttons (media, system status) remain unchanged. Contextual overlays rather than wholesale layout replacement.

**Bridge implication:** Per-key updates are the right abstraction — the client decides which keys change, the bridge applies them individually.

### UC-7: External Application Integration

A monitoring tool connects to the bridge's WebSocket and pushes a red alert icon to a button with status text. When the user presses the button, the bridge sends a key_down event to all connected clients. The monitoring tool receives the event and acknowledges the alert.

**Bridge implication:** Core bridge functionality — this is exactly what the bridge does.

### UC-8: Smart Home Control Panel

Buttons for home automation: lights (toggle, brightness), thermostat (current temp + target), lock status, scene buttons (Movie Night, Work Mode, Bedtime). Each button shows current device state and updates in real time via Home Assistant or similar.

**Bridge implication:** None beyond standard per-key image updates.

### UC-9: Application Launching and Keyboard Macros

Buttons that launch or focus applications and trigger keyboard macro sequences. A "Launch" mode shows frequently used apps with their icons. A "Macros" mode shows keyboard shortcuts that are hard to remember or require complex key combinations.

**Bridge implication:** None — all automation is client-side. The bridge just delivers key events and displays images.

### UC-10: Workspace Launcher

One button press arranges the desktop for a specific workflow: launches the right apps, positions windows, sets volume, and switches the deck to the matching mode. For example: "Coding" launches editor + terminal + browser, tiles them across monitors, and switches to dev mode.

**Bridge implication:** None — client-side orchestration.

---

## Requirements

### R-1: Rust USB Bridge (this repo)

A compiled Rust binary that communicates with the Stream Deck XL over USB HID and exposes a WebSocket API.

**Device Communication:**
- Connect to Stream Deck XL (all hardware revisions: PIDs 0x006C, 0x008F, 0x00BA)
- Translate button state bitmaps into individual press/release events with timestamps
- Accept per-key image updates (raw RGBA, PNG, or pre-encoded JPEG)
- Accept panel-wide image updates (one large image, bridge slices into 32 keys)
- Solid color fill per key
- Brightness control (0–100)
- Device info queries (serial number, firmware version, model)
- Maintain internal image cache (32 slots) — only send to device when image actually changes
- Device hot-plug detection (connect/disconnect events)
- Restore cached images on device reconnect
- Reset to logo / clear individual keys / clear all keys

**WebSocket Server:**
- Listen on configurable localhost port (default: 9001)
- Accept multiple concurrent client connections
- JSON text frames for commands and events
- Binary frames for image data
- Broadcast key events to all connected clients
- Send device state (connected/disconnected, info) on client connect
- Graceful handling of client connect/disconnect

### R-2: Client SDK

A library that wraps the bridge's WebSocket connection for a specific language or framework.

**Capabilities:**
- WebSocket client with auto-reconnection
- Typed API for button images, colors, brightness, clearing
- Event dispatch for button press/release events
- Device state tracking (connected/disconnected, serial number, model)

### R-3: Mode and Layer System

A state machine that manages the current button layout and handles mode transitions.

**Capabilities:**
- Mode stack (push/pop) with named modes
- Each mode defines: button images, press actions, long-press actions, chord actions
- Modifier/shift keys (momentary layers while held)
- Context-aware overlays: persistent regions + context-sensitive regions
- Persistent mode (survives restart — last active mode restored)
- Mode definition format (data-driven, not hardcoded)

### R-4: Rendering Engine

Generates 96x96 pixel images for button display from higher-level descriptions.

**Capabilities:**
- Text rendering (configurable font, size, color, alignment)
- Icon rendering (from icon libraries or custom SVGs)
- Gauge/bar/sparkline chart rendering
- Composite layouts (icon + text, icon + badge count, etc.)
- Background color and border support
- Template system (define a button style once, apply with different data)
- Dirty tracking (only re-render when underlying data changes)

### R-5: WebSocket Protocol (this repo)

The communication protocol between the bridge and its clients. See [`protocol.md`](protocol.md) for the full reference.

### R-6: OS Automation Modules

Platform-specific modules for controlling the operating system.

**Capabilities:**
- Volume control (get/set system volume, mute/unmute)
- Media playback (play/pause, next, previous, now-playing metadata + album art)
- Microphone mute/unmute
- Application launch by name or bundle ID
- Application switch/focus (bring to front)
- Keyboard macro sequences (keystroke simulation with configurable delays)
- Window positioning (workspace presets)
- Active application detection (frontmost app polling)
- System metrics (CPU, memory, network, battery)

### R-7: Integration Modules

Connectors for external services and platforms.

**Candidates:**
- GitHub Actions (CI/CD status polling)
- Home Assistant (WebSocket API, entity state, service calls)
- Slack (notification count, status)
- Custom webhook endpoints

---

## User Stories

### Bridge (this repo)
- As a developer, I want to run the bridge binary and have it auto-detect my Stream Deck XL so I can start using it immediately.
- As a developer, I want the bridge to reconnect automatically and restore button images when I unplug and replug the device.
- As a developer, I want to connect with any WebSocket client (websocat, browser console) to test the bridge without writing application code.
- As a developer, I want key events to include timestamps so my client can implement timing-based gestures (long press, double tap, chords).
- As a developer, I want the bridge to skip USB transfers when I send the same image for a key that hasn't changed.
- As a developer, I want multiple WebSocket clients to connect simultaneously so different tools can share the device.

### Client Applications
- As a user, I want to see what each button does via its LCD label so I never have to memorize button positions.
- As a user, I want to press a mode button and have the relevant buttons change to a new context.
- As a user, I want persistent buttons (media controls, status indicators) to stay visible across mode changes.
- As a user, I want to hold a modifier button and see context-sensitive buttons show their chord actions.
- As a user, I want the deck to adapt contextually when I switch between applications, updating only the relevant button regions.
- As a user, I want manual mode switches to override automatic context switching.
