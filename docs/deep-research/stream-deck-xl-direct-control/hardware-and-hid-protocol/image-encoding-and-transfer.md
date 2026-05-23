# Image Encoding and Transfer

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Hardware & HID Protocol

---

## Key Findings

- The Stream Deck XL accepts **96x96 pixel JPEG images** per key, sent over USB HID in **1024-byte reports** with an 8-byte header and 1016-byte payload per chunk.
- Images must be **flipped both horizontally and vertically** (equivalent to 180-degree rotation) before encoding to JPEG and sending.
- Buttons are addressed individually by **zero-based key index** (0-31), and you can update any single button independently.
- The protocol uses a simple chunked transfer: each chunk carries a **page index**, **chunk size**, and a **"transfer done" flag** on the final chunk.
- JPEG quality should be set to **100** to avoid visible compression artifacts on the small LCD screens.
- There is **no double-buffering or vsync mechanism** -- the device renders each chunk as it arrives, making transfer speed the bottleneck for animations.
- A typical 96x96 JPEG at quality 100 is roughly 2-5 KB, requiring only 3-5 HID reports per button update.
- The older Original (V1) used a completely different protocol: **BMP format**, **8191-byte reports**, **16-byte headers**, and **1-based key indexing**.

---

## 1. Image Format Requirements by Model

### Stream Deck XL (the focus of this document)

| Property | Value |
|---|---|
| Key resolution | 96 x 96 pixels |
| Image format | JPEG |
| Color space | RGB (standard JPEG) |
| Image flip | Horizontal + Vertical (both axes) |
| Image rotation | 0 degrees (flip handles orientation) |
| Key count | 32 (8 columns x 4 rows) |
| USB Product ID | 0x006c |
| HID report size | 1024 bytes |

### Comparison Across Models

| Model | Resolution | Format | Report Size | Flip/Rotate | Keys |
|---|---|---|---|---|---|
| Original V1 | 72x72 | BMP | 8191 bytes | Flip H+V | 15 (5x3) |
| Original V2 / MK.2 | 72x72 | JPEG | 1024 bytes | Flip H+V | 15 (5x3) |
| Mini | 80x80 | BMP | 1024 bytes | Rotate 90 CW | 6 (3x2) |
| XL | 96x96 | JPEG | 1024 bytes | Flip H+V | 32 (8x4) |
| Plus | 120x120 | JPEG | 1024 bytes | None | 8 (4x2) |

The critical takeaway: the XL, Original V2, MK.2, and Plus all share the same **1024-byte report, JPEG-based protocol** (command 0x07). The Original V1 and Mini use older, incompatible protocols.

---

## 2. Image Preparation Pipeline

Before any image can be sent to the Stream Deck XL, it must go through this pipeline:

### Step 1: Resize to 96x96

The source image must be exactly 96x96 pixels. Use high-quality resampling (Lanczos) when scaling.

### Step 2: Flip on Both Axes

The Stream Deck XL LCD panels are physically mounted upside-down relative to the logical key layout. You must flip the image both horizontally and vertically before encoding. This is equivalent to rotating the image 180 degrees, but implementations typically use two separate flip operations:

```
Original Image    -->    Flip H+V    -->    What gets sent
  A B                      D C              (device displays
  C D                      B A               it right-side-up)
```

In Python PIL terms:
```python
image = image.transpose(Image.FLIP_LEFT_RIGHT)   # horizontal flip
image = image.transpose(Image.FLIP_TOP_BOTTOM)    # vertical flip
```

### Step 3: Encode to JPEG

Encode the flipped image as JPEG with **quality=100**. Lower quality settings (e.g., the PIL default of 75) cause visible compression artifacts, especially with high-contrast content like colored text on dark backgrounds.

```python
import io
buffer = io.BytesIO()
image.save(buffer, "JPEG", quality=100)
jpeg_bytes = buffer.getvalue()
```

### Why Quality 100?

At 96x96 pixels, a quality-100 JPEG is still very small (typically 2-5 KB depending on image complexity). The file-size savings from lower quality are negligible, while the artifacts are clearly visible on the LCD. The python-elgato-streamdeck library switched to quality 100 in v0.7.0 specifically to fix user-reported artifact issues.

### Typical JPEG Sizes at Quality 100

| Content type | Approximate size | HID reports needed |
|---|---|---|
| Solid color / simple icon | 1-2 KB | 2-3 |
| Text on colored background | 3-5 KB | 4-6 |
| Photographic / complex | 5-10 KB | 6-11 |
| Maximum theoretical (96x96) | ~27 KB | ~27 |

