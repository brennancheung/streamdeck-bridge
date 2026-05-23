# Architecture Patterns

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Full Applications & Frameworks

---

## Key Findings

- The dominant pattern for custom Stream Deck software is a **layered architecture** with a HID transport layer, a core device abstraction, and application logic on top. The Julusian `@elgato-stream-deck` monorepo exemplifies this with its `core` / `node` / `webhid` package split.
- **Bitfocus Companion** is the most architecturally mature open-source Stream Deck application. It uses process-isolated plugin modules, a worker-pool rendering pipeline, dual-level image caching, and tRPC over WebSocket for frontend-backend communication.
- Button state should be managed **declaratively** with a reactive data flow: state changes invalidate cached renders, which triggers re-rendering through a worker pool and pushes updated images to the device. Imperative `fillKeyColor` calls are fine for prototyping but do not scale.
- The **rendering pipeline** in production apps follows: state change -> variable substitution -> canvas draw (worker thread) -> PNG/JPEG encode -> cache -> device transfer. Companion uses `@napi-rs/canvas` in worker threads with LRU caching and debounced invalidation.
- Configuration storage splits into two camps: **JSON files** for simplicity and API compatibility, and **SQLite** for complex multi-device, multi-profile setups. Companion migrated to SQLite; simpler apps like deckxstream use `~/.deckxstream.json`.
- Device hot-plug detection should use the `usb` library's OS-native attach/detach events, not HID device polling (which is expensive and blocks the event loop).
- The official Elgato plugin architecture uses **WebSocket IPC** between the Stream Deck host app and plugins, with each plugin getting a dedicated WebSocket connection. Custom apps that separate UI from device control typically follow the same pattern.
- For button interactions beyond simple press/release, implement timing-based gesture detection in userland: track press timestamps and use configurable thresholds for long-press (e.g., 500ms) and double-press (e.g., 300ms between presses).

---

## 1. Reference Architectures from Existing Projects

### 1.1 Julusian `@elgato-stream-deck` -- The Foundation Library

This is the most widely used Node.js library for direct Stream Deck hardware control. It is a monorepo with a clean layered architecture:

```
@elgato-stream-deck/core     -- Platform-agnostic device logic
@elgato-stream-deck/node     -- Node.js HID binding (via node-hid)
@elgato-stream-deck/webhid   -- Browser WebHID binding
@elgato-stream-deck/tcp      -- TCP socket transport
```

**Key design decision:** The core package contains all Stream Deck protocol logic (button grid layout, image encoding, report chunking), while platform packages are thin wrappers that provide a HID read/write implementation. You can plug in your own HID backend and reuse all the protocol handling.

**API surface:**

```typescript
// Device discovery
listStreamDecks(): Promise<StreamDeckDeviceInfo[]>
openStreamDeck(path: string, options?: OpenStreamDeckOptionsNode): Promise<StreamDeck>

// Image operations
fillKeyColor(keyIndex: number, r: number, g: number, b: number): Promise<void>
fillKeyBuffer(keyIndex: number, buffer: Buffer, options?: { format: 'rgb' | 'rgba' | 'bgr' | 'bgra' }): Promise<void>
fillPanelBuffer(buffer: Buffer, options?: object): Promise<void>
clearKey(keyIndex: number): Promise<void>
clearPanel(): Promise<void>

// Device control
setBrightness(percentage: number): Promise<void>
resetToLogo(): Promise<void>
getSerialNumber(): Promise<string>
getFirmwareVersion(): Promise<string>

// Events (EventEmitter pattern)
on('down', (keyIndex: number) => void)
on('up', (keyIndex: number) => void)
on('rotate', (encoderIndex: number, amount: number) => void)
on('error', (error: Error) => void)
```

This library is a device driver, not an application framework. It gives you raw hardware access and nothing else -- no state management, no pages, no configuration. Any custom app must build those layers on top.

### 1.2 Bitfocus Companion -- Production-Grade Application

Companion is a full application supporting 700+ external devices (ATEM switchers, OBS, etc.) with Stream Deck as the control surface. Its architecture is the best reference for building a serious custom app.

**Component structure (Yarn workspaces monorepo):**

