# Node.js Ecosystem

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Open-Source Libraries

---

## Key Findings

- **`@elgato-stream-deck/node` (by Julusian) is the definitive Node.js library** for direct USB HID control of Stream Deck hardware. It is actively maintained (v7.6.2, last published March 2026), written in TypeScript, and supports every Stream Deck model including the XL.
- The library uses **`node-hid` (v3.x)** under the hood for USB HID communication. Prebuilt native binaries ship for all major platforms, so a compiler toolchain is usually unnecessary.
- The project is a **monorepo with four packages**: `@elgato-stream-deck/node` (Node.js), `@elgato-stream-deck/webhid` (browser), `@elgato-stream-deck/tcp` (network docks), and `@elgato-stream-deck/core` (platform-agnostic internals).
- **TypeScript is native** -- 82.8% of the codebase is TypeScript. Full type definitions ship with the package (`dist/index.d.ts`).
- The API is **fully async/Promise-based** with an EventEmitter pattern for key presses, encoder rotations, and LCD touch events.
- **No viable alternatives exist.** The legacy `elgato-stream-deck` package is deprecated in favor of this one. `@stream-deck-for-node/sdk` is abandoned. This is the only actively maintained direct-control library in the Node.js ecosystem.
- **Hot-plug detection is not built in** -- you must poll `listStreamDecks()` or use a separate USB monitoring library. The library handles device opening and error events, but not USB connect/disconnect notifications.
- The optional **`@julusian/jpeg-turbo`** package dramatically improves image write performance on the XL and Original-v2 models.

---

## 1. Library Overview: `@elgato-stream-deck/node`

