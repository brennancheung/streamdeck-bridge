# Requirement Gathering

This document covers use cases and requirements for the full system (bridge + client). The bridge repo implements R-1 (Rust USB Bridge) and R-5 (WebSocket API). Everything else is client-side, documented here for context on what the bridge needs to support.

---

## Use Cases

### UC-1: Mode-Based Button Layouts

A developer presses a "DEV" button. The remaining 31 buttons change to show developer tools: git status, run tests, start/stop dev server, toggle terminal, etc. Pressing "GIT" within dev mode narrows further to git-specific actions: commit, push, pull, branch status, PR creation. Pressing the mode button again (or a "back" button) returns to the parent mode.

**Requirements:**
- Mode stack with push/pop semantics
- All button images update atomically when switching modes
- Each mode defines its own button layout (image + action per key)
- Visual indicator of current mode (e.g., colored border or header row)

**Bridge implication:** Must handle rapid 32-key image updates efficiently. Batch updates (multiple keys in quick succession) should not bottleneck.

### UC-2: Media and Volume Control

A row of buttons controls media playback: play/pause, next, previous, volume up, volume down, mute. The play/pause button shows the current state (play icon when paused, pause icon when playing). A button displays the currently playing track name and album art, updating in real time.

**Requirements:**
- Media state polling (currently playing track, playback state, volume level)
- Album art retrieval and rendering to 96x96
- Hold-to-repeat for volume buttons (press and hold = continuous volume change)
- Now-playing metadata rendered as text + image on a button

**Bridge implication:** Key down/up events with timestamps enable the client to implement hold-to-repeat logic.

### UC-3: CI/CD Build Status Dashboard

Four buttons represent four projects. Each shows green/yellow/red based on the latest CI build status. Pressing a red button opens the failed build in a browser. The buttons update automatically when builds complete, pushed by the CI system via the WebSocket API.

**Requirements:**
- WebSocket API for external systems to set button images
- Color-coded status rendering (green = passing, yellow = in progress, red = failed)
- Press action: open URL in default browser

**Bridge implication:** Multiple WebSocket clients must be able to connect and push images to specific keys.

### UC-4: System Monitoring Dashboard

A cluster of buttons displays real-time system stats: CPU usage (as a gauge), memory usage (as a bar), network throughput (as a sparkline), and battery level. These update every 1-2 seconds.

**Requirements:**
- System metric polling at configurable intervals
- Gauge/bar/sparkline rendering to 96x96 images
- Efficient dirty-tracking (only re-render and push buttons whose data changed)
- Configurable thresholds for color changes (e.g., CPU > 80% turns red)

**Bridge implication:** Image cache with dedup — if the client sends the same image bytes for a key, the bridge skips the USB transfer.

### UC-5: Chord Combinations

A developer holds the bottom-left button (modifier) and presses button 12 to trigger "deploy to staging." The modifier button glows while held. Without the modifier, button 12 does something else entirely (e.g., run tests). The button labels change while the modifier is held to show the chord actions.

**Requirements:**
- Modifier/shift key detection (hold one button, press another)
- Simultaneous press detection within a configurable time window (~50ms)
- Dynamic label update while modifier is held (show chord actions)
- Labels revert when modifier is released

**Bridge implication:** Key events must include accurate timestamps so the client can implement chord detection windows.

### UC-6: Context-Aware Adaptation

The developer switches from VS Code to Figma. Rather than swapping the entire layout, specific button regions adapt — a cluster of buttons changes to show Figma-relevant actions while persistent buttons (media, system status) remain unchanged. The system tracks the active application and applies contextual overlays rather than wholesale layout replacement.

**Requirements:**
- Active application detection on macOS (frontmost app polling)
- Contextual overlay model: some buttons are persistent across contexts, others are context-sensitive
- Region-based updates rather than full layout swap
- Priority system: manual actions override auto-context, auto-context overrides defaults
- Smooth transitions — only the context-sensitive buttons change, not the entire deck