| Component | Technology | Role |
|-----------|-----------|------|
| `companion` (core) | Node.js, Express, tRPC, SQLite | Business logic, device management, module lifecycle |
| `webui` | React, MobX, TanStack Router, tRPC | Browser-based configuration UI |
| `launcher` | Electron | Desktop wrapper, system tray, process management |
| `shared-lib` | TypeScript | Shared types and utilities |

**Key architectural patterns:**

1. **Process isolation for plugins:** Each module (plugin) runs in its own child process with JSON-RPC over IPC. Crash isolation prevents a bad plugin from taking down the whole app. Node.js runtime version is configurable per module.

2. **Permission sandboxing:** Module processes launch with `--allow-fs-read` and `--allow-net` flags derived from manifest declarations.

3. **Surface abstraction:** All control surfaces (USB HID, network satellite, web emulator) implement the `SurfacePanel` interface. The `SurfaceController` manages the registry and lifecycle of all surfaces.

4. **Device grouping:** `SurfaceGroup` coordinates multiple physical devices with shared page navigation, history, and synchronized lock states.

### 1.3 streamdeck-ui-node -- Lightweight Application Framework

This library by mrfigg sits between the raw device driver and a full app. It provides:

- **Pages** as logical containers for button layouts, with `onFocus`/`onBlur` lifecycle hooks
- **Keys** with built-in image rendering (via sharp), animation support (GIF, WebP, AVIF), and interaction events (`click`, `hold`, `held`)
- **Instance-based state** where custom properties attach directly to key/page objects

```typescript
const mainPage = streamDeck.createPage({ brightness: 80 })
const key = streamDeck.createKey({
  image: 'icon.png',
  attachToPages: [{ page: mainPage, row: 1, column: 1 }],
  onCreate() { this.intervalId = setInterval(() => this.update(), 1000) },
  onDestroy() { clearInterval(this.intervalId) },
  onDown() { /* pressed */ },
  onClick() { /* click (not hold) */ },
  onHold() { /* long press */ },
})
```

This is a good reference for the page/key abstraction layer, though it lacks multi-device support and networked configuration.

### 1.4 deckxstream -- Configuration-Driven Controller

A Linux-focused controller that reads from `~/.deckxstream.json` and supports:

- Multiple pages with dynamic buttons (content generated by shell commands)
- Hotkeys, text input (via libxdo), and application launching
- SVG, PNG, and animated GIF icons
- Sticky buttons that persist across page switches

This demonstrates a purely declarative, config-file-driven approach -- no GUI, no plugin system.

---

## 2. Application Layering Pattern

Based on the reference projects, the recommended architecture for a custom Node.js/TypeScript Stream Deck app follows five layers:

```
+----------------------------------------------------------+
|  Layer 5: Configuration UI                                |
|  (React web app, Electron wrapper, or CLI)                |
+----------------------------------------------------------+
|  Layer 4: Application Logic                               |
|  (Actions, integrations, macros, automations)             |
+----------------------------------------------------------+
|  Layer 3: State & Rendering Engine                        |
|  (Button state, pages/profiles, image pipeline, cache)    |
+----------------------------------------------------------+
|  Layer 2: Device Abstraction                              |
|  (Multi-device registry, hot-plug, surface interface)     |
+----------------------------------------------------------+
|  Layer 1: HID Transport                                   |
|  (@elgato-stream-deck/node or custom HID implementation)  |
+----------------------------------------------------------+
```

### Layer 1: HID Transport

Use `@elgato-stream-deck/node` directly. It handles:
- HID report construction and parsing
- Image format conversion (JPEG for most devices, BMP for Mini)
- Report chunking for large image transfers
- Device-specific protocol quirks (rotation, key ordering)

Do not reimplement the HID protocol unless you need to support non-Elgato hardware.

### Layer 2: Device Abstraction

This layer manages the lifecycle of all connected devices:

```typescript
interface DeviceManager {
  listDevices(): DeviceInfo[]
  getDevice(serialNumber: string): StreamDeckDevice | undefined
  onDeviceConnected(callback: (device: StreamDeckDevice) => void): void
  onDeviceDisconnected(callback: (serialNumber: string) => void): void
}
```

Responsibilities: discovery, hot-plug handling, device identity tracking by serial number, and providing a uniform interface regardless of device model (Mini, MK.2, XL, Plus, Neo).

### Layer 3: State & Rendering Engine

The core of any non-trivial Stream Deck app. Manages what should be displayed on each button and converts that intent into device-ready images. See sections 3 and 4 below for details.

