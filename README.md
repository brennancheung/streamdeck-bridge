# Stream Deck XL — USB-to-WebSocket Bridge

Rust binary that provides direct USB HID control of the **Elgato Stream Deck XL** (32-button, 8x4 grid, 96x96px per-button LCD), exposed via a WebSocket API on localhost.

## Build & Run

```bash
# Install system dependency
brew install hidapi

# Build
cargo build --release

# Run (default port 9001)
./target/release/streamdeck-bridge

# Custom port
./target/release/streamdeck-bridge --port 8080
```

The bridge auto-detects the Stream Deck on USB, starts the WebSocket server, and reconnects automatically if the device is unplugged/replugged.

## WebSocket API

Connect to `ws://localhost:9001`. Commands are JSON text frames; image data is sent as binary frames.

### Commands (client → bridge)

```json
{"cmd": "set_color", "key": 5, "r": 255, "g": 0, "b": 0}
{"cmd": "set_brightness", "value": 80}
{"cmd": "clear_key", "key": 5}
{"cmd": "clear_all"}
{"cmd": "reset_to_logo"}
{"cmd": "get_device_info"}

// Image commands — send JSON, then a binary frame with the image data:
{"cmd": "set_image", "key": 5, "format": "rgba", "width": 96, "height": 96}
{"cmd": "set_image_jpeg", "key": 5}
```

### Events (bridge → client)

```json
{"event": "key_down", "key": 5, "timestamp": 1716480000123}
{"event": "key_up", "key": 5, "timestamp": 1716480000456}
{"event": "device_connected", "serial": "AB12CD34", "model": "xl", "firmware": "1.02.005", "keys": 32, "rows": 4, "cols": 8, "icon_size": 96}
{"event": "device_disconnected"}
{"event": "device_info", "serial": "AB12CD34", "model": "xl", ...}
{"event": "image_set", "key": 5}
{"event": "error", "message": "invalid key index: 33"}
```

## Features

- Per-key image updates (RGBA, PNG, JPEG) with 32-slot image cache and dedup
- Solid color fill per key
- Individual key press/release events with millisecond timestamps
- Brightness control (0–100)
- Device hot-plug detection and automatic image restoration on reconnect
- Multiple concurrent WebSocket clients
- All Stream Deck XL hardware revisions supported (PIDs 0x006C, 0x008F, 0x00BA)

## Architecture

The bridge is a thin hardware abstraction layer with no business logic. See [`docs/architecture.md`](docs/architecture.md) for layer definitions and [`docs/vision.md`](docs/vision.md) for design principles.

```
Client (Phoenix App)
        │ WebSocket (localhost)
┌───────┴────────────────────┐
│     Rust Bridge            │
│  WebSocket ←→ Image Cache  │
│         Device Driver      │
└───────┬────────────────────┘
        │ USB HID
   Stream Deck XL
```

## Research

Deep research covering hardware protocol, libraries, automation, and performance in [`docs/deep-research/`](docs/deep-research/stream-deck-xl-direct-control/overview.md).