---

## 3. HID Report Structure for Image Transfer

### Output Report Format (Stream Deck XL)

Every image data packet is a **1024-byte output report** written via HID WRITE. The report consists of an 8-byte header followed by up to 1016 bytes of JPEG payload, padded with zeros to fill the full 1024 bytes.

```
Total report: 1024 bytes
 +---------+------------------+----------+
 | Header  |   JPEG payload   | Zero pad |
 | 8 bytes | up to 1016 bytes | to 1024  |
 +---------+------------------+----------+
```

### Header Format (8 bytes)

| Offset | Size | Field | Description |
|---|---|---|---|
| +0x00 | UINT8 | Report ID | Always `0x02` |
| +0x01 | UINT8 | Command | `0x07` = Update Key Image |
| +0x02 | UINT8 | Key Index | 0-31 (zero-based) |
| +0x03 | UINT8 | Is Last Chunk | `0x00` = more chunks follow, `0x01` = final chunk |
| +0x04 | UINT16 LE | Chunk Size | Number of JPEG bytes in this chunk (little-endian) |
| +0x06 | UINT16 LE | Chunk Index | Zero-based page/chunk number (little-endian) |

### Byte-Level Example: Sending a 3000-byte JPEG to Key 5

A 3000-byte JPEG requires 3 chunks (1016 + 1016 + 968 bytes):

**Chunk 0** (first, 1016 bytes of payload):
```
Offset: 00 01 02 03 04 05 06 07 08 09 ...
Data:   02 07 05 00 F8 03 00 00 [1016 bytes of JPEG data]
                                 [zero-padded to 1024 total]

02 = Report ID
07 = Command: Update Key Image
05 = Key index 5
00 = Not the last chunk
F8 03 = 0x03F8 = 1016 (chunk size, little-endian)
00 00 = Chunk index 0
```

**Chunk 1** (second, 1016 bytes of payload):
```
Offset: 00 01 02 03 04 05 06 07 08 09 ...
Data:   02 07 05 00 F8 03 01 00 [1016 bytes of JPEG data]
                                 [zero-padded to 1024 total]

02 = Report ID
07 = Command: Update Key Image
05 = Key index 5
00 = Not the last chunk
F8 03 = 1016 (chunk size)
01 00 = Chunk index 1
```

**Chunk 2** (final, 968 bytes of payload):
```
Offset: 00 01 02 03 04 05 06 07 08 09 ...
Data:   02 07 05 01 C8 03 02 00 [968 bytes of JPEG data]
                                 [zero-padded to 1024 total]

02 = Report ID
07 = Command: Update Key Image
05 = Key index 5
01 = LAST chunk (transfer done)
C8 03 = 0x03C8 = 968 (chunk size)
02 00 = Chunk index 2
```

### Constants

```
IMAGE_REPORT_LENGTH         = 1024   // Total HID report size
IMAGE_REPORT_HEADER_LENGTH  = 8      // Header size
IMAGE_REPORT_PAYLOAD_LENGTH = 1016   // 1024 - 8, max JPEG data per chunk
```

---

## 4. The Complete Transfer Algorithm

Here is the complete chunking and transfer algorithm, derived from the python-elgato-streamdeck reference implementation:

```python
def set_key_image(key, jpeg_data, device):
    """
    Send a JPEG image to a specific key on the Stream Deck XL.
    
    Args:
        key: Key index 0-31
        jpeg_data: Pre-encoded JPEG bytes (96x96, flipped, quality=100)
        device: HID device handle
    """
    IMAGE_REPORT_LENGTH = 1024
    IMAGE_REPORT_HEADER_LENGTH = 8
    IMAGE_REPORT_PAYLOAD_LENGTH = IMAGE_REPORT_LENGTH - IMAGE_REPORT_HEADER_LENGTH
    
    page_number = 0
    bytes_remaining = len(jpeg_data)
    
    while bytes_remaining > 0:
        # Calculate how much JPEG data goes in this chunk
        this_length = min(bytes_remaining, IMAGE_REPORT_PAYLOAD_LENGTH)
        bytes_sent = page_number * IMAGE_REPORT_PAYLOAD_LENGTH
        
        # Build the 8-byte header
        header = bytes([
            0x02,                              # Report ID
            0x07,                              # Command: Update Key Image
            key,                               # Key index (0-based)
            0x01 if this_length == bytes_remaining else 0x00,  # Is last chunk?
            this_length & 0xFF,                # Chunk size low byte
            this_length >> 8,                  # Chunk size high byte
            page_number & 0xFF,                # Chunk index low byte
            page_number >> 8,                  # Chunk index high byte
        ])
        
        # Assemble: header + jpeg slice + zero padding
        payload = header + jpeg_data[bytes_sent:bytes_sent + this_length]
        padding = bytearray(IMAGE_REPORT_LENGTH - len(payload))
        
        # Write the complete 1024-byte report
        device.write(payload + padding)
        
        bytes_remaining -= this_length
        page_number += 1
```