**Bridge implication:** Per-key updates are the right abstraction — the client decides which keys change, the bridge just applies them individually.

### UC-7: External Application Integration via API

A monitoring tool connects to the bridge's WebSocket and pushes a red alert icon to button 15 with the text "DB Latency 2.3s". When the user presses the button, the bridge sends a key_down event to all connected clients. The monitoring tool receives the event and acknowledges the alert.

**Requirements:**
- WebSocket API for setting button images (by key index)
- WebSocket event streaming for button presses to all connected clients
- Multiple concurrent clients with independent subscriptions

**Bridge implication:** Core bridge functionality — this is what the bridge does.

### UC-8: Smart Home Control Panel

A mode dedicated to home control shows buttons for: living room lights (toggle, brightness indicator), thermostat (current temp + target), front door lock status, and scene buttons (Movie Night, Work Mode, Bedtime). Each button shows the current device state and updates when the state changes via Home Assistant WebSocket subscription.

**Requirements:**
- Home Assistant WebSocket API integration (client-side)
- Real-time state subscription (light state, thermostat, lock status)
- Scene activation via HA service calls
- Status rendering: on/off icons, temperature numbers, brightness levels

**Bridge implication:** None beyond standard per-key image updates.

### UC-9: Application Launching and Keyboard Macros

Buttons that launch or switch to specific applications (similar to Raycast), and buttons that trigger keyboard macro sequences (similar to Karabiner Elements). A "Launch" mode shows frequently used apps with their icons. A "Macros" mode shows keyboard shortcuts that are hard to remember or require multiple key combinations.

**Requirements:**
- Application launch by bundle ID or name
- Application switch/focus (bring to front if already running)
- Keyboard macro sequences (e.g., Cmd+Shift+P → type "format document" → Enter)
- Configurable delays between keystrokes in a macro
- Visual feedback: button shows the app icon or a macro description

**Bridge implication:** None — all automation is client-side. The bridge just delivers key events and displays images.

### UC-10: Workspace Launcher

One button press arranges the desktop for a specific workflow: launches the right apps, positions windows on specific monitors, sets volume, and switches the deck to the matching mode. For example: "Coding" launches VS Code + terminal + browser, tiles them across monitors, mutes Slack notifications, and switches to dev mode.

**Requirements:**
- Window positioning via JXA/AppleScript
- App launch and focus control
- Compound action sequences (multiple actions from one button press)
- Named workspace presets stored as configuration

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
- Solid color fill per key (using HID command 03 06, bypassing JPEG)
- Brightness control (0-100)
- Device info queries (serial number, firmware version, model)
- Maintain internal image cache (32 slots) — only send to device when image actually changes
- Device hot-plug detection (connect/disconnect events)
- Restore cached images on device reconnect
- Reset to logo / clear individual keys / clear all keys

**WebSocket Server:**
- Listen on configurable localhost port (default: 9001)
- Accept multiple concurrent client connections
- JSON text frames for commands and events
- Binary frames for image data (with key index header)
- Broadcast key events to all connected clients
- Send device state (connected/disconnected, info) on client connect
- Graceful handling of client connect/disconnect

**CLI Mode:**
- One-shot commands: set image, set color, set brightness, query device info
- Useful for scripting and testing without a persistent client

### R-2: Phoenix Device Manager (client repo)

An Elixir GenServer that connects to the bridge's WebSocket and provides the application-level API.

**Capabilities:**
- WebSocket client with auto-reconnection
- Elixir API: `set_key_image(key, image_data)`, `set_key_color(key, r, g, b)`, `set_brightness(value)`, `clear_key(key)`, `clear_all()`
- Event broadcasting: button press/release events dispatched via Phoenix.PubSub
- Device state tracking (connected/disconnected, serial number, model)

### R-3: Mode and Layer System (client repo)

A state machine that manages the current button layout and handles mode transitions.

**Capabilities:**
- Mode stack (push/pop) with named modes
- Each mode defines: button images, press actions, long-press actions, chord actions
- Modifier/shift keys (momentary layers while held)
- Context-aware overlays: persistent regions + context-sensitive regions
- Persistent mode (survives restart — last active mode restored)
- Mode definition format (data-driven, not hardcoded)

