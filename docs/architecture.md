# Architecture

## Decisions

### D-1: Rust for the bridge (2026-05-24)

The bridge is built in Rust. Swift was considered for native macOS integration (menu bar via `NSStatusItem`, login item via `SMAppService`) but rejected because:

- The `elgato-streamdeck` Rust crate already exists with 15 models supported. Swift has no viable Stream Deck library — we'd write raw IOKit HID code.
- `hidapi`, `tokio-tungstenite`, and `image-rs` are mature Rust crates that cover the entire bridge spec. The Swift equivalents (IOKit, swift-nio-websocket, CoreGraphics) are more verbose and less commonly used for this type of server workload.
- Rust produces a single static binary with no runtime dependencies.
- The only meaningful Swift advantage is the menu bar, which can be handled by a thin Swift wrapper (~50 lines) that launches and monitors the Rust binary, if we ever want it.

Running on startup is language-agnostic (launchd plist in `~/Library/LaunchAgents`).

### D-2: WebSocket protocol (2026-05-24)

The bridge exposes a WebSocket server on localhost (default port 9001). Chosen over alternatives:

- **vs. stdin/stdout pipes**: WebSocket is standalone, testable with any WS client, supports multiple simultaneous clients, handles binary frames without base64 overhead, and decouples the bridge from any particular parent process.
- **vs. Unix socket**: WebSocket gives us structured framing (text/binary), broad client library support, and browser compatibility for testing. Unix sockets would need a custom framing protocol.
- **vs. TCP**: Same as Unix socket — we'd build our own framing on top.
- **vs. gRPC**: Heavier dependency, requires protobuf tooling, less natural for browser-based testing.

---

## Layers

The system is organized into six layers. Layers 1–3 live in this repo (the Rust bridge). Layers 4–6 are implemented by client applications.

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
│  event dispatch                                     │
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
- Slice panel-wide images into individual key images
- Solid color fill via JPEG-encoded solid image
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

**Does not know about:** Modes, rendering, automation. Passes messages between clients and the device abstraction layer.

See [`protocol.md`](protocol.md) for the full WebSocket protocol reference.

---

### Layer 4: Client SDK (client applications)

A library or module that wraps the WebSocket connection for a specific language/framework.

**Responsibilities:**
- WebSocket client with auto-reconnection and exponential backoff
- Typed API for setting images, colors, brightness
- Event dispatch for key press/release events
- Device state tracking (connected/disconnected, serial, model)

### Layer 5: Application Logic (client applications)

Where all the interesting behavior lives.

**Responsibilities:**
- Mode and layer system (push/pop stack, modifier keys, contextual overlays)
- Chord and gesture detection (long press, double tap, simultaneous press)
- Context-aware adaptation (active app detection, regional button updates)
- Rendering engine (text, icons, gauges, charts → 96x96 images)
- OS automation (volume, media, app launch, keyboard macros, window management)

### Layer 6: Integrations (client applications)

Connectors to external services.

**Responsibilities:**
- CI/CD status polling (GitHub Actions)
- Home automation (entity state, service calls)
- Messaging (notification counts)
- Custom webhook endpoints
