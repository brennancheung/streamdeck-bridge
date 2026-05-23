# Throughput and Latency

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Image Update Performance

---

## Key Findings

- The Stream Deck XL uses USB 2.0 High-Speed (480 Mbps) with a 1024-byte OUT interrupt endpoint, giving a theoretical raw throughput of ~1 MB/s for image data
- A typical 96x96 JPEG button image is 2-5 KB, requiring 2-5 HID report packets to transfer
- Transferring a single key image takes approximately 0.25-0.65 ms at the USB level; all 32 keys can be updated in roughly 8-20 ms (theoretical)
- Dashboard-style updates every 1-5 seconds are trivially achievable with massive headroom
- Full 30 fps animation across all 32 buttons simultaneously is theoretically possible but practically constrained by JPEG encoding overhead and OS scheduling
- The bottleneck is typically host-side JPEG encoding and HID driver scheduling, not USB bandwidth

---

## 1. USB Interface Characteristics

### Device Identification

The Stream Deck XL identifies as USB Vendor ID `0x0fd9`, Product ID `0x006c` (Elgato Systems GmbH). It enumerates as a USB 2.0 HID device.

### USB Speed Classification

Despite being marketed with a "USB 3.0" interface and using a USB-C connector, the Stream Deck XL operates as a **USB 2.0 High-Speed device** (480 Mbps). The `bcdUSB` descriptor reports `2.00`. This is common for HID devices -- they don't need USB 3.0 SuperSpeed bandwidth and the HID specification was designed around USB 2.0.

The device draws up to 500 mA (`MaxPower: 500mA`) and supports Bus Powered with Remote Wakeup.

### Endpoint Descriptors (from lsusb)

The Stream Deck XL exposes two interrupt endpoints:

| Endpoint | Direction | Transfer Type | wMaxPacketSize | bInterval |
|----------|-----------|---------------|----------------|-----------|
| 0x81     | IN        | Interrupt      | 512 bytes      | 1         |
| 0x02     | OUT       | Interrupt      | 1024 bytes     | 1         |

These are critical numbers:

- **OUT endpoint (0x02)**: Used for sending images to the device. The 1024-byte max packet size confirms this is a high-speed endpoint (full-speed USB limits interrupt endpoints to 64 bytes).
- **IN endpoint (0x81)**: Used for receiving button press events. The 512-byte report carries key state data.
- **bInterval = 1**: For high-speed USB, this means 2^(1-1) = 1 microframe = 125 microseconds between transactions.

### Theoretical USB Bandwidth

With a 1024-byte OUT endpoint polled every microframe (125 us):

```
1024 bytes/microframe x 8000 microframes/second = 8,192,000 bytes/second = ~7.8 MB/s
```

However, the USB specification limits periodic transfers (interrupt + isochronous) to 80% of available microframe bandwidth on high-speed buses. The practical ceiling is therefore around **6.2 MB/s** for this endpoint. In reality, only one transaction per microframe is needed for most image transfers, giving a working throughput of approximately **1 MB/s** with typical HID driver scheduling overhead.

---

## 2. Image Format and Size

### Resolution and Format

The Stream Deck XL uses **96x96 pixel** key images in **JPEG format**, with no rotation but both horizontal and vertical mirroring required before upload. This is larger than the original Stream Deck's 72x72 pixels.

| Device              | Key Resolution | Image Format | Rotation | Mirroring     |
|---------------------|----------------|--------------|----------|---------------|
| Stream Deck (v1)    | 72x72          | BMP (raw)    | 0 deg    | H+V           |
| Stream Deck (v2)    | 72x72          | JPEG         | 0 deg    | H+V           |
| Stream Deck Mini    | 80x80          | BMP          | 90 deg   | H+V           |
| **Stream Deck XL**  | **96x96**      | **JPEG**     | **0 deg**| **H+V**       |
| Stream Deck +       | 120x120        | JPEG         | 0 deg    | H+V           |

### Typical JPEG File Sizes

