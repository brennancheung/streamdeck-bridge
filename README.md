# Stream Deck Bridge

The Elgato Stream Deck XL has 32 LCD buttons — each one a tiny 96x96 pixel screen that can display anything. But the official software locks you into a plugin marketplace, and every open-source alternative is a complete application with its own UI, its own config format, and its own opinion about what buttons should do.

Stream Deck Bridge takes a different approach. It's a small Rust binary that handles the USB protocol and gets out of the way. It exposes the hardware over a WebSocket API on localhost — you connect with any language, send JSON to set button images, and receive events when buttons are pressed. The bridge manages JPEG encoding, HID report chunking, device detection, and an image cache. Your code just says "make button 5 red" or "show this PNG on button 12."

There are no modes, no plugins, no config files. The bridge is a translation layer: USB HID on one side, WebSocket on the other. What you build on top — a developer dashboard, a home automation panel, a CI status board, a media controller — is entirely your call.

## Quick Start

```bash
cargo build --release
./target/release/streamdeck-bridge
```

The bridge auto-detects the Stream Deck on USB and starts listening on `ws://localhost:9001`. That's it.

## What You Can Build

Connect from any language with a WebSocket library. Here are a few examples in Python to give you a feel for the API.

### Paint buttons

```python
import asyncio, json, websockets

async def main():
    async with websockets.connect("ws://localhost:9001") as ws:
        await ws.recv()  # device_connected event

        # Set button 0 to red
        await ws.send(json.dumps({"cmd": "set_color", "key": 0, "r": 255, "g": 0, "b": 0}))

        # Set button 1 to blue
        await ws.send(json.dumps({"cmd": "set_color", "key": 1, "r": 0, "g": 0, "b": 255}))

asyncio.run(main())
```

### React to button presses

```python
async def listen():
    async with websockets.connect("ws://localhost:9001") as ws:
        await ws.recv()  # device_connected
        while True:
            event = json.loads(await ws.recv())
            if event["event"] == "key_down":
                print(f"Button {event['key']} pressed")

asyncio.run(listen())
```

### Display an image

```python
async def show_image():
    async with websockets.connect("ws://localhost:9001") as ws:
        await ws.recv()

        # Send image command, then the image data as a binary frame
        await ws.send(json.dumps({"cmd": "set_image_jpeg", "key": 0}))
        with open("icon.jpg", "rb") as f:
            await ws.send(f.read())

asyncio.run(show_image())
```

### Build a CI status board

Multiple clients can connect at the same time. A monitoring script can update a few status buttons while your main application controls the rest.

```python
async def ci_monitor():
    async with websockets.connect("ws://localhost:9001") as ws:
        await ws.recv()
        while True:
            status = check_build_status()  # your CI polling logic
            color = {"pass": (0,255,0), "fail": (255,0,0), "running": (255,255,0)}[status]
            r, g, b = color
            await ws.send(json.dumps({"cmd": "set_color", "key": 24, "r": r, "g": g, "b": b}))
            await asyncio.sleep(30)
```

These are toy examples — the point is that anything that can open a WebSocket can control the hardware. Elixir, Go, Ruby, Rust, a shell script piping through `websocat`, JavaScript in a browser (for testing) — the bridge doesn't care.

## Protocol Overview

The full protocol is documented in [`docs/protocol.md`](docs/protocol.md). Here's the short version.

**Commands** are JSON text frames sent from client to bridge:

| Command | Description |
|---------|-------------|
| `set_color` | Fill a button with a solid RGB color |
| `set_image` | Set a button's image (RGBA or PNG, followed by binary frame) |
| `set_image_jpeg` | Set a button's image from JPEG (followed by binary frame) |
| `set_brightness` | Set display brightness (0–100) |
| `clear_key` | Clear a single button |
| `clear_all` | Clear all buttons |
| `reset_to_logo` | Reset to the Elgato logo |
| `get_device_info` | Query device serial, model, firmware |

**Events** are JSON text frames sent from bridge to all connected clients:

