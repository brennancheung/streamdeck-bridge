# HID Protocol Deep Dive

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Hardware & HID Protocol

---

## Key Findings

- The Stream Deck XL uses standard USB HID with three report types: Input (device-to-host, 512 bytes max), Output (host-to-device, 1024 bytes max), and Feature (bidirectional, 32 bytes fixed)
- **Full N-key rollover is supported**: the device reports a complete state bitmap of all 32 buttons on every change -- each button gets one byte (0x00=released, 0x01=pressed), so all 32 buttons can be detected simultaneously
- The device reports **both press and release events** as state transitions -- every time any button changes state, the entire 32-button state array is sent
- There is **no hold/repeat behavior** at the HID level; the device only sends reports on state changes, not continuously while held
- Input reports use Report ID 0x01 with a 4-byte header followed by 32 state bytes (36 bytes total for XL)
- Recommended host polling interval is 50ms; the device only sends data when state changes occur
- Images must be JPEG format, rotated 180 degrees before upload, sent in 1016-byte chunks via Output reports
- The Stream Deck Mini uses a **completely different HID protocol** from all other models (different report IDs, BMP images, different header format)
- Elgato has published official HID documentation at docs.elgato.com/streamdeck/hid/ covering all models

---

## 1. Device Identification

### USB Vendor and Product IDs

All Stream Deck devices share Vendor ID **0x0FD9** (Elgato Systems GmbH).

| Model | Product ID | Keys | Layout | Key Image Size |
|-------|-----------|------|--------|---------------|
| Stream Deck Original (V1) | 0x0060 | 15 | 5x3 | 72x72 px |
| Stream Deck Original (2019) | 0x006D | 15 | 5x3 | 72x72 px |
| Stream Deck MK.2 | 0x0080 | 15 | 5x3 | 72x72 px |
| Stream Deck MK.2 Scissor | 0x00A5 | 15 | 5x3 | 72x72 px |
| **Stream Deck XL** | **0x006C** | **32** | **8x4** | **96x96 px** |
| Stream Deck XL (2022) | 0x008F | 32 | 8x4 | 96x96 px |
| Stream Deck Mini | 0x0063 | 6 | 3x2 | 80x80 px |
| Stream Deck Neo | 0x009A | 8 + 2 sensors | 4x2 | 96x96 px |
| Stream Deck + | 0x0084 | 8 + 4 dials | 4x2 | 120x120 px |
| Stream Deck Module 15 | 0x00B9 | 15 | 5x3 | 72x72 px |
| Stream Deck Module 32 | 0x00BA | 32 | 8x4 | 96x96 px |

### Stream Deck XL Physical Specifications

- **Key Matrix**: 8 columns x 4 rows (32 LCD keys)
- **LCD Display**: 1024 x 600 pixels total
- **Individual Key Image**: 96 x 96 pixels
- **Connection**: USB 2.0 HID
- **Endpoints**: 2 (IN and OUT interrupt endpoints)

---

## 2. HID Report Architecture

Stream Deck communication uses three HID report types. Every report begins with a single-byte Report ID.

### 2.1 Input Reports (Device to Host)

Input reports carry events from the device to the host (button presses, encoder rotations, touch events on devices that support them).

- **Maximum size**: 512 bytes
- **Report ID**: 0x01
- **Delivery**: Interrupt IN endpoint; host polls with HID READ at a recommended interval of 50ms
- **Behavior**: A TIMEOUT response from HID READ indicates no state change occurred

### 2.2 Output Reports (Host to Device)

Output reports carry bulk data from the host to the device (images, firmware).

- **Maximum size**: 1024 bytes
- **Report ID**: 0x02
- **Delivery**: Interrupt OUT endpoint
- **Padding**: The host MUST pad the remaining bytes with zeroes up to the full 1024-byte report size

### 2.3 Feature Reports (Bidirectional)

Feature reports carry configuration commands and device queries.

- **Fixed size**: 32 bytes (padded with zeroes)
- **Report IDs**: 0x03 (setters), 0x04-0x0A (getters)
- **Sending**: HID SEND FEATURE REPORT
- **Reading**: HID GET FEATURE REPORT

### Report Structure Summary

