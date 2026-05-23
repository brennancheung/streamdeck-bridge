# Rust, Go, and Other Languages

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Open-Source Libraries

---

## Key Findings

- **Rust has the most mature ecosystem** with two actively maintained crates (`elgato-streamdeck` and `streamdeck`) that support the Stream Deck XL, async input, and direct HID image writes via JPEG encoding.
- **Go has four libraries** ranging from CGO-dependent to pure-Go implementations. The `rafaelmartins/streamdeck` library is notable for requiring zero C dependencies.
- **C/C++ options are minimal** but `hidapi` (the C library underlying most higher-level libraries) can be used directly for maximum control. `streamdeckpp` wraps it with ImageMagick integration.
- **C#/.NET has the strongest community** among non-scripting languages: `StreamDeckSharp` (395 stars) and `DeckSurf SDK` both offer direct HID access without the Elgato software.
- **Swift has one library** (`Codedeck`) that wraps IOKit HID, but it is WIP and lightly maintained.
- **The USB HID bottleneck is the same for all languages.** The Stream Deck XL accepts 96x96 JPEG images over 1024-byte HID output reports. The performance ceiling is USB transfer speed, not language speed -- but JPEG encoding speed matters for high-frequency updates.
- **Rust is the strongest choice for performance-critical work** due to native JPEG encoding (via `image-rs`, `turbojpeg`, or `mozjpeg-rs`), zero-cost abstractions over HID, and no GC pauses.
- **Language-agnostic approaches exist**: the Elgato SDK plugin protocol uses WebSocket/JSON, and Bitfocus Companion exposes HTTP/OSC/WebSocket control. These add latency unsuitable for high-frequency image updates.

---

## Stream Deck XL: Protocol Context

Before evaluating libraries, it helps to understand what any library must do under the hood.

The Stream Deck XL (32 keys, 8x4 grid) communicates over USB HID. Each key image is 96x96 pixels, encoded as JPEG. Images are sent via HID output reports capped at 1024 bytes, meaning each key image must be chunked across multiple reports. The final chunk sets a "Transfer is Done" flag. The device uses USB 3.0 but is backward-compatible with USB 2.0.

**Performance-critical path for image updates:**
1. Render/compose the image (CPU-bound)
2. Encode to JPEG (CPU-bound, quality vs. size tradeoff)
3. Chunk into 1024-byte HID reports (trivial)
4. Write chunks to USB HID device (I/O-bound, USB bandwidth)

The encoding step is where language choice matters most. Steps 3-4 are I/O-bound and language-agnostic in practice. For updating all 32 keys, you are sending 32 separate JPEG images sequentially over HID -- the total transfer time depends on JPEG file sizes and USB throughput.

---

## Rust Libraries

Rust has the richest ecosystem for direct Stream Deck control, with multiple crates at different abstraction levels.

### 1. `elgato-streamdeck` (Recommended)

