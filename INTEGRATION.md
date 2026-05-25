# Stream Deck Bridge — Integration Guide

This document is for AI agents and developers building client applications that talk to the Stream Deck Bridge. It contains everything you need to connect, send commands, and handle events. For agents in other repos, point your CLAUDE.md or AGENTS.md here.

## Overview

The bridge is a Rust binary that runs on the same machine as the Stream Deck. It exposes a WebSocket server on `localhost:9001` (configurable). You connect, send JSON commands, and receive JSON events. Image data is sent as binary WebSocket frames.

The bridge handles USB protocol details (JPEG encoding, HID reports, axis flipping, device detection). Your client just works with button indices, RGB colors, and image bytes.

## Connecting

```
ws://localhost:9001
```

On connect, if a device is present, you immediately receive a `device_connected` event:

```json
{"event":"device_connected","serial":"CL40I1A02711","model":"xl","firmware":"1.01.000","keys":32,"rows":4,"cols":8,"icon_size":96}
```

Use `keys`, `rows`, and `cols` from this event to know the button layout. Do not hardcode 32 — other Stream Deck models have fewer buttons.

## Button Numbering

Buttons are numbered left-to-right, top-to-bottom starting at 0:

```
 0   1   2   3   4   5   6   7      ← row 0
 8   9  10  11  12  13  14  15      ← row 1
16  17  18  19  20  21  22  23      ← row 2
24  25  26  27  28  29  30  31      ← row 3
```

To convert between grid position and index:
```
index = row * cols + col
row   = index / cols
col   = index % cols
```

## Commands

All commands are JSON text frames with a `cmd` field.

### Set a button color

```json
{"cmd": "set_color", "key": 0, "r": 255, "g": 0, "b": 0}
```

Response: `{"event":"image_set","key":0}`

### Set brightness

```json
{"cmd": "set_brightness", "value": 80}
```

Range 0 (off) to 100 (max). No response event.

### Clear buttons

```json
{"cmd": "clear_key", "key": 0}
{"cmd": "clear_all"}
{"cmd": "reset_to_logo"}
```

### Query device info

```json
{"cmd": "get_device_info"}
```

Response:
```json
{"event":"device_info","serial":"CL40I1A02711","model":"xl","firmware":"1.01.000","keys":32,"rows":4,"cols":8,"icon_size":96,"brightness":80}
```

### Set a button image

Image commands use two frames: a JSON text frame declaring the key and format, followed by a binary frame with the image data. **The binary frame must be the very next frame after the JSON command.**

**JPEG (simplest — send a JPEG file):**
```
Text frame:  {"cmd": "set_image_jpeg", "key": 0}
Binary frame: <JPEG bytes>
```

**RGBA raw pixels:**
```
Text frame:  {"cmd": "set_image", "key": 0, "format": "rgba", "width": 96, "height": 96}
Binary frame: <96 * 96 * 4 = 36864 bytes, RGBA order>
```

**PNG:**
```
Text frame:  {"cmd": "set_image", "key": 0, "format": "png", "width": 96, "height": 96}
Binary frame: <PNG file bytes>
```

The bridge handles resizing and encoding. Images don't need to be exactly 96x96, but that's the native resolution — other sizes will be scaled.

Response: `{"event":"image_set","key":0}`

## Events

All events are JSON text frames broadcast to every connected client.

### Button press/release

```json
{"event": "key_down", "key": 5, "timestamp": 1716480000123}
{"event": "key_up", "key": 5, "timestamp": 1716480000456}
```

`timestamp` is milliseconds since Unix epoch. Use the difference between `key_down` and `key_up` to detect long presses. Use the gap between consecutive `key_down` events for double-tap detection. For chord detection, look for multiple `key_down` events within ~50ms.

### Device connection

```json
{"event": "device_connected", "serial": "...", "model": "xl", ...}
{"event": "device_disconnected"}
```

The bridge reconnects automatically when the device is replugged and restores all cached button images. Clients don't need to re-send images after reconnect.

### Errors

```json
{"event": "error", "message": "invalid key index: 33"}
```

The connection stays open after an error. You can keep sending commands.

## Complete Examples

### Python