```
Every report: [Report ID (1 byte)] [Payload (variable)]

Input:   [0x01] [Command (1)] [Payload Length (2, UINT16 LE)] [Payload...]
Output:  [0x02] [Command (1)] [Payload...]
Feature: [Report ID] [Command (1)] [Payload...] (padded to 32 bytes)
```

---

## 3. Input Report Format: Key State Events

This is the most critical section for understanding the event model.

### 3.1 Input Report Structure (Command 0x00 -- Key/Button State Change)

When any button is pressed or released, the device sends an input report containing the **complete state of every button**:

```
Byte offset:
+0x00  Report ID     UINT8    Always 0x01
+0x01  Command       UINT8    0x00 (key/button state change)
+0x02  Payload Len   UINT16   Number of buttons (little-endian)
+0x04  Key 0 State   UINT8    0x00 = released, 0x01 = pressed
+0x05  Key 1 State   UINT8    0x00 = released, 0x01 = pressed
...
+0x23  Key 31 State  UINT8    0x00 = released, 0x01 = pressed
```

For the Stream Deck XL, the total input report is **36 bytes** (4-byte header + 32 key state bytes).

### 3.2 Example Packets

**No keys pressed (idle state -- only sent on release transition):**
```
01 00 20 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00
```

**Key 0 (top-left) pressed:**
```
01 00 20 00 01 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00
```

**Key 11 pressed:**
```
01 00 20 00 00 00 00 00 00 00 00 01 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00
```

**Keys 0, 5, and 31 pressed simultaneously:**
```
01 00 20 00 01 00 00 00 00 01 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 01
```

### 3.3 Header Byte Breakdown

| Byte | Field | Value (XL) | Notes |
|------|-------|-----------|-------|
| 0x00 | Report ID | 0x01 | Always 0x01 for input reports |
| 0x01 | Command | 0x00 | Key/button state change command |
| 0x02 | Payload Length Lo | 0x20 | Low byte of key count (32 = 0x20) |
| 0x03 | Payload Length Hi | 0x00 | High byte of key count |

The payload length field (bytes 2-3) tells you how many buttons the device has. For XL this is 0x0020 (32). For the 15-key models it would be 0x000F (15). This byte also tells software how many state bytes follow the header.

---

## 4. N-Key Rollover and Simultaneous Press Detection

### 4.1 Full N-Key Rollover: Confirmed

The Stream Deck XL supports **complete N-key rollover**. All 32 buttons can be pressed and detected simultaneously. This is an inherent property of the protocol design:

- Each button has its own dedicated byte in the state report
- The device sends the **entire button state bitmap** on every state change
- There is no matrix scanning limitation because each key has an independent switch circuit
- No ghosting or blocking is possible

### 4.2 How Simultaneous Presses Are Reported

Simultaneous button presses are **NOT reported as separate events**. Instead:

1. When button A is pressed: one report is sent with A=0x01, all others=0x00
2. While A is held, button B is pressed: one report is sent with A=0x01, B=0x01, all others=0x00
3. While both held, button A is released: one report is sent with A=0x00, B=0x01, all others=0x00

The device reports the **complete instantaneous state** -- not individual transitions. Software must diff the current report against the previous report to determine which buttons changed.

### 4.3 State Change Only

The device sends input reports **only when a state change occurs**. It does NOT:
- Send periodic/continuous reports while buttons are held
- Implement key repeat at the HID level
- Generate separate "press" and "release" events as distinct event types

Instead, it sends a single event type (Command 0x00) containing the full state. The distinction between press and release is determined by comparing the previous state to the new state:

```
Previous: Key5=0x00 (released)
Current:  Key5=0x01 (pressed)   --> This is a PRESS event for Key5

Previous: Key5=0x01 (pressed)
Current:  Key5=0x00 (released)  --> This is a RELEASE event for Key5
```

### 4.4 Implications for Software Design

- You MUST maintain a copy of the previous button state
- On each input report, diff current vs. previous to detect individual press/release transitions
- Multiple keys can change in a single report (e.g., two keys pressed at exactly the same polling interval)
- There is no "key down" vs "key up" report ID distinction -- it is all state-based

---

## 5. Polling and Timing

### 5.1 Recommended Polling Rate

Elgato's official documentation recommends a **50ms polling interval** (20 Hz) for HID READ calls. This is a software-level recommendation for the host application.

### 5.2 USB-Level Timing