A 96x96 pixel JPEG image size varies with content complexity and compression quality:

| Content Type           | Quality | Typical Size | Packets Needed |
|------------------------|---------|--------------|----------------|
| Solid color / simple   | 80%     | ~900 bytes   | 1              |
| Icon with text         | 80%     | 2-3 KB       | 2-3            |
| Photographic / complex | 80%     | 4-6 KB       | 4-6            |
| Pre-encoded black key  | N/A     | ~915 bytes   | 1              |

For reference, a 120x120 JPEG (used by the Stream Deck +) is approximately 7.2 KB and requires 8 packets. The XL's 96x96 images are proportionally smaller.

The raw uncompressed size of a 96x96 RGB image would be 27,648 bytes (96 x 96 x 3). JPEG at quality 80 typically achieves 8:1 to 12:1 compression, yielding 2.3-3.5 KB for typical button content.

---

## 3. HID Report Structure and Chunking

### Packet Format

Each image write uses a 1024-byte HID output report with an 8-byte header:

```
Offset  Type    Description
+0x00   UINT8   Report ID (0x02)
+0x01   UINT8   Command (0x07 = Update Key Image)
+0x02   UINT8   Key Index (0-31)
+0x03   UINT8   Transfer Done flag (0x01 = final chunk)
+0x04   UINT16  Chunk Contents Size (little-endian)
+0x06   UINT16  Chunk Index / Page Number (little-endian, zero-based)
+0x08   UINT8[] Chunk Data (up to 1016 bytes)
```

Key constants from the python-elgato-streamdeck library:

```
IMAGE_REPORT_LENGTH         = 1024 bytes (total report)
IMAGE_REPORT_HEADER_LENGTH  = 8 bytes
IMAGE_REPORT_PAYLOAD_LENGTH = 1016 bytes (1024 - 8)
```

### Chunking Logic

The image data is split into sequential 1016-byte chunks. For each chunk:

1. Calculate `this_length = min(bytes_remaining, 1016)`
2. Set the Transfer Done flag to 1 if `this_length == bytes_remaining`
3. Copy the chunk data starting at `page_number * 1016`
4. Pad the remainder of the 1024-byte report with zeros
5. Send via `hid_write()`
6. Increment `page_number`

A 3 KB JPEG requires 3 HID writes (1016 + 1016 + 968 bytes of payload). A 915-byte blank key image fits in a single write.

---

## 4. Transfer Time Calculations

### Single Key Update

For a typical 3 KB icon image:

| Step                          | Time Estimate          |
|-------------------------------|------------------------|
| JPEG encoding (96x96)        | 0.1-2 ms (varies)     |
| HID writes (3 packets)       | 0.375 ms (3 x 125 us) |
| Device JPEG decode + render  | ~1-2 ms (estimated)   |
| **Total per key**             | **~1.5-4 ms**         |

The USB transfer itself is fast: at 1024 bytes per 125-microsecond microframe, three packets take only 375 microseconds. The dominant latency factors are JPEG encoding on the host and JPEG decoding on the device.

### All 32 Keys Update

Assuming serial updates (which is how all known libraries operate):

| Scenario                    | Image Size | Packets | USB Transfer Time | Total (with encoding) |
|-----------------------------|------------|---------|-------------------|-----------------------|
| All blank/solid (best case) | ~1 KB each | 32      | 4 ms              | ~10-15 ms             |
| All typical icons           | ~3 KB each | ~96     | 12 ms             | ~50-130 ms            |
| All complex images          | ~5 KB each | ~160    | 20 ms             | ~80-200 ms            |

These are estimates based on:
- USB transfer: `packets x 125 us` per packet (one packet per microframe)
- JPEG encoding: 0.1 ms (jpeg-turbo native) to 2 ms (pure-Python jpeg-js) per image
- HID driver overhead and OS scheduling: adds 10-50% depending on platform

### Dashboard Update (1-5 second interval)

For updating all 32 keys once per second, the budget is 1000 ms. Even in the worst case (~200 ms for 32 complex images), this uses only **20% of the available time budget**. At 5-second intervals, utilization drops to 4%.