### Key observations about the algorithm:

1. **No acknowledgment**: The host sends chunks consecutively without waiting for device acknowledgment. HID WRITE is fire-and-forget at the application level.
2. **Zero padding**: Every report is padded to exactly 1024 bytes regardless of actual payload size.
3. **Chunk index is zero-based**: Unlike the Original V1 which used 1-based page numbers.
4. **Key index is zero-based**: Unlike the Original V1 which used 1-based key indices.
5. **The "is last" flag** tells the device the transfer is complete and it can render the image.

---

## 5. Button Addressing and Layout

### Stream Deck XL: 32 Keys in 8x4 Grid

Key indices are **zero-based**, numbered left-to-right, top-to-bottom:

```
 +----+----+----+----+----+----+----+----+
 |  0 |  1 |  2 |  3 |  4 |  5 |  6 |  7 |   Row 0
 +----+----+----+----+----+----+----+----+
 |  8 |  9 | 10 | 11 | 12 | 13 | 14 | 15 |   Row 1
 +----+----+----+----+----+----+----+----+
 | 16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 |   Row 2
 +----+----+----+----+----+----+----+----+
 | 24 | 25 | 26 | 27 | 28 | 29 | 30 | 31 |   Row 3
 +----+----+----+----+----+----+----+----+
```

To convert between row/column and key index:
```
key_index = row * 8 + col
row = key_index // 8
col = key_index % 8
```

### No Key Conversion Needed

Unlike the Original V1, which numbered keys right-to-left (requiring a conversion function), the XL uses straightforward left-to-right ordering. The Original V1 had this quirky layout:

```
Original V1 key indices (right-to-left!):
 +----+----+----+----+----+
 |  4 |  3 |  2 |  1 |  0 |
 +----+----+----+----+----+
 |  9 |  8 |  7 |  6 |  5 |
 +----+----+----+----+----+
 | 14 | 13 | 12 | 11 | 10 |
 +----+----+----+----+----+
```

The XL eliminated this oddity.

---

## 6. Full Screen Image Transfer

The Stream Deck XL also supports writing a **full-screen image** that spans the entire LCD panel behind all keys, with resolution **1024 x 600 pixels**.

### Full Screen Command: 0x08

| Offset | Size | Field | Description |
|---|---|---|---|
| +0x00 | UINT8 | Report ID | `0x02` |
| +0x01 | UINT8 | Command | `0x08` = Update Full Screen |
| +0x02 | UINT8 | Reserved | `0x00` |
| +0x03 | UINT8 | Is Last Chunk | `0x00` or `0x01` |
| +0x04 | UINT16 LE | Chunk Size | Bytes in this chunk |
| +0x06 | UINT16 LE | Chunk Index | Zero-based |
| +0x08 | UINT8[] | Chunk Data | JPEG payload |

The full-screen command is nearly identical to the key image command, but uses command byte `0x08` instead of `0x07`, and the key index field at offset +0x02 is reserved (set to 0x00).

A 1024x600 JPEG can be much larger (50-200 KB+), requiring many more chunks. This is slower and primarily useful for splash screens or static backgrounds, not rapid updates.

---

## 7. Performance Characteristics

### Transfer Speed Analysis

The Stream Deck XL connects via **USB 2.0** (despite some marketing mentioning USB 3.0, the HID protocol operates at USB 2.0 Full Speed rates for control transfers).

| Parameter | Value |
|---|---|
| HID report size | 1024 bytes |
| USB Full Speed interrupt endpoint | 64 bytes/ms max (for standard HID) |
| Practical per-report overhead | ~1-2 ms (including USB scheduling) |
| Typical JPEG size (quality 100) | 2-5 KB |
| Reports per key update | 3-5 |
| Estimated time per key update | ~5-15 ms |
| Theoretical max key updates/sec | ~66-200 per second |

### Practical Performance Considerations

1. **Individual key updates are fast**: Updating a single key takes roughly 5-15 ms end-to-end, making it viable for status displays that change every second or so.

