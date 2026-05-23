# Python Ecosystem

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Open-Source Libraries

---

## Key Findings

- **python-elgato-streamdeck** (PyPI: `streamdeck`) is the dominant Python library for direct hardware control -- 1.1k GitHub stars, actively maintained, latest stable release 0.9.8 (Sep 2025), dev release 0.10.0 (Dec 2025)
- Supports all Stream Deck models including XL (32 keys, 8x4 grid, 96x96px JPEG per key)
- Uses HID via `hidapi`/`libusb` -- requires native library installation on all platforms (brew on macOS, apt on Linux, DLL on Windows)
- PIL/Pillow integration is built-in via `PILHelper` module -- handles scaling, margins, format conversion, and native encoding
- Animated GIF support works by pre-rendering frames with `ImageSequence` and cycling them in a timing loop
- Thread-safe via context manager (`with deck:`) for concurrent key updates
- Multiple device support is first-class -- `DeviceManager().enumerate()` returns all connected decks
- Higher-level frameworks exist (streamdeck-linux-gui, streamdeckui) but are small/maintenance-mode projects
- Python is excellent for rapid prototyping even if the final solution targets Node.js -- the API concepts map 1:1

---

## 1. python-elgato-streamdeck (Primary Library)

### Overview

| Attribute | Value |
|-----------|-------|
| **PyPI Package** | `streamdeck` |
| **GitHub** | `abcminiuser/python-elgato-streamdeck` |
| **Author** | Dean Camera |
| **License** | MIT |
| **Stars** | ~1.1k |
| **Python** | >= 3.10 (as of 0.10.0) |
| **Latest Stable** | 0.9.8 (Sep 24, 2025) |
| **Latest Dev** | 0.10.0 (Dec 4, 2025) |
| **Docs** | https://python-elgato-streamdeck.readthedocs.io |

This is the de facto standard Python library for controlling Elgato Stream Deck hardware directly, bypassing the official Elgato software entirely. It communicates over USB HID using `hidapi`.

### Supported Devices

- Stream Deck Mini (6 keys)
- Stream Deck Original / MK2 (15 keys)
- Stream Deck XL (32 keys)
- Stream Deck Neo
- Stream Deck Plus (8 keys + 4 dials + touchscreen)
- Stream Deck Studio
- Stream Deck Pedal (3 foot switches, no display)
- Stream Deck 6/16/32 Key Modules

### Stream Deck XL Specifics

| Property | Value |
|----------|-------|
| Key Count | 32 |
| Layout | 8 columns x 4 rows |
| Key Resolution | 96 x 96 pixels |
| Image Format | JPEG |
| Image Rotation | 0 degrees |
| Image Flip | Horizontal + Vertical |
| Report Length | 1024 bytes (8 header + 1016 payload) |

### Version History (Recent)

| Version | Key Changes |
|---------|-------------|
| 0.10.0 | Stream Deck Studio support, min Python bumped to 3.10, callback exception fixes |
| 0.9.8 | Documentation for 6-key module support |
| 0.9.7 | Type hints for public APIs, Stream Deck 16/32-key modules, new MK2 PID |
| 0.9.6 | Stream Deck Neo support, FreeBSD support, dial/key state fixes |
| 0.9.5 | Stream Deck Plus support (dials + touchscreen) |

---

## 2. Installation & Dependencies

### Python Dependencies

```bash
pip install streamdeck
pip install pillow    # Required for image helpers (PILHelper)
```

The library itself has no hard Python dependencies beyond the standard library. `Pillow` is needed only if you use the `PILHelper` module (you almost certainly will).

### Native Dependencies (hidapi)

The library requires `hidapi` compiled as a native library. This is the most common installation friction point.

**macOS:**
```bash
brew install hidapi
```

**Linux (Debian/Ubuntu):**
```bash
sudo apt install -y libhidapi-libusb0
# Full setup including image processing deps:
sudo apt install -y libudev-dev libusb-1.0-0-dev libhidapi-libusb0
sudo apt install -y libjpeg-dev zlib1g-dev libopenjp2-7 libtiff5
```

**Linux udev rules (required for non-root access):**
```bash
sudo tee /etc/udev/rules.d/10-streamdeck.rules << 'EOF'
SUBSYSTEMS=="usb", ATTRS{idVendor}=="0fd9", GROUP="users", TAG+="uaccess"
EOF
sudo udevadm control --reload-rules
# Reconnect Stream Deck after applying
```

