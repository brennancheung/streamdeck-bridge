# Browser WebHID

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Open-Source Libraries

---

## Key Findings

- **WebHID works with Stream Deck devices in the browser.** The `@elgato-stream-deck/webhid` package (v7.6.2, March 2026) provides a high-level TypeScript API that wraps the raw WebHID browser API. It supports all current Stream Deck models including the XL.
- **Browser support is Chromium-only.** Chrome 89+, Edge 89+, and Opera 76+ support WebHID. Firefox explicitly rejects it (Mozilla calls it "harmful"). Safari has no support. Mobile browsers have no support. Global browser coverage is ~27%.
- **localhost counts as a secure context.** WebHID requires HTTPS, but `http://localhost` is treated as trustworthy by all browsers that implement WebHID. A local React dev server works without SSL certificates.
- **The Elgato Stream Deck app must not be running.** Only one process can hold the HID device handle. Close the native app before the browser can access the device.
- **Persistent device permissions work across page reloads.** After initial user-gesture-triggered pairing via `requestDevice()`, the browser remembers the grant. Subsequent visits can call `getDevices()` to silently reconnect without a permission dialog.
- **Canvas-based image rendering is built in.** The WebHID variant adds `fillKeyCanvas()` and `fillPanelCanvas()` methods that accept an `HTMLCanvasElement`, making React/Canvas integration straightforward.
- **JPEG encoding uses a hidden canvas internally.** The library creates an offscreen `<canvas>`, draws raw pixel data, then calls `canvas.toBlob('image/jpeg', 0.9)` to produce the format the Stream Deck hardware expects.
- **Several working browser demos exist** including the official Julusian demo, a Daft Punk soundboard, a Jitsi meeting controller, and a Next.js test app. This is a proven pattern, not theoretical.

---

## 1. What Is WebHID?