2. **Full-deck refresh is the bottleneck**: Updating all 32 keys sequentially requires 32 x (3-5 reports) = ~96-160 reports. At ~1-2 ms per report, a full refresh takes approximately 100-300 ms, limiting full-deck animation to roughly 3-10 fps.

3. **JPEG encoding time matters**: On a Raspberry Pi or low-power device, the CPU time to encode 32 JPEG images can exceed the USB transfer time. The node-elgato-stream-deck project recommends using `jpeg-turbo` for hardware-accelerated JPEG encoding on the XL.

4. **Selective updates are critical**: Only update keys whose content has actually changed. Pre-render and cache JPEG data for static content.

5. **There is no vsync or double-buffering**: The device renders each key as soon as the final chunk arrives. If you update multiple keys sequentially, there may be visible "tearing" where some keys show the new state while others still show the old state.

### Strategies for Animation

For status displays with occasional updates:
- Pre-render all possible states as JPEG byte arrays
- Only send updates when state actually changes
- Update the most important keys first

For animation (e.g., progress bars, waveforms):
- Target 5-10 fps for a pleasing animation on a single key
- Reduce JPEG quality to 80-90 to shrink transfer size (acceptable for animated content)
- Use simpler images (solid colors, gradients) which compress to smaller JPEGs
- Consider updating only a subset of keys per frame

---

## 8. The Original V1 BMP Protocol (Historical Comparison)

Understanding the older protocol helps contextualize the XL's design and avoid confusion with legacy code.

### Original V1 Key Differences

| Property | Original V1 | XL (and V2 devices) |
|---|---|---|
| Image format | BMP (24-bit, BGR) | JPEG |
| Report size | 8191 bytes | 1024 bytes |
| Header size | 16 bytes | 8 bytes |
| Key indexing | 1-based | 0-based |
| Key ordering | Right-to-left | Left-to-right |
| Command byte | 0x01 | 0x07 |
| Chunks per image | Always 2 | Variable (depends on JPEG size) |
| Image data size | 15,552 bytes (fixed) | Variable (JPEG compressed) |

### V1 Header Format (16 bytes)

```
Byte: 00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15
Data: 02 01 PP 00 FL KI 00 00 00 00 00 00 00 00 00 00

02 = Report ID
01 = Command (set key image, V1 style)
PP = Page number (1-based: 0x01, 0x02)
FL = Is final page (0x00 or 0x01)
KI = Key index (1-based: 0x01 through 0x0F)
```

### V1 BMP Image Structure

The V1 sends raw BMP data including a 54-byte BMP file header:
- Bytes 0-1: `0x42 0x4D` ("BM" magic)
- Bytes 2-5: File size (15,606 bytes = 54 header + 15,552 pixel data)
- Bytes 10-13: Pixel data offset (54)
- Bytes 14-17: DIB header size (40)
- Bytes 18-21: Width (72)
- Bytes 22-25: Height (72)
- Bytes 28-29: Bits per pixel (24)
- Pixel data: BGR format, 3 bytes per pixel, 72x72 = 15,552 bytes

The total image payload is always 15,606 bytes, split across exactly 2 reports of 8,191 bytes each:
- Report 1: 16-byte header + 7,749 bytes of BMP data + zero padding to 8,191
- Report 2: 16-byte header + 7,803 bytes of BMP data + zero padding to 8,191

### Why the Switch to JPEG?

The BMP protocol was inefficient:
- Fixed 15.6 KB per image regardless of complexity
- 31.2 KB total transfer for 2 reports (mostly padding)
- A solid-color icon is the same size as a photograph

JPEG compression typically reduces a 96x96 icon to 2-5 KB, making transfers 3-8x faster and enabling reasonable animation on the larger XL with its 32 keys.

---

## 9. Related Commands

### Reset Key Stream (Clear Stuck State)

If a transfer is interrupted, the key may be in a partial-receive state. Send a reset:

```
Payload: 02 00 00 ... (1024 bytes, all zeros except byte 0)
```

This is simply a 1024-byte report with `0x02` as the Report ID and all other bytes zero.

### Set Brightness

Controls LCD backlight brightness (affects all keys):

```
Feature Report (32 bytes):
03 08 PP 00 00 ... (padded to 32 bytes)

03 = Feature Report ID
08 = Set Brightness command
PP = Brightness percentage (0x00 to 0x64, i.e., 0-100)
```

### Device Reset

Resets the device to factory state:

```
Feature Report (32 bytes):
03 02 00 00 ... (padded to 32 bytes)
```