**Windows:**
Download `hidapi.dll` from the [libusb/hidapi releases](https://github.com/libusb/hidapi/releases) and place it in a directory on your `%PATH%`. Match architecture (32/64-bit) to your Python installation.

---

## 3. Core API Reference

### Device Discovery

The `DeviceManager` class is the entry point for finding connected Stream Decks.

```python
from StreamDeck.DeviceManager import DeviceManager

# Find all connected Stream Deck devices
streamdecks = DeviceManager().enumerate()
print(f"Found {len(streamdecks)} Stream Deck(s)")

for index, deck in enumerate(streamdecks):
    deck.open()
    print(f"  [{index}] {deck.deck_type()}")
    print(f"       Serial: {deck.get_serial_number()}")
    print(f"       Firmware: {deck.get_firmware_version()}")
    print(f"       Keys: {deck.key_count()} ({deck.key_layout()[0]}x{deck.key_layout()[1]})")
    deck.close()
```

### Device Lifecycle Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `open()` | None | Opens device for I/O. Must be called first. |
| `close()` | None | Closes device. |
| `reset()` | None | Clears all button images, shows standby. |
| `connected()` | bool | Whether physical device is still connected. |
| `is_open()` | bool | Whether device is open and ready. |

### Device Information Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `deck_type()` | str | Model name (e.g., "Stream Deck XL") |
| `id()` | str | Physical device ID |
| `get_serial_number()` | str | Device serial number |
| `get_firmware_version()` | str | Firmware version string |
| `vendor_id()` | int | USB vendor ID (0x0fd9 for Elgato) |
| `product_id()` | int | USB product ID |

### Key/Button Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `key_count()` | `() -> int` | Number of physical buttons |
| `key_layout()` | `() -> (int, int)` | Grid as (rows, columns) |
| `key_states()` | `() -> list[bool]` | Current press states of all keys |
| `key_image_format()` | `() -> dict` | Image specs: size, format, rotation, flip |
| `set_key_image(key, image)` | `(int, bytes) -> None` | Set a button's image (native format bytes) |
| `set_brightness(percent)` | `(int) -> None` | Set brightness 0-100 |
| `set_key_callback(fn)` | `(Callable) -> None` | Register key state change callback |
| `set_key_callback_async(fn, loop)` | `(Callable, loop) -> None` | Async version of callback |
| `set_poll_frequency(hz)` | `(int) -> None` | Button polling frequency (1-1000 Hz) |

### Additional Methods (Stream Deck Plus / Studio)

| Method | Description |
|--------|-------------|
| `dial_count()` | Number of rotary dials |
| `dial_states()` | Current dial press states |
| `set_dial_callback(fn)` | Register dial interaction callback |
| `is_visual()` | Whether device has a visual display |
| `is_touch()` | Whether device supports touch events |
| `set_touchscreen_image(image, x, y, w, h)` | Draw on touchscreen at position |
| `set_touchscreen_callback(fn)` | Register touchscreen event callback |
| `set_screen_image(image)` | Draw on full screen |

### Thread Safety

The `StreamDeck` object implements Python's context manager protocol for thread-safe operations:

```python
# The 'with' statement acquires an exclusive update lock
with deck:
    deck.set_key_image(0, image_a)
    deck.set_key_image(1, image_b)
    deck.set_brightness(50)
# Lock released on exit
```

This is critical when updating multiple keys from callback threads -- callbacks fire from an internal reader thread.

---

## 4. Image Processing (PILHelper)

The `PILHelper` module bridges PIL/Pillow and the Stream Deck's native image format.

### PILHelper Functions

| Function | Description |
|----------|-------------|
| `create_key_image(deck, background='black')` | Create blank PIL Image at correct key dimensions |
| `create_scaled_key_image(deck, image, margins, background)` | Scale/fit image to key size with margins |
| `to_native_key_format(deck, image)` | Convert PIL Image to device bytes (JPEG/BMP) |
| `create_touchscreen_image(deck, background)` | Blank image for touchscreen |
| `create_scaled_touchscreen_image(deck, image, margins, background)` | Scale for touchscreen |
| `to_native_touchscreen_format(deck, image)` | Convert for touchscreen |
| `create_screen_image(deck, background)` | Blank image for full screen |
| `create_scaled_screen_image(deck, image, margins, background)` | Scale for screen |
| `to_native_screen_format(deck, image)` | Convert for screen |

### Image Workflow

The typical image pipeline is:

1. Load or create a PIL Image
2. Scale to key dimensions with `create_scaled_key_image()`
3. Optionally draw text/overlays with PIL's `ImageDraw`
4. Convert to native bytes with `to_native_key_format()`
5. Send to device with `deck.set_key_image(key, bytes)`

```python
from PIL import Image, ImageDraw, ImageFont
from StreamDeck.ImageHelpers import PILHelper

def render_key(deck, icon_path, label):
    # Load and scale icon to fit key, leave 20px margin at bottom
    icon = Image.open(icon_path)
    image = PILHelper.create_scaled_key_image(deck, icon, margins=[0, 0, 20, 0])

    # Draw text label at bottom center
    draw = ImageDraw.Draw(image)
    font = ImageFont.truetype("Roboto-Regular.ttf", 14)
    draw.text(
        (image.width / 2, image.height - 5),
        text=label,
        font=font,
        anchor="ms",
        fill="white"
    )

    # Convert to device-native format (JPEG for XL)
    return PILHelper.to_native_key_format(deck, image)
```

### key_image_format() Dictionary

The `key_image_format()` method returns a dict describing what the device expects:

```python
{
    'size': (96, 96),        # Width x Height in pixels
    'format': 'JPEG',        # Image encoding (JPEG for XL, BMP for Mini/Original)
    'rotation': 0,           # Degrees of rotation needed
    'flip': (True, True),    # (horizontal_mirror, vertical_mirror)
}
```

The `PILHelper` functions handle rotation and flipping automatically -- you never need to manually mirror images.

---

## 5. Code Examples

### Minimal: Set All Keys to a Color

```python
from PIL import Image
from StreamDeck.DeviceManager import DeviceManager
from StreamDeck.ImageHelpers import PILHelper

deck = DeviceManager().enumerate()[0]
deck.open()
deck.set_brightness(50)

# Create a solid red key image
for key in range(deck.key_count()):
    img = PILHelper.create_key_image(deck, background='red')
    native = PILHelper.to_native_key_format(deck, img)
    deck.set_key_image(key, native)

input("Press Enter to exit...")
deck.reset()
deck.close()
```

### Key Press Handling with Callbacks

```python
import threading
from StreamDeck.DeviceManager import DeviceManager
from StreamDeck.Transport.Transport import TransportError

def key_change_callback(deck, key, state):
    action = "pressed" if state else "released"
    print(f"Key {key} {action}")

    # Exit on last key press
    if state and key == deck.key_count() - 1:
        with deck:
            deck.reset()
            deck.close()

deck = DeviceManager().enumerate()[0]
deck.open()
deck.reset()
deck.set_brightness(30)
deck.set_key_callback(key_change_callback)

print("Listening for key presses... Press last key to exit.")

# Block main thread until all background threads finish
for t in threading.enumerate():
    try:
        t.join()
    except (TransportError, RuntimeError):
        pass
```

The full basic example from the official repo combines all the patterns above (image rendering with text labels, key style switching on press/release, exit key handling) in a single script. See [example_basic.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/example_basic.py) for the complete source.

### Animated GIFs on Keys

```python
#!/usr/bin/env python3

import itertools
import threading
import time
from fractions import Fraction
from PIL import Image, ImageSequence
from StreamDeck.DeviceManager import DeviceManager
from StreamDeck.ImageHelpers import PILHelper
from StreamDeck.Transport.Transport import TransportError

FRAMES_PER_SECOND = 30

def create_animation_frames(deck, image_path):
    """Pre-render all frames of an animated GIF to native format."""
    frames = []
    icon = Image.open(image_path)
    for frame in ImageSequence.Iterator(icon):
        scaled = PILHelper.create_scaled_key_image(deck, frame)
        native = PILHelper.to_native_key_format(deck, scaled)
        frames.append(native)
    return frames

def animate(deck, key_images, fps):
    """Frame loop using absolute time to prevent drift."""
    frame_time = Fraction(1, fps)
    next_frame = Fraction(time.monotonic())

    while deck.is_open():
        try:
            with deck:
                for key, frames in key_images.items():
                    deck.set_key_image(key, next(frames))
        except TransportError:
            break

        next_frame += frame_time
        sleep_interval = float(next_frame) - time.monotonic()
        if sleep_interval >= 0:
            time.sleep(sleep_interval)

if __name__ == "__main__":
    deck = DeviceManager().enumerate()[0]
    deck.open()
    deck.reset()
    deck.set_brightness(30)

    # Pre-render animation frames
    animations = [
        create_animation_frames(deck, "walking.gif"),
        create_animation_frames(deck, "spinning.gif"),
    ]

    # Assign animations to keys (cycling through available animations)
    key_images = {
        k: itertools.cycle(animations[k % len(animations)])
        for k in range(deck.key_count())
    }

    # Start animation thread
    threading.Thread(target=animate, args=[deck, key_images, FRAMES_PER_SECOND]).start()

    # Any key press exits
    deck.set_key_callback(lambda d, k, s: (d.reset(), d.close()))

    for t in threading.enumerate():
        try:
            t.join()
        except (TransportError, RuntimeError):
            pass
```

Key points about animation:
- Frames are **pre-rendered** to native format at startup for performance
- `itertools.cycle()` loops frames infinitely
- `Fraction`-based timing prevents frame drift over time
- 30 FPS is the target, but actual throughput depends on USB bandwidth and key count

### Tiled Image Across All Keys

```python
from PIL import Image, ImageOps
from StreamDeck.DeviceManager import DeviceManager
from StreamDeck.ImageHelpers import PILHelper

def create_full_deck_image(deck, image_path, key_spacing):
    """Resize image to span the entire deck surface."""
    key_rows, key_cols = deck.key_layout()
    key_w, key_h = deck.key_image_format()['size']

    # Total pixel dimensions including spacing between keys
    total_w = key_cols * key_w + (key_cols - 1) * key_spacing
    total_h = key_rows * key_h + (key_rows - 1) * key_spacing

    image = Image.open(image_path)
    return ImageOps.fit(image, (total_w, total_h), Image.LANCZOS)

def crop_key_image(deck, full_image, key, key_spacing):
    """Extract the portion of the full image for a specific key."""
    key_rows, key_cols = deck.key_layout()
    key_w, key_h = deck.key_image_format()['size']

    row = key // key_cols
    col = key % key_cols

    x = col * (key_w + key_spacing)
    y = row * (key_h + key_spacing)

    return full_image.crop((x, y, x + key_w, y + key_h))

deck = DeviceManager().enumerate()[0]
deck.open()
deck.set_brightness(30)

full_image = create_full_deck_image(deck, "panorama.jpg", key_spacing=36)

for key in range(deck.key_count()):
    tile = crop_key_image(deck, full_image, key, key_spacing=36)
    native = PILHelper.to_native_key_format(deck, tile)
    deck.set_key_image(key, native)

input("Press Enter to exit...")
deck.reset()
deck.close()
```

See also the official [example_deckinfo.py](https://python-elgato-streamdeck.readthedocs.io/en/stable/examples/deckinfo.html) for a complete device inspection script that prints key image format details (size, format, rotation, flip) for every connected device.

---

## 6. Multiple Devices, Async, and Architecture

### Multiple Device Handling

`DeviceManager().enumerate()` returns all connected Stream Decks. Each `deck` object is independent -- open, configure, and close separately. Callbacks receive the `deck` as the first argument for identification. The `with deck:` context manager locks per-device, so different devices can update concurrently.

```python
for deck in DeviceManager().enumerate():
    deck.open()
    if deck.deck_type() == "Stream Deck XL":
        setup_xl(deck)
    elif deck.deck_type() == "Stream Deck Mini":
        setup_mini(deck)
    deck.set_key_callback(lambda d, k, s: handle_key(d, k, s))
```

### Async Support

The library supports asyncio callbacks dispatched thread-safely via `loop.call_soon_threadsafe()`:

```python
import asyncio

async def async_key_handler(deck, key, state):
    print(f"Key {key} {'pressed' if state else 'released'}")

loop = asyncio.get_event_loop()
deck.set_key_callback_async(async_key_handler, loop=loop)
```

### Architecture

The library stack: Application -> PILHelper -> Device Classes (StreamDeckXL, etc.) -> Transport (abstract) -> LibUSB HIDAPI Backend -> hidapi native lib -> USB HID -> Hardware. A Dummy backend exists for testing without hardware.

---

## 7. Other Python Libraries

### streamdeck-linux-gui (formerly streamdeck-ui)

| Attribute | Value |
|-----------|-------|
| **GitHub** | `streamdeck-linux-gui/streamdeck-linux-gui` |
| **Original** | `timothycrosley/streamdeck-ui` |
| **Stars** | ~416 |
| **Status** | Maintenance mode (bug fixes only, no new features) |
| **Latest** | v4.1.4 (Feb 2026) |

A full GUI application for Linux that provides a no-code Stream Deck configuration experience. Built on top of `python-elgato-streamdeck`.

**Features:** Multi-device, button pages, brightness control, animated icons, drag-and-drop, import/export, systemd service, auto-reconnect, hotkey/command/text actions.

**Limitations:** Uses `pynput` for simulating keypresses -- limited Wayland support. Maintenance mode means no new features accepted.

**Relevance:** Not a library you'd build on, but demonstrates what a full application built on `python-elgato-streamdeck` looks like.

### streamdeckui (kneufeld)

| Attribute | Value |
|-----------|-------|
| **PyPI** | `streamdeckui` |
| **GitHub** | `kneufeld/streamdeckui` |
| **Stars** | 1 |
| **Status** | Minimal activity |

A small higher-level GUI framework that introduces `Deck`, `Page`, and `Key` abstractions:

```python
import asyncio
from StreamDeck.DeviceManager import DeviceManager
from streamdeckui import Deck, Page, Key
from streamdeckui.mixins import QuitKeyMixin

class QuitKey(QuitKeyMixin, Key):
    def __init__(self, page, **kw):
        super().__init__(page, **kw)
        self.set_image(Key.UP, 'power.png')

class NumberedPage(Page):
    def __init__(self, deck, keys):
        super().__init__(deck, [])
        self._keys = [Key(self, label=str(i)) for i in range(self.device.key_count())]
        self._keys[-1] = QuitKey(self, label='quit')

async def main(deck):
    deck.add_page('numbers', NumberedPage(deck, None))
    deck.change_page('numbers')
    deck.turn_on()
    await deck.block_until_quit()

streamdeck = DeviceManager().enumerate()[0]
streamdeck.open()
loop = asyncio.get_event_loop()
deck = Deck(streamdeck, clear=True, loop=loop)
loop.run_until_complete(main(deck))
deck.release()
```

**Relevance:** Interesting Page/Key abstraction pattern, but too small/unmaintained for production use.

### streamdeck-sdk (gri-gus)

| Attribute | Value |
|-----------|-------|
| **PyPI** | `streamdeck-sdk` |
| **GitHub** | `gri-gus/streamdeck-python-sdk` |
| **Latest** | 1.2.1 (Sep 2024) |

For creating **plugins for the official Elgato Stream Deck software**, not for direct hardware control. Uses pydantic for typing, includes a Property Inspector generator. Has a companion `streamdeck-sdk-pi` package.

**Relevance:** Different use case -- this creates plugins that run inside the Elgato software, not standalone hardware control.

### streamdeck-plugin-sdk (strohganoff)

| Attribute | Value |
|-----------|-------|
| **PyPI** | `streamdeck-plugin-sdk` |
| **GitHub** | `strohganoff/python-streamdeck-plugin-sdk` |

Another Python SDK for Elgato Stream Deck plugin development using WebSocket communication. More Pythonic API conventions.

**Relevance:** Same plugin-for-official-software use case. Not direct hardware control.

### pystreamdeck (infintropy)

| Attribute | Value |
|-----------|-------|
| **GitHub** | `infintropy/pystreamdeck` |
| **Stars** | Minimal |

A high-level Python wrapper and GUI with simple button/action classes.

**Relevance:** Experimental, minimal adoption.

---

## 8. Python for Prototyping (Even if Final is Node.js)

Python is an excellent prototyping choice for Stream Deck projects:

**Advantages:**
- Fastest path from zero to buttons lighting up (~20 lines of code)
- PIL/Pillow is the most mature image processing library in any language
- Interactive REPL -- `python -i script.py` lets you call `deck.set_key_image()` live
- Extensive examples in the official repo cover every use case
- Easy to test image generation pipelines before porting

**Concept Mapping to Node.js:**
| Python (`streamdeck`) | Node.js (`@elgato-stream-deck/node`) |
|----------------------|---------------------------------------|
| `DeviceManager().enumerate()` | `listStreamDecks()` / `openStreamDeck()` |
| `deck.set_key_image(key, bytes)` | `deck.fillKeyBuffer(key, buffer)` |
| `deck.set_key_callback(fn)` | `deck.on('down', fn)` / `deck.on('up', fn)` |
| `deck.set_brightness(n)` | `deck.setBrightness(n)` |
| `PILHelper.to_native_key_format()` | Sharp/Canvas buffer conversion |
| `with deck:` (lock) | Not needed (single-threaded) |

**Prototyping Workflow:**
1. Use Python to explore device capabilities and validate image pipelines
2. Test animation frame rates and USB bandwidth limits
3. Prototype complex layouts (tiled images, dynamic labels)
4. Port proven concepts to Node.js for the production implementation

---

## 9. Limitations & Gotchas

1. **hidapi installation** -- The native dependency is the #1 source of setup issues. On macOS, `brew install hidapi` usually works. On Linux, the udev rules step is easy to forget.

2. **Image format varies by model** -- XL uses JPEG, Mini/Original use BMP. The `PILHelper` handles this automatically, but if you're generating raw bytes manually, check `key_image_format()`.

3. **Threading model** -- Callbacks fire from an internal reader thread. Always use `with deck:` when updating the device from a callback to avoid race conditions.

4. **No official releases on GitHub** -- The project publishes to PyPI but has no GitHub Releases page. Version tracking is via PyPI and the changelog in docs.

5. **Image flip required** -- The XL requires horizontal + vertical mirroring. `PILHelper` does this for you, but it's a gotcha if bypassing the helper.

6. **USB bandwidth for animations** -- At 30 FPS across 32 keys (XL), you're pushing significant data over USB. Pre-render frames to native format to minimize per-frame processing.

7. **Python 3.10 minimum** -- As of v0.10.0, Python 3.9 is no longer supported.

---

## Sources

1. [python-elgato-streamdeck GitHub Repository](https://github.com/abcminiuser/python-elgato-streamdeck)
2. [streamdeck PyPI Package](https://pypi.org/project/streamdeck/)
3. [python-elgato-streamdeck Documentation](https://python-elgato-streamdeck.readthedocs.io/en/stable/)
4. [python-elgato-streamdeck v0.10.0 Documentation](https://python-elgato-streamdeck.readthedocs.io/en/latest/)
5. [Basic Example Script](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/example_basic.py)
6. [Animated Images Example](https://python-elgato-streamdeck.readthedocs.io/en/stable/examples/animated.html)
7. [Tiled Image Example](https://python-elgato-streamdeck.readthedocs.io/en/stable/examples/tiled.html)
8. [Device Info Example](https://python-elgato-streamdeck.readthedocs.io/en/stable/examples/deckinfo.html)
9. [StreamDeck Devices API Reference](https://python-elgato-streamdeck.readthedocs.io/en/latest/modules/devices.html)
10. [LibUSB HIDAPI Backend Setup](https://python-elgato-streamdeck.readthedocs.io/en/latest/pages/backend_libusb_hidapi.html)
11. [StreamDeckXL Device Class Source](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckXL.py)
12. [PILHelper Source](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/ImageHelpers/PILHelper.py)
13. [Changelog](https://python-elgato-streamdeck.readthedocs.io/en/latest/pages/changelog.html)
14. [streamdeck-linux-gui GitHub](https://github.com/streamdeck-linux-gui/streamdeck-linux-gui)
15. [streamdeckui GitHub](https://github.com/kneufeld/streamdeckui)
16. [streamdeck-sdk (gri-gus)](https://github.com/gri-gus/streamdeck-python-sdk)
17. [streamdeck-plugin-sdk (strohganoff)](https://github.com/strohganoff/python-streamdeck-plugin-sdk)
18. [Elgato Stream Deck HID Protocol Docs](https://docs.elgato.com/streamdeck/hid/)
