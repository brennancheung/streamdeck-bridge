# Bitfocus Companion

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Full Applications & Frameworks

---

## Key Findings

- Companion is a mature, open-source (MIT-licensed) Node.js/Electron application with 700+ modules, primarily targeting broadcast/AV professionals who need to control video switchers, audio mixers, cameras, and production software from Stream Deck hardware
- Architecture is a monorepo with four workspaces: core server (Node.js/Express/tRPC/SQLite), web UI (React/MobX/TanStack Router), Electron launcher, and shared TypeScript library
- Modules run in isolated child processes communicating via JSON-RPC IPC, providing crash protection but adding overhead; each module exposes actions, feedbacks, variables, and presets
- Button rendering uses `@napi-rs/canvas` at 288x288 pixels (4x the native 72x72) with a worker pool, LRU cache, and 60-second TTL; images are encoded as PNG buffers and sent to hardware via `@elgato-stream-deck/node`
- The variables system supports five types (connection, custom, expression, local, builtin) with `$(prefix:name)` syntax and a full expression engine built on JSEP
- Generic modules (HTTP, OSC, TCP/UDP, MQTT, Art-Net) provide protocol-level extensibility, but there are no built-in modules for OS-level tasks like volume control, app launching, or shell execution
- Performance issues are documented: CPU spikes to 30%+ with active device connections, memory leaks during extended operation, and high CPU when button values change frequently
- For a custom developer tool (not broadcast control), Companion's architecture is instructive but its broadcast-centric module ecosystem and Electron overhead make building from scratch with `@elgato-stream-deck/node` the better path

---

## 1. Architecture Overview

### Tech Stack

Bitfocus Companion is built as a monorepo using Yarn 4 workspaces with four primary packages:

| Workspace | Purpose | Technology |
|-----------|---------|------------|
| `companion` | Core Node.js server, database, module runtime | Node.js, Express, tRPC, SQLite (better-sqlite3) |
| `webui` | Web-based administration interface | React, MobX, TanStack Router |
| `launcher` | Electron wrapper and process manager | Electron, Sentry |
| `shared-lib` | Common types, validation, expression parsing | TypeScript, Zod, JSEP |

The application runs as an Electron desktop app on Windows, macOS, and Linux. It can also run headless in Docker for server deployments, though USB device passthrough is not supported in Docker (requiring Companion Satellite for remote hardware).

### Core Internal Components

The internal architecture has several key controllers:

- **ControlsController**: Manages buttons, triggers, and their action sequences. Handles multi-step button logic where a single button can cycle through different states
- **GraphicsController**: Renders button images using a worker pool with `@napi-rs/canvas`. Maintains a primary `renderCache` (Map-based) and a `renderLRUCache` (QuickLRU) to minimize redundant renders
- **SurfaceController**: Discovers and initializes hardware surfaces (USB scan, network connections). Maintains the registry of active surfaces and groups
- **InstanceController**: Manages module lifecycle -- loading, starting, stopping, and restarting module child processes
- **VariablesController**: Manages the variable system, caching values and notifying subscribers of changes

### How It Talks to Stream Deck

Companion uses the `@elgato-stream-deck/node` library (maintained by Julusian, a Companion contributor) for direct USB HID communication. Since Companion v4.0, Stream Deck support is implemented as a **surface module** rather than being baked into the core.

The connection flow:

1. `SurfaceController` monitors USB hotplug events via node-hid
2. Connected devices are identified by USB Vendor ID (VID) and Product ID (PID) listed in the surface module's `manifest.json`
3. The `SurfaceHandler` wraps each physical device, translating button coordinates to control identifiers
4. Button presses flow from hardware through `SurfaceHandler` to `ControlsController` to action execution
5. Rendered images flow back: `GraphicsController` emits `button_drawn` events, `SurfaceHandler` listens and sends PNG buffers to the device

Supported Stream Deck models include: Mini, Original (V1/V2), XL, MK.2, Plus, Neo, Pedal, Studio, and the Corsair Galleon K100. The Plus and Plus XL models additionally support page swiping, and all models except Pedal support brightness control.