### Layer 4: Application Logic

Actions triggered by button presses: launching apps, sending HTTP requests, toggling smart home devices, controlling OBS scenes, executing shell commands, sending keystrokes. This is where plugins/modules live.

### Layer 5: Configuration UI

A web-based UI served over HTTP/WebSocket, optionally wrapped in Electron or Tauri for desktop distribution. The UI never talks to USB directly -- it communicates with the backend via WebSocket or tRPC.

---

## 3. Button State Management

### 3.1 Declarative vs. Imperative

**Imperative** (direct device calls):

```typescript
// Simple but does not scale
await deck.fillKeyColor(0, 255, 0, 0)  // red
await deck.fillKeyColor(0, 0, 255, 0)  // green
```

**Declarative** (state-driven rendering):

```typescript
// State describes what should be displayed
const buttonState: ButtonState = {
  icon: 'microphone',
  label: 'Mute',
  backgroundColor: '#ff0000',
  active: true,
}

// Renderer converts state to device image
const image = await renderer.render(buttonState)
await deck.fillKeyBuffer(0, image)
```

The declarative approach wins for any app with more than a handful of buttons because:
- State is serializable (save/restore configurations)
- State changes trigger re-renders automatically
- Multiple devices can display the same state
- Undo/redo becomes trivial

### 3.2 Pages, Profiles, and Layers

The organizational hierarchy used by most Stream Deck software:

```
Profile (per-application or user-defined context)
  └── Page (a full grid of buttons, up to 10 per profile in official app)
       └── Button (individual key state)
            ├── Visual: icon, label, background color
            ├── Behavior: action on press, long-press, double-press
            └── Feedback: dynamic visual updates from external state
```

**Pages** are the primary navigation unit. Companion implements page history with back/forward, page locking (prevent accidental navigation), and page cycling.

**Profiles** can be switched manually or automatically based on active application (the official Stream Deck app does this via OS-level window focus detection).

**Layers** are less common but useful for overlay patterns -- e.g., showing a "shift" layer when a modifier button is held, then reverting when released. Companion handles this with multi-step buttons that cycle through different action/visual states.

### 3.3 Reactive State Flow

The recommended pattern, drawn from Companion's architecture:

```
State Change (user action, external event, timer)
       │
       ▼
Variable Update (broadcast to all listeners)
       │
       ▼
Feedback Evaluation (which buttons are affected?)
       │
       ▼
Style Invalidation (mark affected buttons dirty)
       │
       ▼
Render Queue (debounced, batched)
       │
       ▼
Worker Thread Render (canvas draw + encode)
       │
       ▼
Cache Update (store result, emit button_drawn event)
       │
       ▼
Device Transfer (send image to hardware)
```

Key implementation details from Companion:
- Variable changes are **debounced** to prevent render storms from rapid updates
- Feedback evaluation uses **dirty checking** -- only re-evaluate feedbacks whose input variables actually changed
- The render queue uses an `ImageWriteQueue` with a max of 5 pending operations per location
- Rendered images are cached in an LRU cache (max 100 entries, 60-second TTL)

---

## 4. Rendering Pipeline

### 4.1 Image Requirements by Device

| Device Family | Image Format | Key Size (px) | Notes |
|--------------|-------------|---------------|-------|
| Stream Deck Mini | BMP | 80x80 | Only device family using BMP |
| Stream Deck MK.2 | JPEG | 72x72 | |
| Stream Deck XL | JPEG | 96x96 | 32 keys (4x8 grid) |
| Stream Deck Plus | JPEG | 120x120 | Also has LCD touchstrip |
| Stream Deck Neo | JPEG | 100x100 | Also has info strip |

The `@elgato-stream-deck/core` library handles format conversion internally, so your rendering pipeline should output raw pixel buffers and let the library handle encoding.

### 4.2 Rendering Stack Options

**Option A: `sharp` (recommended for most apps)**

```typescript
import sharp from 'sharp'

async function renderButton(icon: Buffer, label: string, size: number): Promise<Buffer> {
  const canvas = sharp({
    create: { width: size, height: size, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 1 } }
  })

  // Composite icon and text overlay
  return canvas
    .composite([
      { input: await sharp(icon).resize(size * 0.6, size * 0.6).toBuffer(), gravity: 'north' },
      { input: renderTextSVG(label, size), gravity: 'south' },
    ])
    .raw()
    .toBuffer()
}
```