The Stream Deck XL is a USB 2.0 Full-Speed HID device with interrupt endpoints. The actual USB polling rate is determined by the `bInterval` field in the endpoint descriptor:

- For full-speed devices: bInterval specifies the maximum polling interval in milliseconds
- Typical values for HID devices range from 1ms to 10ms
- The USB host controller handles the actual polling at the bus level

### 5.3 Non-Blocking Reads

The host should use **non-blocking (timed) HID READ** calls. If no state change has occurred since the last report, the read will return a TIMEOUT rather than data. This is the expected idle behavior.

---

## 6. Key Index Mapping and Layout

### 6.1 Stream Deck XL Key Layout (8x4 Grid)

Keys are indexed 0-31, laid out left-to-right, top-to-bottom:

```
+----+----+----+----+----+----+----+----+
|  0 |  1 |  2 |  3 |  4 |  5 |  6 |  7 |
+----+----+----+----+----+----+----+----+
|  8 |  9 | 10 | 11 | 12 | 13 | 14 | 15 |
+----+----+----+----+----+----+----+----+
| 16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 |
+----+----+----+----+----+----+----+----+
| 24 | 25 | 26 | 27 | 28 | 29 | 30 | 31 |
+----+----+----+----+----+----+----+----+
```

### 6.2 Original Stream Deck Key Ordering (Important Difference)

The original V1 Stream Deck (PID 0x0060) uses a **mirrored** key ordering -- keys are numbered right-to-left within each row. The XL and all V2+ devices use left-to-right ordering.

The V1 key remapping formula (from the python library):
```python
def _convert_key_id_origin(self, key):
    key_col = key % KEY_COLS       # Column within row
    return (key - key_col) + ((KEY_COLS - 1) - key_col)  # Horizontal flip
```

The XL does NOT need this transformation -- its key indices match the physical layout directly.

---

## 7. Feature Reports: Device Control

### 7.1 Setter Feature Reports (Report ID 0x03)

All setter feature reports use Report ID 0x03, followed by a command byte and payload. The report is always padded to 32 bytes with zeroes.

#### Set Brightness (Command 0x08)

Controls LCD backlight brightness via PWM.

```
Byte:  03 08 XX 00 00 00 00 00 00 00 00 00 00 00 00 00
       00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

| Offset | Value | Description |
|--------|-------|-------------|
| +0x00 | 0x03 | Report ID |
| +0x01 | 0x08 | Brightness command |
| +0x02 | 0x00-0x64 | Brightness percentage (0-100) |

Setting brightness to 0 effectively turns off the display (sleep-like state). Lower values have a disproportionately larger dimming effect due to PWM characteristics.

**Note on V1 protocol**: The original Stream Deck used a different brightness format:
```
05 55 AA D1 01 XX 00 00 00 00 00 00 00 00 00 00 00
```
Where byte 5 (XX) is the percentage. This format does NOT apply to the XL.

#### Show Logo (Command 0x02)

Forces the display of the boot/Elgato logo. Also used as a soft reset.

```
Byte:  03 02 00 00 00 00 00 00 00 00 00 00 00 00 00 00
       00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

#### Fill LCD with Color (Command 0x05)

Fills the entire LCD with a solid RGB color.

```
Byte:  03 05 RR GG BB 00 00 00 00 00 00 00 00 00 00 00
       00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

| Offset | Field | Type |
|--------|-------|------|
| +0x02 | Red | UINT8 (0-255) |
| +0x03 | Green | UINT8 (0-255) |
| +0x04 | Blue | UINT8 (0-255) |

#### Fill Single Key with Color (Command 0x06)

Fills a single key with a solid RGB color.

```
Byte:  03 06 KK RR GG BB 00 00 00 00 00 00 00 00 00 00
       00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

| Offset | Field | Type |
|--------|-------|------|
| +0x02 | Key Index | UINT8 (0-31 for XL) |
| +0x03 | Red | UINT8 |
| +0x04 | Green | UINT8 |
| +0x05 | Blue | UINT8 |

#### Set Sleep Mode Duration (Command 0x0D)

Sets the idle timeout before the device enters sleep mode.