---

## 10. Implementation Checklist

When implementing Stream Deck XL image transfer from scratch:

1. **Open the HID device** -- Find the device with Vendor ID `0x0FD9` (Elgato) and Product ID `0x006C` (Stream Deck XL).

2. **Prepare the image**:
   - Resize to exactly 96x96 pixels (use Lanczos resampling)
   - Flip horizontally, then flip vertically
   - Encode as JPEG with quality=100

3. **Chunk the JPEG data**:
   - Split into 1016-byte chunks
   - Build 8-byte header for each chunk
   - Pad each report to exactly 1024 bytes

4. **Send via HID WRITE**:
   - Write each chunk sequentially
   - Set "is last" flag on the final chunk
   - No acknowledgment needed -- just send them

5. **Handle errors**:
   - If a transfer seems stuck, send the reset key stream command
   - Validate key index is 0-31 before sending

### Common Pitfalls

- **Forgetting the flip**: Image will appear upside-down on the device
- **Using low JPEG quality**: Visible artifacts, especially on text/icons
- **Not padding to 1024 bytes**: Device may reject short reports
- **Using 1-based indexing**: That was the V1 protocol; the XL uses 0-based
- **Sending BMP data**: The XL only accepts JPEG
- **Not setting the "is last" flag**: The device will wait indefinitely for more chunks

---

## 11. USB Product IDs Reference

For device detection and differentiation:

| Model | Vendor ID | Product ID | Protocol Family |
|---|---|---|---|
| Stream Deck Original V1 | 0x0FD9 | 0x0060 | Legacy (BMP, 8191-byte) |
| Stream Deck Original V2 | 0x0FD9 | 0x006D | Modern (JPEG, 1024-byte) |
| Stream Deck Original MK.2 | 0x0FD9 | 0x0080 | Modern |
| Stream Deck Mini | 0x0FD9 | 0x0063 | Mini (BMP, 1024-byte, own commands) |
| Stream Deck XL | 0x0FD9 | 0x006C | Modern |
| Stream Deck Plus | 0x0FD9 | 0x0084 | Modern |

---

## Sources

1. [Elgato Stream Deck HID API - General Reference](https://docs.elgato.com/streamdeck/hid/general/) -- Official protocol documentation covering report structure and common concepts.
2. [Elgato Stream Deck HID API - Stream Deck Classic](https://docs.elgato.com/streamdeck/hid/stream-deck-classic/) -- Official docs for Classic/XL family with command tables and key image protocol.
3. [Elgato Stream Deck HID API - Stream Deck XL](https://docs.elgato.com/streamdeck/hid/stream-deck-xl/) -- Official XL-specific documentation with resolution, layout, and rotation requirements.
4. [Elgato Stream Deck HID API - Mini](https://docs.elgato.com/streamdeck/hid/mini/) -- Mini protocol for comparison (BMP-based, different rotation).
5. [Elgato Stream Deck HID API - Module 15/32](https://docs.elgato.com/streamdeck/hid/module-15_32/) -- Module protocol documentation.
6. [Notes on the Stream Deck HID protocol (Cliff Rowley)](https://gist.github.com/cliffrowley/d18a9c4569537b195f2b1eb6c68469e0) -- Reverse-engineered V1 protocol notes with byte-level packet details.
7. [python-elgato-streamdeck - StreamDeckXL.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckXL.py) -- Reference implementation of XL image transfer in Python.
8. [python-elgato-streamdeck - StreamDeckOriginal.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckOriginal.py) -- Reference implementation of Original V1 BMP protocol.
9. [python-elgato-streamdeck - PILHelper.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/ImageHelpers/PILHelper.py) -- Image preparation pipeline (resize, flip, JPEG encode).
10. [node-elgato-stream-deck (Julusian)](https://github.com/Julusian/node-elgato-stream-deck) -- Node.js/TypeScript implementation with JPEG-turbo performance recommendations.
11. [dh1tw/streamdeck (Go)](https://pkg.go.dev/github.com/dh1tw/streamdeck) -- Go implementation with device config structs showing protocol differences per model.
12. [JPEG compression artifacts issue (streamdeck-ui #52)](https://github.com/timothycrosley/streamdeck-ui/issues/52) -- User reports and fix for JPEG quality issues, confirming quality=100 recommendation.
13. [Stream Deck XL Performance (python-elgato-streamdeck #36)](https://github.com/abcminiuser/python-elgato-streamdeck/issues/36) -- Real-world performance discussion for multi-key updates.