```python
import asyncio
import json
import websockets

async def main():
    async with websockets.connect("ws://localhost:9001") as ws:
        # Wait for device info
        device = json.loads(await ws.recv())
        print(f"Connected to {device['model']} with {device['keys']} keys")

        # Set button 0 to red
        await ws.send(json.dumps({"cmd": "set_color", "key": 0, "r": 255, "g": 0, "b": 0}))
        await ws.recv()  # image_set confirmation

        # Set button 1 to a JPEG image
        await ws.send(json.dumps({"cmd": "set_image_jpeg", "key": 1}))
        with open("icon.jpg", "rb") as f:
            await ws.send(f.read())
        await ws.recv()  # image_set confirmation

        # Listen for button presses
        while True:
            event = json.loads(await ws.recv())
            if event["event"] == "key_down":
                print(f"Button {event['key']} pressed")
            elif event["event"] == "key_up":
                print(f"Button {event['key']} released")

asyncio.run(main())
```

### Elixir

```elixir
# Using the WebSockex library
defmodule StreamDeck do
  use WebSockex

  def start_link(opts \\ []) do
    WebSockex.start_link("ws://localhost:9001", __MODULE__, %{}, opts)
  end

  def set_color(pid, key, r, g, b) do
    cmd = Jason.encode!(%{cmd: "set_color", key: key, r: r, g: g, b: b})
    WebSockex.send_frame(pid, {:text, cmd})
  end

  def set_image_jpeg(pid, key, jpeg_bytes) do
    cmd = Jason.encode!(%{cmd: "set_image_jpeg", key: key})
    WebSockex.send_frame(pid, {:text, cmd})
    WebSockex.send_frame(pid, {:binary, jpeg_bytes})
  end

  def set_brightness(pid, value) do
    cmd = Jason.encode!(%{cmd: "set_brightness", value: value})
    WebSockex.send_frame(pid, {:text, cmd})
  end

  @impl true
  def handle_frame({:text, msg}, state) do
    case Jason.decode!(msg) do
      %{"event" => "key_down", "key" => key} ->
        IO.puts("Button #{key} pressed")
        {:ok, state}
      %{"event" => "device_connected"} = info ->
        IO.puts("Device connected: #{info["model"]}")
        {:ok, state}
      _ ->
        {:ok, state}
    end
  end
end
```

### Node.js

```javascript
const WebSocket = require("ws");

const ws = new WebSocket("ws://localhost:9001");

ws.on("open", () => {
  // Set button 0 to green
  ws.send(JSON.stringify({ cmd: "set_color", key: 0, r: 0, g: 255, b: 0 }));
});

ws.on("message", (data) => {
  const event = JSON.parse(data);
  if (event.event === "key_down") {
    console.log(`Button ${event.key} pressed`);
  }
});

// Send a JPEG image to button 1
const fs = require("fs");
ws.send(JSON.stringify({ cmd: "set_image_jpeg", key: 1 }));
ws.send(fs.readFileSync("icon.jpg"));
```

### Shell (using websocat)

```bash
# Set button 0 to blue
echo '{"cmd":"set_color","key":0,"r":0,"g":0,"b":255}' | websocat ws://localhost:9001

# Listen for events
websocat ws://localhost:9001
```

## Common Patterns

### Batch updates

To update multiple buttons at once, send commands in sequence. The bridge processes them in order and each hits USB individually. There is no explicit batch command, but the bridge's image dedup means unchanged buttons are skipped automatically.

### Polling status

For a status dashboard, connect once and loop: poll your data source, set button colors/images for each status, sleep, repeat. The bridge deduplicates — if a button's image hasn't changed, the USB transfer is skipped even if you re-send it.

### Multiple clients

Multiple WebSocket clients can connect simultaneously. All clients receive all events. There is no isolation between clients — if two clients set the same button, the last write wins. Coordinate button ownership in your application logic if needed.

### Reconnection

If the bridge restarts, your client's WebSocket connection will drop. Implement reconnection with backoff. When reconnected, you'll receive a `device_connected` event if the device is present. You do not need to re-send images — the bridge has its own cache. But if the bridge process itself was restarted, the cache is empty and you should re-send.

## Gotchas

1. **Binary frame ordering**: Image commands (`set_image`, `set_image_jpeg`) require the binary frame to be the *very next* frame. Do not send another text command between the JSON and the binary frame.

2. **Don't hardcode 32 keys**: Read `keys`, `rows`, and `cols` from the `device_connected` event. Other Stream Deck models have 6, 8, or 15 keys.

3. **Timestamps are host-side**: The `timestamp` in key events comes from the host system clock, not the device. They're monotonic within a session but not comparable across machines.

4. **Events are broadcast**: Every connected client receives every event. If you have multiple clients, each will see button presses and image_set confirmations from all clients, not just their own.

5. **Brightness affects all keys**: `set_brightness` is a global setting, not per-key.

6. **Cache survives device unplug**: If the user unplugs and replugs the Stream Deck, the bridge restores cached images automatically. But if the bridge *process* restarts, the cache is lost.