```
Byte:  03 0D SS SS SS SS 00 00 00 00 00 00 00 00 00 00
       00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

| Offset | Field | Type | Notes |
|--------|-------|------|-------|
| +0x02 | Duration | INT32 (LE) | Seconds until sleep; 0 = disable auto-sleep |

#### Show Background by Index (Command 0x13) -- XL Specific

Displays a previously stored background image by index.

```
Byte:  03 13 II 00 00 00 00 00 00 00 00 00 00 00 00 00
       00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

| Offset | Field | Type |
|--------|-------|------|
| +0x02 | Background Index | UINT8 |

### 7.2 Getter Feature Reports

Getter feature reports are requested via HID GET FEATURE REPORT with the appropriate Report ID. The device responds with the requested data.

#### Get Firmware Version (Report IDs 0x04, 0x05, 0x07)

Three separate firmware version queries exist:

```
Request:  Send GET FEATURE REPORT with Report ID 0x05
Response: [0x05] [Data Length (1)] [Checksum (4, UINT32 LE)] [Version String (8, ASCII)]
```

| Offset | Field | Type |
|--------|-------|------|
| +0x00 | Report ID | UINT8 |
| +0x01 | Data Length | UINT8 |
| +0x02 | Checksum | UINT32 (LE) |
| +0x06 | Version String | CHAR[8] (ASCII) |

From the python library, the version string begins at byte 6:
```python
version = self.device.read_feature(0x05, 32)
version_string = extract_string(version[6:])
```

**Note on V1 protocol**: The original Stream Deck used Report ID 0x04 with format:
```
04 55 AA D4 04 [VERSION_STRING...]
```
Version string at byte 5. This does NOT apply to the XL.

#### Get Serial Number (Report ID 0x06)

```
Request:  Send GET FEATURE REPORT with Report ID 0x06
Response: [0x06] [Data Length (1)] [Serial String (ASCII)]
```

| Offset | Field | Type | Notes |
|--------|-------|------|-------|
| +0x00 | Report ID | 0x06 | |
| +0x01 | Data Length | UINT8 | Typically 0x0C or 0x0E |
| +0x02 | Serial String | CHAR[] | ASCII serial number |

From the python library:
```python
serial = self.device.read_feature(0x06, 32)
serial_string = extract_string(serial[2:])
```

**Note on V1 protocol**: The original Stream Deck used Report ID 0x03:
```
03 55 AA D3 03 [SERIAL_BYTES...]
```
Serial at byte 5. This does NOT apply to the XL.

#### Get Unit Information (Report ID 0x08)

Returns comprehensive hardware metadata about the device.

```
Request:  Send GET FEATURE REPORT with Report ID 0x08
```

| Offset | Field | Type | XL Value |
|--------|-------|------|----------|
| +0x00 | Report ID | UINT8 | 0x08 |
| +0x01 | Keypad Rows | UINT8 | 4 |
| +0x02 | Keypad Columns | UINT8 | 8 |
| +0x03 | Key Width | UINT16 (LE) | 96 |
| +0x05 | Key Height | UINT16 (LE) | 96 |
| +0x07 | LCD Width | UINT16 (LE) | 1024 |
| +0x09 | LCD Height | UINT16 (LE) | 600 |
| +0x0B | Image BPP | UINT8 | Bits per pixel |
| +0x0C | Color Scheme | UINT8 | Image color format |
| +0x0D | Key Gallery Count | UINT8 | Stored key images |
| +0x0E | LCD Gallery Count | UINT8 | Stored LCD images |
| +0x0F | Demo Frames | UINT8 | Demo animation frames |
| +0x10 | Reserved | UINT8 | |

This report is extremely useful for writing model-agnostic software -- it tells you everything about the device's capabilities.

#### Get Sleep Duration (Report ID 0x0A)

```
Request:  Send GET FEATURE REPORT with Report ID 0x0A
Response: [0x0A] [Data Length (1)] [Duration (4, INT32 LE)]
```

---

## 8. Output Reports: Image Transfer Protocol

### 8.1 Image Requirements for Stream Deck XL

- **Format**: JPEG (not BMP, not raw RGB)
- **Size per key**: 96 x 96 pixels
- **Full LCD size**: 1024 x 600 pixels
- **CRITICAL**: Images must be **rotated 180 degrees** before upload (required for all V2 models)
- **Report size**: 1024 bytes total per output report

### 8.2 Output Report Header (8 bytes for XL)

All output reports for the XL use Report ID 0x02 with an 8-byte header:

