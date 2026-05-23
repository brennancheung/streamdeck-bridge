# Hardware Specs and Variants

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Hardware & HID Protocol

---

## Key Findings

- The Stream Deck XL has 32 keys in an 8x4 grid, each at 96x96 pixel resolution, using JPEG image format over a single 1024x600 LCD panel
- All Stream Deck devices share USB Vendor ID `0x0FD9` (Elgato Systems GmbH); each model has a distinct Product ID (PID)
- The XL has two hardware revisions: V1 (PID `0x006C`) and V2/2022 (PID `0x008F`), plus a Module variant (PID `0x00BA`) -- all three share the same HID command set
- There are now 10+ distinct Stream Deck hardware models spanning key-only, dial+touch, pedal, rack-mount, and maker module form factors
- The new Stream Deck Module line (6/15/32 keys) targets makers and OEMs with unbranded aluminum chassis, USB-C 2.0, and full HID API access
- Despite product page claims of "USB 3.0," the official HID documentation classifies all Stream Deck devices as USB 2.0-compliant HID devices
- Each model family uses a different image format and rotation: XL uses JPEG with 180-degree rotation; Mini uses BMP with 90-degree rotation; Plus uses JPEG with no rotation; Plus XL uses JPEG with 90-degree counter-clockwise rotation
- The "buttons" are not individual screens -- every Stream Deck uses a single shared LCD panel behind a grid of translucent silicone/membrane keys

---

## Table of Contents

