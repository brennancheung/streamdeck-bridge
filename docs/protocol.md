# WebSocket Protocol Reference

The bridge runs a WebSocket server on `localhost` (default port `9001`). Clients communicate using two frame types:

- **Text frames** carry JSON — commands (client → bridge) and events (bridge → client)
- **Binary frames** carry image data, always immediately following a command that declares the target key and format

Multiple clients can connect simultaneously. All events are broadcast to every connected client.

---

## Connection Lifecycle

1. Client opens a WebSocket connection to `ws://localhost:9001`
2. If a device is connected, the bridge immediately sends a `device_connected` event
3. The client sends commands; the bridge sends events
4. Either side can close the connection at any time

```
Client                              Bridge
  │                                    │
  │──── WebSocket connect ────────────>│
  │<──── device_connected ─────────────│  (if device present)
  │                                    │
  │──── set_color ────────────────────>│
  │<──── image_set ────────────────────│
  │                                    │
  │<──── key_down ─────────────────────│  (user pressed a button)
  │<──── key_up ───────────────────────│
  │                                    │
  │──── close ────────────────────────>│
```

---

## Button Layout

The Stream Deck XL has 32 buttons in a 4-row, 8-column grid. Buttons are numbered left-to-right, top-to-bottom:

```
 0   1   2   3   4   5   6   7
 8   9  10  11  12  13  14  15
16  17  18  19  20  21  22  23
24  25  26  27  28  29  30  31
```

Each button has a 96x96 pixel LCD display.

---

## Commands (Client → Bridge)

Commands are JSON objects with a `cmd` field. The bridge processes them in order.

### `set_color`

Fill a button with a solid color.

```json
{"cmd": "set_color", "key": 5, "r": 255, "g": 0, "b": 0}
```

| Field | Type | Description |
|-------|------|-------------|
| `key` | `u8` | Button index (0–31) |
| `r` | `u8` | Red (0–255) |
| `g` | `u8` | Green (0–255) |
| `b` | `u8` | Blue (0–255) |

Response: `image_set` event.

### `set_image`

Set a button's image from raw pixel data or PNG. Send the JSON command as a text frame, then the image data as a binary frame.

```json
{"cmd": "set_image", "key": 5, "format": "rgba", "width": 96, "height": 96}
```

Then send a binary frame containing the image data.

| Field | Type | Description |
|-------|------|-------------|
| `key` | `u8` | Button index (0–31) |
| `format` | `string` | `"rgba"` (raw pixels) or `"png"` |
| `width` | `u32` | Image width in pixels |
| `height` | `u32` | Image height in pixels |

For `rgba` format, the binary frame must contain exactly `width * height * 4` bytes (RGBA, 8 bits per channel). For `png`, send the complete PNG file.

The bridge scales the image to 96x96 if needed, encodes to JPEG, flips both axes (device requirement), and pushes it over USB.

Response: `image_set` event.

### `set_image_jpeg`

Set a button's image from a pre-encoded JPEG. The bridge decodes it, applies the required axis flip, and re-encodes for the device.

```json
{"cmd": "set_image_jpeg", "key": 5}
```

Then send a binary frame containing the JPEG data.

| Field | Type | Description |
|-------|------|-------------|
| `key` | `u8` | Button index (0–31) |

Response: `image_set` event.

### `set_brightness`

Set the display brightness for all buttons.

```json
{"cmd": "set_brightness", "value": 80}
```

| Field | Type | Description |
|-------|------|-------------|
| `value` | `u8` | Brightness (0 = off, 100 = max) |

No response event.

### `clear_key`

Clear a single button (returns to black).

```json
{"cmd": "clear_key", "key": 5}
```

### `clear_all`

Clear all buttons.

```json
{"cmd": "clear_all"}
```

### `reset_to_logo`

Reset the device to its default Elgato logo screen.

```json
{"cmd": "reset_to_logo"}
```

### `get_device_info`

Request current device information.

```json
{"cmd": "get_device_info"}
```