```
Byte:  02 CC KK DD LL LL PP PP [image data...] [zero padding to 1024]
```

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| +0x00 | Report ID | UINT8 | Always 0x02 |
| +0x01 | Command | UINT8 | 0x07=key, 0x08=LCD, 0x0D=background |
| +0x02 | Key Index | UINT8 | 0-31 for keys; reserved for LCD |
| +0x03 | Done Flag | UINT8 | 0x01 if this is the last chunk |
| +0x04 | Chunk Size | UINT16 (LE) | Bytes of image data in this chunk |
| +0x06 | Chunk Index | UINT16 (LE) | Zero-based chunk sequence number |
| +0x08 | Chunk Data | UINT8[] | JPEG image data |

### 8.3 Chunked Transfer Example

A 96x96 JPEG image is typically 3-8 KB. With 1016 bytes of payload per chunk (1024 - 8 byte header), a typical key image requires 3-8 chunks.

```
Chunk 0:  02 07 05 00 F8 03 00 00 [1016 bytes of JPEG data]
Chunk 1:  02 07 05 00 F8 03 01 00 [1016 bytes of JPEG data]
Chunk 2:  02 07 05 01 A0 02 02 00 [672 bytes of JPEG data] [344 bytes zero padding]
```

In this example:
- Key index 5 is being updated
- Chunks 0 and 1 have Done=0x00 (more chunks follow)
- Chunk 2 has Done=0x01 (final chunk)
- Chunk sizes are 0x03F8 (1016) for full chunks, 0x02A0 (672) for the last
- Each report is padded to exactly 1024 bytes

### 8.4 Output Report Commands

| Command | Function | Key Index Field |
|---------|----------|----------------|
| 0x07 | Update individual key image | Key index (0-31) |
| 0x08 | Update full LCD screen | Reserved (0x00) |
| 0x0D | Store background image | Background index |

### 8.5 Key Stream Reset

To reset the image transfer state, send a 1024-byte report with only the Report ID set:

```python
payload = bytearray(1024)
payload[0] = 0x02
device.write(payload)
```

---

## 9. Initialization and Communication Workflow

### 9.1 Connection Sequence

1. **Enumerate USB HID devices** -- scan for VID 0x0FD9 with a known PID
2. **Open the HID device** -- platform-specific HID open call
3. **Query device information** (optional but recommended):
   - GET FEATURE REPORT 0x06 -- serial number
   - GET FEATURE REPORT 0x05 -- firmware version
   - GET FEATURE REPORT 0x08 -- unit information (rows, cols, key size, LCD size)
4. **Set initial brightness** -- SEND FEATURE REPORT 0x03 with command 0x08
5. **Upload key images** -- Output reports with command 0x07
6. **Begin polling loop** -- HID READ at 50ms intervals

### 9.2 Polling Loop

```
while running:
    data = hid_read(device, timeout=50ms)
    if data is not None:
        parse_key_states(data)
        diff_with_previous_state()
        fire_callbacks()
```

### 9.3 Disconnection Sequence

1. Optionally show the Elgato logo (SEND FEATURE REPORT 0x03, command 0x02)
2. Close the HID device handle
3. Release any platform-specific resources

### 9.4 No Handshake Required

There is **no handshake or initialization protocol** required before communication. Once the HID device is opened, you can immediately:
- Read input reports (button states)
- Send feature reports (brightness, queries)
- Send output reports (images)

The device is ready to communicate as soon as the USB HID connection is established.

---

## 10. Protocol Differences Between Models

### 10.1 Protocol Families

Stream Deck devices fall into three protocol families:

**Family 1: "V1" Original Protocol**
- Devices: Stream Deck Original (PID 0x0060)
- Input report: 1-byte header + key states (no command byte, no payload length)
- Image format: BMP, raw BGR pixels
- Image transfer: 8191-byte reports, 16-byte headers
- Key numbering: Right-to-left within each row (horizontally mirrored)
- Feature reports: Different Report IDs and header format (0x05 55 AA D1 01 pattern)

**Family 2: "V2" Main Protocol (XL uses this)**
- Devices: Stream Deck MK.2, XL, XL 2022, Neo, Plus, Modules
- Input report: 4-byte header (Report ID + Command + Payload Length) + key states
- Image format: JPEG
- Image transfer: 1024-byte reports, 8-byte headers
- Key numbering: Left-to-right, top-to-bottom
- Feature reports: Report ID 0x03 for setters, 0x04-0x0A for getters