### R-4: Rendering Engine (client repo)

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

The communication protocol between the bridge and its clients.

**Client → Bridge (commands):**
```json
{"cmd": "set_image", "key": 5, "format": "rgba", "width": 96, "height": 96}
// followed by a binary frame with the pixel data

{"cmd": "set_image_jpeg", "key": 5}
// followed by a binary frame with pre-encoded JPEG

{"cmd": "set_panel_image", "format": "rgba", "width": 768, "height": 384}
// followed by a binary frame — bridge slices into 32 keys

{"cmd": "set_color", "key": 5, "r": 255, "g": 0, "b": 0}

{"cmd": "set_brightness", "value": 80}

{"cmd": "clear_key", "key": 5}

{"cmd": "clear_all"}

{"cmd": "reset_to_logo"}

{"cmd": "get_device_info"}
```

**Bridge → Client (events):**
```json
{"event": "key_down", "key": 5, "timestamp": 1716480000123}

{"event": "key_up", "key": 5, "timestamp": 1716480000456}

{"event": "device_connected", "serial": "AB12CD34EF56", "model": "xl", "firmware": "1.02.005", "keys": 32, "rows": 4, "cols": 8, "icon_size": 96}

{"event": "device_disconnected"}

{"event": "device_info", "serial": "AB12CD34EF56", "model": "xl", "firmware": "1.02.005", "keys": 32, "rows": 4, "cols": 8, "icon_size": 96, "brightness": 80}

{"event": "error", "message": "invalid key index: 33"}

{"event": "image_set", "key": 5}
```

### R-6: macOS Automation Modules (client repo)

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

### R-7: Integration Modules (client repo)

Connectors for external services and platforms.

**Candidates (not all required for v1):**
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
- As a developer, I want to use a CLI mode to set a button image from a script or terminal command.
- As a developer, I want key events to include timestamps so my client can implement timing-based gestures (long press, double tap, chords).
- As a developer, I want the bridge to skip USB transfers when I send the same image for a key that hasn't changed.
- As a developer, I want multiple WebSocket clients to connect simultaneously so different tools can share the device.

### Client (Phoenix app, separate repo)
- As a user, I want to see what each button does right now via its LCD label so I never have to memorize button positions.
- As a user, I want to press a mode button and have the relevant buttons change to a new context.
- As a user, I want persistent buttons (media controls, status indicators) to stay visible across mode changes.
- As a user, I want to hold a modifier button and see context-sensitive buttons show their chord actions.
- As a user, I want a button that launches VS Code or switches to it if it's already running.
- As a user, I want a button that triggers a keyboard macro sequence.
- As a user, I want the deck to adapt contextually when I switch between applications, updating only the relevant button regions.
- As a user, I want manual mode switches to override automatic context switching.

---

## Development Phases

### Phase 1 — Bridge (this repo)
- Rust binary with USB HID communication to Stream Deck XL
- WebSocket server on localhost with JSON commands and binary image frames
- Per-key image updates (RGBA, PNG, JPEG) with internal caching
- Individual key press/release events with timestamps
- Solid color fill, brightness control, clear, reset
- Device connect/disconnect detection and image restoration
- CLI mode for one-shot commands

### Phase 2 — Client Foundation (Phoenix repo)
- WebSocket client GenServer with auto-reconnection
- Mode and layer system with push/pop stack
- Basic rendering engine (text, icons, solid colors)
- Chord and long-press detection from timestamped key events

### Phase 3 — Automation (Phoenix repo)
- macOS automation modules (volume, media, app launch, keyboard macros)
- Context-aware adaptation (active app detection, regional overlay updates)
- Workspace presets

### Phase 4 — Ecosystem (Phoenix repo)
- External service integrations (CI/CD, Home Assistant, Slack)
- Multi-client coordination and button claims
- Advanced rendering (gauges, sparklines, charts)