WebHID is a W3C Web API that gives JavaScript in the browser direct read/write access to Human Interface Devices over USB or Bluetooth. It operates at the HID protocol level -- sending and receiving HID reports -- rather than at the raw USB packet level (which is WebUSB's domain).

The Stream Deck communicates with the host over USB HID. Every key press arrives as an HID input report. Every image pushed to a key is sent as a series of HID output reports containing JPEG chunks. This makes WebHID a natural fit.

### WebHID vs. Native HID Access

| Aspect | Native (`node-hid`) | Browser (WebHID) |
|---|---|---|
| Runtime | Node.js process | Browser tab |
| Binding | C++ native addon | Browser API, no native code |
| Permissions | OS-level (udev on Linux) | Browser permission dialog per origin |
| HTTPS required | No | Yes (localhost exempt) |
| User gesture required | No | Yes, for initial `requestDevice()` |
| Multi-tab | N/A | Single tab ownership |
| Background throttling | No | Yes, reduced draw rate in background tabs |
| Protected devices | No restrictions | Keyboards, mice, FIDO keys are blocked |
| Cross-platform | Win/Mac/Linux | Win/Mac/Linux (Chromium browsers only) |

---

## 2. Browser Support

| Browser | Version | Status |
|---|---|---|
| Chrome | 89+ (March 2021) | Fully supported |
| Edge | 89+ (March 2021) | Fully supported (Chromium-based) |
| Opera | 76+ (April 2021) | Fully supported |
| Chrome (Android) | -- | Not supported (OS-level USB restriction) |
| Firefox (all) | -- | Explicitly rejected by Mozilla |
| Safari (all) | -- | No support, no stated position from WebKit |
| Samsung Internet | -- | Disabled despite Chromium base |

**Global browser support: ~27% of users** (April 2026). This number is misleading for a local-tool use case -- if you are building a personal dashboard that runs on your own machine in Chrome, 100% of your "users" are supported.

### Why Firefox Rejects WebHID

Mozilla classifies WebHID as "harmful" due to concerns about device fingerprinting and exposing hardware attack surface to web pages. This is a philosophical position, not a technical limitation.

---

## 3. The `@elgato-stream-deck/webhid` Package

| Field | Value |
|---|---|
| Package | `@elgato-stream-deck/webhid` |
| Version | 7.6.2 |
| Published | 2026-03-29 |
| Author | Julian Waller (Julusian) |
| License | MIT |
| Repository | [github.com/Julusian/node-elgato-stream-deck](https://github.com/Julusian/node-elgato-stream-deck) |
| Monorepo sibling | `@elgato-stream-deck/node` (same API, Node.js runtime) |
| Core dependency | `@elgato-stream-deck/core` (shared, platform-agnostic) |

This is the browser variant of the same Julusian library documented in the Node.js ecosystem research. It shares the `@elgato-stream-deck/core` package with the Node.js variant, so the device-level API (methods, events, types) is identical. The WebHID package adds browser-specific wrappers for device selection and canvas-based image rendering.

### Installation

```bash
pnpm add @elgato-stream-deck/webhid
```

### Supported Stream Deck Models

All models supported by the core library work through WebHID. Elgato's USB Vendor ID is `0x0FD9`. Known Product IDs:

| Model | Product ID | Keys | Key Size |
|---|---|---|---|
| Stream Deck Original | 0x0060 | 15 | 72x72 px |
| Stream Deck Original V2 | 0x006D | 15 | 72x72 px |
| Stream Deck MK.2 | 0x0080 | 15 | 72x72 px |
| Stream Deck Mini | 0x0063 | 6 | 80x80 px |
| Stream Deck Mini MK.2 | 0x0090 | 6 | 80x80 px |
| Stream Deck XL | 0x006C | 32 | 96x96 px |
| Stream Deck + | -- | 8 + 4 encoders + LCD | 120x120 px |
| Stream Deck + XL | -- | (added v7.6.0) | -- |
| Stream Deck Neo | -- | 8 + 2 touch keys | 72x72 px |
| Stream Deck Studio | -- | (added v7.0.0) | -- |

---

## 4. API Reference

### 4.1 Connection Functions

Three exported functions handle device discovery and connection:

```typescript
import {
  requestStreamDecks,
  getStreamDecks,
  openDevice,
} from '@elgato-stream-deck/webhid'

// Prompt user to select a Stream Deck (requires user gesture)
const decks = await requestStreamDecks()

// Reconnect to previously authorized devices (no dialog)
const decks = await getStreamDecks()

// Open a specific HIDDevice handle
const deck = await openDevice(hidDevice)
```

**`requestStreamDecks(options?)`** -- Calls `navigator.hid.requestDevice()` with Elgato's vendor ID filter. The browser shows a device picker. Returns `Promise<StreamDeckWeb[]>`. Must be called from a click/keypress handler.

**`getStreamDecks(options?)`** -- Calls `navigator.hid.getDevices()` and filters for Elgato vendor ID. Returns previously authorized devices without user interaction. Use this for auto-reconnect on page load.

**`openDevice(browserDevice, options?)`** -- Opens a specific `HIDDevice` and wraps it in a `StreamDeckWeb` instance. Looks up the device model from the product ID, opens the HID connection, and initializes the JPEG encoder.

### 4.2 The `StreamDeckWeb` Class

`StreamDeckWeb` extends `StreamDeckProxy` from core, adding browser-specific methods:

```typescript
class StreamDeckWeb extends StreamDeckProxy {
  // Revoke browser permission for this device
  async forget(): Promise<void>

  // Render a canvas directly to a single key
  async fillKeyCanvas(keyIndex: KeyIndex, canvas: HTMLCanvasElement): Promise<void>

  // Render a canvas across the entire key panel
  async fillPanelCanvas(canvas: HTMLCanvasElement): Promise<void>
}
```

### 4.3 Inherited Methods from `StreamDeckProxy`

All methods from the core library are available. Key ones:

```typescript
// Display
await deck.fillKeyColor(keyIndex, r, g, b)        // Fill a key with solid color
await deck.fillKeyBuffer(keyIndex, buffer, opts)   // Fill a key with image data
await deck.fillPanelBuffer(buffer, opts)           // Fill entire panel with image
await deck.clearKey(keyIndex)                      // Clear a single key
await deck.clearPanel()                            // Clear all keys

// Device control
await deck.setBrightness(75)                       // 0-100
await deck.resetToLogo()                           // Show Elgato boot logo

// Device info
await deck.getFirmwareVersion()                    // e.g. "1.02.006"
await deck.getSerialNumber()                       // Device serial string

// Stream Deck + / Neo specific
await deck.fillLcd(buffer, opts)                   // Fill LCD strip
await deck.fillLcdRegion(x, y, w, h, buffer, opts) // Fill LCD region
await deck.setEncoderColor(index, r, g, b)         // Set encoder LED color

// Properties
deck.CONTROLS          // Array of control definitions (buttons, encoders, LCD)
deck.MODEL             // DeviceModelId enum value
deck.PRODUCT_NAME      // Human-readable model name
```

### 4.4 Events

```typescript
// Key pressed
deck.on('down', (control) => {
  console.log('Key pressed:', control.index)
})

// Key released
deck.on('up', (control) => {
  console.log('Key released:', control.index)
})

// Encoder rotated (Stream Deck + / Neo)
deck.on('rotateLeft', (control, amount) => { })
deck.on('rotateRight', (control, amount) => { })

// LCD touch (Stream Deck +)
deck.on('lcdShortPress', (control, position) => { })
deck.on('lcdLongPress', (control, position) => { })
deck.on('lcdSwipe', (control, from, to) => { })

// Error (MUST always have a listener)
deck.on('error', (err) => {
  console.error('Stream Deck error:', err)
})
```

---

## 5. Complete Working Example

### 5.1 Minimal HTML + JavaScript

```html
<!DOCTYPE html>
<html>
<head><title>Stream Deck WebHID</title></head>
<body>
  <button id="connect">Connect Stream Deck</button>
  <pre id="log"></pre>

  <script type="module">
    import {
      requestStreamDecks,
      getStreamDecks,
    } from '@elgato-stream-deck/webhid'

    const log = (msg) => {
      document.getElementById('log').textContent += msg + '\n'
    }

    // Auto-reconnect on page load
    const existing = await getStreamDecks()
    if (existing.length > 0) {
      setupDeck(existing[0])
    }

    // User-triggered connection
    document.getElementById('connect').addEventListener('click', async () => {
      const decks = await requestStreamDecks()
      if (decks.length > 0) setupDeck(decks[0])
    })

    function setupDeck(deck) {
      log(`Connected: ${deck.PRODUCT_NAME} (${deck.CONTROLS.length} controls)`)

      // Must always listen for errors
      deck.on('error', (e) => log(`Error: ${e.message}`))

      // Listen for key presses
      deck.on('down', (control) => {
        log(`Key ${control.index} pressed`)
        // Flash the key red
        deck.fillKeyColor(control.index, 255, 0, 0)
      })

      deck.on('up', (control) => {
        log(`Key ${control.index} released`)
        // Clear the key
        deck.clearKey(control.index)
      })

      // Set brightness
      deck.setBrightness(80)
    }
  </script>
</body>
</html>
```

### 5.2 React Hook Pattern

```typescript
// useStreamDeck.ts
import { useState, useEffect, useCallback, useRef } from 'react'
import type { StreamDeckWeb } from '@elgato-stream-deck/webhid'
import { requestStreamDecks, getStreamDecks } from '@elgato-stream-deck/webhid'

interface UseStreamDeckOptions {
  onKeyDown?: (keyIndex: number) => void
  onKeyUp?: (keyIndex: number) => void
  brightness?: number
}

export function useStreamDeck(options: UseStreamDeckOptions = {}) {
  const [deck, setDeck] = useState<StreamDeckWeb | null>(null)
  const [connected, setConnected] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const deckRef = useRef<StreamDeckWeb | null>(null)

  // Attempt auto-reconnect on mount
  useEffect(() => {
    getStreamDecks().then((decks) => {
      if (decks.length > 0) {
        initDeck(decks[0])
      }
    })
    return () => {
      deckRef.current?.close()
    }
  }, [])

  const initDeck = useCallback((d: StreamDeckWeb) => {
    deckRef.current = d
    setDeck(d)
    setConnected(true)
    setError(null)

    d.on('error', (err) => {
      setError(err.message)
      setConnected(false)
    })

    d.on('down', (control) => options.onKeyDown?.(control.index))
    d.on('up', (control) => options.onKeyUp?.(control.index))

    if (options.brightness !== undefined) {
      d.setBrightness(options.brightness)
    }
  }, [options])

  const connect = useCallback(async () => {
    try {
      const decks = await requestStreamDecks()
      if (decks.length > 0) initDeck(decks[0])
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Connection failed')
    }
  }, [initDeck])

  const disconnect = useCallback(async () => {
    await deckRef.current?.close()
    deckRef.current = null
    setDeck(null)
    setConnected(false)
  }, [])

  return { deck, connected, error, connect, disconnect }
}
```

```tsx
// StreamDeckPanel.tsx
import { useStreamDeck } from './useStreamDeck'

export function StreamDeckPanel() {
  const { deck, connected, connect, disconnect } = useStreamDeck({
    brightness: 80,
    onKeyDown: (keyIndex) => {
      console.log('Pressed:', keyIndex)
      deck?.fillKeyColor(keyIndex, 0, 128, 255)
    },
    onKeyUp: (keyIndex) => {
      deck?.clearKey(keyIndex)
    },
  })

  return (
    <div>
      {connected ? (
        <>
          <p>Connected: {deck?.PRODUCT_NAME}</p>
          <button onClick={disconnect}>Disconnect</button>
          <button onClick={() => deck?.clearPanel()}>Clear All</button>
          <button onClick={() => deck?.resetToLogo()}>Show Logo</button>
        </>
      ) : (
        <button onClick={connect}>Connect Stream Deck</button>
      )}
    </div>
  )
}
```

### 5.3 Rendering Images to Keys via Canvas

```typescript
async function renderTextToKey(
  deck: StreamDeckWeb,
  keyIndex: number,
  text: string,
  bgColor = '#1a1a2e',
  textColor = '#ffffff'
) {
  // Get key dimensions from controls metadata
  const control = deck.CONTROLS.find(
    (c) => c.type === 'button' && c.index === keyIndex
  )
  if (!control || control.feedbackType !== 'lcd') return

  const { width, height } = control.pixelSize
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const ctx = canvas.getContext('2d')!

  // Draw background
  ctx.fillStyle = bgColor
  ctx.fillRect(0, 0, width, height)

  // Draw text centered
  ctx.fillStyle = textColor
  ctx.font = `bold ${Math.floor(height / 4)}px sans-serif`
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.fillText(text, width / 2, height / 2)

  // Send to device
  await deck.fillKeyCanvas(keyIndex, canvas)
}
```

---

## 6. Security and Permissions Model

### 6.1 The Permission Flow

1. **Secure context required.** The page must be served over HTTPS. Exception: `http://localhost` and `http://127.0.0.1` are treated as secure contexts by Chrome/Edge/Opera. A local Vite or Next.js dev server works without SSL.

2. **User gesture required.** `navigator.hid.requestDevice()` must be called inside a click, keypress, or touch handler. Calling it from `setTimeout`, `fetch().then()`, or `DOMContentLoaded` throws: `DOMException: Must be handling a user gesture`.

3. **Device picker dialog.** The browser shows a native dialog listing matching HID devices. The user selects one and clicks "Connect". The permission is granted per-origin.

4. **Persistent grant.** Once granted, `navigator.hid.getDevices()` returns the device on subsequent visits without showing the dialog. The grant persists until the user revokes it in browser settings or the page calls `device.forget()`.

5. **Single-tab ownership.** Only one tab can hold an open HID connection at a time. A second tab attempting `device.open()` will fail.

### 6.2 Protected Device Classes

Chrome blocks WebHID access to certain device types at the browser level:

- Generic keyboards and mice (prevents keylogging)
- FIDO/U2F security keys (prevents credential theft)
- System controllers (pointer devices, multi-axis)

Stream Deck devices are **not** in the blocked list -- they use vendor-specific HID usages.

### 6.3 Chrome Extensions

Chrome extensions can use WebHID but with restrictions:
- `requestDevice()` cannot be called from a service worker
- Must use an extension page (popup, options, side panel) and pass the handle via messaging
- Requires the `"hid"` permission in `manifest.json`

---

## 7. Connection Lifecycle and Persistence

### 7.1 Auto-Reconnect Pattern

```typescript
// On page load, check for previously authorized devices
const decks = await getStreamDecks()
if (decks.length > 0) {
  // Silently reconnect -- no user gesture needed
  const deck = decks[0]
  startApp(deck)
}

// Listen for USB connect/disconnect
navigator.hid.addEventListener('connect', (event) => {
  // A new HID device was plugged in
  if (event.device.vendorId === 0x0fd9) {
    openDevice(event.device).then(startApp)
  }
})

navigator.hid.addEventListener('disconnect', (event) => {
  // Device was unplugged
  handleDisconnect()
})
```

### 7.2 Background Tab Throttling

When a tab is in the background, Chrome throttles timers and reduces the frequency of HID report processing. This means:

- Key press events still arrive but may be batched/delayed
- Image writes (output reports) will be slower
- Animation frame rates drop significantly

For a Stream Deck dashboard that needs real-time updates, **keep the tab in the foreground** or run as a dedicated Chrome window/PWA. Alternatively, use the `@elgato-stream-deck/node` variant in a persistent Node.js process.

### 7.3 Exclusive Device Access

The Stream Deck uses a single HID interface. Only one consumer can open it:

| Running | Browser WebHID | Result |
|---|---|---|
| Elgato Stream Deck app | Attempt connect | **Fails** -- device in use |
| Nothing | Attempt connect | Works |
| Browser tab A | Browser tab B | **Fails** -- single tab ownership |
| Node.js app using `node-hid` | Attempt connect | **Fails** -- device in use |

**You must close the Elgato Stream Deck application** before using WebHID. The device will display the Elgato logo when no software has claimed it.

---

## 8. Architecture: Local Web App as Stream Deck Controller

The WebHID approach enables a compelling architecture for a local-first Stream Deck controller:

```
React App (Vite/Next.js on localhost:3000)
  |
  |-- WebHID API (navigator.hid)
  |     |
  |     |-- HID Output Reports (JPEG images to keys)
  |     |-- HID Input Reports (key press events)
  |     |-- HID Feature Reports (brightness, firmware)
  |
  +-- Stream Deck XL (USB)
```

### 8.1 Why This Architecture Works

1. **No native code.** No Electron, no Tauri, no native addons. Pure browser JavaScript.
2. **Hot reload.** Vite/Next.js HMR works normally. Change a button layout, see it on the device instantly.
3. **React ecosystem.** Use any React library for the configuration UI. Canvas-based rendering for key images.
4. **localhost is secure.** No SSL certificate gymnastics needed for development.
5. **Persistent permissions.** Pair the device once, auto-reconnect on every page load.
6. **Single dependency.** `@elgato-stream-deck/webhid` is the only hardware-facing dependency.

### 8.2 Tradeoffs vs. Node.js Backend

| Consideration | Browser WebHID | Node.js + `node-hid` |
|---|---|---|
| Setup complexity | Lower (no native addons) | Higher (C++ build toolchain) |
| Background operation | Tab must stay open | Runs as daemon/service |
| Performance | Canvas-based JPEG, good enough | Can use `jpeg-turbo` for speed |
| Hot-plug detection | `navigator.hid` events | Requires USB polling or `udev` |
| Multi-device | Single tab per device | Unlimited |
| System integration | Limited to browser APIs | Full OS access (scripts, apps) |
| Deployment | Open a URL | Install and run a process |

### 8.3 Hybrid Architecture

For more demanding use cases, combine both:

```
React UI (localhost:3000)
  |
  |-- WebHID (direct device control for key images/events)
  |
  |-- WebSocket to local Node.js server (localhost:8080)
        |
        |-- System commands (launch apps, run scripts)
        |-- Home automation APIs
        |-- OBS WebSocket
        |-- Spotify API
```

The browser handles the Stream Deck display and input. A lightweight Node.js server handles the actions that require system access.

---

## 9. Existing Browser-Based Stream Deck Projects

### 9.1 Julusian Official WebHID Demo

- **URL:** [julusian.github.io/node-elgato-stream-deck](https://julusian.github.io/node-elgato-stream-deck/)
- **What it does:** Functional demo of the library. Connects to a Stream Deck, fills keys with colors and chase animations on button press.
- **Tech:** Uses the library directly, vanilla JS.

### 9.2 Daft Punk Soundboard (Bram Van Damme)

- **URL:** [github.com/bramus/webhid-elgato-stream-deck-daft-punk-soundboard](https://github.com/bramus/webhid-elgato-stream-deck-daft-punk-soundboard)
- **What it does:** Maps Daft Punk audio samples to Stream Deck buttons. Paged navigation across multiple sound sets.
- **Tech:** Vanilla JavaScript + Snowpack. MIT license.
- **Notable:** Demonstrates that the browser remembers the device automatically after first pairing.

### 9.3 Jitsi Meeting Controller

- **URL:** [jitsi.org/blog/custom-meeting-controls-with-elgato-stream-deck-and-webhid](https://jitsi.org/blog/custom-meeting-controls-with-elgato-stream-deck-and-webhid/)
- **What it does:** Maps Stream Deck buttons to Jitsi meeting controls (mute, camera, screen share) with dynamic icons.
- **Tech:** Uses `@elgato-stream-deck/webhid` + Jitsi iframe API.
- **Notable:** Real production use case. Each button shows a contextual icon that updates based on meeting state.

### 9.4 Next.js WebHID Test App (dysbulic)

- **URL:** [github.com/dysbulic/stream-deck-webhid-driver](https://github.com/dysbulic/stream-deck-webhid-driver)
- **What it does:** Next.js + TypeScript test app for experimenting with WebHID Stream Deck control.
- **Tech:** Next.js, TypeScript (86.1%), React components.
- **Notable:** Demonstrates the React/Next.js integration pattern.

### 9.5 Nikita Dubko's WebHID Talk Demo

- **URL:** [mefody.github.io/talks/webhid/demos/streamdeck](https://mefody.github.io/talks/webhid/demos/streamdeck/)
- **What it does:** Conference talk demo showing direct WebHID protocol usage with Stream Deck.
- **Notable:** Lower-level raw WebHID code, useful for understanding the protocol.

---

## 10. How JPEG Encoding Works in the Browser

Stream Deck hardware expects JPEG images for key displays. The `@elgato-stream-deck/webhid` library handles this automatically using a clever canvas-based approach:

```typescript
// Internal implementation (from packages/webhid/src/jpeg.ts)
export async function encodeJPEG(
  buffer: Uint8Array,
  width: number,
  height: number
): Promise<Uint8Array> {
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const ctx = canvas.getContext('2d')!

  const imageData = ctx.createImageData(width, height)
  imageData.data.set(buffer)
  ctx.putImageData(imageData, 0, 0)

  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob(
      (b) => b ? resolve(b) : reject(new Error('No image generated')),
      'image/jpeg',
      0.9   // 90% quality
    )
  })
  return new Uint8Array(await blob.arrayBuffer())
}
```

**Performance notes:**
- Creates a temporary offscreen canvas per encode call
- `canvas.toBlob()` is browser-native and hardware-accelerated on most platforms
- 90% JPEG quality is a reasonable default (lower = smaller reports, faster transfer)
- For the XL's 32 keys at 96x96 px each, encoding is fast enough for interactive use
- Custom `encodeJPEG` function can be passed in `OpenStreamDeckOptions` to override

---

## 11. Limitations and Caveats

### Must close Elgato software
The native Stream Deck app and WebHID cannot access the device simultaneously. One must release it for the other to connect.

### Chromium-only
No Firefox or Safari support. Not a concern for a personal tool, but prevents distribution as a general web app.

### Single tab
Only one browser tab can hold the HID connection. Opening a second instance of your app will fail to connect.

### Background throttling
Background tabs get reduced HID report frequency. Keep the tab in the foreground or run as a standalone PWA window.

### No system-level actions
The browser cannot launch applications, run shell commands, or interact with the OS beyond browser APIs. A companion Node.js server is needed for system integration.

### User gesture for initial pairing
The very first connection requires a button click. Subsequent page loads can auto-reconnect silently via `getStreamDecks()`.

### Image size constraints
Each key image must match the exact pixel dimensions of the device model (e.g., 96x96 for XL). The library handles format conversion but the source must be correctly sized.

---

## 12. Stream Deck XL HID Protocol Summary

For context on what the WebHID library abstracts:

| Report Type | Report ID | Direction | Purpose |
|---|---|---|---|
| Input | 0x01 | Device to Host | Key state (1 byte per key, 0x00=up, 0x01=down) |
| Output | varies | Host to Device | JPEG image chunks (1024-byte packets) |
| Feature (set) | 0x03 | Host to Device | Brightness (cmd 0x08), show logo (cmd 0x02), fill color (cmd 0x06) |
| Feature (get) | 0x04-0x0A | Bidirectional | Firmware version, serial number |

Image transfer splits a JPEG into chunks with an 8-byte header per packet. The final chunk sets a completion flag. The library handles all framing, chunking, and header construction internally.

---

## Sources

1. [Chrome Developers: Connect to uncommon HID devices](https://developer.chrome.com/docs/capabilities/hid)
2. [MDN: WebHID API](https://developer.mozilla.org/en-US/docs/Web/API/WebHID_API)
3. [MDN: HID.requestDevice()](https://developer.mozilla.org/en-US/docs/Web/API/HID/requestDevice)
4. [MDN: HID.getDevices()](https://developer.mozilla.org/en-US/docs/Web/API/HID/getDevices)
5. [WICG WebHID Specification](https://wicg.github.io/webhid/)
6. [Julusian/node-elgato-stream-deck (GitHub)](https://github.com/Julusian/node-elgato-stream-deck)
7. [Julusian WebHID Demo](https://julusian.github.io/node-elgato-stream-deck/)
8. [@elgato-stream-deck/webhid (npm)](https://www.npmjs.com/package/@elgato-stream-deck/webhid)
9. [Jitsi: Custom meeting controls with Stream Deck and WebHID](https://jitsi.org/blog/custom-meeting-controls-with-elgato-stream-deck-and-webhid/)
10. [bramus/webhid-elgato-stream-deck-daft-punk-soundboard (GitHub)](https://github.com/bramus/webhid-elgato-stream-deck-daft-punk-soundboard)
11. [dysbulic/stream-deck-webhid-driver (GitHub)](https://github.com/dysbulic/stream-deck-webhid-driver)
12. [WebHID Browser Support and Limitations (TestMu AI)](https://www.testmuai.com/learning-hub/webhid-browser-support/)
13. [Elgato Stream Deck HID API: Introduction](https://docs.elgato.com/streamdeck/hid/intro/)
14. [Elgato Stream Deck HID API: General Reference](https://docs.elgato.com/streamdeck/hid/general/)
15. [Stream Deck XL USB Product ID (DeviceHunt)](https://devicehunt.com/view/type/usb/vendor/0FD9/device/006C)
16. [Reverse Engineering The Stream Deck (Den Delimarsky)](https://den.dev/blog/reverse-engineering-stream-deck/)
17. [awesome-webhid (GitHub)](https://github.com/robatwilliams/awesome-webhid)
18. [W3C Secure Contexts Specification](https://www.w3.org/TR/secure-contexts/)
19. [Chrome Extensions: Use WebHID](https://developer.chrome.com/docs/extensions/how-to/web-platform/webhid)
20. [WICG WebHID Security and Privacy Questionnaire](https://github.com/WICG/webhid/blob/gh-pages/security-and-privacy-questionnaire.md)