**Family 3: Mini Protocol**
- Devices: Stream Deck Mini (PID 0x0063) and Mini variants
- Input report: 1-byte header + key states (similar to V1 but different details)
- Image format: BMP exclusively
- Image rotation: 90 degrees clockwise (NOT 180 degrees like V2)
- Report IDs and commands differ from both V1 and V2
- Does NOT support full LCD updates (key images only)

### 10.2 Key Differences Table

| Feature | V1 Original | V2 (XL, MK.2, etc.) | Mini |
|---------|------------|---------------------|------|
| Input header bytes | 1 | 4 | 1 |
| Output report size | 8191 | 1024 | 1024 |
| Output header size | 16 | 8 | 16 |
| Image format | BMP/BGR | JPEG | BMP |
| Image rotation | 180 deg | 180 deg | 90 deg CW |
| Key order | Right-to-left | Left-to-right | Left-to-right |
| Brightness Report ID | 0x05 | 0x03 | Different |
| Serial Report ID | 0x03 | 0x06 | Different |
| Firmware Report ID | 0x04 | 0x05 | Different |
| Reset Report ID | 0x0B | 0x03 | Different |

### 10.3 V1 vs V2 Input Report Comparison

**V1 Original (15 keys):**
```
Read 16 bytes: [Report ID (1)] [Key0..Key14 states (15)]
No command byte, no payload length field.
```

**V2 XL (32 keys):**
```
Read 36 bytes: [Report ID (1)] [Command (1)] [PayloadLen (2)] [Key0..Key31 states (32)]
Command = 0x00 for key state change.
```

### 10.4 Stream Deck Plus Extended Events

The Stream Deck Plus (PID 0x0084) extends the V2 protocol with additional input report commands:

**Encoder Events (Command 0x03):**
- Contents Type 0x00: Encoder button press/release (one byte per encoder)
- Contents Type 0x01: Encoder rotation (INT8 per encoder; positive=CW, negative=CCW)

**Touch Events (Command 0x02):**
- Contents Type 0x01: Tap (X, Y coordinates as UINT16)
- Contents Type 0x02: Long press (same format as tap)
- Contents Type 0x03: Flick (start X/Y, end X/Y coordinates)

```
Encoder rotation report:
01 03 LLLL 01 [INT8 per encoder]
     ^cmd  ^contents type

Touch tap report:
01 02 LLLL 01 00 XXXX YYYY
     ^cmd  ^tap    ^X    ^Y
```

### 10.5 Stream Deck Neo Extended Elements

The Neo (PID 0x009A) adds 2 capacitive touch sensors below the main 8-button grid, reported as buttons 8 and 9 in the key state array. It also has a 248x58 pixel info strip that can be updated via Command 0x0B.

---

## 11. Data Type Conventions

All multi-byte integer values in the Stream Deck protocol are **little-endian**:

| Type | Size | Notes |
|------|------|-------|
| UINT8 / BYTE | 1 byte | Unsigned 0-255 |
| INT8 / CHAR | 1 byte | Signed -128 to 127 (used for ASCII and encoder rotation) |
| UINT16 | 2 bytes | Little-endian unsigned |
| INT16 | 2 bytes | Little-endian signed |
| UINT32 | 4 bytes | Little-endian unsigned |
| INT32 | 4 bytes | Little-endian signed (used for sleep duration) |
| RGB Triplet | 3 bytes | R at +0x00, G at +0x01, B at +0x02 |

---

## 12. Practical Implementation Notes

### 12.1 Reading Key States (XL-Specific Code)

Based on the python-elgato-streamdeck library implementation:

```python
KEY_COUNT = 32

def read_control_states(device):
    """Read a key state report from the Stream Deck XL."""
    states = device.read(4 + KEY_COUNT)  # 36 bytes total
    if states is None:
        return None  # Timeout -- no state change
    
    # Skip the 4-byte header (Report ID, Command, Payload Length)
    key_states = states[4:]
    
    # Each byte is 0x00 (released) or 0x01 (pressed)
    return [bool(s) for s in key_states]
```

### 12.2 Detecting Press and Release Events