### Deployment Models

- **Desktop**: Standard Electron app with direct USB access
- **Headless/Docker**: Web UI only, no USB (use Satellite for hardware)
- **Companion Satellite**: Lightweight app that connects local USB surfaces to a remote Companion server over TCP (port 16622). Works across subnets and VPNs. Available as desktop app, Raspberry Pi image, or headless systemd service

---

## 2. Module System Deep Dive

### Module Architecture

Modules are the core extensibility mechanism. Each module is an independent npm package that:

- Extends the `@companion-module/base` base class
- Runs in its own isolated child process for crash protection
- Communicates with the core via JSON-RPC over IPC
- Defines actions, feedbacks, variables, and presets

The isolation model means a faulty module cannot crash the entire application, but it also means each module incurs process overhead (memory, CPU for IPC serialization).

### IPC Message Protocol

Communication between core and modules uses typed JSON-RPC messages:

**Core to Module:**
- `init` -- Initialize with config
- `updateConfig` -- Configuration changed by user
- `updateVariables` -- Variable values changed
- `runAction` -- Execute an action (button pressed)
- `subscribeFeedbacks` -- Register for feedback updates

**Module to Core:**
- `setVariables` -- Update variable values
- `saveConfig` -- Persist configuration changes
- `setStatus` -- Report connection status (OK, warning, error)
- `checkFeedbacks` -- Request feedback re-evaluation
- `recordAction` -- Record an action for preset creation

### Module Lifecycle

Modules implement three primary lifecycle methods:

```typescript
class MyModule extends InstanceBase<Config> {
  async init(config: Config): Promise<void> {
    // Initialize network connections, timers
  }

  async destroy(): Promise<void> {
    // Clean up: close sockets, clear timers
  }

  async configUpdated(config: Config): Promise<void> {
    // React to user configuration changes
  }
}
```

### Defining Actions, Feedbacks, Variables, Presets

- **Actions**: Operations triggered by button press/release/rotation. Registered via `this.setActionDefinitions()`. Before execution, the `VariablesAndExpressionParser` evaluates embedded variables and the system validates inputs against module definitions
- **Feedbacks**: Real-time state information that dynamically updates button appearance (colors, text, visibility). Modules call `this.checkFeedbacks()` when state changes
- **Variables**: Dynamic data exposed to the rest of Companion. Set via `this.setVariableValues()`. Other modules and buttons can reference them with `$(modulename:variablename)` syntax
- **Presets**: Pre-configured button templates users can drag and drop. Include default actions, feedbacks, and styling

### Module Discovery and Installation

Since v4.0, modules are downloadable plugins distributed separately from the core:

1. **Module Store API**: Online discovery via Bitfocus Developer API, metadata cached with 6-hour staleness threshold
2. **Offline Bundles**: Pre-packaged tar.gz archives for air-gapped environments
3. **Custom Uploads**: Direct .tgz upload for user-developed modules
4. **Git Submodules**: Development modules in `lib/module/*`

Modules are stored in versioned subdirectories: `{moduleId}-{version}`. When a configuration requires a missing module version, Companion auto-downloads it.

### Module Security

The runtime uses Node.js permission flags based on manifest declarations:
- `filesystem` maps to `--allow-fs-read` / `--allow-fs-write`
- `network` maps to `--allow-net`
- `child-process` maps to child process permissions

Modules can also specify their Node.js runtime version (e.g., `node18`, `node22`).

### Developer Portal

Bitfocus maintains a Developer Portal (developer.bitfocus.io) for submitting and managing modules. A TypeScript template repository (`bitfocus/companion-module-template-ts`) provides the starting scaffold.

---

## 3. Available Modules and Automation Capabilities

### Module Ecosystem Scale

Companion has **700+ published modules** covering primarily broadcast and AV equipment. The module ecosystem is heavily weighted toward professional production:

### Module Categories (Notable Examples)