**Verdict: Dashboard-style updates every 1-5 seconds are trivially achievable.**

---

## 5. Frame Rate Analysis

### Can You Achieve 30 FPS?

The python-elgato-streamdeck library's animated example explicitly targets 30 FPS (`FRAMES_PER_SECOND = 30`). Here's the analysis:

**Time budget per frame at 30 FPS: 33.3 ms**

| Scenario                         | Time per Frame | Achievable at 30 FPS? |
|----------------------------------|----------------|-----------------------|
| Animate 1 key (simple)           | ~1.5 ms        | Yes, easily           |
| Animate 1 key (complex)          | ~4 ms          | Yes, easily           |
| Animate 8 keys (simple)          | ~12-15 ms      | Yes                   |
| Animate 15 keys (simple)         | ~25-30 ms      | Borderline            |
| Animate 32 keys (simple, native) | ~15-20 ms      | Yes, with native JPEG |
| Animate 32 keys (simple, Python) | ~60-130 ms     | No (7-15 FPS max)     |
| Animate 32 keys (complex)        | ~80-200 ms     | No (5-12 FPS max)     |

The library's animation code explicitly handles the case where the target FPS exceeds the device/host combination's capability:

> "sleep_interval can be a negative number when current FPS setting is too high for the combination of host and StreamDeck to handle. If this is the case, we skip sleeping and immediately render the next frame to try to catch up."

### Pre-rendering Optimization

The animated example pre-converts frames to the Stream Deck's native format so that JPEG encoding is done once upfront rather than per-frame. This is critical: with pre-rendered frames, the bottleneck shifts entirely to USB transfer time, and 30 FPS for a moderate number of keys becomes realistic.

### Language and Library Impact

JPEG encoding performance varies dramatically:

| Implementation         | Encoding Time (96x96) | Notes                          |
|------------------------|------------------------|--------------------------------|
| libjpeg-turbo (C/C++)  | ~0.1-0.3 ms           | Uses SIMD instructions         |
| jpeg-turbo (Node.js)   | ~0.2-0.5 ms           | Native binding to libjpeg-turbo|
| Pillow (Python)        | ~0.5-1.5 ms           | C extension to libjpeg         |
| jpeg-js (pure JS)      | ~2-5 ms               | Pure JavaScript, no SIMD       |
| Pure Python JPEG       | ~3-10 ms              | Extremely slow                 |

For Node.js users, the `@julusian/jpeg-turbo` package is strongly recommended for the Stream Deck XL. Without it, the library falls back to `jpeg-js`, which is "noticeably more CPU intensive and slower."

---

## 6. Bottleneck Analysis

### Where is the bottleneck?

The performance pipeline has four stages:

```
[Host: Image Generation] -> [Host: JPEG Encode] -> [USB: Transfer] -> [Device: JPEG Decode + LCD Write]
```

#### Stage 1: Image Generation (Host)

Rendering text, icons, or status indicators into a 96x96 pixel buffer. Typically 0.1-1 ms with Cairo/Pillow/Canvas. Not usually the bottleneck.

#### Stage 2: JPEG Encoding (Host) -- OFTEN THE BOTTLENECK

This is the most variable and often the slowest stage:
- With native libjpeg-turbo: 0.1-0.3 ms per image -- fast enough for 30 FPS on all 32 keys
- With pure-JavaScript or pure-Python encoders: 2-10 ms per image -- the primary bottleneck

For 32 keys at 30 FPS, you need to encode 960 images per second. At 0.2 ms each (native), that's 192 ms/s of CPU time (19.2% of one core). At 5 ms each (pure JS), that's 4800 ms/s -- impossible on a single core.

#### Stage 3: USB Transfer -- RARELY THE BOTTLENECK

With 1 MB/s effective throughput and ~3 KB per image, USB can handle:
- ~333 images per second
- At 32 keys x 30 FPS = 960 images/s needed -- this exceeds the estimate

