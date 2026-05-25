# Stream Deck Bridge

The Elgato Stream Deck XL is 32 LCD buttons in an 8x4 grid — each one a tiny 96x96 pixel screen you can set to any image, backed by a physical key that registers press and release. It's a remarkably capable piece of hardware trapped behind limited software. Elgato's official app locks you into a plugin marketplace. Every open-source alternative ships as a complete application with its own UI, its own config format, and its own ideas about what your buttons should do. None of them let you just talk to the device.

Stream Deck Bridge is a control plane for the hardware. It's a small Rust binary that handles the USB protocol — JPEG encoding, HID report chunking, device detection, image caching — and exposes everything through a WebSocket API on localhost. Your code connects, sends JSON to set button images, and receives events when buttons are pressed. The bridge manages the device. You manage the experience.

This is intentionally half of the picture. The system is designed as two pieces:

1. **The bridge** (this project) — a device server that translates between USB and WebSocket. It handles the hardware, maintains state, and provides a clean protocol. It runs in the background, serves any number of clients, and has no opinion about what you display or what a button press means.

2. **Your client application** (whatever you want to build) — connects over WebSocket and implements the actual behavior: what images to show, how to respond to presses, how to organize buttons into modes and layouts, what systems to integrate with. This is where the interesting logic lives, and it's entirely your domain.

The bridge doesn't ship with modes, macros, dashboards, or shortcuts. It ships with a protocol. That separation is the point — it means your client can be a Python script, a TypeScript app, an Elixir server, a Go service, or all of them at once. Different tools can share the device, each claiming a few buttons. A CI monitor can push status colors to the bottom row while a media controller owns the top row. The bridge doesn't coordinate this — it just faithfully applies whatever the last client told it to do for each key.

## Quick Start

```bash
cargo build --release
./target/release/streamdeck-bridge
```

The bridge auto-detects the Stream Deck on USB and starts listening on `ws://localhost:37984`. That's it.

## What You Can Build

Connect from any language with a WebSocket library. Here are a few examples to give you a feel for the API.

### Paint buttons

<details open>
<summary>Python</summary>

```python
import asyncio, json, websockets

async def main():
    async with websockets.connect("ws://localhost:37984") as ws:
        await ws.recv()  # device_connected event
        await ws.send(json.dumps({"cmd": "set_color", "key": 0, "r": 255, "g": 0, "b": 0}))
        await ws.send(json.dumps({"cmd": "set_color", "key": 1, "r": 0, "g": 0, "b": 255}))

asyncio.run(main())
```

</details>

<details>
<summary>TypeScript</summary>

```typescript
import WebSocket from "ws";

const ws = new WebSocket("ws://localhost:37984");

ws.on("open", () => {
  ws.send(JSON.stringify({ cmd: "set_color", key: 0, r: 255, g: 0, b: 0 }));
  ws.send(JSON.stringify({ cmd: "set_color", key: 1, r: 0, g: 0, b: 255 }));
});
```

</details>

### React to button presses

<details open>
<summary>Python</summary>

```python
async def listen():
    async with websockets.connect("ws://localhost:37984") as ws:
        await ws.recv()  # device_connected
        while True:
            event = json.loads(await ws.recv())
            if event["event"] == "key_down":
                print(f"Button {event['key']} pressed")

asyncio.run(listen())
```

</details>

<details>
<summary>TypeScript</summary>

```typescript
import WebSocket from "ws";

const ws = new WebSocket("ws://localhost:37984");

ws.on("message", (data: WebSocket.Data) => {
  const event = JSON.parse(data.toString());
  if (event.event === "key_down") {
    console.log(`Button ${event.key} pressed`);
  }
});
```

</details>

### Display an image

Image commands use two frames: a JSON command, then the image data as a binary frame.

<details open>
<summary>Python</summary>

```python
async def show_image():
    async with websockets.connect("ws://localhost:37984") as ws:
        await ws.recv()
        await ws.send(json.dumps({"cmd": "set_image_jpeg", "key": 0}))
        with open("icon.jpg", "rb") as f:
            await ws.send(f.read())

asyncio.run(show_image())
```

</details>

<details>
<summary>TypeScript</summary>

```typescript
import WebSocket from "ws";
import { readFileSync } from "fs";

const ws = new WebSocket("ws://localhost:37984");

ws.on("open", () => {
  ws.send(JSON.stringify({ cmd: "set_image_jpeg", key: 0 }));
  ws.send(readFileSync("icon.jpg"));
});
```

</details>

### Build a CI status board

Multiple clients can connect at the same time. A monitoring script can update a few status buttons while your main application controls the rest.

<details open>
<summary>Python</summary>

```python
async def ci_monitor():
    async with websockets.connect("ws://localhost:37984") as ws:
        await ws.recv()
        while True:
            status = check_build_status()  # your CI polling logic
            r, g, b = {"pass": (0,255,0), "fail": (255,0,0), "running": (255,255,0)}[status]
            await ws.send(json.dumps({"cmd": "set_color", "key": 24, "r": r, "g": g, "b": b}))
            await asyncio.sleep(30)
```

</details>

<details>
<summary>TypeScript</summary>