| Category | Examples |
|----------|----------|
| Video Switchers | Blackmagic ATEM, Ross Video, Grass Valley, FOR-A |
| Streaming Software | OBS Studio, vMix, Wirecast, XSplit |
| Media Servers | ProPresenter, QLab, Resolume, Millumin |
| Audio Mixers | Behringer X32/M32, Yamaha, Allen & Heath, Midas |
| Camera Control | PTZ cameras via VISCA, Panasonic, Sony |
| Lighting | DMX via Art-Net/sACN, MA Lighting, ETC |
| Graphics | CasparCG, Singular.live, Vizrt |
| Communication | RTS/Telex intercom, ClearCom |
| Recording | Blackmagic HyperDeck, AJA Ki Pro |
| Home Automation | Home Assistant (community) |

### Generic Protocol Modules

These are the most relevant for custom/general-purpose use:

- **Generic HTTP**: Send arbitrary HTTP requests (GET, POST, PUT, etc.) to any REST API
- **Generic OSC**: Send/receive Open Sound Control messages
- **Generic TCP/Serial**: Raw TCP or serial port communication
- **Generic MQTT**: Publish/subscribe to MQTT topics
- **Generic Art-Net**: Send DMX data over network
- **Generic Ember+**: Ember+ protocol for broadcast routing

### What Is Missing for General Desktop Automation

Companion does **not** include built-in modules for common desktop automation tasks:

- No native volume control module (would need to go through Generic HTTP to a local service, or write a custom module)
- No app launcher / shell execution module (modules run in sandboxed child processes with restricted permissions)
- No clipboard management
- No window management
- No file system operations
- No system tray integration
- No keyboard shortcut simulation (beyond what connected software supports)

The module ecosystem is purpose-built for controlling external AV/broadcast devices over network protocols. OS-level desktop automation is outside its design scope.

---

## 4. Web-Based Configuration UI

### Interface Overview

Companion runs a web server (default port 8000) that serves a full React-based administration interface. Users access it at `http://localhost:8000` or via the "Launch GUI" button in the Electron launcher.

### Key UI Sections

- **Buttons Tab**: Visual grid matching the Stream Deck layout. Click any position to configure a button with actions, feedbacks, and styling. Supports drag-and-drop of presets
- **Connections Tab**: Add, configure, and monitor module connections. Shows connection status (OK/warning/error) for each configured device
- **Triggers Tab**: Create automation rules that fire on time intervals, variable changes, or other events without requiring physical button presses
- **Variables Tab**: Browse and search all available variables from all modules. Create custom variables and expression variables
- **Settings Tab**: Configure installation name, admin PIN protection, network interfaces, and global preferences
- **Surfaces Tab**: Map physical devices to pages, configure button grids, group multiple devices

### UI Features

- PIN-code protection for the admin interface with configurable lockout timeout
- Installation naming for environments with multiple Companion instances
- Real-time button preview matching what appears on physical hardware
- Variable browser with search across all connection and custom variables
- Web-based button emulator for testing without physical hardware

---

## 5. Image Rendering Pipeline

### Rendering Architecture

Companion renders button images server-side using `@napi-rs/canvas` (a Rust-based Node.js canvas implementation, faster than node-canvas). The rendering pipeline:

1. **Variable Substitution**: `parseVariablesInButtonStyle()` replaces `$(prefix:name)` references in button text with current values
2. **Canvas Drawing**: `GraphicsRenderer.drawButton()` renders text, icons, backgrounds, and feedback overlays onto a canvas
3. **PNG Encoding**: Canvas is encoded to a PNG buffer wrapped in an `ImageResult`
4. **Caching**: Results stored in `locationCache` with a 60-second TTL. A primary `renderCache` and `renderLRUCache` prevent redundant renders
5. **Distribution**: `button_drawn` event emitted, `SurfaceHandler` picks it up and sends to hardware

### Resolution and Compatibility