Response: `device_info` event (broadcast to all clients).

---

## Events (Bridge → Client)

Events are JSON objects with an `event` field. All events are broadcast to every connected client.

### `key_down`

A button was pressed.

```json
{"event": "key_down", "key": 5, "timestamp": 1716480000123}
```

| Field | Type | Description |
|-------|------|-------------|
| `key` | `u8` | Button index (0–31) |
| `timestamp` | `u64` | Milliseconds since Unix epoch |

### `key_up`

A button was released.

```json
{"event": "key_up", "key": 5, "timestamp": 1716480000456}
```

Same fields as `key_down`.

### `device_connected`

A Stream Deck was connected (or was already connected when the client joined).

```json
{
  "event": "device_connected",
  "serial": "CL40I1A02711",
  "model": "xl",
  "firmware": "1.02.005",
  "keys": 32,
  "rows": 4,
  "cols": 8,
  "icon_size": 96
}
```

### `device_disconnected`

The Stream Deck was unplugged. The bridge continues running and will send `device_connected` when a device is plugged back in. Cached button images are automatically restored on reconnect.

```json
{"event": "device_disconnected"}
```

### `device_info`

Response to `get_device_info`. Same fields as `device_connected` plus `brightness`.

```json
{
  "event": "device_info",
  "serial": "CL40I1A02711",
  "model": "xl",
  "firmware": "1.02.005",
  "keys": 32,
  "rows": 4,
  "cols": 8,
  "icon_size": 96,
  "brightness": 80
}
```

### `image_set`

Confirms that a button's image was updated (or skipped via dedup).

```json
{"event": "image_set", "key": 5}
```

### `error`

Something went wrong processing a command.

```json
{"event": "error", "message": "invalid key index: 33"}
```

---

## Image Caching and Deduplication

The bridge maintains a 32-slot image cache. When you send an image for a button:

1. The bridge hashes the incoming data
2. If the hash matches the cached image for that button, the USB transfer is skipped
3. If different, the image is pushed to the device and the cache is updated
4. On device reconnect (unplug/replug), all cached images are automatically restored

This means you can safely send images at high frequency — only actual changes hit the USB bus.

---

## Binary Frame Protocol

Three commands expect a binary frame immediately after the JSON text frame:

| Command | Binary frame contents |
|---------|----------------------|
| `set_image` (format: `rgba`) | Raw RGBA pixels, `width * height * 4` bytes |
| `set_image` (format: `png`) | Complete PNG file |
| `set_image_jpeg` | Complete JPEG file |

The binary frame must be the next frame after the command. Sending a binary frame without a preceding image command results in an error.

---

## Multi-Client Behavior

Multiple WebSocket clients can connect simultaneously. Behavior:

- **Events are broadcast** — all clients receive all key press/release and device events
- **Commands are processed in order** — if two clients set different images on the same button, the last one wins
- **Image cache is shared** — the cache reflects the current state of the device regardless of which client set it
- **No client isolation** — there is no concept of button ownership; coordination is the clients' responsibility

This design supports scenarios like:
- A main application controlling most buttons while a monitoring script updates a status indicator
- A test client connecting alongside a production client to observe events
- Multiple independent tools each claiming a region of the button grid

---

## Timestamps

Key events include a `timestamp` field: milliseconds since the Unix epoch, sourced from the host system clock. These timestamps enable client-side gesture detection:

- **Long press**: `key_up.timestamp - key_down.timestamp > threshold`
- **Double tap**: second `key_down.timestamp - first key_down.timestamp < threshold`
- **Chord**: two `key_down` events within a small window (e.g., 50ms)

The bridge does not interpret these patterns — it provides the raw timing data for clients to implement their own interaction models.

---

## Error Handling

Invalid commands produce an `error` event:

```json
{"event": "error", "message": "invalid key index: 33"}
{"event": "error", "message": "missing binary data"}
{"event": "error", "message": "invalid command: unknown variant `foo`"}
```

The bridge does not close the connection on errors. The client can continue sending commands after receiving an error.