1. [Stream Deck XL Deep Dive](#stream-deck-xl-deep-dive)
2. [Complete Model Comparison](#complete-model-comparison)
3. [USB Identification Reference](#usb-identification-reference)
4. [Stream Deck Module Line](#stream-deck-module-line)
5. [Hardware Architecture](#hardware-architecture)
6. [Protocol Family Groupings](#protocol-family-groupings)
7. [Specifications That Matter for Custom Software](#specifications-that-matter-for-custom-software)

---

## Stream Deck XL Deep Dive

### Physical Specifications

| Parameter | Value |
|---|---|
| Model Name | Stream Deck XL |
| SKU | 10GAT9901 |
| Keys | 32 customizable LCD keys |
| Key Layout | 8 columns x 4 rows |
| Dimensions (without stand) | 182 x 112 x 34 mm (7.17 x 4.41 x 1.34 in) |
| Weight (without stand) | 410 g (14.46 oz) |
| Weight (with stand) | 690 g (1.5 lb) |
| Stand | Removable magnetic stand, included |
| Cable | USB-C to USB-C, 150 cm (59 in) |
| Price (MSRP) | $249.99 |

### Display Specifications

| Parameter | Value |
|---|---|
| Full LCD Resolution | 1024 x 600 pixels (high DPI) |
| Per-Key Resolution | 96 x 96 pixels |
| Image Format | JPEG |
| Image Rotation Required | 180 degrees (both axes flipped) |
| Display Type | Single TFT LCD behind translucent key grid |
| Color Depth | 24-bit (via JPEG compression) |

The Stream Deck XL does not have 32 individual screens. It has one continuous 1024x600 LCD panel. The firmware maps each 96x96 pixel region to a key position. The silicone membrane key caps sit above the LCD, with each key acting as a translucent window into the corresponding display region.

### USB and HID Specifications

| Parameter | Value |
|---|---|
| USB Vendor ID (VID) | `0x0FD9` |
| USB Product ID (PID) - V1 | `0x006C` |
| USB Product ID (PID) - V2 (2022) | `0x008F` |
| USB Product ID (PID) - Module 32 | `0x00BA` |
| USB Compliance | USB 2.0 HID |
| HID Output Report ID | `0x02` |
| HID Input Report ID | `0x01` |
| HID Feature Report IDs (setters) | `0x03` |
| HID Feature Report IDs (getters) | `0x04` - `0x08`, `0x0A` |
| Image Report Length | 1024 bytes |
| Image Report Header Length | 8 bytes |
| Image Report Payload Length | 1016 bytes |

Note: The product page advertises "USB 3.0" but the official Elgato HID API documentation explicitly states these are "USB 2.0-compliant input devices." The HID protocol itself operates at USB 2.0 speeds regardless of the physical connector. The USB-C connector is electrically USB 2.0 for data transfer.

### Key Commands (Output Report 0x02)

| Command ID | Function |
|---|---|
| `0x07` | Update Key Image (single key, 96x96 JPEG) |
| `0x08` | Update Full Screen Image (1024x600 JPEG) |
| `0x0D` | Update Background |

### Feature Reports

| Report ID | Direction | Function |
|---|---|---|
| `0x03` | Host -> Device | Setter (brightness, reset, etc.) |
| `0x04` | Device -> Host | Getter (serial number) |
| `0x05` | Device -> Host | Getter (firmware version) |
| `0x06` | Host -> Device | Fill key with solid color |
| `0x08` | Bidirectional | Various getters/setters |

### Hardware Revisions

The Stream Deck XL has two known hardware revisions, identifiable by USB Product ID:

**V1 (PID `0x006C`)** -- Original release. This is the version most commonly documented in open-source libraries and reverse engineering efforts.

**V2 / 2022 (PID `0x008F`)** -- Released around 2022. According to the python-elgato-streamdeck library and the official Elgato HID documentation, both V1 and V2 share the same HID command set (the `StreamDeckXL` device class handles both PIDs identically). Externally, the two revisions appear identical. The change likely reflects an internal PCB revision or component substitution, not a user-facing feature change.

**Module 32 (PID `0x00BA`)** -- The maker/OEM module variant. Same 32-key layout, same HID commands, different form factor (see Module section below).

For custom software, all three PIDs should be enumerated when scanning for XL-class devices. The python-elgato-streamdeck library maps all three to the same `StreamDeckXL` class with identical key count, resolution, image format, and report structure.

### System Requirements

| OS | Minimum Version |
|---|---|
| Windows | Windows 11 or newer |
| macOS | macOS 13 (Ventura) or newer |

Note: These are Elgato's official requirements for their Stream Deck software. For custom HID-based software, any OS with USB HID support will work -- the device is a standard HID peripheral.

---

## Complete Model Comparison

### Key-Only Models

| Spec | Mini | MK.2 / Classic | XL | Neo |
|---|---|---|---|---|
| Keys | 6 | 15 | 32 | 8 + 2 touch |
| Layout | 3x2 | 5x3 | 8x4 | 4x2 |
| Per-Key Resolution | 80x80 px | 72x72 px | 96x96 px | 96x96 px |
| Full LCD Resolution | 320x240 px | 480x272 px | 1024x600 px | 480x320 px |
| LCD DPI Class | Low | Low | High | High |
| Image Format | BMP | JPEG | JPEG | JPEG |
| Image Rotation | 90 deg CW | 180 deg | 180 deg | 180 deg |
| USB Version | USB 2.0 | USB 2.0 | USB 2.0 | USB 2.0 |
| Dimensions (mm) | 84x60x58 | 118x84x25 | 182x112x34 | 107x26x78 |
| Weight | 160 g | ~240 g | 410 g | 210 g |
| Price | ~$79.99 | ~$149.99 | $249.99 | ~$79.99 |

### Dial + Touch Models

| Spec | Stream Deck + | Stream Deck + XL |
|---|---|---|
| Keys | 8 | 36 |
| Key Layout | 4x2 | 9x4 |
| Per-Key Resolution | 120x120 px | 112x112 px |
| Full LCD Resolution | 800x480 px | 1280x800 px |
| LCD DPI Class | High | High |
| Touch Strip | 800x100 px | 1200x100 px |
| Touch Strip Size | 108x14 mm | 161x14 mm |
| Rotary Encoders | 4 (360 deg, push) | 6 (360 deg, push) |
| Image Format | JPEG | JPEG |
| Image Rotation | None | 90 deg CCW |
| USB Version | USB 2.0 | USB 2.0 |
| Dimensions (mm) | 140x138x110 | 205x147x175 |
| Weight | 465 g | 1085 g |
| Price | ~$199.99 | $349.99 |

### Specialty Models

| Spec | Stream Deck Pedal | Stream Deck Studio |
|---|---|---|
| Inputs | 3 foot pedals | 32 LCD keys + 2 dials |
| Per-Key Resolution | N/A (no display) | Not publicly documented |
| Dial Features | N/A | 360 deg, push, LED ring |
| Connection | USB 2.0 (USB-C to A) | RJ45 PoE+ or USB-C PD |
| Power | USB bus powered | Up to 15W (PoE+ 802.3at or USB-C PD 9V/3A) |
| Form Factor | Floor pedal | 19" rack mount (1U) |
| NFC | No | Yes |
| Dimensions (mm) | 244x175x49 | Rack mount form factor |
| Weight | 930 g | Not documented |
| Price | ~$89.99 | ~$499.99 |

### Additional Device Types (from SDK)

The Stream Deck SDK defines device type IDs for additional platforms that are not standalone hardware:

| Device Type ID | Name | Description |
|---|---|---|
| 3 | Stream Deck Mobile | iOS/Android app, up to 64 virtual keys |
| 4 | Corsair G-Keys | 6 macro keys on Corsair keyboards |
| 6 | Corsair Voyager | Up to 10 capacitive keys on Corsair laptops |
| 8 | SCUF Controller | 5 macro buttons on SCUF gaming controllers |
| 11 | Virtual Stream Deck | Software-only virtual device |
| 12 | Galleon 100 SD | 12 LCD keys + 2 dials + keyboard |

---

## USB Identification Reference

All Elgato Stream Deck devices share the USB Vendor ID `0x0FD9`.

### Complete PID Table

This is the authoritative PID list sourced from the python-elgato-streamdeck library's `ProductIDs.py` and cross-referenced with Elgato's official HID API documentation.

| PID | Hex | Model | Device Class | Protocol Family |
|---|---|---|---|---|
| `0x0060` | 0060 | Stream Deck Original (V1) | StreamDeckOriginal | Legacy (not in HID API) |
| `0x0063` | 0063 | Stream Deck Mini | StreamDeckMini | Mini |
| `0x006C` | 006C | Stream Deck XL (V1) | StreamDeckXL | XL |
| `0x006D` | 006D | Stream Deck Original V2 (2019) | StreamDeckOriginalV2 | Classic |
| `0x0080` | 0080 | Stream Deck MK.2 | StreamDeckOriginalV2 | Classic |
| `0x0084` | 0084 | Stream Deck + | StreamDeckPlus | Plus |
| `0x0086` | 0086 | Stream Deck Pedal | StreamDeckPedal | Pedal |
| `0x008F` | 008F | Stream Deck XL V2 (2022) | StreamDeckXL | XL |
| `0x0090` | 0090 | Stream Deck Mini MK.2 | StreamDeckMini | Mini |
| `0x009A` | 009A | Stream Deck Neo | StreamDeckNeo | Neo |
| `0x00A5` | 00A5 | Stream Deck MK.2 Scissor Keys | StreamDeckOriginalV2 | Classic |
| `0x00AA` | 00AA | Stream Deck Studio | StreamDeckStudio | Studio |
| `0x00B3` | 00B3 | Stream Deck Mini Discord Edition | StreamDeckMini | Mini |
| `0x00B8` | 00B8 | Stream Deck Module 6-Key | StreamDeckMini | Mini |
| `0x00B9` | 00B9 | Stream Deck Module 15-Key | StreamDeckOriginalV2 | Classic |
| `0x00BA` | 00BA | Stream Deck Module 32-Key | StreamDeckXL | XL |
| `0x00C6` | 00C6 | Stream Deck + XL | StreamDeckPlusXL | Plus XL |

### Important Notes for Device Enumeration

1. **Always scan for all PIDs in a family.** If you want "any XL," scan for `0x006C`, `0x008F`, and `0x00BA`.
2. **The Original V1 (`0x0060`) uses a completely different protocol.** It uses 8191-byte BMP transfers instead of 1024-byte JPEG transfers. The official Elgato HID API documentation does not cover this device -- it is considered legacy.
3. **Module PIDs map to the same device classes** as their consumer counterparts. A 32-Key Module (`0x00BA`) speaks exactly the same protocol as a Stream Deck XL (`0x006C`).
4. **The MK.2 V2 PID (`0x00B9`) overlaps with the Module 15-Key PID (`0x00B9`).** Both use the same StreamDeckOriginalV2 class and are functionally identical at the protocol level.

---

## Stream Deck Module Line

### Overview

Announced in mid-2025, the Stream Deck Module line targets makers, OEMs, and integrators who want to embed Stream Deck hardware into custom enclosures, kiosks, dashboards, or DIY control panels. The key difference from consumer models: Modules ship as bare, unbranded keypads without stands, faceplates, or retail packaging.

### Module Variants

| Spec | Module 6-Key | Module 15-Key | Module 32-Key |
|---|---|---|---|
| SKU | 10GBS9901 | 10GBT9901 | 10GBU9901 |
| PID | `0x00B8` | `0x00B9` | `0x00BA` |
| Keys | 6 | 15 | 32 |
| Layout | 3x2 | 5x3 | 8x4 |
| Per-Key Resolution | 80x80 px | 72x72 px | 96x96 px |
| Image Format | BMP | JPEG | JPEG |
| Dimensions (DxWxH) | 19x71x51 mm | 20x107x70 mm | 25x169x103 mm |
| Weight | 57 g | 115 g | 292 g |
| Interface | USB-C 2.0 | USB-C 2.0 | USB-C 2.0 |
| Data Rate | 480 Mbps | 480 Mbps | 480 Mbps |
| Switch Type | Membrane | Membrane | Membrane |
| Chassis | Aluminum | Aluminum | Aluminum |
| Cable Included | USB-C, 150 cm | USB-C, 150 cm | USB-C, 150 cm |
| Processor | ARM MCU | ARM MCU | ARM MCU |

### How Modules Differ from Consumer Models

**What is the same:**
- Identical HID protocol (same commands, same report structures)
- Same per-key resolution as corresponding consumer models
- Same image format requirements
- Same SDK and plugin compatibility
- Full HID API documentation from Elgato

**What is different:**
- **No stand or faceplate** -- bare unit only
- **Unbranded** -- no Elgato logo, designed to embed in custom hardware
- **Aluminum chassis** -- designed for mounting/integration
- **CAD files available** -- Elgato provides downloadable CAD models for custom enclosure design
- **Different form factor** -- thinner and more compact than consumer models
- **USB-C 2.0 only** -- no USB 3.0 claims (consumer XL product page says "USB 3.0")

### Why Modules Matter for Custom Software

The Module line is significant because Elgato is officially supporting direct hardware control use cases. The HID API documentation (docs.elgato.com/streamdeck/hid/) explicitly covers the Module variants alongside consumer models. This means:

1. Elgato considers direct HID control a supported use case, not just a reverse-engineered hack
2. Module PIDs are documented and stable
3. The protocol between modules and consumer devices is identical -- code written for one works on the other
4. System requirements specify "Stream Deck 6.9 or newer" for the modules, but HID access does not require the Stream Deck software at all

---

## Hardware Architecture

### Single LCD Design

All Stream Deck models (except the Pedal) use the same fundamental architecture:

1. **One continuous LCD panel** covers the entire key area
2. **A translucent silicone or membrane key grid** sits above the LCD
3. **An ARM microcontroller** manages USB communication, key scanning, and LCD driving
4. **The host sends individual key images** via HID output reports; the MCU composites them onto the correct LCD region

This means the "32 individual screens" marketing is somewhat misleading. The XL has one 1024x600 LCD, and the firmware places each 96x96 key image at the correct (x, y) offset on that panel. The gaps between keys are physical (the silicone grid), not display boundaries.

### Why This Matters for Custom Software

The single-LCD architecture enables the `0x08` (Full Screen Image) command -- you can send a single 1024x600 JPEG that fills the entire display, ignoring key boundaries entirely. This opens possibilities like:

- Displaying a single large image or dashboard across all 32 keys
- Drawing custom layouts that don't align to the 8x4 grid
- Rendering visualizations, graphs, or status displays

However, the key press detection still operates on the 8x4 grid, so input mapping always uses 32 discrete buttons regardless of what the display shows.

### Key Switch Types

Different models use different switch mechanisms:

| Switch Type | Models |
|---|---|
| Membrane (classic) | Stream Deck XL, Mini, Neo, all Modules |
| Scissor | Stream Deck MK.2 Scissor Keys variant |
| Membrane (MK.2) | Stream Deck MK.2 standard |
| Foot pedal (spring-loaded) | Stream Deck Pedal |

The XL uses membrane switches. These provide a soft, quiet press with moderate tactile feedback. They are not mechanical switches -- there is no click or defined actuation point.

### Power

The Stream Deck XL draws power from the USB bus. Reported measurements indicate approximately 8.4W of power consumption. This is within USB 3.0 specification limits (up to 4.5W at 5V/900mA for USB 3.0, but the device may negotiate higher power). A powered USB hub is recommended if connecting multiple Stream Deck devices to avoid power delivery issues.

The Stream Deck Studio is a notable outlier -- it supports PoE+ (802.3at Class 4, up to 25.5W) or USB-C Power Delivery (9V/3A, up to 27W), consuming up to 15W. This is the only Stream Deck that can run over Ethernet.

---

## Protocol Family Groupings

Not all Stream Deck models speak the same protocol. Elgato's HID API documentation groups devices into distinct protocol families. This is critical for custom software -- you cannot use XL commands on a Mini.

### Family: Legacy (Original V1)

- **PID:** `0x0060`
- **Not covered by official HID API documentation**
- **Image format:** Raw BMP
- **Report size:** 8191 bytes (vastly different from all other models)
- **Per-key resolution:** 72x72 px
- **Status:** Effectively deprecated; no longer sold

### Family: Mini

- **PIDs:** `0x0063`, `0x0090`, `0x00B3`, `0x00B8`
- **Image format:** BMP
- **Image rotation:** 90 degrees clockwise
- **Per-key resolution:** 80x80 px
- **LCD resolution:** 320x240 px
- **Input report size:** 65 bytes
- **Output report size:** 1024 bytes
- **Feature report max:** 32 bytes

### Family: Classic (MK.2 / Original V2)

- **PIDs:** `0x006D`, `0x0080`, `0x00A5`, `0x00B9`
- **Image format:** JPEG
- **Image rotation:** 180 degrees
- **Per-key resolution:** 72x72 px
- **LCD resolution:** 480x272 px (low DPI)

### Family: XL

- **PIDs:** `0x006C`, `0x008F`, `0x00BA`
- **Image format:** JPEG
- **Image rotation:** 180 degrees
- **Per-key resolution:** 96x96 px
- **LCD resolution:** 1024x600 px (high DPI)
- **Output report size:** 1024 bytes
- **Key commands:** `0x07` (key image), `0x08` (full screen), `0x0D` (background)

### Family: Neo

- **PID:** `0x009A`
- **Image format:** JPEG
- **Image rotation:** 180 degrees
- **Per-key resolution:** 96x96 px
- **LCD resolution:** 480x320 px (high DPI)
- **Extra:** Info bar window (248x58 px), 2 touch sensors (96x16 px each, mapped as buttons)

### Family: Plus

- **PID:** `0x0084`
- **Image format:** JPEG
- **Image rotation:** None
- **Per-key resolution:** 120x120 px
- **LCD resolution:** 800x480 px (high DPI)
- **Touch strip:** 800x100 px
- **Rotary encoders:** 4

### Family: Plus XL

- **PID:** `0x00C6`
- **Image format:** JPEG
- **Image rotation:** 90 degrees counter-clockwise
- **Per-key resolution:** 112x112 px
- **LCD resolution:** 1280x800 px (high DPI)
- **Touch strip:** 1200x100 px
- **Rotary encoders:** 6

### Family: Studio

- **PID:** `0x00AA`
- **Connection:** RJ45 PoE+ or USB-C PD
- **Rotary encoders:** 2 (with LED ring)
- **NFC:** Yes
- **HID API documentation:** Not publicly available at time of research

### Family: Pedal

- **PID:** `0x0086`
- **No display** -- input only
- **3 foot pedals** with customizable spring tension
- **USB 2.0**

---

## Specifications That Matter for Custom Software

If you own a Stream Deck XL and want to write custom software to control it directly via HID, here is what you need to know:

### Device Detection

```
Vendor ID:  0x0FD9
Product IDs: 0x006C (V1), 0x008F (V2), 0x00BA (Module 32)
```

Scan for all three PIDs. Use your platform's HID enumeration API (hidapi, node-hid, WebHID, etc.) to find the device.

### Image Pipeline

1. Render a 96x96 pixel image (any format your graphics library supports)
2. Rotate the image 180 degrees (flip both horizontally and vertically)
3. Encode as JPEG
4. Send via HID output report `0x02`, command `0x07`, specifying the key index (0-31)
5. If the JPEG exceeds 1016 bytes (the payload capacity per report), chunk it across multiple reports with sequence numbers

### Key Numbering

Keys are numbered 0-31, left to right, top to bottom:

```
 0  1  2  3  4  5  6  7
 8  9 10 11 12 13 14 15
16 17 18 19 20 21 22 23
24 25 26 27 28 29 30 31
```

### Polling for Input

Read HID input reports (ID `0x01`) with a recommended polling interval of 50ms. Each input report contains the press/release state of all 32 keys simultaneously, enabling multi-key detection.

### Brightness Control

Use feature report `0x03` (setter) to adjust LCD brightness from 0-100%. Note that the relationship between the set percentage and perceived brightness is non-linear (i.e., 50% does not look like half brightness).

### Full-Screen Mode

Command `0x08` accepts a 1024x600 JPEG that fills the entire LCD. This bypasses individual key regions and renders a single image across the whole display. Key input still reports as 32 discrete buttons.

### Critical Gotchas

1. **Images MUST be rotated 180 degrees before sending.** If your keys display upside-down, you forgot this step.
2. **JPEG quality affects transfer speed.** Lower quality = smaller file = fewer HID reports needed = faster update. For rapid animations, use aggressive JPEG compression.
3. **The device is USB 2.0 HID regardless of the USB-C connector.** Maximum theoretical throughput is limited by HID report rates, not USB bandwidth. Updating all 32 keys takes measurable time.
4. **The Original V1 (PID `0x0060`) is a completely different protocol.** Do not assume V1 code works on V2+ devices or vice versa.
5. **No authentication or pairing required.** Any process that can open the HID device can control it. There is no handshake beyond standard USB HID enumeration.
6. **Elgato's Stream Deck software claims exclusive access on some platforms.** You may need to quit the official app (or configure it) before your custom software can open the HID device.

---

## Sources

1. [Elgato Stream Deck XL Product Page](https://www.elgato.com/us/en/p/stream-deck-xl) -- Physical specs, price, package contents
2. [Elgato Stream Deck HID API -- Introduction](https://docs.elgato.com/streamdeck/hid/intro/) -- Official HID documentation overview
3. [Elgato Stream Deck HID API -- Stream Deck XL](https://docs.elgato.com/streamdeck/hid/stream-deck-xl/) -- Official XL protocol specs, PIDs, LCD resolution
4. [Elgato Stream Deck HID API -- Stream Deck Classic](https://docs.elgato.com/streamdeck/hid/stream-deck-classic/) -- Classic/MK.2 protocol specs
5. [Elgato Stream Deck HID API -- Stream Deck Mini](https://docs.elgato.com/streamdeck/hid/mini/) -- Mini protocol specs
6. [Elgato Stream Deck HID API -- Stream Deck Neo](https://docs.elgato.com/streamdeck/hid/stream-deck-neo/) -- Neo protocol specs
7. [Elgato Stream Deck HID API -- Stream Deck +](https://docs.elgato.com/streamdeck/hid/stream-deck-plus/) -- Plus protocol specs
8. [Elgato Stream Deck HID API -- Stream Deck + XL](https://docs.elgato.com/streamdeck/hid/stream-deck-plus-xl/) -- Plus XL protocol specs
9. [Elgato Stream Deck HID API -- General Reference](https://docs.elgato.com/streamdeck/hid/general/) -- Common protocol elements, report sizes
10. [Elgato Stream Deck HID API -- Module 15/32 Keys](https://docs.elgato.com/streamdeck/hid/module-15_32/) -- Module protocol documentation
11. [Elgato Stream Deck Module 32-Key Product Page](https://www.elgato.com/us/en/p/stream-deck-module-32-keys) -- Module 32 specs
12. [Elgato Stream Deck Module 15-Key Product Page](https://www.elgato.com/us/en/p/stream-deck-module-15-keys) -- Module 15 specs
13. [Elgato Stream Deck Module 6-Key Product Page](https://www.elgato.com/us/en/p/stream-deck-module-6-keys) -- Module 6 specs
14. [Elgato Stream Deck + XL Product Page](https://www.elgato.com/us/en/p/stream-deck-plus-xl) -- Plus XL physical specs
15. [Elgato Stream Deck SDK -- Devices](https://docs.elgato.com/streamdeck/sdk/guides/devices/) -- Device type IDs, all models
16. [python-elgato-streamdeck ProductIDs.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/ProductIDs.py) -- Complete PID reference
17. [python-elgato-streamdeck DeviceManager.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/DeviceManager.py) -- PID-to-device-class mapping
18. [python-elgato-streamdeck StreamDeckXL.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckXL.py) -- XL device constants
19. [python-elgato-streamdeck StreamDeckMini.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckMini.py) -- Mini device constants
20. [python-elgato-streamdeck StreamDeckOriginal.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckOriginal.py) -- Original device constants
21. [DeviceHunt -- Elgato USB Devices](https://devicehunt.com/view/type/usb/vendor/0FD9) -- USB VID/PID database
22. [Stream Deck HID Protocol Notes (Cliff Rowley)](https://gist.github.com/cliffrowley/d18a9c4569537b195f2b1eb6c68469e0) -- Reverse engineering notes, V1 protocol
23. [Elgato Stream Deck Device Comparison](https://www.elgato.com/us/en/explorer/products/stream-deck/stream-deck-device-comparison/) -- Official model comparison
24. [iFixit -- Elgato Stream Deck Classic Teardown](https://www.ifixit.com/Teardown/Elgato+Stream+Deck+Classic+Teardown/194063) -- Hardware teardown
25. [Elgato Stream Deck Pedal Product Page](https://www.elgato.com/us/en/p/stream-deck-pedal) -- Pedal specs