However, the 1 MB/s figure is conservative. The endpoint can theoretically push 8000 packets/s of 1024 bytes. Even accounting for protocol overhead, ~4-6 MB/s is achievable with optimized HID drivers, giving headroom for 1300-2000 images/s.

More realistically, HID driver scheduling on most operating systems introduces gaps, and practical throughput falls in the **1-3 MB/s** range.

#### Stage 4: Device JPEG Decode + LCD Render -- UNKNOWN BUT LIKELY FAST

The Stream Deck uses an ARM microcontroller to receive JPEG data over USB, decode it, and write the result to the LCD. Elgato describes it as a "fast ARM microcontroller." Some STM32 family chips (commonly used in such devices) include hardware JPEG decoders that can decode a 96x96 JPEG in well under 1 ms.

There is no documented internal frame buffer delay. The device appears to render each key's image independently as soon as the complete JPEG is received (when the Transfer Done flag is set). There is no evidence of any vsync or frame-buffer swap mechanism -- images appear on individual keys as they are received.

### Bottleneck Summary

| Factor               | Impact    | Notes                                   |
|----------------------|----------|-----------------------------------------|
| JPEG encoding        | **High** | Primary bottleneck; use native libs     |
| USB transfer         | Medium   | Usually not limiting for dashboards     |
| Device-side decode   | Low      | Hardware-accelerated on ARM MCU         |
| OS HID scheduling    | Medium   | Varies by OS; can add 1-5 ms jitter    |
| Image generation     | Low      | Simple icons/text are sub-millisecond   |

---

## 7. HID Report Size Impact on Throughput

### Why 1024 Bytes Matters

The Stream Deck XL uses the maximum allowed HID interrupt endpoint packet size for USB 2.0 High-Speed: **1024 bytes**. This is significant because:

1. **Fewer packets per image**: A 3 KB image requires only 3 packets instead of 47 packets at the full-speed maximum of 64 bytes
2. **Less header overhead**: 8 bytes of header per 1016 bytes of payload = 0.8% overhead, vs. 8 bytes per 56 bytes = 14.3% overhead with 64-byte packets
3. **Higher throughput**: Each microframe can carry 1024 bytes instead of 64 bytes

### Comparison: Hypothetical Full-Speed vs Actual High-Speed

| Parameter                   | Full-Speed (hypothetical) | High-Speed (actual)      |
|-----------------------------|---------------------------|--------------------------|
| Max packet size             | 64 bytes                  | 1024 bytes               |
| Payload per packet          | 56 bytes (64 - 8 header)  | 1016 bytes (1024 - 8)    |
| Polling interval            | 1 ms                      | 125 us                   |
| Packets for 3 KB image      | 55                        | 3                        |
| Time for 1 image            | 55 ms                     | 0.375 ms                 |
| Time for 32 images          | 1760 ms                   | 12 ms                    |

If the Stream Deck XL were a full-speed device, updating all 32 keys would take nearly 2 seconds -- far too slow for any real-time use. The high-speed interface is essential.

### Evolution: V1 vs V2 Protocol

The original Stream Deck V1 used **8191-byte** transfers with raw BMP data (no JPEG compression). The V2 protocol switched to **1024-byte** transfers with JPEG compression. This actually increased efficiency despite the smaller packet size:

- V1: 15,552 bytes raw BMP / 8191-byte packets = 2 packets, but huge data (no compression)
- V2/XL: ~3,000 bytes JPEG / 1024-byte packets = 3 packets, much less total data

---

## 8. USB 2.0 vs USB 3.0 Connection

### Does USB 3.0 help?

**No, it does not meaningfully affect image transfer performance.** Here's why:

1. The Stream Deck XL is a **USB 2.0 device** (bcdUSB = 2.00). Even when plugged into a USB 3.0 port, it negotiates at USB 2.0 High-Speed (480 Mbps).

2. USB 3.0 ports may provide more stable power delivery (important since the device draws 500 mA), which can prevent brownout-related disconnects.