The system internally renders at **288x288 pixels** (4x the Stream Deck's native 72x72) but tells modules the resolution is 72x72 for backward compatibility. This means button graphics are actually high-resolution, downscaled by the hardware.

### Current Limitations (From Maintainer Discussion)

- The button styling structure "has become a bit of a mess as new bits have been added over time"
- Multiple aspect ratios are handled with letterboxing, which can waste screen space
- GIF support is "usually super ugly but highly requested"
- SVG support is planned but not yet implemented
- No native animation support (scrolling text, transitions)
- Each layer could theoretically be cached independently for performance, but this optimization is not yet implemented

### Worker Pool

Rendering is offloaded to worker threads to keep the main event loop responsive. This is important because with many buttons updating simultaneously (e.g., timers, status indicators), rendering can become CPU-intensive.

---

## 6. Dynamic Content System

### Variable Types

Companion supports five categories of variables:

| Type | Description | Scope |
|------|-------------|-------|
| **Connection Variables** | Read-only data from configured modules (e.g., `$(obs:scene_name)`) | Global |
| **Custom Variables** | User-defined, with current value, startup value, and persistence settings | Global |
| **Expression Variables** | User-defined computed values that auto-recalculate reactively | Global |
| **Local Variables** | Button/trigger-scoped variables, inaccessible outside parent (since v4.1) | Per-button/trigger |
| **Builtin Local Variables** | System-provided: `$(this:page)`, `$(this:row)`, `$(this:column)`, `$(this:surface_id)` | Per-button |

### Variable Data Types

Custom and expression variables support:
- **String**: UTF-8 text
- **Number**: IEEE754 64-bit floating point
- **Boolean**: true/false
- **Object**: JSON objects or arrays
- **Null**: Special invalidity marker

### Expression Engine

The expression system is built on JSEP (JavaScript Expression Parser) and supports:
- Arithmetic operations
- String manipulation (including URI encode/decode)
- Logical operators and conditionals
- Variable references within expressions

Users can toggle between "Value" mode (static text) and "Expression" mode in the UI, with the latter enabling variable substitution and mathematical expressions.

### Feedbacks

Feedbacks dynamically update button appearance based on real-time module state:
- Change foreground/background colors
- Show/hide text overlays
- Swap icons based on conditions
- Stack multiple feedbacks on a single button with priority ordering

### Triggers

Triggers execute actions automatically without physical button presses:
- **Time interval**: Run every N seconds/minutes
- **Random time interval**: Run at randomized intervals
- **Time of day**: Run at specific times
- **Variable change**: React when a variable's value changes
- **Condition checks**: Optional conditions that must be met before trigger fires

Each trigger can contain multiple event configurations and fires when any enabled event occurs, subject to optional condition checks.

---

## 7. Remote Control APIs

Companion exposes multiple remote control interfaces:

### HTTP REST API

Available on the same port as the web UI:

```
POST /api/location/<page>/<row>/<column>/press    -- Press and release
POST /api/location/<page>/<row>/<column>/down     -- Press and hold
POST /api/location/<page>/<row>/<column>/up       -- Release
POST /api/location/<page>/<row>/<column>/rotate-left
POST /api/location/<page>/<row>/<column>/rotate-right
POST /api/location/<page>/<row>/<column>/style    -- Change text/colors
GET/POST /api/variable/<name>                      -- Get/set custom variables
```

### OSC Control

Listens on configurable port (default 12321) for OSC messages to trigger buttons and control pages.

### TCP Control

Raw TCP socket interface for integration with broadcast automation systems.

### Elgato Plugin

A plugin for the native Elgato Stream Deck software that bridges to Companion, allowing Companion-controlled buttons to appear alongside native Stream Deck actions.

---

## 8. Limitations and Pain Points

### Performance Issues

- **CPU spikes**: Baseline is 1-2% CPU with no devices, but connecting an ATEM or similar device can push CPU to 30%+. Buttons with frequently changing values (timers, counters) cause additional CPU load from constant re-rendering
- **Memory leaks**: Multiple reports of memory climbing over extended operation periods, sometimes by gigabytes
- **Module overhead**: Each module runs as a separate child process, multiplying memory usage. A setup with 20 modules means 20+ Node.js child processes
- **Periodic CPU bursts**: Some configurations show 45% CPU spikes every 30 seconds from internal housekeeping

### Architectural Limitations

- **Electron dependency**: The full application bundles Electron, which adds ~200MB+ to the install size and consumes significant memory even when the GUI is not visible
- **Broadcast-centric design**: The entire module ecosystem, UI metaphors, and architecture are designed for broadcast/AV control. Adapting it for desktop automation or developer tooling means working against the grain
- **No OS-level integration**: No built-in capability for shell execution, volume control, app launching, keyboard shortcuts, or system monitoring. The sandboxed module system actively restricts these capabilities
- **Button grid model**: The UI is built around a fixed grid matching physical Stream Deck layouts. Custom layouts, dynamic button counts, or non-grid arrangements are not supported
- **Image rendering overhead**: Server-side canvas rendering for every button update is heavyweight compared to directly pushing pre-rendered bitmaps to hardware

### Multi-Device Issues

- Page navigation controls affect all surfaces simultaneously by default
- Running multiple Stream Decks with independent control requires careful configuration
- Some users report buttons disappearing or failing to change pages when running alongside the native Elgato software

### Development Friction

- Module development requires understanding the Companion module API, IPC protocol, and variable system
- Testing requires running the full Companion application
- The module sandbox restricts what custom modules can access (filesystem, network, child processes require explicit manifest permissions)
- Hot reload exists for development but can cause high CPU usage (70%+ while idle)

---

## 9. Build vs. Use Decision Analysis

### When Companion Makes Sense

- You need to control **professional broadcast/AV equipment** (ATEM, vMix, OBS, audio mixers, PTZ cameras)
- You want a **ready-made solution** with 700+ device integrations
- Your use case is **production control** -- switching cameras, triggering graphics, managing audio
- You need **multi-device orchestration** across multiple Stream Decks, potentially on different machines via Satellite
- You want a **web-based configuration UI** without writing any code

### When Building Custom Makes More Sense

- Your use case is **developer tooling or desktop automation** (app launching, volume control, system monitoring, deployment triggers)
- You want **direct OS-level integration** (shell commands, clipboard, window management, keyboard simulation)
- You need **minimal resource overhead** -- a custom app using `@elgato-stream-deck/node` directly can run in a single process with ~50MB memory vs. Companion's multi-process Electron footprint
- You want **full control over the rendering pipeline** -- push exactly the images you want without Companion's canvas abstraction
- You need **real-time, high-frequency updates** on buttons without the rendering overhead and caching layers
- You want to use **modern tooling** (Tauri instead of Electron, custom state management, direct integration with your dev stack)
- You want **simpler architecture** -- one process, direct HID access, no IPC serialization, no module sandbox

### What to Learn from Companion's Architecture

Even if building custom, Companion's architecture offers valuable lessons:

1. **Surface abstraction layer**: The `SurfaceHandler` pattern of wrapping hardware devices behind a common interface is worth replicating
2. **Variable system**: The `$(prefix:name)` syntax with expression support is a well-designed approach to dynamic content
3. **Feedback model**: Having modules report state changes that automatically update button appearance is cleaner than polling
4. **Worker pool rendering**: Offloading canvas rendering to worker threads prevents UI jank
5. **LRU caching**: Caching rendered button images with TTL prevents redundant work
6. **Module isolation**: Even if you don't use child processes, designing integrations as independent modules with defined interfaces improves maintainability
7. **Multi-step buttons**: The concept of buttons cycling through states (press once for action A, again for action B) is useful for mode switching
8. **Remote control APIs**: Exposing HTTP/OSC/TCP control endpoints makes the system composable with other tools

### The Key Library

Both Companion and a custom solution would use the same underlying library: **`@elgato-stream-deck/node`** by Julusian. This library provides:

- Direct USB HID communication with all Stream Deck models
- Event-driven API for button presses (down/up)
- Methods to set button images (raw pixel buffers)
- Device discovery and lifecycle management
- Promise-based, well-maintained, actively updated

This is the same library Companion uses internally. Building on it directly eliminates all of Companion's abstraction layers while retaining full hardware access. The library has nothing to do with Elgato's official software -- it is specifically designed for developers building alternative Stream Deck applications.

---

## 10. Community and Project Health

- **License**: MIT (core) and MIT (modules) -- fully permissive for commercial use
- **GitHub**: ~672 stars, ~368 forks on the main repository
- **Contributors**: Maintained by Bitfocus (a Norwegian company) and community volunteers
- **Module ecosystem**: 700+ modules, many maintained by community contributors
- **Release cadence**: Active development with regular releases (v4.3.x as of 2025-2026)
- **Support channels**: Community Slack, GitHub Issues, official documentation at companion.free
- **Commercial relationship**: Bitfocus also produces commercial products; Companion remains free and open-source. Stream Deck Studio was designed with Bitfocus involvement. Elgato's Network Dock (PoE+ Stream Deck connectivity) was revealed at ISE 2025

---

## Sources

1. [Bitfocus Companion GitHub Repository](https://github.com/bitfocus/companion)
2. [Bitfocus Companion Official Site](https://bitfocus.io/companion)
3. [Companion DeepWiki - Introduction](https://deepwiki.com/bitfocus/companion/1-introduction-to-bitfocus-companion)
4. [Companion DeepWiki - Variables System](https://deepwiki.com/bitfocus/companion/4.3-variables-system)
5. [Companion DeepWiki - Module System](https://deepwiki.com/bitfocus/companion/4.2-module-system)
6. [Companion DeepWiki - Developing Custom Modules](https://deepwiki.com/bitfocus/companion/4.3-developing-custom-modules)
7. [Companion DeepWiki - Surface and Hardware Management](https://deepwiki.com/bitfocus/companion/3.3-surface-and-hardware-management)
8. [Companion DeepWiki - Surface Module Supported Hardware](https://deepwiki.com/bitfocus/companion-surface-elgato-stream-deck/1.2-supported-hardware)
9. [Companion DeepWiki - Data Flow and Integration](https://deepwiki.com/bitfocus/companion/8-module-development)
10. [companion-module-base GitHub Repository](https://github.com/bitfocus/companion-module-base)
11. [Bitfocus Developer Portal](https://developer.bitfocus.io/)
12. [companion-module-template-ts GitHub Repository](https://github.com/bitfocus/companion-module-template-ts)
13. [Button Drawing Overhaul Discussion (GitHub #3293)](https://github.com/bitfocus/companion/discussions/3293)
14. [HTTP Remote Control Documentation](https://companion.free/user-guide/v4.1/remote-control/http-remote-control/)
15. [Variables User Guide](https://companion.free/user-guide/v4.2/config/variables/)
16. [Triggers Documentation](https://companion.free/user-guide/beta/config/triggers/)
17. [Companion Satellite](https://bitfocus.io/companion-satellite)
18. [node-elgato-stream-deck Library (Julusian)](https://github.com/Julusian/node-elgato-stream-deck)
19. [@elgato-stream-deck/node on npm](https://www.npmjs.com/package/@elgato-stream-deck/node)
20. [Generic HTTP Module](https://github.com/bitfocus/companion-module-generic-http)
21. [Generic OSC Module](https://github.com/bitfocus/companion-module-generic-osc)
22. [Generic MQTT Module](https://github.com/bitfocus/companion-module-generic-mqtt)
23. [High CPU Usage Issue (GitHub #2536)](https://github.com/bitfocus/companion/issues/2536)
24. [Memory Utilization Issue (GitHub #2030)](https://github.com/bitfocus/companion/issues/2030)
25. [Companion CPU Usage Community Thread](https://community.bitfocus.io/t/companion-uses-45-of-cpu-every-30-seconds/50)
26. [Stream Deck + and Companion (heretorecord.com)](https://heretorecord.com/blog/stream-deck-and-bitfocus-companion/)
27. [Sound on Sound Forum Notes on Companion](https://www.soundonsound.com/forum/viewtopic.php?t=94257)
28. [Companion Docker Deployment (DeepWiki)](https://deepwiki.com/bitfocus/companion/7.3-docker-deployment)