Pros: Fast native bindings, excellent format support including animated GIF/WebP/AVIF. Used by streamdeck-ui-node.

**Option B: `@napi-rs/canvas` (recommended for complex layouts)**

```typescript
import { createCanvas } from '@napi-rs/canvas'

function renderButton(state: ButtonState, size: number): Buffer {
  const canvas = createCanvas(size, size)
  const ctx = canvas.getContext('2d')

  ctx.fillStyle = state.backgroundColor
  ctx.fillRect(0, 0, size, size)

  // Draw icon, text, indicators, progress bars, etc.
  // Full Canvas 2D API available

  return canvas.toBuffer('raw')
}
```

Pros: Full Canvas 2D API, complex text rendering, shapes, gradients. Used by Companion. Better for dynamic content like clocks, meters, multi-line text.

**Option C: Headless browser (Puppeteer/Playwright)**

Render HTML/CSS to screenshots. Maximum flexibility but significantly slower. Only viable if rendering is infrequent or heavily cached.

### 4.3 Worker Thread Architecture

For responsive applications, rendering must happen off the main thread. Companion's approach:

```typescript
// Main thread
const renderPool = new WorkerPool('./render-worker.js', {
  minThreads: 2,
  maxThreads: 6,
  maxRetries: 10,
})

async function requestRender(buttonId: string, state: ButtonState): Promise<Buffer> {
  const cached = cache.get(buttonId)
  if (cached && !isDirty(buttonId)) return cached

  const result = await renderPool.execute({ buttonId, state })
  cache.set(buttonId, result, { ttl: 60_000 })
  return result
}
```

```typescript
// render-worker.js (worker thread)
import { parentPort } from 'worker_threads'
import { createCanvas } from '@napi-rs/canvas'

parentPort.on('message', ({ buttonId, state }) => {
  const buffer = drawButton(state)
  parentPort.postMessage({ buttonId, buffer }, [buffer.buffer])
})
```

The worker pool should:
- Auto-recover crashed workers (Companion allows up to 10 retries, terminates after 30 crashes in 60 seconds)
- Transfer image buffers via `Transferable` to avoid copying
- Use a job queue to prevent flooding workers with redundant renders

---

## 5. IPC and Communication Patterns

### 5.1 Official Elgato SDK Pattern: WebSocket

The official Stream Deck plugin system uses WebSocket for all communication:

```
Stream Deck App ──WebSocket──> Plugin Backend (Node.js)
Stream Deck App ──WebSocket──> Property Inspector (Chromium UI)
```

Each plugin gets a dedicated WebSocket connection on a port assigned by the Stream Deck app. The protocol uses JSON messages for events (`keyDown`, `keyUp`, `dialRotate`, `willAppear`, `willDisappear`) and commands (`setImage`, `setTitle`, `setSettings`).

### 5.2 Companion Pattern: tRPC + IPC

Companion uses two communication channels:

1. **Frontend <-> Backend:** tRPC over HTTP and WebSocket. Type-safe RPC with automatic serialization. The React frontend uses MobX stores that sync with backend state via tRPC subscriptions.

2. **Core <-> Modules:** JSON-RPC over Node.js IPC (child process `stdio`). Messages include `init`, `updateConfig`, `runAction` (core -> module) and `setVariables`, `setStatus`, `checkFeedbacks` (module -> core).

### 5.3 Recommended Pattern for Custom Apps

For a custom app with a configuration UI:

```
Device Process (Node.js)
  ├── USB HID access (requires native modules)
  ├── Button state management
  ├── Rendering pipeline
  └── WebSocket server on localhost:PORT

Configuration UI (React app in browser or Electron)
  └── WebSocket client
       ├── Receive: button state updates, device events
       └── Send: configuration changes, page edits, action assignments
```

**Why WebSocket over REST:**
- Bidirectional: the backend pushes device events (button presses) and state changes to the UI in real-time
- Low latency: button press -> UI highlight is sub-millisecond
- Persistent connection: no reconnection overhead per interaction
- Protocol is simple: JSON messages with a `type` field

