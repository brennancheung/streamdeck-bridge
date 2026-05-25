# Architecture

## Decisions

### D-1: Rust for the bridge (2026-05-24)

The bridge is built in Rust. Swift was considered for native macOS integration (menu bar via `NSStatusItem`, login item via `SMAppService`) but rejected because:

- The `elgato-streamdeck` Rust crate already exists with 11 models supported. Swift has no viable Stream Deck library — we'd write raw IOKit HID code.
- `hidapi`, `tokio-tungstenite`, and `image-rs` are mature Rust crates that cover the entire bridge spec. The Swift equivalents (IOKit, swift-nio-websocket, CoreGraphics) are more verbose and less commonly used for this type of server workload.
- Rust produces a single static binary with no runtime dependencies.
- The only meaningful Swift advantage is the menu bar, which can be handled by a thin Swift wrapper (~50 lines) that launches and monitors the Rust binary, if we ever want it.

Running on startup is language-agnostic (launchd plist in `~/Library/LaunchAgents`).

### D-2: WebSocket protocol (2026-05-24)

The bridge exposes a WebSocket server on localhost (default port 9001). Chosen over stdin/stdout BEAM Port because:

- The bridge is a standalone binary, testable independently with any WebSocket client
- Multiple clients can connect simultaneously
- Binary frames handle image data without base64 overhead
- Decoupled from the BEAM — the Phoenix app connects as a client, not a parent process

### D-3: Elixir Phoenix for the client application (2026-05-24)

Layers 4-6 (client SDK, application logic, integrations) will live in a separate Phoenix app repo. The bridge repo has no Elixir code. Phoenix was chosen for its real-time capabilities (PubSub, LiveView), supervision trees, and the team's preference.

---

## Layers

The system is organized into six layers. Layers 1-3 live in this repo (the Rust bridge). Layers 4-6 live in the Phoenix app (separate repo, future work).

```
┌─────────────────────────────────────────────────────┐
│  Layer 6: Integrations                              │
│  CI/CD, Home Assistant, Slack, external webhooks    │
├─────────────────────────────────────────────────────┤
│  Layer 5: Application Logic                         │
│  Modes/layers, chord detection, context awareness,  │
│  rendering engine, automation modules               │
├─────────────────────────────────────────────────────┤
│  Layer 4: Client SDK                                │
│  Typed API over WebSocket, auto-reconnect,          │
│  PubSub event dispatch                              │
╠═════════════════════════════════════════════════════╣  ← repo boundary
│  Layer 3: WebSocket Protocol                        │
│  JSON commands/events, binary image frames,         │
│  multi-client, connection lifecycle                 │
├─────────────────────────────────────────────────────┤
│  Layer 2: Device Abstraction                        │
│  Image cache (32 slots), state diff → key events,   │
│  JPEG encoding, axis flip, brightness, color fill   │
├─────────────────────────────────────────────────────┤
│  Layer 1: USB HID Driver                            │
│  Raw HID reports, device detection,                 │
│  connect/disconnect                                 │
└─────────────────────────────────────────────────────┘
          │
          │ USB
    ┌─────┴──────┐
    │ Stream Deck│
    └────────────┘
```

### Layer 1: USB HID Driver

Raw communication with the Stream Deck hardware over USB HID.

**Responsibilities:**
- Open/close HID device by VID/PID
- Send HID output reports (images, commands)
- Send/receive HID feature reports (brightness, serial, firmware)
- Read HID input reports (button state bitmaps)
- Detect device connect/disconnect (hot-plug)
- Handle multiple XL hardware revisions (PIDs 0x006C, 0x008F, 0x00BA)

**Does not know about:** Images, keys, events, clients. Deals only in byte arrays and HID report IDs.

### Layer 2: Device Abstraction

Translates between the raw HID protocol and a meaningful device API.

**Responsibilities:**
- Maintain image cache (32 slots, one per key)
- Diff button state bitmaps into individual press/release events with timestamps
- Encode images to JPEG, flip axes, chunk into HID reports
- Slice panel-wide images into 32 individual key images
- Solid color fill via HID command (bypassing JPEG)
- Brightness control (0-100 → HID feature report)
- Device info queries (serial number, firmware version)
- Restore cached images after device reconnect
- Skip USB transfer when image hasn't changed (dedup)

**Does not know about:** WebSocket, clients, JSON. Exposes a Rust API that Layer 3 calls.

### Layer 3: WebSocket Protocol

Network interface to the bridge. Runs a WebSocket server on localhost.

**Responsibilities:**
- Accept multiple concurrent WebSocket client connections
- Parse JSON text frames as commands, dispatch to Layer 2
- Accept binary frames as image data, associate with preceding command
- Broadcast key events (from Layer 2) to all connected clients
- Send device state on client connect
- Handle client connect/disconnect gracefully
- Configurable listen port (default: 9001)
- CLI mode for one-shot commands (connect, send, disconnect)

**Does not know about:** Modes, rendering, automation. Passes messages between clients and the device abstraction layer.

---

### Layer 4: Client SDK (future — Phoenix repo)

Elixir GenServer that wraps the WebSocket connection.

**Responsibilities:**
- WebSocket client with auto-reconnection and exponential backoff
- Typed Elixir API: `set_key_image/2`, `set_key_color/4`, `set_brightness/1`, `clear_key/1`, `clear_all/0`
- Phoenix.PubSub broadcast of key press/release events
- Device state tracking (connected/disconnected, serial, model)

### Layer 5: Application Logic (future — Phoenix repo)

Where all the interesting behavior lives.

**Responsibilities:**
- Mode and layer system (push/pop stack, modifier keys, contextual overlays)
- Chord and gesture detection (long press, double tap, simultaneous press)
- Context-aware adaptation (active app detection, regional button updates)
- Rendering engine (text, icons, gauges, charts → 96x96 images)
- macOS automation (volume, media, app launch, keyboard macros, window management)

### Layer 6: Integrations (future — Phoenix repo)

Connectors to external services.

**Responsibilities:**
- CI/CD status polling (GitHub Actions)
- Home Assistant (entity state, service calls)
- Slack (notification counts)
- Custom webhook endpoints