| Event | Description |
|-------|-------------|
| `key_down` | Button pressed (with millisecond timestamp) |
| `key_up` | Button released (with millisecond timestamp) |
| `device_connected` | Device plugged in (serial, model, firmware, button count) |
| `device_disconnected` | Device unplugged |
| `device_info` | Response to `get_device_info` |
| `image_set` | Confirms a button image was updated |
| `error` | Something went wrong |

**Button layout** — buttons are numbered left-to-right, top-to-bottom:

```
 0   1   2   3   4   5   6   7
 8   9  10  11  12  13  14  15
16  17  18  19  20  21  22  23
24  25  26  27  28  29  30  31
```

## Features

- **Image cache with dedup** — the bridge keeps all 32 button images in memory. Send the same image twice and it skips the USB transfer. Unplug the device and plug it back in — the bridge restores every button automatically.

- **Individual key events** — the hardware sends a 32-byte state bitmap on every change. The bridge diffs it and emits discrete `key_down`/`key_up` events per button, each with a millisecond timestamp. This gives clients the raw data to implement long-press, double-tap, chord detection, or any other gesture model.

- **Multi-client** — multiple WebSocket connections at once. A main application, a monitoring script, and a test client can all share the device. Events are broadcast to everyone.

- **Single binary** — `cargo build --release` produces one file with no runtime dependencies. Start it, connect to it, done.

- **Hot-plug** — start the bridge before or after plugging in the device. It polls for connection, auto-connects, and reconnects if the device is unplugged.

- **Full 32-key rollover** — all buttons can be pressed simultaneously. The bridge correctly tracks the state of every button independently.

## Architecture

The bridge is intentionally minimal — three layers in ~500 lines of Rust:

```
Client Application(s)
        │ WebSocket (localhost:9001)
┌───────┴────────────────────────────┐
│  Layer 3: WebSocket Protocol       │
│  JSON commands/events,             │
│  binary image frames, multi-client │
├────────────────────────────────────┤
│  Layer 2: Device Abstraction       │
│  Image cache, state diffing,       │
│  JPEG encoding, brightness         │
├────────────────────────────────────┤
│  Layer 1: USB HID Driver           │
│  Raw HID reports, device detection │
└───────┬────────────────────────────┘
        │ USB
   Stream Deck XL
```

The bridge has no opinions about what you display or what button presses mean. It translates between USB HID and WebSocket. Everything above that — modes, layouts, rendering, automation — belongs in client code.

See [`docs/architecture.md`](docs/architecture.md) for detailed layer definitions and design decisions.

## Tested Hardware

| Model | PIDs | Status |
|-------|------|--------|
| Stream Deck XL | `0x006C`, `0x008F`, `0x00BA` | Tested, confirmed working |

The underlying [`elgato-streamdeck`](https://crates.io/crates/elgato-streamdeck) crate supports 15 device models including the Original, Mini, MK2, Plus, Neo, and Pedal. These should work with the bridge but are untested — button counts, image sizes, and LCD capabilities vary by model.

## Building from Source

**Requirements:** A Rust toolchain ([rustup.rs](https://rustup.rs)). No other system dependencies — the USB HID library links against OS-provided frameworks (IOKit on macOS, hidraw on Linux).

```bash
git clone https://github.com/brennancheung/streamdeck.git
cd streamdeck
cargo build --release
```

**Run:**

```bash
# Default port (9001)
./target/release/streamdeck-bridge

# Custom port
./target/release/streamdeck-bridge --port 8080
```

**Linux note:** You may need udev rules to access the device without root. Create `/etc/udev/rules.d/50-streamdeck.rules`:

```
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", TAG+="uaccess"
```

Then reload: `sudo udevadm control --reload-rules && sudo udevadm trigger`.

## Research

This project started with a deep investigation into Stream Deck hardware, protocols, and the open-source ecosystem. The research covers USB HID protocol details, image encoding and transfer performance, existing libraries across six languages, macOS automation APIs, and novel use cases. Available in [`docs/deep-research/`](docs/deep-research/stream-deck-xl-direct-control/overview.md).

## License

[MIT](LICENSE)