```typescript
// Message protocol example
type ServerMessage =
  | { type: 'deviceConnected'; device: DeviceInfo }
  | { type: 'deviceDisconnected'; serialNumber: string }
  | { type: 'buttonDown'; deviceSerial: string; keyIndex: number }
  | { type: 'buttonUp'; deviceSerial: string; keyIndex: number }
  | { type: 'stateUpdate'; page: string; buttons: ButtonState[] }

type ClientMessage =
  | { type: 'setButtonConfig'; page: string; keyIndex: number; config: ButtonConfig }
  | { type: 'switchPage'; deviceSerial: string; page: string }
  | { type: 'setBrightness'; deviceSerial: string; brightness: number }
  | { type: 'saveProfile'; profile: ProfileConfig }
```

If using TypeScript end-to-end, consider **tRPC** (as Companion does) for automatic type safety between frontend and backend without manual message type definitions.

---

## 6. Configuration Storage

### 6.1 JSON Files (Simple Apps)

```jsonc
// ~/.streamdeck/config.json
{
  "version": 1,
  "devices": {
    "AL12H1A00001": {
      "brightness": 80,
      "activePage": "main"
    }
  },
  "profiles": {
    "default": {
      "pages": {
        "main": {
          "buttons": {
            "0": { "icon": "mute", "action": "toggleMute", "label": "Mute" },
            "1": { "icon": "camera", "action": "toggleCamera", "label": "Cam" }
          }
        }
      }
    }
  }
}
```

Pros: Human-readable, easy to version control, simple to implement.
Cons: No concurrent writes, no querying, file corruption risk on crash.

**File watching for live reload:** Use `chokidar` or `fs.watch` to detect external edits and reload configuration without restart. Companion's dev mode uses this pattern for module hot-reload.

### 6.2 SQLite (Complex Apps)

Companion uses SQLite (via `better-sqlite3`) with tables for:

```sql
-- Device identity and settings
CREATE TABLE surfaces (
  id TEXT PRIMARY KEY,      -- serial number
  config JSON NOT NULL      -- brightness, rotation, offset
);

-- Device grouping
CREATE TABLE surface_groups (
  id TEXT PRIMARY KEY,
  config JSON NOT NULL      -- shared page, lock state
);

-- Button configurations
CREATE TABLE controls (
  page INTEGER,
  row INTEGER,
  column INTEGER,
  config JSON NOT NULL,     -- action, feedback, style
  PRIMARY KEY (page, row, column)
);
```

Pros: ACID transactions, concurrent access, efficient querying, no corruption.
Cons: Not human-editable, requires migration strategy.

### 6.3 Recommendation

Start with JSON files. Migrate to SQLite when you need:
- Multiple devices with independent configurations
- Undo/redo (transaction log)
- Import/export of profiles
- Concurrent access from UI and device processes

---

## 7. Device Hot-Plug Handling

### 7.1 Detection Mechanism

Do **not** poll `HID.devices()` or `HID.devicesAsync()`. Each call triggers a full USB bus enumeration that is expensive and can slow down active HID communication with connected devices.

Instead, use the `usb` npm package for OS-native hotplug events:

```typescript
import { usb } from 'usb'
import { listStreamDecks, openStreamDeck } from '@elgato-stream-deck/node'

const ELGATO_VENDOR_ID = 0x0fd9

usb.on('attach', async (device) => {
  if (device.deviceDescriptor.idVendor !== ELGATO_VENDOR_ID) return

  // Small delay for OS to fully enumerate the device
  await new Promise(resolve => setTimeout(resolve, 500))

  const devices = await listStreamDecks()
  for (const info of devices) {
    if (!registry.has(info.serialNumber)) {
      const deck = await openStreamDeck(info.path)
      registry.register(deck)
    }
  }
})

usb.on('detach', (device) => {
  if (device.deviceDescriptor.idVendor !== ELGATO_VENDOR_ID) return
  registry.handleDisconnect()
})
```

### 7.2 Reconnection Strategy

When a device disconnects and reconnects (USB cable pulled, hub power cycle):

1. **Detect disconnect** via the `error` event on the StreamDeck instance or the `usb` detach event
2. **Preserve state** -- keep the device's configuration, active page, and button states in memory, keyed by serial number
3. **On reconnect**, match by serial number and restore the full visual state: brightness, all button images, active page
4. **Debounce** reconnection attempts -- USB enumeration during device initialization can be flaky; wait 500-1000ms after attach before opening