```python
previous_states = [False] * 32

def process_state_change(new_states):
    global previous_states
    for key in range(32):
        if new_states[key] and not previous_states[key]:
            on_key_down(key)
        elif not new_states[key] and previous_states[key]:
            on_key_up(key)
    previous_states = new_states[:]
```

### 12.3 Setting Brightness

```python
def set_brightness(device, percent):
    """Set LCD brightness. percent: 0-100."""
    payload = bytearray(32)
    payload[0] = 0x03  # Report ID
    payload[1] = 0x08  # Brightness command
    payload[2] = max(0, min(100, percent))
    device.write_feature(payload)
```

### 12.4 Uploading a Key Image

```python
IMAGE_REPORT_LENGTH = 1024
IMAGE_HEADER_LENGTH = 8
IMAGE_PAYLOAD_LENGTH = 1016  # 1024 - 8

def set_key_image(device, key_index, jpeg_data):
    """Upload a JPEG image to a key. Image must be 96x96, rotated 180 deg."""
    page_number = 0
    bytes_remaining = len(jpeg_data)
    
    while bytes_remaining > 0:
        this_length = min(bytes_remaining, IMAGE_PAYLOAD_LENGTH)
        bytes_sent = page_number * IMAGE_PAYLOAD_LENGTH
        is_last = (this_length == bytes_remaining)
        
        header = bytes([
            0x02,                       # Report ID
            0x07,                       # Command: update key image
            key_index,                  # Key index (0-31)
            0x01 if is_last else 0x00,  # Done flag
            this_length & 0xFF,         # Chunk size (low byte)
            this_length >> 8,           # Chunk size (high byte)
            page_number & 0xFF,         # Chunk index (low byte)
            page_number >> 8,           # Chunk index (high byte)
        ])
        
        chunk = jpeg_data[bytes_sent:bytes_sent + this_length]
        padding = bytes(IMAGE_REPORT_LENGTH - len(header) - len(chunk))
        
        device.write(header + chunk + padding)
        
        bytes_remaining -= this_length
        page_number += 1
```

### 12.5 Querying Device Information

```python
def get_serial_number(device):
    response = device.read_feature(0x06, 32)
    return bytes(response[2:]).decode('ascii').rstrip('\x00')

def get_firmware_version(device):
    response = device.read_feature(0x05, 32)
    return bytes(response[6:]).decode('ascii').rstrip('\x00')

def get_unit_info(device):
    response = device.read_feature(0x08, 32)
    return {
        'rows': response[1],
        'cols': response[2],
        'key_width': response[3] | (response[4] << 8),
        'key_height': response[5] | (response[6] << 8),
        'lcd_width': response[7] | (response[8] << 8),
        'lcd_height': response[9] | (response[10] << 8),
    }
```

### 12.6 Reset / Show Logo

```python
def reset_to_logo(device):
    """Reset the device to show the Elgato boot logo."""
    payload = bytearray(32)
    payload[0] = 0x03  # Report ID
    payload[1] = 0x02  # Show Logo command
    device.write_feature(payload)
```

---

## 13. V1 Protocol Reference (Original Stream Deck, PID 0x0060)

Included for completeness since older documentation and code frequently references this protocol.

### 13.1 V1 Feature Reports