| Field | Value |
|---|---|
| Package | `@elgato-stream-deck/node` |
| Version | 7.6.2 |
| Published | 2026-03-29 |
| Author | Julian Waller (Julusian) |
| License | MIT |
| Repository | [github.com/Julusian/node-elgato-stream-deck](https://github.com/Julusian/node-elgato-stream-deck) |
| GitHub Stars | ~197 |
| Commits | 560+ |
| Node.js Requirement | >= 18.18 |
| Language | TypeScript (82.8%) |

This library talks directly to Stream Deck hardware over USB HID. It has **nothing to do with Elgato's official Stream Deck software or plugin SDK** (`@elgato/streamdeck`). It is a low-level device driver that lets you build your own applications from scratch.

### What It Is Not

- It is not a plugin for the Elgato Stream Deck app
- It does not require the Elgato Stream Deck app to be running
- It provides no GUI -- it is a library for developers

---

## 2. Monorepo Package Architecture

The repository is organized as a monorepo containing six packages:

| Package | Purpose | Dependencies |
|---|---|---|
| `@elgato-stream-deck/core` | Platform-agnostic protocol logic, types, device models | `eventemitter3`, `tslib` |
| `@elgato-stream-deck/node-lib` | Shared Node.js utilities (JPEG encoding) | `jpeg-js`, `tslib` |
| `@elgato-stream-deck/node` | **Main package for Node.js apps** | `core`, `node-lib`, `node-hid`, `eventemitter3`, `p-queue`, `tslib` |
| `@elgato-stream-deck/webhid` | Browser-based control via WebHID API | `core`, `@types/w3c-web-hid`, `eventemitter3`, `p-queue`, `tslib` |
| `@elgato-stream-deck/tcp` | Network dock support over TCP | `core`, `node-lib`, `@julusian/bonjour-service`, `tslib` |
| `webhid-demo` | Live browser demo (not published to npm) | -- |

The architecture is layered: `core` provides all the Stream Deck protocol logic. The `node`, `webhid`, and `tcp` packages are thin wrappers that provide the HID transport. You can even build your own transport by implementing the HID interface against `core` directly.

---

## 3. USB HID Backend

### node-hid

The library uses **`node-hid` v3.x** as its USB HID backend.

| Field | Value |
|---|---|
| Package | `node-hid` |
| Version Used | ^3.2.0 |
| Underlying C Library | hidapi (cross-platform HID library) |
| API Style | NAPI-based, supports both sync and async |
| Prebuilt Platforms | Windows x64, macOS (Intel + ARM), Linux x64, Linux ARM/ARM64 |

Key characteristics of `node-hid`:
- Ships prebuilt native binaries for common platforms via `prebuild`
- Based on NAPI, so it works across Node.js versions without recompilation
- Since v3.0.0, supports both synchronous and asynchronous APIs (the async API is recommended)
- On unsupported platforms (e.g., Raspberry Pi), it falls back to compiling from source

### Native Compilation Requirements

Prebuilt binaries cover most cases, but if you need to compile from source:

- **macOS**: `xcode-select --install`
- **Windows**: `npm install --global windows-build-tools`
- **Linux / Raspberry Pi**: Standard build essentials (`sudo apt-get install build-essential`)

### Linux udev Rules

On Linux, USB HID devices are blocked by default. You must install udev rules:

```bash
# For headless/server environments:
sudo cp 50-elgato-stream-deck-headless.rules /etc/udev/rules.d/

# For desktop environments:
sudo cp 50-elgato-stream-deck-user.rules /etc/udev/rules.d/

# Reload rules:
sudo udevadm control --reload-rules

# Unplug and re-plug the device
```

The rules files ship with the npm package.

---

## 4. Supported Devices

The library supports 16 device model IDs as of v7.6.2:

```typescript
enum DeviceModelId {
  ORIGINAL = 'original',          // Stream Deck (v1, 15 keys)
  ORIGINALV2 = 'originalv2',      // Stream Deck (v2, 15 keys)
  ORIGINALMK2 = 'original-mk2',   // Stream Deck MK.2 (15 keys)
  ORIGINALMK2SCISSOR = 'original-mk2-scissor',
  MINI = 'mini',                   // Stream Deck Mini (6 keys)
  XL = 'xl',                      // Stream Deck XL (32 keys)
  PEDAL = 'pedal',                // Stream Deck Pedal (3 foot switches)
  PLUS = 'plus',                  // Stream Deck + (8 keys + 4 encoders + LCD)
  PLUS_XL = 'plus-xl',            // Stream Deck + XL (new, 2026)
  NEO = 'neo',                    // Stream Deck Neo (8 keys + touch strip)
  STUDIO = 'studio',              // Stream Deck Studio (encoders + screens)
  MODULE6 = '6-module',           // 6-key modular unit
  MODULE15 = '15-module',         // 15-key modular unit
  MODULE32 = '32-module',         // 32-key modular unit
  NETWORK_DOCK = 'network-dock',  // Network dock for modular units
  GALLEON_K100 = 'galleon-k100',  // Corsair Galleon K100 SD
}
```

The Stream Deck XL (`DeviceModelId.XL`) is a 32-key model with 96x96 pixel LCD keys arranged in a 4x8 grid.

Vendor IDs:
- Elgato: `0x0fd9` (4057)
- Corsair: `0x1b1c` (6940, for the Galleon K100)

---

## 5. Installation

### Basic Installation

```bash
pnpm add @elgato-stream-deck/node
```

### With Performance Optimization (Recommended for XL)

```bash
pnpm add @elgato-stream-deck/node @julusian/jpeg-turbo@^2.0.0
```

The `@julusian/jpeg-turbo` package provides native libjpeg-turbo bindings that are dramatically faster than the pure-JS `jpeg-js` fallback. This matters most for the XL (32 keys at 96x96 pixels) and Original-v2, where image encoding is the bottleneck. Without it, image transfers are "noticeably more CPU intensive and slower."

### Full Dependency Tree

```
@elgato-stream-deck/node@7.6.2
  +-- @elgato-stream-deck/core@7.6.2
  |     +-- eventemitter3@^5.0.4
  |     +-- tslib@^2.8.1
  +-- @elgato-stream-deck/node-lib@7.6.2
  |     +-- jpeg-js@^0.4.4       (pure-JS JPEG encoder, fallback)
  |     +-- tslib@^2.8.1
  +-- node-hid@^3.2.0            (native USB HID, prebuilt binaries)
  +-- eventemitter3@^5.0.4
  +-- p-queue@^6.6.2             (promise queue for serial writes)
  +-- tslib@^2.8.1

Optional:
  +-- @julusian/jpeg-turbo@^2.0.0  (native JPEG encoder, much faster)
```

---

## 6. Core API Reference

### Top-Level Functions

```typescript
import { listStreamDecks, openStreamDeck, getStreamDeckInfo } from '@elgato-stream-deck/node'

// Scan for all connected Stream Deck devices
function listStreamDecks(): Promise<StreamDeckDeviceInfo[]>

// Open a specific device by path
function openStreamDeck(
  devicePath: string,
  userOptions?: OpenStreamDeckOptionsNode
): Promise<StreamDeck>

// Get info about a specific device without opening it
function getStreamDeckInfo(path: string): Promise<StreamDeckDeviceInfo | undefined>
```

### OpenStreamDeckOptionsNode

```typescript
interface OpenStreamDeckOptionsNode extends OpenStreamDeckOptions {
  jpegOptions?: JPEGEncodeOptions
  resetToLogoOnClose?: boolean  // Reset to Elgato logo when closing
}
```

### StreamDeck Interface -- Complete API Surface

The `StreamDeck` interface extends `EventEmitter<StreamDeckEvents>` and provides:

#### Properties

```typescript
interface StreamDeck {
  readonly CONTROLS: StreamDeckControlDefinition[]  // All inputs/outputs
  readonly MODEL: DeviceModelId                      // e.g., 'xl'
  readonly PRODUCT_NAME: string                      // e.g., 'Stream Deck XL'
  readonly HAS_NFC_READER: boolean                   // NFC capability
}
```

#### Key Display Methods

```typescript
// Fill a single key with a solid RGB color
fillKeyColor(keyIndex: KeyIndex, r: number, g: number, b: number): Promise<void>

// Fill a single key with a raw pixel buffer (RGB/RGBA/BGR/BGRA)
fillKeyBuffer(keyIndex: KeyIndex, buffer: Buffer, options?: FillImageOptions): Promise<void>

// Fill all keys with a single large image buffer
fillPanelBuffer(buffer: Buffer, options?: FillPanelOptions): Promise<void>

// Pre-encode an image for repeated use (avoids re-encoding JPEG each time)
prepareFillKeyBuffer(keyIndex: KeyIndex, buffer: Buffer, options?: FillImageOptions): Promise<PreparedBuffer>
prepareFillPanelBuffer(buffer: Buffer, options?: FillPanelOptions): Promise<PreparedBuffer[]>

// Send a pre-encoded buffer
sendPreparedBuffer(prepared: PreparedBuffer): Promise<void>
```

#### LCD Methods (for Plus, Neo, Studio models)

```typescript
// Fill the entire LCD strip
fillLcd(buffer: Buffer, options?: FillLcdImageOptions): Promise<void>

// Fill a region of the LCD
fillLcdRegion(
  lcdIndex: number,
  x: number, y: number,
  buffer: Buffer,
  options: FillLcdImageOptions
): Promise<void>

// Pre-encode LCD region data
prepareFillLcdRegion(
  lcdIndex: number,
  x: number, y: number,
  buffer: Buffer,
  options: FillLcdImageOptions
): Promise<PreparedBuffer[]>

// Clear the LCD segment
clearLcdSegment(lcdIndex: number): Promise<void>
```

#### Clearing Methods

```typescript
// Clear a single key (set to black)
clearKey(keyIndex: KeyIndex): Promise<void>

// Clear all keys
clearPanel(): Promise<void>
```

#### Encoder Methods (for Plus, Studio models)

```typescript
// Set encoder LED to a solid color
setEncoderColor(encoderIndex: number, r: number, g: number, b: number): Promise<void>

// Set encoder ring to a single color
setEncoderRingSingleColor(encoderIndex: number, r: number, g: number, b: number): Promise<void>

// Set each LED in the encoder ring individually
setEncoderRingColors(encoderIndex: number, colors: Buffer): Promise<void>
```

#### System Methods

```typescript
// Set panel brightness (0-100)
setBrightness(brightness: number): Promise<void>

// Reset all keys to the Elgato logo
resetToLogo(): Promise<void>

// Close the device connection
close(): Promise<void>

// Get firmware version
getFirmwareVersion(): Promise<string>
getAllFirmwareVersions(): Promise<Record<string, string>>

// Get serial number
getSerialNumber(): Promise<string>

// Get underlying HID device info
getHidDeviceInfo(): HIDDeviceInfo

// Get child device info (for network docks)
getChildDeviceInfo(): Promise<StreamDeckTcpChildDeviceInfo[]>

// Calculate pixel dimensions for fillPanelBuffer
calculateFillPanelDimensions(options?: FillPanelDimensionsOptions): Dimension
```

### Events

```typescript
interface StreamDeckEvents {
  // Button pressed
  down: (keyIndex: KeyIndex) => void

  // Button released
  up: (keyIndex: KeyIndex) => void

  // Encoder rotated (for Plus/Studio models)
  rotate: (encoderIndex: EncoderIndex, amount: number) => void

  // LCD short press (for Plus/Studio models)
  lcdShortPress: (position: LcdPosition) => void

  // LCD long press
  lcdLongPress: (position: LcdPosition) => void

  // LCD swipe gesture
  lcdSwipe: (from: LcdPosition, to: LcdPosition) => void

  // NFC tag read
  nfcRead: (tagId: string) => void

  // Device error
  error: (error: Error) => void
}
```

### CONTROLS Property (v7+ Architecture)

In v7, device capabilities are described by the `CONTROLS` array. Each entry describes a physical control:

```typescript
// Button with LCD feedback (most Stream Deck keys)
interface StreamDeckButtonControlDefinitionLcdFeedback {
  type: 'button'
  row: number
  column: number
  index: number        // Logical key index for API calls
  hidIndex: number     // Raw HID index
  feedbackType: 'lcd'
  pixelSize: { width: number; height: number }
}

// Button with RGB LED only (e.g., Neo)
interface StreamDeckButtonControlDefinitionRgbFeedback {
  type: 'button'
  feedbackType: 'rgb'
  // ... same positional fields
}

// Button with no feedback (e.g., Pedal)
interface StreamDeckButtonControlDefinitionNoFeedback {
  type: 'button'
  feedbackType: 'none'
}

// Rotary encoder (Plus, Studio)
interface StreamDeckEncoderControlDefinition {
  type: 'encoder'
  index: number
  hidIndex: number
  hasLed: boolean
  ledRingSteps: number
  lcdRingOffset?: number
}

// LCD touchscreen segment (Plus, Studio)
interface StreamDeckLcdSegmentControlDefinition {
  type: 'lcd-segment'
  id: 0
  columnSpan: number
  rowSpan: number
  pixelSize: { width: number; height: number }
  drawRegions: boolean
}
```

---

## 7. Code Examples

### Basic: Open Device and Listen for Key Presses

```typescript
import { openStreamDeck, listStreamDecks } from '@elgato-stream-deck/node'

async function main() {
  // Discover connected devices
  const devices = await listStreamDecks()
  if (devices.length === 0) {
    throw new Error('No Stream Decks found!')
  }

  console.log(`Found ${devices.length} device(s):`)
  for (const dev of devices) {
    console.log(`  ${dev.path} - ${dev.model}`)
  }

  // Open the first device
  const deck = await openStreamDeck(devices[0].path)

  console.log(`Opened: ${deck.PRODUCT_NAME}`)
  console.log(`Model: ${deck.MODEL}`)
  console.log(`Serial: ${await deck.getSerialNumber()}`)
  console.log(`Firmware: ${await deck.getFirmwareVersion()}`)
  console.log(`Controls: ${deck.CONTROLS.length}`)

  // Listen for key presses
  deck.on('down', (keyIndex) => {
    console.log(`Key ${keyIndex} pressed`)
  })

  deck.on('up', (keyIndex) => {
    console.log(`Key ${keyIndex} released`)
  })

  deck.on('error', (error) => {
    console.error('Stream Deck error:', error)
  })
}

main().catch(console.error)
```

### Setting Key Colors

```typescript
import { openStreamDeck, listStreamDecks } from '@elgato-stream-deck/node'

async function main() {
  const devices = await listStreamDecks()
  const deck = await openStreamDeck(devices[0].path)

  // Set key 0 to red
  await deck.fillKeyColor(0, 255, 0, 0)

  // Set key 1 to green
  await deck.fillKeyColor(1, 0, 255, 0)

  // Set key 2 to blue
  await deck.fillKeyColor(2, 0, 0, 255)

  // Set brightness to 80%
  await deck.setBrightness(80)

  // Clear all keys after 5 seconds
  setTimeout(async () => {
    await deck.clearPanel()
  }, 5000)
}

main().catch(console.error)
```

### Drawing Images with Sharp

```typescript
import { openStreamDeck, listStreamDecks } from '@elgato-stream-deck/node'
import sharp from 'sharp'

async function main() {
  const devices = await listStreamDecks()
  const deck = await openStreamDeck(devices[0].path)

  // Find the pixel size for buttons on this device
  const buttonControl = deck.CONTROLS.find(
    (c) => c.type === 'button' && c.feedbackType === 'lcd'
  )
  if (!buttonControl || buttonControl.feedbackType !== 'lcd') {
    throw new Error('No LCD buttons found')
  }

  const { width, height } = buttonControl.pixelSize
  // Stream Deck XL: 96x96 pixels per key

  // Resize and convert an image to raw RGB buffer
  const imageBuffer = await sharp('icon.png')
    .resize(width, height)
    .flatten()          // Remove alpha channel
    .raw()              // Output raw pixel data
    .toBuffer()

  // Write to key 0
  await deck.fillKeyBuffer(0, imageBuffer, { format: 'rgb' })

  // You can also use RGBA
  const rgbaBuffer = await sharp('icon.png')
    .resize(width, height)
    .ensureAlpha()
    .raw()
    .toBuffer()

  await deck.fillKeyBuffer(1, rgbaBuffer, { format: 'rgba' })
}

main().catch(console.error)
```

### Full Panel Image (Spanning All Keys)

```typescript
import { openStreamDeck, listStreamDecks } from '@elgato-stream-deck/node'
import sharp from 'sharp'

async function main() {
  const devices = await listStreamDecks()
  const deck = await openStreamDeck(devices[0].path)

  // Calculate the full panel pixel dimensions
  const panelDimensions = deck.calculateFillPanelDimensions()
  // Stream Deck XL: 8 cols x 4 rows of 96x96 = 768x384 pixels

  const panelBuffer = await sharp('wallpaper.png')
    .resize(panelDimensions.width, panelDimensions.height)
    .flatten()
    .raw()
    .toBuffer()

  await deck.fillPanelBuffer(panelBuffer, { format: 'rgb' })
}

main().catch(console.error)
```

### Interactive Key Handler with Toggle State

```typescript
import { openStreamDeck, listStreamDecks } from '@elgato-stream-deck/node'

async function main() {
  const devices = await listStreamDecks()
  const deck = await openStreamDeck(devices[0].path, {
    resetToLogoOnClose: true
  })

  // Track toggle state per key
  const keyStates = new Map<number, boolean>()

  deck.on('down', async (keyIndex) => {
    const isOn = keyStates.get(keyIndex) ?? false
    const newState = !isOn
    keyStates.set(keyIndex, newState)

    if (newState) {
      await deck.fillKeyColor(keyIndex, 0, 255, 0)   // Green = on
    } else {
      await deck.fillKeyColor(keyIndex, 50, 50, 50)   // Dim gray = off
    }
  })

  // Clean shutdown
  process.on('SIGINT', async () => {
    await deck.resetToLogo()
    await deck.close()
    process.exit(0)
  })
}

main().catch(console.error)
```

### Prepared Buffers for Performance

```typescript
import { openStreamDeck, listStreamDecks } from '@elgato-stream-deck/node'
import sharp from 'sharp'

async function main() {
  const devices = await listStreamDecks()
  const deck = await openStreamDeck(devices[0].path)

  const buttonControl = deck.CONTROLS.find(
    (c) => c.type === 'button' && c.feedbackType === 'lcd'
  )
  if (!buttonControl || buttonControl.feedbackType !== 'lcd') return

  const { width, height } = buttonControl.pixelSize

  // Pre-encode images for rapid switching (e.g., animation frames)
  const redBuffer = await sharp({
    create: { width, height, channels: 3, background: { r: 255, g: 0, b: 0 } }
  }).raw().toBuffer()

  const blueBuffer = await sharp({
    create: { width, height, channels: 3, background: { r: 0, g: 0, b: 255 } }
  }).raw().toBuffer()

  // Pre-encode once -- this does the expensive JPEG compression
  const preparedRed = await deck.prepareFillKeyBuffer(0, redBuffer, { format: 'rgb' })
  const preparedBlue = await deck.prepareFillKeyBuffer(0, blueBuffer, { format: 'rgb' })

  // Toggle rapidly -- sendPreparedBuffer is much faster than fillKeyBuffer
  let toggle = false
  setInterval(async () => {
    await deck.sendPreparedBuffer(toggle ? preparedRed : preparedBlue)
    toggle = !toggle
  }, 100)
}

main().catch(console.error)
```

---

## 8. TypeScript Support

TypeScript support is first-class:

- The entire codebase is written in TypeScript (82.8%)
- Type declarations ship at `dist/index.d.ts`
- All public APIs are fully typed, including events, options, and control definitions
- The `CONTROLS` array uses discriminated unions (`type: 'button' | 'encoder' | 'lcd-segment'` and `feedbackType: 'none' | 'rgb' | 'lcd'`), making it easy to narrow types with standard TypeScript patterns

```typescript
// Example: type narrowing with CONTROLS
for (const control of deck.CONTROLS) {
  switch (control.type) {
    case 'button':
      if (control.feedbackType === 'lcd') {
        console.log(`Button ${control.index}: ${control.pixelSize.width}x${control.pixelSize.height}`)
      }
      break
    case 'encoder':
      console.log(`Encoder ${control.index}: ${control.ledRingSteps} ring steps`)
      break
    case 'lcd-segment':
      console.log(`LCD: ${control.pixelSize.width}x${control.pixelSize.height}`)
      break
  }
}
```

---

## 9. Version History and Migration

### Major Versions

| Version | Date | Node.js | Key Changes |
|---|---|---|---|
| v7.0.0 | 2024-09-08 | >= 18 | Reworked CONTROLS system, Studio support, encoder events, breaking API restructuring |
| v7.3.0 | 2025-05-30 | >= 18 | Prepared buffers, network dock support |
| v7.5.0 | 2026-01-26 | >= 18.18 | Corsair Galleon K100, multiple vendor IDs |
| v7.6.0 | 2026-03-03 | >= 18.18 | Stream Deck + XL support |
| v7.6.2 | 2026-03-29 | >= 18.18 | Network dock fixes |

### v6 to v7 Breaking Changes

The v7 release was a major restructuring:

1. **CONTROLS property replaces individual capability flags.** Instead of `KEY_COLUMNS`, `KEY_ROWS`, `ICON_SIZE` properties, you now inspect the `CONTROLS` array to discover what a device can do.
2. **Event consolidation.** Encoder and LCD events are now part of a unified event system.
3. **Removed deprecated options.** Various legacy configuration options were dropped.
4. **Dropped Node.js 16 support.** Minimum is now Node.js 18.

### Legacy Package

The original `elgato-stream-deck` package (v4.3.0) is officially deprecated:

> "This library has been renamed to @elgato-stream-deck/node. The new version provides a promise based api, with better names as well as a webhid version at @elgato-stream-deck/webhid"

The old package used `node-hid` v2.x and had a synchronous, callback-based API. It should not be used for new projects.

---

## 10. Hot-Plug Detection

The library **does not include built-in hot-plug detection**. It provides:

- `listStreamDecks()` -- scans for currently connected devices at call time
- `error` event -- fires when a device disconnects or encounters a USB error

For hot-plug support, you have several options:

### Option A: Polling (Simplest)

```typescript
import { listStreamDecks } from '@elgato-stream-deck/node'

let knownPaths = new Set<string>()

setInterval(async () => {
  const devices = await listStreamDecks()
  const currentPaths = new Set(devices.map(d => d.path))

  // Detect new devices
  for (const path of currentPaths) {
    if (!knownPaths.has(path)) {
      console.log('New device connected:', path)
      // Open and configure the device
    }
  }

  // Detect removed devices
  for (const path of knownPaths) {
    if (!currentPaths.has(path)) {
      console.log('Device disconnected:', path)
      // Clean up
    }
  }

  knownPaths = currentPaths
}, 2000)
```

### Option B: node-usb (Event-Driven)

```typescript
import { usb } from 'usb'
import { VENDOR_ID, listStreamDecks, openStreamDeck } from '@elgato-stream-deck/node'

usb.on('attach', async (device) => {
  if (device.deviceDescriptor.idVendor === VENDOR_ID) {
    // Small delay for device to initialize
    setTimeout(async () => {
      const devices = await listStreamDecks()
      // Open newly connected device
    }, 1000)
  }
})

usb.on('detach', (device) => {
  if (device.deviceDescriptor.idVendor === VENDOR_ID) {
    console.log('Stream Deck disconnected')
  }
})
```

---

## 11. WebHID Variant: `@elgato-stream-deck/webhid`

The WebHID package provides the same API surface but runs in the browser using the WebHID API.

### Browser Requirements

- Chromium v89+ (Chrome, Edge, Opera, Brave)
- Firefox and Safari do **not** support WebHID
- Must be served over HTTPS (or localhost)
- Requires user gesture to request device access

### Installation

```bash
pnpm add @elgato-stream-deck/webhid
```

### API Differences from Node

```typescript
import { requestStreamDecks, getStreamDecks, openDevice } from '@elgato-stream-deck/webhid'

// Prompt user to select a device (requires user gesture like button click)
const decks = await requestStreamDecks()

// Re-open previously authorized devices (no prompt)
const decks = await getStreamDecks()

// Open a manually acquired HIDDevice
const deck = await openDevice(hidDevice)
```

Once opened, the `StreamDeckWeb` instance has the same API as the Node.js `StreamDeck` -- same events, same methods.

### WebHID Example

```typescript
// In a button click handler (requires user gesture):
document.getElementById('connect')!.addEventListener('click', async () => {
  const decks = await requestStreamDecks()
  if (decks.length === 0) return

  const deck = decks[0]
  console.log(`Connected: ${deck.PRODUCT_NAME}`)

  deck.on('down', (keyIndex) => {
    console.log(`Key ${keyIndex} pressed`)
  })

  deck.on('up', (keyIndex) => {
    console.log(`Key ${keyIndex} released`)
  })

  await deck.fillKeyColor(0, 255, 0, 0)
})
```

### WebHID Limitations

- The **original v1 15-key model is not supported on Linux** via WebHID
- Background tab throttling affects draw rates -- if your tab is not focused, updates slow down
- No filesystem access for loading images (must use fetch or canvas)
- The live demo is at [julusian.github.io/node-elgato-stream-deck](https://julusian.github.io/node-elgato-stream-deck/)

---

## 12. TCP Variant: `@elgato-stream-deck/tcp`

The TCP package connects to Stream Deck devices over the network via Elgato's network dock. This was added in v7.3.0 (May 2025).

```bash
pnpm add @elgato-stream-deck/tcp
```

It uses Bonjour/mDNS for device discovery (`@julusian/bonjour-service`) and provides the same `StreamDeck` interface over a TCP connection. The API documentation is still marked TODO in the official README, but TypeScript types are complete.

---

## 13. Alternative Libraries

### Dead / Deprecated Libraries

| Package | Status | Notes |
|---|---|---|
| `elgato-stream-deck` (v4.3.0) | **Deprecated** | Renamed to `@elgato-stream-deck/node`. Sync API, no TypeScript. |
| `@stream-deck-for-node/sdk` (v1.0.15) | **Abandoned** | "Package no longer supported." Used WebSocket, not USB HID. Last updated 3+ years ago. |
| `stream-deck-ts` (TimLuq) | **Unmaintained** | Minimal TypeScript wrapper. No recent activity. |

### Related But Different

| Package | Purpose |
|---|---|
| `@elgato/streamdeck` (v2.1.0) | Official Elgato plugin SDK. Requires the Elgato Stream Deck app. Communicates via WebSocket, not USB HID. For building plugins that run inside the official app, not standalone applications. |
| Bitfocus Companion | Full application (not a library) for broadcast control surfaces. Uses `@elgato-stream-deck/node` internally. 2,176 GitHub stars. |

### Verdict

**`@elgato-stream-deck/node` is the only viable option** for direct USB HID control from Node.js. There are no actively maintained alternatives. The ecosystem has consolidated around this single library.

---

## 14. Image Format Reference

The Stream Deck XL keys are 96x96 pixels each. The library accepts raw pixel buffers in these formats:

| Format | Bytes per Pixel | Description |
|---|---|---|
| `'rgb'` | 3 | Red, Green, Blue |
| `'rgba'` | 4 | Red, Green, Blue, Alpha |
| `'bgr'` | 3 | Blue, Green, Red (native format for some models) |
| `'bgra'` | 4 | Blue, Green, Red, Alpha |

For a single XL key: `96 * 96 * 3 = 27,648 bytes` (RGB) or `96 * 96 * 4 = 36,864 bytes` (RGBA).

For the full XL panel: `768 * 384 * 3 = 884,736 bytes` (RGB).

The library internally converts your pixel buffer to JPEG for transmission to the device. This is where `@julusian/jpeg-turbo` makes a significant difference -- native libjpeg-turbo vs pure-JS jpeg-js.

---

## 15. Practical Recommendations for Stream Deck XL Projects

1. **Always install `@julusian/jpeg-turbo`.** The XL has 32 keys, and the pure-JS JPEG encoder becomes a real bottleneck when updating multiple keys rapidly.

2. **Use prepared buffers for animations or frequent updates.** The `prepareFillKeyBuffer()` / `sendPreparedBuffer()` pattern pre-encodes the JPEG once, so repeated sends skip the encoding step entirely.

3. **Use `sharp` for image processing.** It is the fastest Node.js image library and produces raw pixel buffers in exactly the format the Stream Deck API expects.

4. **Handle the `error` event.** USB disconnects emit errors. Without a handler, they crash your process via unhandled EventEmitter exceptions.

5. **Use `resetToLogoOnClose: true`.** This restores the Elgato logo when your app exits, leaving the device in a clean state.

6. **Inspect `CONTROLS` for device-agnostic code.** Do not hard-code key counts. The `CONTROLS` array tells you exactly what the connected device supports.

7. **Poll for hot-plug if needed.** A 2-second `listStreamDecks()` polling interval is reasonable for detecting connect/disconnect.

---

## Sources

1. [Julusian/node-elgato-stream-deck -- GitHub Repository](https://github.com/Julusian/node-elgato-stream-deck)
2. [@elgato-stream-deck/node -- npm](https://www.npmjs.com/package/@elgato-stream-deck/node)
3. [@elgato-stream-deck/webhid -- npm](https://www.npmjs.com/package/@elgato-stream-deck/webhid)
4. [@elgato-stream-deck/tcp -- npm](https://www.npmjs.com/package/@elgato-stream-deck/tcp)
5. [@elgato-stream-deck/core -- npm](https://www.npmjs.com/package/@elgato-stream-deck/core)
6. [node-elgato-stream-deck Releases](https://github.com/Julusian/node-elgato-stream-deck/releases)
7. [node-elgato-stream-deck CHANGELOG](https://github.com/Julusian/node-elgato-stream-deck/blob/main/CHANGELOG.md)
8. [node-hid -- GitHub Repository](https://github.com/node-hid/node-hid)
9. [@julusian/jpeg-turbo -- npm](https://www.npmjs.com/package/@julusian/jpeg-turbo)
10. [elgato-stream-deck (legacy) -- npm](https://www.npmjs.com/package/elgato-stream-deck)
11. [@stream-deck-for-node/sdk -- npm](https://www.npmjs.com/package/@stream-deck-for-node/sdk)
12. [Bitfocus Companion -- GitHub](https://github.com/bitfocus/companion)
13. [WebHID Live Demo](https://julusian.github.io/node-elgato-stream-deck/)
14. [Jitsi WebHID Stream Deck Blog Post](https://jitsi.org/blog/custom-meeting-controls-with-elgato-stream-deck-and-webhid/)
15. [DeepWiki: Companion Stream Deck Supported Hardware](https://deepwiki.com/bitfocus/companion-surface-elgato-stream-deck/1.2-supported-hardware)