### 7.3 Lifecycle State Machine

```
      ┌──────────┐
      │DISCOVERED│ (USB attach event)
      └─────┬────┘
            │ open()
            ▼
      ┌──────────┐
      │  OPENING  │ (HID connection in progress)
      └─────┬────┘
            │ success
            ▼
      ┌──────────┐
      │  ACTIVE   │ (normal operation)
      └─────┬────┘
            │ error/detach
            ▼
      ┌──────────┐
      │DISCONNECTED│ (state preserved, awaiting reconnect)
      └─────┬────┘
            │ re-attach
            ▼
      ┌──────────┐
      │RECONNECTING│ (restoring state)
      └─────┬────┘
            │ success
            ▼
         ACTIVE
```

---

## 8. Button Interaction Patterns

### 8.1 Basic Events from Hardware

The Stream Deck hardware only reports two events per key: **down** (pressed) and **up** (released). All higher-level interactions must be detected in software.

### 8.2 Gesture Detection Implementation

```typescript
interface GestureConfig {
  longPressThreshold: number    // ms, default 500
  doublePressWindow: number     // ms, default 300
  holdRepeatInterval: number    // ms, default 0 (disabled)
}

class GestureDetector {
  private pressTimestamps = new Map<number, number>()
  private pressTimers = new Map<number, NodeJS.Timeout>()
  private lastReleaseTime = new Map<number, number>()

  constructor(
    private deck: StreamDeck,
    private config: GestureConfig,
    private callbacks: GestureCallbacks,
  ) {
    deck.on('down', (key) => this.handleDown(key))
    deck.on('up', (key) => this.handleUp(key))
  }

  private handleDown(key: number) {
    this.pressTimestamps.set(key, Date.now())

    // Schedule long-press detection
    const timer = setTimeout(() => {
      this.callbacks.onLongPress?.(key)
      this.pressTimers.delete(key)
    }, this.config.longPressThreshold)

    this.pressTimers.set(key, timer)
  }

  private handleUp(key: number) {
    const pressTime = this.pressTimestamps.get(key) ?? 0
    const holdDuration = Date.now() - pressTime

    // Cancel long-press timer
    const timer = this.pressTimers.get(key)
    if (timer) {
      clearTimeout(timer)
      this.pressTimers.delete(key)
    }

    // Was it a long press? (already fired via timer)
    if (holdDuration >= this.config.longPressThreshold) return

    // Check for double press
    const lastRelease = this.lastReleaseTime.get(key) ?? 0
    const gap = Date.now() - lastRelease
    this.lastReleaseTime.set(key, Date.now())

    if (gap < this.config.doublePressWindow) {
      this.callbacks.onDoublePress?.(key)
      this.lastReleaseTime.delete(key)  // prevent triple-press
    } else {
      // Delay single press to allow double-press detection
      setTimeout(() => {
        if (Date.now() - (this.lastReleaseTime.get(key) ?? 0) >= this.config.doublePressWindow) {
          this.callbacks.onPress?.(key)
        }
      }, this.config.doublePressWindow)
    }
  }
}
```

### 8.3 Interaction Catalog

| Gesture | Detection | Latency | Use Case |
|---------|-----------|---------|----------|
| Single press | Up event + no double-press within window | ~300ms (delayed by double-press window) | Primary action |
| Long press | Down timer exceeds threshold | 500ms | Secondary action, context menu |
| Double press | Two up events within window | ~100ms (second press) | Tertiary action |
| Hold + repeat | Timer fires repeatedly while held | Configurable | Volume control, scrolling |
| Combo (two keys) | Two down events within short window | ~50ms | Modifier patterns, shift layers |

**Performance note:** Single-press detection has inherent latency equal to the double-press window because the system must wait to determine if a second press is coming. If double-press is not needed, disable it to get instant single-press response.

### 8.4 Shift/Modifier Layer Pattern

A common pattern where holding one button modifies the behavior of all other buttons:

```typescript
const SHIFT_KEY = 31  // bottom-right corner
let shiftActive = false

deck.on('down', (key) => {
  if (key === SHIFT_KEY) {
    shiftActive = true
    renderShiftLayer()  // show alternate icons
    return
  }
  const action = shiftActive ? shiftActions[key] : normalActions[key]
  action?.execute()
})

deck.on('up', (key) => {
  if (key === SHIFT_KEY) {
    shiftActive = false
    renderNormalLayer()  // restore normal icons
  }
})
```

