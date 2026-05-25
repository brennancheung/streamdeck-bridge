# Vision

## What

A Rust bridge that provides direct USB HID control of the Elgato Stream Deck XL, exposed via a WebSocket API. The bridge is a standalone binary — a thin, reliable layer between the hardware and any client application. It handles device communication, image caching, state diffing, and the WebSocket server. All higher-level logic (modes, rendering, automation) lives in client code.

## Why

No existing Stream Deck software exposes a programmable control plane. Elgato's app is closed and plugin-limited. Bitfocus Companion is broadcast-focused with 700+ AV modules but no OS-level automation. Open-source alternatives are end-user apps, not platforms.

The gap: a clean, protocol-driven interface to Stream Deck hardware that any application can talk to. The bridge turns a USB HID device into a network-accessible service.

## Core Principles

**The bridge is the hardware abstraction layer.** It translates between USB HID and WebSocket. It has no opinion about what's displayed, what button presses mean, or what automations to run. Those decisions belong to the client.

**One bitmap per key is the API contract.** A client sends "set key 5 to this image." The bridge handles JPEG encoding, axis flipping, HID chunking, and transfer. The client never thinks about the USB protocol.

**The bridge maintains the image cache.** It keeps all 32 current key images in memory. When a client sends a new image for key 5, the bridge updates slot 5 and pushes only that key to the device. The client doesn't manage the full panel state.

**Individual key events, not state bitmaps.** The hardware sends a 32-byte state bitmap on every change. The bridge diffs it and emits discrete press/release events per key. The client receives `key_down(5)` and `key_up(5)`, not raw bitmaps.

**WebSocket is the protocol.** The bridge runs a WebSocket server on localhost. Any number of clients can connect — an application server, a CLI tool, a test harness, a monitoring script. Binary frames for image data, JSON text frames for commands and events.

**Standalone binary.** No runtime dependencies. `cargo build --release` produces one file. Start it, connect to it, done.

## Architecture

```
┌──────────────────────────────────────────┐
│          Client Application(s)           │
│                                          │
│  Modes, rendering, automation,           │
│  context awareness, integrations         │
└──────────────────┬───────────────────────┘
                   │ WebSocket (localhost)
┌──────────────────┴───────────────────────┐
│            Rust Bridge (this repo)        │
│                                          │
│  ┌────────────┐  ┌───────────────────┐   │
│  │ WebSocket  │  │  Image Cache      │   │
│  │ Server     │  │  (32 slots)       │   │
│  └─────┬──────┘  └────────┬──────────┘   │
│        │                  │              │
│  ┌─────┴──────────────────┴──────────┐   │
│  │         Device Driver             │   │
│  │   (HID reports, state diffing,    │   │
│  │    JPEG encoding, axis flip)      │   │
│  └──────────────────┬────────────────┘   │
└─────────────────────┬────────────────────┘
                      │ USB HID
              ┌───────┴────────┐
              │ Stream Deck XL │
              └────────────────┘
```

## What Success Looks Like

- Run the bridge binary, it finds the Stream Deck and starts the WebSocket server
- Connect with any WebSocket client and immediately receive device info (model, serial, firmware)
- Send a JSON message to set key 5 to red — it appears on the device in under 50ms
- Send a binary frame with a PNG/JPEG — the bridge encodes and pushes it
- Unplug the Stream Deck — connected clients receive a disconnect event
- Plug it back in — clients receive a reconnect event, bridge restores cached images
- Multiple clients can connect simultaneously
- A CLI tool can send one-shot commands (set an image, query device state)

## Scope

### In Scope (this repo)
- Rust binary with USB HID communication
- WebSocket server on localhost
- Per-key image management with caching
- State diff → individual key events
- Solid color fill, brightness, clear, reset
- Device hot-plug detection and reconnection
- Image restoration on reconnect
- Panel-wide image update (bridge slices into keys)
- CLI mode for one-shot commands

### Out of Scope (client applications)
- Modes, layers, and interaction patterns
- Rendering engine (text, icons, gauges, charts)
- OS automation (volume, media, keyboard, windows)
- Context awareness and application detection
- External service integrations (CI, Home Assistant, etc.)
- Chord detection and gesture recognition
- Configuration UI