```typescript
import WebSocket from "ws";

const ws = new WebSocket("ws://localhost:37984");

ws.on("open", async () => {
  while (true) {
    const status = await checkBuildStatus();
    const colors = { pass: [0,255,0], fail: [255,0,0], running: [255,255,0] };
    const [r, g, b] = colors[status];
    ws.send(JSON.stringify({ cmd: "set_color", key: 24, r, g, b }));
    await new Promise((resolve) => setTimeout(resolve, 30000));
  }
});
```

</details>

The protocol is just JSON over WebSocket — any language works. See [`INTEGRATION.md`](INTEGRATION.md) for complete examples in Elixir, Node.js, and shell.

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

The full system has six layers. This project provides the bottom three — everything below the line. Everything above it is yours to build.

```
┌─────────────────────────────────────────────────────┐
│  Integrations                                       │
│  CI/CD, Home Assistant, Slack, webhooks, ...        │
├─────────────────────────────────────────────────────┤
│  Application Logic                                  │
│  Modes, layers, chord detection, context switching, │
│  rendering engine, OS automation                    │
├─────────────────────────────────────────────────────┤
│  Client SDK                                         │
│  Typed API, auto-reconnect, event dispatch          │
╠═════════════════════════════════════════════════════╣
│  WebSocket Protocol            ┐                    │
│  JSON commands/events,         │                    │
│  binary image frames           │                    │
├────────────────────────────────┤  Stream Deck       │
│  Device Abstraction            │  Bridge            │
│  Image cache, state diffing,   │  (this project)    │
│  JPEG encoding, brightness     │                    │
├────────────────────────────────┤                    │
│  USB HID Driver                │                    │
│  Raw HID reports, detection    ┘                    │
└───────┬─────────────────────────────────────────────┘
        │ USB
   Stream Deck XL
```

The bridge handles the hard parts of talking to the hardware so your client doesn't have to. It deals with JPEG encoding (the device doesn't accept PNG or raw pixels), axis flipping (the hardware expects images mirrored on both axes), HID report chunking (each image is split across multiple 1024-byte USB reports), state bitmap diffing (the device sends all 32 button states at once on every change), and hot-plug lifecycle (detection, reconnection, image restoration). Your client just sends `"make button 5 red"` and receives `"button 5 was pressed"`.

### What the bridge does

- Connects to the Stream Deck over USB and keeps the connection alive
- Maintains a 32-slot image cache — one per button — with hash-based dedup
- Diffs the hardware's 32-byte state bitmap into individual `key_down`/`key_up` events with millisecond timestamps
- Runs a WebSocket server that accepts multiple concurrent clients
- Restores all cached images automatically when the device is unplugged and replugged

### What your client does

The bridge gives you two primitives: **set a button's image** and **receive button events**. Everything else is client logic. Here's what that opens up:

- **Modes and layers** — maintain a stack of button layouts. Press a "DEV" button, push a new mode, update all 32 images. Press "back," pop the mode, restore the previous layout. The bridge just sees 32 image updates; the mode concept lives in your code.

- **Gesture detection** — the bridge sends raw `key_down` and `key_up` events with timestamps. Your client decides what those mean: a press under 200ms is a tap, over 500ms is a long-press, two taps within 300ms is a double-tap, two buttons pressed within 50ms is a chord. The bridge provides the timing data; the interpretation is yours.

- **Context awareness** — detect which application is in the foreground and swap a subset of buttons to match. VS Code gets one layout, Figma gets another, Slack gets a third. Persistent buttons (media controls, status indicators) stay fixed across contexts.

- **Rendering** — generate 96x96 images for each button from text, icons, charts, or composite layouts. Render a CPU gauge, a Slack notification count, or a git branch name as an image and push it to a button.

- **Integrations** — connect to external systems and reflect their state on the deck. Poll GitHub Actions and show green/red per repo. Subscribe to Home Assistant and show light status. Watch a deploy pipeline and show progress.

- **OS automation** — map button presses to system actions: launch apps, simulate keyboard shortcuts, control volume, manage windows. The bridge delivers the press event; your client calls the OS API.

None of this is in the bridge, and none of it should be. The bridge is the device driver. Your client is the application.

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
scripts/build.sh        # or: cargo build --release
```

**Test your device:**

```bash
./target/release/streamdeck-bridge --test
```

This runs a rainbow animation across all buttons to verify your Stream Deck is connected and working.

**Run:**

```bash
# Default port (37984)
./target/release/streamdeck-bridge

# Custom port
./target/release/streamdeck-bridge --port 8080
```

**Install as a background service (macOS):**

```bash
scripts/install-launchd.sh
scripts/install-launchd.sh --port 8080  # custom port
```

The service starts on login and restarts on crash. Uninstall with `scripts/uninstall-launchd.sh`.

**Linux note:** You may need udev rules to access the device without root. Create `/etc/udev/rules.d/50-streamdeck.rules`:

```
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", TAG+="uaccess"
```

Then reload: `sudo udevadm control --reload-rules && sudo udevadm trigger`.

## Research

This project started with a deep investigation into Stream Deck hardware, protocols, and the open-source ecosystem. The research covers USB HID protocol details, image encoding and transfer performance, existing libraries across six languages, macOS automation APIs, and novel use cases. Available in [`docs/deep-research/`](docs/deep-research/stream-deck-xl-direct-control/overview.md).

## License

[MIT](LICENSE)