---

## 9. Multi-Device Management

### 9.1 Device Identity

Each Stream Deck has a unique 14-character serial number retrievable via `getSerialNumber()`. Use this as the stable identifier for configuration persistence -- USB paths change across reboots and port changes.

```typescript
interface DeviceRegistry {
  devices: Map<string, ManagedDevice>  // keyed by serial number

  register(deck: StreamDeck): Promise<ManagedDevice>
  unregister(serial: string): void
  getBySerial(serial: string): ManagedDevice | undefined
  getAll(): ManagedDevice[]
}

interface ManagedDevice {
  serial: string
  model: DeviceModelId
  deck: StreamDeck
  config: DeviceConfig       // brightness, rotation, assigned profile
  activePage: string
  state: 'active' | 'disconnected'
}
```

### 9.2 Device Grouping

Companion's `SurfaceGroup` pattern -- treating multiple devices as a single logical surface:

- **Unified page navigation:** All devices in a group switch pages together
- **Coordinate offset:** Each device maps to a region of a larger virtual grid
- **Shared lock state:** When one device locks, all grouped devices lock

This is useful for setups with multiple Stream Decks side by side acting as one large control surface.

### 9.3 Independent vs. Synchronized Operation

| Mode | Description | Use Case |
|------|-------------|----------|
| Independent | Each device has its own profile and pages | Different functions per device |
| Synchronized | All devices share page navigation | Large unified control surface |
| Leader/Follower | One device controls page state for others | Main deck + satellite decks |

### 9.4 Model-Aware Rendering

Different models have different key counts and sizes. The rendering engine must adapt:

```typescript
const MODEL_SPECS: Record<DeviceModelId, ModelSpec> = {
  STREAMDECK_MINI:    { columns: 3, rows: 2, keySize: 80,  imageFormat: 'bmp' },
  STREAMDECK_MK2:     { columns: 5, rows: 3, keySize: 72,  imageFormat: 'jpeg' },
  STREAMDECK_XL:      { columns: 8, rows: 4, keySize: 96,  imageFormat: 'jpeg' },
  STREAMDECK_PLUS:    { columns: 4, rows: 2, keySize: 120, imageFormat: 'jpeg' },
  STREAMDECK_NEO:     { columns: 4, rows: 2, keySize: 100, imageFormat: 'jpeg' },
  STREAMDECK_PEDAL:   { columns: 3, rows: 1, keySize: 0,   imageFormat: 'none' },
}
```

Profiles and pages should define buttons by logical position (row, column), not by absolute key index. The device abstraction layer maps logical positions to device-specific key indices.

---

## 10. Recommended Architecture for a New TypeScript App

Pulling together all patterns, here is a concrete architecture recommendation:

### 10.1 Project Structure

```
src/
├── transport/              # Layer 1: HID communication
│   └── device-adapter.ts   # Wraps @elgato-stream-deck/node
├── devices/                # Layer 2: Device management
│   ├── device-manager.ts   # Discovery, hot-plug, registry
│   ├── device-registry.ts  # Serial number -> device mapping
│   └── gesture-detector.ts # Press/long-press/double-press
├── state/                  # Layer 3: State management
│   ├── store.ts            # Central state (profiles, pages, buttons)
│   ├── page-manager.ts     # Page navigation, history
│   └── feedback-engine.ts  # Dynamic state from external sources
├── renderer/               # Layer 3: Rendering pipeline
│   ├── render-manager.ts   # Cache, queue, invalidation
│   ├── render-worker.ts    # Worker thread (canvas drawing)
│   └── templates/          # Button visual templates
├── actions/                # Layer 4: Application logic
│   ├── action-registry.ts  # Available action types
│   ├── action-runner.ts    # Execution engine
│   └── builtin/            # Built-in actions (launch, hotkey, etc.)
├── server/                 # Layer 5: IPC
│   ├── ws-server.ts        # WebSocket server for UI
│   └── api.ts              # Message handlers
├── config/                 # Configuration
│   ├── config-manager.ts   # Load/save/watch
│   └── migrations.ts       # Version migration
└── main.ts                 # Entry point, wiring
```

### 10.2 Key Technology Choices