3. USB 3.0 hubs and controllers generally have better scheduling and lower latency than older USB 2.0 controllers, which may reduce OS-level HID scheduling jitter by a small amount.

4. Elgato's troubleshooting guidance recommends connecting to "direct motherboard USB 3.0 or 3.1 ports" to minimize input lag -- this is about avoiding shared USB 2.0 hub bandwidth, not about USB 3.0 speeds.

**Recommendation**: Use a direct USB 3.0/3.1 port on the motherboard for the most reliable performance, but don't expect a speed increase beyond what USB 2.0 High-Speed provides.

---

## 9. Polling Period and Update Limits

### The 50 ms Recommended Polling Period

Elgato's documentation recommends a **50 ms polling period** for input (button press) monitoring. This refers to reading button state -- **not** to image push rate. There is no documented rate limit for sending images to the device.

### Can You Push Images Faster Than 20 Hz?

Yes. The 50 ms recommendation applies to the IN endpoint (reading button presses). The OUT endpoint (sending images) operates independently:

- The OUT endpoint has `bInterval = 1` (one microframe = 125 us), allowing up to 8000 transactions per second
- Image writes are initiated by the host, not polled by the device
- There is no known firmware rate limiter on incoming image data

### Practical Limits

The practical limit is determined by:
1. How fast you can encode JPEGs
2. How fast the HID driver processes write requests
3. Whether the device's microcontroller can keep up with JPEG decoding

In practice, the device appears to handle whatever the host can send without dropping frames or requiring flow control. The "Transfer Done" flag mechanism ensures the device knows when a complete image has been received.

---

## 10. Real-World Performance Expectations

### Dashboard Use Case (1-5 Second Updates)

Updating all 32 keys every 1-5 seconds for a status dashboard:

```
32 keys x 3 KB/key = 96 KB per full refresh
At ~1 MB/s USB throughput: ~100 ms per full refresh
Available time budget at 1 Hz: 1000 ms
Utilization: ~10%
```

**Status: Easily achievable.** You could update all 32 keys several times per second with room to spare. The device will be idle most of the time.

Optimization for dashboards:
- Only update keys whose content has actually changed (selective update)
- Use simple icons/text that compress to small JPEGs (~1-2 KB)
- Pre-render JPEG data for common states

### Animation Use Case (30 FPS)

Animating all 32 keys at 30 FPS:

```
32 keys x 30 fps = 960 image writes per second
960 x 3 KB = 2.88 MB/s sustained USB throughput needed
960 x 0.2 ms = 192 ms/s JPEG encoding (with native encoder)
```

**Status: Theoretically possible with pre-rendered frames and native JPEG encoding, but challenging in practice.** The USB bandwidth is likely the hard constraint at ~2.88 MB/s needed vs ~1-3 MB/s achievable.

Realistic expectations:
- **1-4 keys at 30 FPS**: Easily achievable in any language
- **8-15 keys at 30 FPS**: Achievable with native JPEG encoder and pre-rendered frames
- **32 keys at 30 FPS**: Only with pre-rendered frames, native encoder, and optimized HID driver. May achieve 15-25 FPS in practice.
- **32 keys at 10-15 FPS**: Reliably achievable with reasonable optimization

### Selective Update Strategy

The most effective pattern for real-time displays is selective updating:

1. Maintain a "dirty" flag per key
2. Only re-render and transfer keys whose data has changed
3. For a typical dashboard, maybe 1-5 keys change per update cycle
4. This reduces the workload to 1-5 image transfers per cycle, easily within budget even at 10+ Hz

---

## 11. Recommendations for Real-Time Status Displays

### Architecture

```
[Data Source] -> [Change Detection] -> [Image Generation] -> [JPEG Encode] -> [HID Write]
                      |
                 Only update
                 changed keys
```

### Language Choice Impact