**Reset (Report ID 0x0B):**
```
0B 63 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

**Set Brightness (Report ID 0x05):**
```
05 55 AA D1 01 [PERCENT] 00 00 00 00 00 00 00 00 00 00 00
```

**Get Serial (Report ID 0x03):**
```
Response: 03 55 AA D3 03 [SERIAL_STRING...]
```

**Get Firmware Version (Report ID 0x04):**
```
Response: 04 55 AA D4 04 [VERSION_STRING...]
```

### 13.2 V1 Image Transfer

- Report size: 8191 bytes
- Header size: 16 bytes
- Image format: BMP (54-byte BMP header + raw BGR pixel data)
- Pixel order: Bottom-up (standard BMP format)
- Key images: 72x72 pixels, 15,552 bytes raw (72 * 72 * 3 bytes BGR)
- Images split across 2 packets per key
- Key index is 1-based (not 0-based)

**V1 Image Header:**
```
02 01 [PAGE] 00 [DONE] [KEY+1] 00 00 00 00 00 00 00 00 00 00
```

---

## 14. USB HID Descriptor Details

### 14.1 Stream Deck XL USB Configuration

```
Vendor ID:   0x0FD9 (Elgato Systems GmbH)
Product ID:  0x006C (Stream Deck XL)
USB Version: 2.0
Device Class: 0x00 (defined at interface level)
Interface Class: 0x03 (HID)
Endpoints: 2 (IN + OUT interrupt)
HID Report Descriptor Length: 177 bytes
```

### 14.2 Endpoint Configuration

The device exposes two interrupt endpoints:
- **IN endpoint**: Device-to-host (input reports / key states)
- **OUT endpoint**: Host-to-device (output reports / images)

Feature reports travel over the control endpoint (endpoint 0), using SET_REPORT and GET_REPORT HID class requests.

---

## 15. Reverse Engineering Methodology

For those who want to explore the protocol further, the community has established proven methods:

### 15.1 USB Traffic Capture

1. Install the Elgato Stream Deck software in a virtual machine
2. Set up USB packet capture (Wireshark + USBPcap on Windows, or usbmon on Linux)
3. Filter traffic: `usb.idVendor == 0x0fd9 && usb.idProduct == 0x006c`
4. Interact with the device through the official software
5. Analyze the captured packets to understand command sequences

### 15.2 Linux Device Inspection

```bash
# List USB devices and find Stream Deck
lsusb | grep 0fd9

# Get detailed descriptor information
lsusb -v -d 0fd9:006c

# Check HID device nodes
ls -la /dev/hidraw*
```

---

## Sources

1. [Elgato Stream Deck HID API - Introduction](https://docs.elgato.com/streamdeck/hid/intro/) -- Official Elgato documentation, overview of HID communication architecture
2. [Elgato Stream Deck HID API - General Reference](https://docs.elgato.com/streamdeck/hid/general/) -- Official protocol reference with report types, commands, and byte layouts
3. [Elgato Stream Deck HID API - Stream Deck XL](https://docs.elgato.com/streamdeck/hid/stream-deck-xl/) -- XL-specific protocol details, PIDs, and specifications
4. [Elgato Stream Deck HID API - Classic Family](https://docs.elgato.com/streamdeck/hid/stream-deck-classic/) -- 15-key model specifications and protocol
5. [Elgato Stream Deck HID API - Mini](https://docs.elgato.com/streamdeck/hid/mini/) -- Mini-specific protocol (different from main protocol)
6. [Elgato Stream Deck HID API - Stream Deck Plus](https://docs.elgato.com/streamdeck/hid/stream-deck-plus/) -- Plus protocol with encoder and touch events
7. [Elgato Stream Deck HID API - Stream Deck Neo](https://docs.elgato.com/streamdeck/hid/stream-deck-neo/) -- Neo protocol with capacitive sensors and info strip
8. [Elgato Stream Deck HID API - Modules 15/32](https://docs.elgato.com/streamdeck/hid/module-15_32/) -- Module variant specifications
9. [Notes on the Stream Deck HID protocol - cliffrowley (GitHub Gist)](https://gist.github.com/cliffrowley/d18a9c4569537b195f2b1eb6c68469e0) -- Community reverse-engineered V1 protocol notes
10. [Reverse Engineering The Stream Deck - Den Delimarsky](https://den.dev/blog/reverse-engineering-stream-deck/) -- Detailed V1 protocol reverse engineering with Wireshark
11. [Reverse Engineering The Stream Deck Plus - Den Delimarsky](https://den.dev/blog/reverse-engineer-stream-deck-plus/) -- Stream Deck Plus protocol analysis
12. [python-elgato-streamdeck - StreamDeckXL.py (GitHub)](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckXL.py) -- Python library XL implementation showing exact byte parsing
13. [python-elgato-streamdeck - StreamDeckOriginal.py (GitHub)](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckOriginal.py) -- Python library V1 implementation for protocol comparison
14. [python-elgato-streamdeck - StreamDeckMini.py (GitHub)](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckMini.py) -- Python library Mini implementation
15. [Some info on HID packets - dh1tw/streamdeck Issue #6 (GitHub)](https://github.com/dh1tw/streamdeck/issues/6) -- Go library protocol discussion with brightness packet details
16. [USB Device Hunt - Stream Deck XL](https://devicehunt.com/view/type/usb/vendor/0FD9/device/006C) -- USB device identification for PID 0x006C