| Concern | Recommendation | Rationale |
|---------|---------------|-----------|
| HID access | `@elgato-stream-deck/node` | Mature, all models supported, TypeScript |
| Hot-plug | `usb` npm package | OS-native events, no polling |
| Image rendering | `@napi-rs/canvas` | Full Canvas 2D API, fast native bindings |
| Image processing | `sharp` + `@julusian/jpeg-turbo` | For icon loading and format conversion |
| Worker threads | Node.js `worker_threads` | Keep main thread responsive |
| WebSocket | `ws` npm package | Lightweight, no framework needed |
| Configuration | JSON files initially, SQLite later | Start simple, migrate when needed |
| Desktop wrapper | Tauri (if needed) | Smaller than Electron, Rust backend |

### 10.3 Startup Sequence

```typescript
async function main() {
  // 1. Load configuration
  const config = await ConfigManager.load('~/.streamdeck/config.json')

  // 2. Initialize render pipeline
  const renderer = new RenderManager({ workerCount: 4, cacheSize: 200 })

  // 3. Initialize device manager (starts hot-plug monitoring)
  const devices = new DeviceManager({
    onConnect: async (deck) => {
      const serial = await deck.getSerialNumber()
      const deviceConfig = config.getDeviceConfig(serial)
      await deck.setBrightness(deviceConfig.brightness)
      await renderFullPage(deck, deviceConfig.activePage)
    },
    onDisconnect: (serial) => {
      console.log(`Device ${serial} disconnected, preserving state`)
    },
  })

  // 4. Start WebSocket server for configuration UI
  const server = new WebSocketServer({ port: 9120 })

  // 5. Open all currently connected devices
  await devices.scanAndConnect()

  console.log('Stream Deck controller running')
}
```

---

## Sources

1. [Julusian/node-elgato-stream-deck (GitHub)](https://github.com/Julusian/node-elgato-stream-deck) -- Primary Node.js/TypeScript library for direct Stream Deck HID control
2. [Bitfocus Companion (GitHub)](https://github.com/bitfocus/companion) -- Production-grade open-source Stream Deck application
3. [Bitfocus Companion Architecture (DeepWiki)](https://deepwiki.com/bitfocus/companion/1-introduction-to-bitfocus-companion) -- Detailed architecture documentation
4. [Companion Surface and Hardware Management (DeepWiki)](https://deepwiki.com/bitfocus/companion/3.3-surface-and-hardware-management) -- Device detection, hot-plug, rendering pipeline
5. [Companion Module System (DeepWiki)](https://deepwiki.com/bitfocus/companion/4.2-module-system) -- Plugin architecture and IPC
6. [Companion Data Flow (DeepWiki)](https://deepwiki.com/bitfocus/companion/8-module-development) -- Action/feedback/variable data flow patterns
7. [Elgato Stream Deck HID API](https://docs.elgato.com/streamdeck/hid/general/) -- Official HID protocol documentation
8. [Elgato Stream Deck Plugin Environment](https://docs.elgato.com/streamdeck/sdk/introduction/plugin-environment/) -- Official plugin architecture
9. [mrfigg/streamdeck-ui-node (GitHub)](https://github.com/mrfigg/streamdeck-ui-node) -- Lightweight page/key abstraction framework
10. [deckxstream (npm)](https://www.npmjs.com/package/deckxstream) -- Configuration-driven Linux Stream Deck controller
11. [node-hid (GitHub)](https://github.com/node-hid/node-hid) -- Node.js HID device access, performance considerations
12. [node-usb (GitHub)](https://github.com/node-usb/node-usb) -- USB hotplug detection for Node.js
13. [Elgato Key Logic](https://www.elgato.com/us/en/explorer/products/stream-deck/key-logic-stream-deck/) -- Official multi-gesture button support
14. [streamdeck-linux-gui (GitHub)](https://github.com/streamdeck-linux-gui/streamdeck-linux-gui) -- Linux GUI app with pages and profiles
15. [home-assistant-streamdeck-yaml (GitHub)](https://github.com/basnijholt/home-assistant-streamdeck-yaml/) -- YAML-based Stream Deck configuration
16. [@elgato-stream-deck/node README](https://github.com/Julusian/node-elgato-stream-deck/blob/main/packages/node/README.md) -- API documentation for the Node.js package