| Language    | JPEG Encoding | HID Access   | Practical FPS (32 keys) |
|-------------|---------------|--------------|-------------------------|
| C/C++       | libjpeg-turbo | libhidapi    | 20-30 FPS               |
| Rust        | image crate   | hidapi crate | 20-30 FPS               |
| Go          | image/jpeg    | hidapi       | 15-25 FPS               |
| Node.js     | jpeg-turbo    | node-hid     | 15-25 FPS               |
| Node.js     | jpeg-js       | node-hid     | 5-10 FPS                |
| Python      | Pillow        | hidapi       | 8-15 FPS                |
| Pure Python | jpeg-js equiv | pure hid     | 3-7 FPS                 |

### Key Optimizations

1. **Use native JPEG encoding** (libjpeg-turbo or equivalent). This is the single most impactful optimization.
2. **Pre-render static content**. Cache JPEG-encoded images for states that don't change.
3. **Selective updates only**. Never re-send an image that hasn't changed.
4. **Use pre-encoded blank/default images**. The library includes a ~915-byte pre-encoded black JPEG for the XL.
5. **Batch encoding in parallel**. JPEG encoding is CPU-bound and embarrassingly parallel.
6. **Use the recommended HID backend**. The `hid` Python library is recommended over `hidapi` for reliability and performance.

---

## Sources

1. [Elgato Stream Deck HID API - General Reference](https://docs.elgato.com/streamdeck/hid/general/) - Official protocol documentation
2. [Elgato Stream Deck HID API - Classic](https://docs.elgato.com/streamdeck/hid/stream-deck-classic/) - Classic model protocol details
3. [Elgato Stream Deck HID API - Module 15/32](https://docs.elgato.com/streamdeck/hid/module-15_32/) - Module protocol details
4. [Notes on the Stream Deck HID Protocol (GitHub Gist)](https://gist.github.com/cliffrowley/d18a9c4569537b195f2b1eb6c68469e0) - Community reverse-engineering notes
5. [python-elgato-streamdeck - StreamDeckXL.py](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckXL.py) - Python library source for XL
6. [python-elgato-streamdeck - Animated Example](https://python-elgato-streamdeck.readthedocs.io/en/stable/examples/animated.html) - 30 FPS animation example
7. [python-elgato-streamdeck - GitHub](https://github.com/abcminiuser/python-elgato-streamdeck) - Main Python library repo
8. [node-elgato-stream-deck - GitHub](https://github.com/Julusian/node-elgato-stream-deck) - Node.js library with jpeg-turbo recommendation
9. [Stream Deck XL - Issue #25 (USB descriptors)](https://github.com/abcminiuser/python-elgato-streamdeck/issues/25) - USB endpoint descriptor dump
10. [Stream Deck XL Performance - Issue #36](https://github.com/abcminiuser/python-elgato-streamdeck/issues/36) - Performance discussion on Raspberry Pi
11. [USB in a NutShell - Endpoint Types](https://www.beyondlogic.org/usbnutshell/usb4.shtml) - USB transfer type specifications
12. [Reverse Engineering the Stream Deck (den.dev)](https://den.dev/blog/reverse-engineering-stream-deck/) - Wireshark packet analysis
13. [Elgato Stream Deck XL Technical Specifications](https://help.elgato.com/hc/en-us/articles/18170516758157-Elgato-Stream-Deck-XL-Technical-Specifications) - Official specs
14. [Programming a Stream Deck with Java](https://spinscale.de/posts/2025-02-11-programming-an-elgato-streamdeck-with-java-part-1.html) - Java implementation with packet sizes
15. [Elgato Stream Deck Icon Specs](https://docs.elgato.com/makers/stream-deck/icon-packs/icon-specs/) - Official image format requirements
16. [dh1tw/streamdeck (Go library)](https://github.com/dh1tw/streamdeck) - Go implementation
17. [Stream Deck Icon Sizes (GitHub Gist)](https://gist.github.com/krabs-github/d7c163a4725a7c66c5861b0dcd0ee3c1) - Per-device icon dimensions
18. [Elgato Stream Deck HID API - Introduction](https://docs.elgato.com/streamdeck/hid/intro/) - HID API overview