| Detail | Value |
|---|---|
| Crate | [elgato-streamdeck](https://crates.io/crates/elgato-streamdeck) |
| Version | 0.12.1 (November 2025) |
| Repository | [OpenActionAPI/rust-elgato-streamdeck](https://github.com/OpenActionAPI/rust-elgato-streamdeck) |
| Stars | 83 |
| License | MPL-2.0 |
| Downloads | Active (used by streamdeck-oxide and others) |

**Supported devices:** Original, Original V2, XL, XL V2, Mini, Mini Mk2, Mini Discord Edition, Mk2, Pedal, Plus, Plus XL, Neo.

**Key API surface:**
```rust
let hid = elgato_streamdeck::new_hidapi()?;
let devices = StreamDeck::list_devices(&hid);
let mut deck = StreamDeck::connect(&hid, pid, serial)?;

deck.set_brightness(80)?;
deck.set_button_image(key_index, image)?;  // image-rs Image type
deck.flush()?;                              // commit pending writes

// Async button reading (requires tokio feature)
let key_events = deck.read_buttons_async().await?;
```

**Why it stands out:**
- Most comprehensive device support (11 models)
- Integrates with `image-rs` for image loading/manipulation
- Async button reading via tokio (feature-gated)
- Buffered writes with explicit `flush()` -- allows batching multiple key updates before committing
- 100% documented API (170/170 items)
- Active maintenance (last release November 2025)

**Dependencies:** `hidapi` ^2.6, `image` ^0.25, optional `tokio` ^1.

**Limitations:** Async button reading uses `block_in_place`, so it cannot run on `current_thread` tokio runtimes. Dropped support for non-Elgato devices (Mirabox/Ajazz) in v0.11.

### 2. `streamdeck` (rust-streamdeck)

| Detail | Value |
|---|---|
| Crate | [streamdeck](https://crates.io/crates/streamdeck) |
| Version | 0.10.0 (February 2026) |
| Repository | [ryankurte/rust-streamdeck](https://github.com/ryankurte/rust-streamdeck) |
| Stars | 78 |
| License | MPL-2.0 |
| Downloads | ~238/month |

**Supported devices:** Mini, Original V1/V2, XL, Module variants (6/15/32 keys).

**Key features:**
- Connection by VID/PID/Serial or by device type matching
- Button reading: polling (blocking/non-blocking), async, and callback modes
- Brightness, color fills, and image writes
- Includes a CLI tool (`cargo install streamdeck`) for testing and scripting
- Linux udev rules included

**Status:** Labeled "WIP" but actively updated (February 2026). The CLI tool is useful for quick device testing.

### 3. `streamdeck-oxide` (High-Level Framework)

| Detail | Value |
|---|---|
| Crate | [streamdeck-oxide](https://crates.io/crates/streamdeck-oxide) |
| Version | 0.2.1 (April 2025) |
| Downloads | ~345/month |
| License | MIT |

**What it is:** A framework built on top of `elgato-streamdeck` that provides application-level abstractions:
- View system with navigation between "screens" of buttons
- Button rendering with text, icons, SVG (via `resvg`), and custom images
- Plugin architecture with shared context
- Async state fetching and event handling
- Text rendering via `ab_glyph`/`rusttype`
- Material Design icon support via `md-icons`

**When to use it:** If you want to build a full Stream Deck application with multiple pages, navigation, and a plugin system rather than raw button control. Adds significant dependencies (~32MB, 590K SLoC).

### 4. `streamdeck-rs` (Plugin SDK, Not Direct Control)

| Detail | Value |
|---|---|
| Crate | [streamdeck-rs](https://crates.io/crates/streamdeck-rs) |
| Repository | [mdonoughe/streamdeck-rs](https://github.com/mdonoughe/streamdeck-rs) |
| Stars | 26 |
| Last updated | April 2023 |

**Important distinction:** This is for writing plugins that run under the Elgato Stream Deck software, communicating via WebSocket. It does NOT provide direct HID access. It parses CLI args and handles the WebSocket protocol. Useful only if you want to extend the official Elgato app, not replace it.

### 5. `streamdeck-hid-rs` (Older, Minimal)

| Crate | [streamdeck-hid-rs](https://crates.io/crates/streamdeck-hid-rs) |
|---|---|
| Last updated | February 2022 |

Thin layer above `hidapi` for Stream Deck interaction. Not actively maintained. Superseded by `elgato-streamdeck`.

### Rust JPEG Encoding Options

For high-frequency image updates, JPEG encoding speed is critical. Rust options:

| Library | Type | Speed | Notes |
|---|---|---|---|
| `image-rs` (built-in) | Pure Rust | Moderate | Default encoder, good enough for most cases |
| `turbojpeg` | Binding to libturbojpeg (C) | Fast | Requires linking C library; ~30% faster than pure Rust |
| `mozjpeg-rs` | Pure Rust port | Fast | 6% faster than C mozjpeg with trellis quantization; better compression ratios |

For a Stream Deck XL updating all 32 keys, the difference between encoders matters. Each 96x96 JPEG at quality 80 is roughly 3-8 KB. Encoding 32 of them with `image-rs` takes single-digit milliseconds total on modern hardware. The USB transfer of ~100-250 KB of JPEG data is the actual bottleneck.

---

## Go Libraries

### 1. `rafaelmartins/streamdeck` (Pure Go, Recommended)

| Detail | Value |
|---|---|
| Module | `rafaelmartins.com/p/streamdeck` |
| Repository | [rafaelmartins/streamdeck](https://github.com/rafaelmartins/streamdeck) |
| Stars | 3 (new project) |
| License | BSD-3-Clause |
| Last updated | March 2026 |

**Key differentiator:** Pure Go with zero CGO dependency. Uses a custom `usbhid` library (also pure Go) instead of `hidapi` or `libusb`. This makes cross-compilation trivial and eliminates the need for a C toolchain.

**Supported devices:** Mini, V2, MK.2, Plus, Neo -- with support for LCD keys, touch points, rotary dials, info bar displays, and touch strips.

**Features:**
- Device discovery and connection
- Callback-based input handling for all input types
- Automatic image scaling (accepts any `image.Image`)
- Cross-platform: Linux, macOS, Windows

**Caveat:** Very new project with only 7 commits and no releases. The API may change. Stream Deck XL support is not explicitly listed, though the underlying protocol is similar.

### 2. `dh1tw/streamdeck`

| Detail | Value |
|---|---|
| Module | `github.com/dh1tw/streamdeck` |
| Repository | [dh1tw/streamdeck](https://github.com/dh1tw/streamdeck) |
| Stars | 87 |
| License | MIT |
| Commits | 77 |

**Most established Go library.** Works on Linux, macOS, Windows, and SoC boards (Raspberry Pi, Orange Pi, Banana Pi).

**Requires CGO** due to `hidapi` dependency for HID device enumeration. This complicates cross-compilation.

**Ecosystem:**
- Companion examples repo: `dh1tw/streamdeck-examples`
- Button rendering library: `dh1tw/streamdeck-buttons`
- Good documentation with godoc

### 3. `magicmonkey/go-streamdeck`

| Detail | Value |
|---|---|
| Repository | [magicmonkey/go-streamdeck](https://github.com/magicmonkey/go-streamdeck) |
| Stars | 79 |
| License | MIT |

**Dual API approach:**
- Low-level `streamdeck.Open()` for raw HID protocol access
- High-level `streamdeck.New()` with button abstractions, action handlers, and decorators

**Architecture includes:** pre-built button types (TextButton, ColourButton), chained actions, and decorator patterns for visual enhancements.

**Limitation:** "Images are the wrong size for other streamdecks" -- primarily tested with Stream Deck XL (32-button model). Bug reports and patches welcomed for other models.

### 4. `matthewpi/streamdeck`

| Detail | Value |
|---|---|
| Repository | [matthewpi/streamdeck](https://github.com/matthewpi/streamdeck) |
| License | MIT |

Built because other Go libraries "didn't work, didn't support needed features, required CGO, or were difficult to use." **Linux-only** -- does not support Windows or macOS.

---

## C/C++ Libraries

### 1. `hidapi` (Foundation Layer)

| Detail | Value |
|---|---|
| Repository | [libusb/hidapi](https://github.com/libusb/hidapi) |
| Language | C |
| License | Multiple (GPL/BSD/custom) |
| Platform | Linux, macOS, Windows, FreeBSD |

This is the foundational C library that most Stream Deck libraries in other languages wrap. It provides:
- `hid_enumerate()` -- find HID devices by VID/PID
- `hid_open()` / `hid_open_path()` -- connect to a device
- `hid_write()` / `hid_read()` -- send/receive HID reports
- `hid_get_feature_report()` / `hid_send_feature_report()`

**Using hidapi directly** means implementing the Stream Deck protocol yourself: constructing the JPEG payloads, chunking into 1024-byte reports, managing the report headers and flags. This is ~200-400 lines of protocol code on top of hidapi calls. The protocol is well-documented in [Elgato's HID API docs](https://docs.elgato.com/streamdeck/hid/general/) and in [community protocol notes](https://gist.github.com/cliffrowley/d18a9c4569537b195f2b1eb6c68469e0).

**Backend options on Linux:** `hidraw` (kernel driver, requires udev rules) or `libusb` (userspace USB, no kernel driver needed).

### 2. `streamdeckpp` (C++ Wrapper)

| Detail | Value |
|---|---|
| Repository | [drepper/streamdeckpp](https://github.com/drepper/streamdeckpp) |
| Stars | 14 |
| Last release | May 2020 (v1.0) |
| License | Not specified |

Authored by Ulrich Drepper (glibc maintainer). Provides a C++ interface over `libhidapi-libusb` with:
- Device discovery via C++ iterators (`for (auto& dev : ctx)`)
- Image setting via file path, ImageMagick `Magick::Image`, or raw data
- Non-blocking read with optional timeout
- Automatic image scaling via ImageMagick

**Limitation:** Linux-focused. Not actively maintained (last activity 2020).

### 3. `StreamDeck-CPPSDK` (Plugin SDK)

| Detail | Value |
|---|---|
| Repository | [fredemmott/StreamDeck-CPPSDK](https://github.com/fredemmott/StreamDeck-CPPSDK) |
| Platform | macOS, Windows |

A fork of Elgato's C++ plugin SDK focused on reusability. This is for building plugins that run under the Elgato software (WebSocket communication), NOT for direct HID access.

### Direct C Approach

For maximum performance and control, writing directly against `hidapi` in C is viable. The Stream Deck protocol is simple enough that a minimal C implementation needs:
- JPEG encoding (via `libjpeg-turbo` for speed)
- HID report construction (fixed header format per device model)
- Chunked write loop

This eliminates all abstraction overhead and gives the tightest control over USB I/O timing.

---

## C# / .NET Libraries

### 1. `StreamDeckSharp` (Most Popular)

| Detail | Value |
|---|---|
| NuGet | [StreamDeckSharp](https://www.nuget.org/packages/StreamDeckSharp/) |
| Repository | [OpenMacroBoard/StreamDeckSharp](https://github.com/OpenMacroBoard/StreamDeckSharp) |
| Stars | 395 (highest of any Stream Deck library) |
| Version | 6.1.0 |
| License | MIT |
| Target | .NET 6+ |

**Part of the OpenMacroBoard project**, which defines an abstraction layer (`OpenMacroBoard.SDK`) that multiple hardware backends can implement. StreamDeckSharp is the Stream Deck backend.

**API style:**
```csharp
using var deck = StreamDeck.OpenDevice();
deck.SetBrightness(80);
var bitmap = KeyBitmap.Create.FromFile("icon.png");
deck.SetKeyBitmap(5, bitmap);
deck.KeyStateChanged += (sender, e) => { /* handle press */ };
```

**Features:**
- Device listener for hot-plug detection (`StreamDeck.CreateDeviceListener()`)
- Bitmap-based image API with `SixLabors.ImageSharp`
- Uses `HidSharp` for cross-platform HID access
- Tested on Windows and Linux; macOS support expected

**Dependencies:** `HidSharp` >= 2.1.0, `OpenMacroBoard.SDK` >= 6.1.0, `SixLabors.ImageSharp` >= 2.1.11.

### 2. `DeckSurf SDK`

| Detail | Value |
|---|---|
| NuGet | [DeckSurf.SDK](https://www.nuget.org/packages/DeckSurf.SDK) |
| Repository | [dend/decksurf-sdk](https://github.com/dend/decksurf-sdk) |
| Version | 0.0.6 (February 2025) |
| Docs | [docs.deck.surf](https://docs.deck.surf/) |

**Independent of Elgato software.** Provides direct device access with:
- Device enumeration via `DeckSurf.SDK.Core`
- Key image setting with `SetKey()` and `ResizeImage()` helper
- Key press event listening

**Limitation:** Windows-only due to Windows API dependencies. Alpha stage (pre-1.0) with potential breaking changes.

### 3. Plugin-Oriented Libraries (WebSocket, Not Direct HID)

- **streamdeck-client-csharp** ([NuGet](https://www.nuget.org/packages/streamdeck-client-csharp)): WebSocket wrapper for Elgato plugin development.
- **Stream-Deck-CSharp-Client** ([GitHub](https://github.com/Aeroverra/Stream-Deck-CSharp-Client)): Cross-platform plugin client using .NET service pattern.
- **StreamDeckToolkit** ([GitHub](https://github.com/FritzAndFriends/StreamDeckToolkit)): .NET Standard library and templates for plugin development.

These communicate with the Elgato software, not the hardware directly.

---

## Swift / macOS

### `Codedeck`

| Detail | Value |
|---|---|
| Repository | [Sherlouk/Codedeck](https://github.com/Sherlouk/Codedeck) |
| Stars | 15 |
| Language | Swift (100%) |
| License | MIT |
| Status | WIP |

**The only Swift library for direct Stream Deck HID access.** Uses Apple's IOKit (`IOHIDDevice`) for HID communication.

**Features:**
- Device monitoring for connect/disconnect events (hot-swap support)
- Multiple simultaneous device connections
- Brightness control
- RGB color fills for individual keys
- Optional Cocoa extensions

**Architecture:** `HIDDeviceMonitor` detects devices, calls delegate methods (`HIDDeviceAdded`, `HIDDeviceRemoved`). Each detected device is wrapped in a `StreamDeck` object.

**Limitations:** WIP status, only 31 commits. Image rendering support appears incomplete. No recent activity indicators in the repository. For serious macOS development, using the Rust libraries via FFI or the C `hidapi` directly would be more reliable.

---

## Language-Agnostic Approaches

### 1. Elgato Plugin SDK (WebSocket/JSON)

The official Elgato SDK communicates between plugins and the Stream Deck application via WebSocket on a localhost port. Plugins are native executables in any language (C++, C#, Go, Rust, etc.) that:
1. Parse CLI arguments for the WebSocket port and registration info
2. Connect to the WebSocket
3. Exchange JSON messages for events and actions

**Limitation for direct control:** Requires the Elgato Stream Deck software to be running. The software owns the HID connection. You control keys via the SDK API, not the hardware directly. Image updates go through the Elgato app's pipeline, adding latency.

### 2. Bitfocus Companion

[Bitfocus Companion](https://github.com/bitfocus/companion) is an open-source Node.js application that owns the Stream Deck HID connection and exposes control via multiple protocols:
- HTTP REST
- WebSocket
- OSC (Open Sound Control)
- TCP/UDP
- ArtNet

**700+ integration modules** for professional broadcast/production equipment. Includes a built-in Stream Deck emulator and web-based touch screen interface.

**Use case:** When you want to control Stream Deck from another application or language without implementing HID yourself. The tradeoff is added latency from the network hop and Companion's processing pipeline.

### 3. StreamDeckWS (WebSocket Proxy)

[StreamDeckWS](https://github.com/ybizeul/StreamDeckWS) is a lightweight WebSocket proxy that forwards Stream Deck key events to a WebSocket service (commonly Node-RED). Simpler than Companion but more limited -- primarily for receiving button events rather than setting images.

---

## Comparison Matrix

| Language | Library | Stars | Direct HID | XL Support | Async | Image API | Cross-Platform | Last Active |
|---|---|---|---|---|---|---|---|---|
| **Rust** | elgato-streamdeck | 83 | Yes | Yes | Yes (tokio) | image-rs | Yes | Nov 2025 |
| **Rust** | streamdeck | 78 | Yes | Yes | Yes | image-rs | Yes | Feb 2026 |
| **Rust** | streamdeck-oxide | -- | Via above | Yes | Yes | image-rs + SVG | Yes | Apr 2025 |
| **Go** | dh1tw/streamdeck | 87 | Yes | Yes | No | stdlib image | Yes (CGO) | Active |
| **Go** | go-streamdeck | 79 | Yes | Yes (primary) | No | stdlib image | Yes (CGO) | Active |
| **Go** | rafaelmartins/streamdeck | 3 | Yes | Unclear | Callbacks | stdlib image | Yes (pure Go) | Mar 2026 |
| **C** | hidapi | 3000+ | Yes | Yes | No | None (BYO) | Yes | Active |
| **C++** | streamdeckpp | 14 | Yes | Unclear | No | ImageMagick | Linux | May 2020 |
| **C#** | StreamDeckSharp | 395 | Yes | Yes | Events | ImageSharp | Win/Linux | Active |
| **C#** | DeckSurf SDK | -- | Yes | Yes | Events | Built-in | Windows | Feb 2025 |
| **Swift** | Codedeck | 15 | Yes | Unclear | Delegate | Partial | macOS | Stale |

---

## Performance Analysis for High-Frequency Image Updates

### Where Language Speed Matters

For a use case like real-time VU meters, status dashboards, or animated visualizations on a Stream Deck XL:

**The pipeline per frame:**
1. Generate 32 images at 96x96 pixels
2. Encode each to JPEG (~3-8 KB each at quality 75-85)
3. Send ~100-250 KB total over USB HID in 1024-byte chunks

**Bottleneck analysis:**
- **Image generation:** Depends on complexity. Simple color fills: microseconds. Text rendering: low milliseconds. Complex compositing: varies.
- **JPEG encoding:** 32 images x 96x96 pixels. In Rust with `image-rs`: ~2-5ms total. In Python with Pillow: ~20-50ms total. In Node.js with `jpeg-turbo`: ~5-10ms total.
- **USB transfer:** 100-250 chunks of 1024 bytes each over USB HID. At USB 2.0 full-speed HID rates, this is the dominant cost at ~50-150ms for all 32 keys.

### Recommendation

**Rust is the best choice for performance-critical image updates** because:
1. JPEG encoding is 5-10x faster than Python, and 2-3x faster than Node.js (when using native encoder)
2. Image composition (blending, text rendering, scaling) benefits from zero-copy and SIMD
3. No garbage collector means consistent frame timing -- no GC pauses causing jank
4. The `elgato-streamdeck` crate's `flush()` pattern naturally batches writes
5. Async input handling via tokio keeps the event loop responsive during USB I/O

**However:** If update frequency is low (a few times per second), any language works fine. The USB transfer time (~50-150ms for all 32 keys) means you cap out around 7-15 full-deck updates per second regardless of language. For single-key updates, the cap is much higher.

**Go is a reasonable alternative** if you prefer its ecosystem. The CGO dependency for HID access is the main downside. The pure-Go `rafaelmartins/streamdeck` library eliminates this but is very new.

**C#/.NET is viable** for Windows-focused projects, especially with StreamDeckSharp's mature API. The .NET JIT produces fast code for image encoding.

**C/C++ with hidapi + libjpeg-turbo** gives absolute maximum performance but requires implementing the protocol manually. Only justified for extreme latency requirements or embedded systems.

---

## Sources

1. [elgato-streamdeck crate (crates.io)](https://crates.io/crates/elgato-streamdeck)
2. [elgato-streamdeck documentation (docs.rs)](https://docs.rs/crate/elgato-streamdeck/latest)
3. [OpenActionAPI/rust-elgato-streamdeck (GitHub)](https://github.com/OpenActionAPI/rust-elgato-streamdeck)
4. [streamdeck crate - ryankurte/rust-streamdeck (GitHub)](https://github.com/ryankurte/rust-streamdeck)
5. [streamdeck crate (lib.rs)](https://lib.rs/crates/streamdeck)
6. [streamdeck-oxide crate (lib.rs)](https://lib.rs/crates/streamdeck-oxide)
7. [streamdeck-rs - mdonoughe (GitHub)](https://github.com/mdonoughe/streamdeck-rs)
8. [dh1tw/streamdeck - Go library (GitHub)](https://github.com/dh1tw/streamdeck)
9. [magicmonkey/go-streamdeck (GitHub)](https://github.com/magicmonkey/go-streamdeck)
10. [rafaelmartins/streamdeck - pure Go library (GitHub)](https://github.com/rafaelmartins/streamdeck)
11. [rafaelmartins/usbhid - pure Go USB HID (GitHub)](https://github.com/rafaelmartins/usbhid)
12. [matthewpi/streamdeck (GitHub)](https://github.com/matthewpi/streamdeck)
13. [libusb/hidapi - C HID library (GitHub)](https://github.com/libusb/hidapi)
14. [drepper/streamdeckpp - C++ interface (GitHub)](https://github.com/drepper/streamdeckpp)
15. [fredemmott/StreamDeck-CPPSDK (GitHub)](https://github.com/fredemmott/StreamDeck-CPPSDK)
16. [OpenMacroBoard/StreamDeckSharp - C# (GitHub)](https://github.com/OpenMacroBoard/StreamDeckSharp)
17. [StreamDeckSharp NuGet package](https://www.nuget.org/packages/StreamDeckSharp/)
18. [DeckSurf SDK (GitHub)](https://github.com/dend/decksurf-sdk)
19. [DeckSurf documentation](https://docs.deck.surf/)
20. [Sherlouk/Codedeck - Swift library (GitHub)](https://github.com/Sherlouk/Codedeck)
21. [Elgato Stream Deck HID API - General Reference](https://docs.elgato.com/streamdeck/hid/general/)
22. [Elgato Stream Deck HID API - Introduction](https://docs.elgato.com/streamdeck/hid/intro/)
23. [Stream Deck HID protocol notes (GitHub Gist)](https://gist.github.com/cliffrowley/d18a9c4569537b195f2b1eb6c68469e0)
24. [Elgato Stream Deck Plugin SDK - WebSocket reference](https://docs.elgato.com/streamdeck/sdk/references/websocket/plugin/)
25. [Bitfocus Companion (GitHub)](https://github.com/bitfocus/companion)
26. [StreamDeckWS - WebSocket proxy (GitHub)](https://github.com/ybizeul/StreamDeckWS)
27. [mozjpeg-rs - Rust JPEG encoder (lib.rs)](https://lib.rs/crates/mozjpeg-rs)
28. [TurboJPEG Rust bindings (lib.rs)](https://lib.rs/crates/turbojpeg)
29. [Reverse Engineering the Stream Deck - Den Delimarsky](https://den.dev/blog/reverse-engineering-stream-deck/)
30. [python-elgato-streamdeck XL performance issue](https://github.com/abcminiuser/python-elgato-streamdeck/issues/36)
