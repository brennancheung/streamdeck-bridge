# Device Control Commands

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Hardware & HID Protocol

---

## Key Findings

- The Stream Deck XL uses the "Main Protocol" shared with Classic, Neo, and Plus models -- NOT the legacy Mini protocol
- All device control commands use 32-byte HID Feature Reports (Report ID 0x03 for setters, various IDs for getters)
- Brightness is set via a single byte (0-100) in a feature report; PWM-driven, non-linear perception curve
- Three separate firmware version queries exist (LD bootloader, AP1, AP2 application processors)
- Serial number retrieval returns 12-14 ASCII characters
- Sleep mode is timer-based (set duration in seconds; 0 disables); no explicit sleep/wake HID command exists
- Show Logo (0x02) is the closest thing to a "soft reset" -- it clears all key images and displays the Elgato boot logo
- On unplug/replug, ALL volatile state is lost: brightness resets to default, key images clear, device shows standby logo
- The Get Unit Information report (0x08) lets software dynamically discover the key matrix, LCD geometry, and image capabilities

---

## Stream Deck XL Device Identification

| Property         | Value                          |
|------------------|--------------------------------|
| USB Vendor ID    | `0x0FD9` (Elgato Systems GmbH)|
| USB Product ID   | `0x006C` (XL original)        |
| USB Product ID   | `0x008F` (XL 2022 revision)   |
| USB Product ID   | `0x00BA` (Module 32, same protocol) |
| Connection       | USB 2.0, HID class            |
| LCD Resolution   | 1024 x 600 px (high DPI)      |
| Key Grid         | 8 columns x 4 rows (32 keys)  |
| Key Image Size   | 96 x 96 px                    |
| Image Format     | JPEG, rotated 180 degrees     |

---

## HID Report Types Overview

The Stream Deck XL communicates through three HID report types:

| Report Type     | Direction       | Max Size  | Method                       |
|-----------------|-----------------|-----------|------------------------------|
| Input Reports   | Device -> Host  | 512 bytes | HID READ (poll at ~50ms)     |
| Output Reports  | Host -> Device  | 1024 bytes| HID WRITE                    |
| Feature Reports | Bidirectional   | 32 bytes  | HID SEND/GET FEATURE REPORT  |

**All device control commands use Feature Reports.** Output reports are only used for image data transfer. Input reports carry key press events.

Feature reports are always **exactly 32 bytes**, zero-padded to fill the buffer.

---

## Setter Feature Reports (Report ID: 0x03)

All setter commands share Report ID `0x03` at offset +0x00, with the command byte at offset +0x01.

### 1. Set Backlight Brightness (Command 0x08)

Sets the LCD backlight brightness level via PWM.

**Byte layout (32 bytes total, zero-padded):**

```
Offset: 00  01  02  03  04  05  06 ... 1F
Data:   03  08  PP  00  00  00  00 ... 00
```

| Offset | Field      | Type  | Description                          |
|--------|------------|-------|--------------------------------------|
| +0x00  | Report ID  | UINT8 | Always `0x03`                        |
| +0x01  | Command    | UINT8 | `0x08` = Set Brightness              |
| +0x02  | Brightness | UINT8 | `0x00` (off) to `0x64` (100, max)    |
| +0x03  | (padding)  | -     | Zeros to fill 32 bytes               |

**Example -- set brightness to 75%:**
```
03 08 4B 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

**Important notes:**
- Brightness is controlled via PWM, resulting in a **non-linear** perceived brightness curve
- Lower values (0-30) produce the most dramatic visible change
- Value `0x00` turns the backlight completely off (keys are dark but device remains active)
- Value `0x64` (100) is maximum brightness
- Brightness does NOT persist across power cycles

### 2. Show Logo (Command 0x02)

Forcibly triggers the display of the Elgato boot logo on the LCD. This clears all currently displayed key images and shows the standby/startup screen. Functions as a "soft reset" for the display.

**Byte layout:**

```
Offset: 00  01  02  03  04 ... 1F
Data:   03  02  00  00  00 ... 00
```

| Offset | Field     | Type  | Description              |
|--------|-----------|-------|--------------------------|
| +0x00  | Report ID | UINT8 | Always `0x03`            |
| +0x01  | Command   | UINT8 | `0x02` = Show Logo       |
| +0x02  | (padding) | -     | Zeros to fill 32 bytes   |

**Example:**
```
03 02 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

### 3. Fill Entire LCD with Color (Command 0x05)

Fills the entire LCD screen with a single solid RGB color. Useful for clearing all key images at once or providing visual feedback.

**Byte layout:**

```
Offset: 00  01  02  03  04  05 ... 1F
Data:   03  05  RR  GG  BB  00 ... 00
```

| Offset | Field     | Type       | Description               |
|--------|-----------|------------|---------------------------|
| +0x00  | Report ID | UINT8      | Always `0x03`             |
| +0x01  | Command   | UINT8      | `0x05` = Fill LCD         |
| +0x02  | Red       | UINT8      | Red channel (0x00-0xFF)   |
| +0x03  | Green     | UINT8      | Green channel (0x00-0xFF) |
| +0x04  | Blue      | UINT8      | Blue channel (0x00-0xFF)  |

**Example -- fill LCD with red:**
```
03 05 FF 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

### 4. Fill Single Key with Color (Command 0x06)

Fills a single key/button area with a solid RGB color.

**Byte layout:**

```
Offset: 00  01  02  03  04  05  06 ... 1F
Data:   03  06  KK  RR  GG  BB  00 ... 00
```

| Offset | Field     | Type  | Description                          |
|--------|-----------|-------|--------------------------------------|
| +0x00  | Report ID | UINT8 | Always `0x03`                        |
| +0x01  | Command   | UINT8 | `0x06` = Fill Key                    |
| +0x02  | Key Index | UINT8 | Key number (0-31 for XL)             |
| +0x03  | Red       | UINT8 | Red channel (0x00-0xFF)              |
| +0x04  | Green     | UINT8 | Green channel (0x00-0xFF)            |
| +0x05  | Blue      | UINT8 | Blue channel (0x00-0xFF)             |

**Example -- set key 5 to green:**
```
03 06 05 00 FF 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

### 5. Set Sleep Mode Duration (Command 0x0D)

Configures the auto-sleep timer. After the specified number of seconds of inactivity, the device enters sleep mode (dims/blanks the display). Setting to 0 disables auto-sleep.

**Byte layout:**

```
Offset: 00  01  02  03  04  05  06 ... 1F
Data:   03  0D  SS  SS  SS  SS  00 ... 00
```

| Offset | Field     | Type  | Description                                    |
|--------|-----------|-------|------------------------------------------------|
| +0x00  | Report ID | UINT8 | Always `0x03`                                  |
| +0x01  | Command   | UINT8 | `0x0D` = Set Sleep Duration                    |
| +0x02  | Duration  | INT32 | Seconds until sleep, little-endian. `0` = never |

**Example -- sleep after 10 minutes (600 seconds = 0x00000258):**
```
03 0D 58 02 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

**Example -- disable auto-sleep:**
```
03 0D 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

### 6. Show Background by Index (Command 0x13)

Displays a previously uploaded background image by its stored index. This command is specific to models in the Main Protocol family (XL, Classic, Neo, Plus) and not present in the Mini.

**Byte layout:**

```
Offset: 00  01  02  03 ... 1F
Data:   03  13  II  00 ... 00
```

| Offset | Field     | Type  | Description                     |
|--------|-----------|-------|---------------------------------|
| +0x00  | Report ID | UINT8 | Always `0x03`                   |
| +0x01  | Command   | UINT8 | `0x13` = Show Background        |
| +0x02  | Index     | UINT8 | Background image index          |

---

## Getter Feature Reports

Getter reports are used to query device information. The host sends a GET FEATURE REPORT request with the appropriate Report ID, and the device returns a 32-byte response.

### 1. Get Firmware Version -- LD Bootloader (Report ID: 0x04)

Retrieves the bootloader (LD) firmware version string.

**Request:** Send HID GET FEATURE REPORT with Report ID `0x04`.

**Response layout (32 bytes):**

```
Offset: 00  01  02  03  04  05  06  07  08  09  0A  0B  0C  0D
Data:   04  0C  CC  CC  CC  CC  VV  VV  VV  VV  VV  VV  VV  VV
```

| Offset | Field          | Type      | Description                     |
|--------|----------------|-----------|---------------------------------|
| +0x00  | Report ID      | UINT8     | `0x04`                          |
| +0x01  | Data Length    | UINT8     | `0x0C` (12 bytes follow)        |
| +0x02  | Checksum       | UINT32    | Firmware checksum               |
| +0x06  | Version String | UINT8[8]  | ASCII version (e.g., "1.02.005")|

### 2. Get Firmware Version -- AP2 (Report ID: 0x05)

Retrieves the primary application processor firmware version.

**Request:** Send HID GET FEATURE REPORT with Report ID `0x05`.

**Response layout:** Same structure as Report ID 0x04, but with Report ID byte `0x05`. This is the most commonly used firmware version query -- the python-elgato-streamdeck library uses `read_feature(0x05, 32)` and extracts the version string from bytes `[6:]`.

```
Offset: 00  01  02  03  04  05  06  07  08  09  0A  0B  0C  0D
Data:   05  0C  CC  CC  CC  CC  VV  VV  VV  VV  VV  VV  VV  VV
```

### 3. Get Firmware Version -- AP1 (Report ID: 0x07)

Retrieves the backup/secondary application processor firmware version.

**Request:** Send HID GET FEATURE REPORT with Report ID `0x07`.

**Response layout:** Same structure, Report ID byte `0x07`.

### 4. Get Unit Serial Number (Report ID: 0x06)

Retrieves the device serial number as an ASCII string.

**Request:** Send HID GET FEATURE REPORT with Report ID `0x06`.

**Response layout (32 bytes):**

```
Offset: 00  01  02  03  04  05  ... up to 0F
Data:   06  LL  SS  SS  SS  SS  ... SS
```

| Offset | Field         | Type          | Description                         |
|--------|---------------|---------------|-------------------------------------|
| +0x00  | Report ID     | UINT8         | `0x06`                              |
| +0x01  | Data Length   | UINT8         | `0x0C` (12) or `0x0E` (14)         |
| +0x02  | Serial String | UINT8[Length] | ASCII serial number (12-14 chars)   |

The python library extracts the serial via `serial[2:]` after reading 32 bytes from feature report `0x06`.

### 5. Get Unit Information (Report ID: 0x08)

Returns comprehensive hardware capability data. This is how software discovers the device layout, LCD geometry, and image format without hardcoding per-model values.

**Request:** Send HID GET FEATURE REPORT with Report ID `0x08`.

**Response layout (32 bytes):**

```
Offset: 00  01  02  03  04  05  06  07  08  09  0A  0B  0C  0D  0E  0F  10
Data:   08  RR  CC  WW  WW  HH  HH  LW  LW  LH  LH  BP  CS  KG  LG  DF  00
```

| Offset | Field                | Type   | Description                                   |
|--------|----------------------|--------|-----------------------------------------------|
| +0x00  | Report ID            | UINT8  | `0x08`                                        |
| +0x01  | Matrix Rows          | UINT8  | Number of key rows (XL: 4)                    |
| +0x02  | Matrix Columns       | UINT8  | Number of key columns (XL: 8)                 |
| +0x03  | Key Width            | UINT16 | Key/button image width in px (XL: 96)         |
| +0x05  | Key Height           | UINT16 | Key/button image height in px (XL: 96)        |
| +0x07  | LCD Width            | UINT16 | Total LCD width in px (XL: 1024)              |
| +0x09  | LCD Height           | UINT16 | Total LCD height in px (XL: 600)              |
| +0x0B  | Image BPP            | UINT8  | Bits per pixel for key images                 |
| +0x0C  | Color Scheme         | UINT8  | Image color format identifier                 |
| +0x0D  | Key Gallery Count    | UINT8  | Number of built-in key/button images           |
| +0x0E  | LCD Gallery Count    | UINT8  | Number of built-in LCD background images       |
| +0x0F  | Demo Frame Count     | UINT8  | Number of frames in demo animation             |
| +0x10  | Reserved             | UINT8  | Reserved for future use                        |

All UINT16 fields are **little-endian**.

### 6. Get Sleep Mode Duration (Report ID: 0x0A)

Retrieves the currently configured auto-sleep timer value.

**Request:** Send HID GET FEATURE REPORT with Report ID `0x0A`.

**Response layout:**

```
Offset: 00  01  02  03  04  05
Data:   0A  LL  DD  DD  DD  DD
```

| Offset | Field      | Type  | Description                               |
|--------|------------|-------|-------------------------------------------|
| +0x00  | Report ID  | UINT8 | `0x0A`                                    |
| +0x01  | Data Length| UINT8 | Length of following data                   |
| +0x02  | Duration   | INT32 | Sleep duration in seconds (0 = disabled)  |

---

## Complete Command Reference Table

### Setter Commands (Host -> Device, Report ID 0x03)

| Command | Hex  | Function                  | Payload                        |
|---------|------|---------------------------|--------------------------------|
| 0x02    | `03 02` | Show Logo              | None                           |
| 0x05    | `03 05` | Fill LCD with Color    | R, G, B (3 bytes)             |
| 0x06    | `03 06` | Fill Key with Color    | Key Index, R, G, B (4 bytes)  |
| 0x08    | `03 08` | Set Brightness         | Percent 0-100 (1 byte)        |
| 0x0D    | `03 0D` | Set Sleep Duration     | INT32 seconds (4 bytes LE)    |
| 0x13    | `03 13` | Show Background        | Index (1 byte)                |

### Getter Commands (Device -> Host)

| Report ID | Function                  | Key Response Fields               |
|-----------|---------------------------|-----------------------------------|
| 0x04      | Get FW Version (LD)       | Checksum + 8-char ASCII string    |
| 0x05      | Get FW Version (AP2)      | Checksum + 8-char ASCII string    |
| 0x06      | Get Serial Number         | 12-14 char ASCII string           |
| 0x07      | Get FW Version (AP1)      | Checksum + 8-char ASCII string    |
| 0x08      | Get Unit Information      | Matrix, LCD dims, BPP, galleries  |
| 0x0A      | Get Sleep Duration        | INT32 duration in seconds         |

---

## Sleep, Wake, and Power Behavior

### Auto-Sleep Timer

The Stream Deck XL supports a configurable auto-sleep timer via the Set Sleep Mode Duration command (0x0D). When the timer expires after a period of no key presses, the device dims/blanks the LCD and shows the standby logo.

- **To enable:** Send `03 0D` with a non-zero INT32 duration in seconds
- **To disable:** Send `03 0D 00 00 00 00`
- **To query current setting:** GET FEATURE REPORT with Report ID `0x0A`

### No Explicit Sleep/Wake Command

There is **no dedicated HID command to force the device into or out of sleep mode**. The auto-sleep timer is the only mechanism. Pressing any key on the device wakes it from sleep. Some third-party libraries note that there is no API to programmatically wake the device.

### Show Logo as Soft Reset

The Show Logo command (`03 02`) is the closest equivalent to a "reset display" command. It:
- Clears all key images from the LCD
- Displays the built-in Elgato boot/standby logo
- Does NOT affect brightness settings
- Does NOT affect the sleep timer configuration

---

## Unplug/Replug Behavior and State Persistence

### What is Lost on Power Cycle

When the Stream Deck XL is unplugged and replugged (or loses USB power briefly):

- **All key images are cleared** -- the device shows the standby Elgato logo
- **Brightness resets** to the device default (typically 100%)
- **Sleep timer resets** to the device default
- **No key state is preserved** -- all buttons report as released

### What Persists

- **Serial number** -- permanently stored in device flash
- **Firmware versions** -- stored in device flash
- **Built-in gallery images** -- stored in device flash (accessible via Show Background command)
- **USB descriptors and device identity** -- hardcoded

### Reconnection Handling

From the host software perspective:
- The HID device handle becomes invalid on disconnect
- Software must detect the disconnect, close the old handle, and re-enumerate USB devices
- After reconnection, software must re-send all key images and re-set brightness
- The recommended approach is to poll for device presence and auto-reconnect
- Libraries like python-elgato-streamdeck and streamdeck-ui implement automatic reconnection

### Power Distribution Issues

Stream Deck XL devices can sometimes experience brief power drops through USB hubs, causing rapid disconnect/reconnect cycles. The device does not indicate insufficient power -- it simply restarts. Using a powered USB hub or a direct motherboard USB port is recommended.

---

## Data Type Reference

All multi-byte integer fields use **little-endian** byte order.

| Type   | Size    | Description                    |
|--------|---------|--------------------------------|
| UINT8  | 1 byte  | Unsigned 8-bit integer (0-255) |
| INT8   | 1 byte  | Signed 8-bit integer           |
| UINT16 | 2 bytes | Unsigned 16-bit, little-endian |
| INT16  | 2 bytes | Signed 16-bit, little-endian   |
| UINT32 | 4 bytes | Unsigned 32-bit, little-endian |
| INT32  | 4 bytes | Signed 32-bit, little-endian   |
| RGB    | 3 bytes | Three consecutive UINT8: R,G,B |

---

## Protocol Comparison: XL vs Other Models

The Stream Deck XL uses the **Main Protocol**, which differs significantly from the Legacy (Mini) Protocol:

| Feature              | Main Protocol (XL)        | Legacy Protocol (Mini)        |
|----------------------|---------------------------|-------------------------------|
| Setter Report ID     | `0x03`                    | `0x05` (cmd `0x55`)          |
| Brightness payload   | `03 08 PP`                | `05 55 AA D1 01 PP`          |
| Image format         | JPEG                      | BMP                           |
| Image rotation       | 180 degrees               | 90 degrees clockwise          |
| Output report size   | 1024 bytes                | 8191 bytes                    |
| Reset command        | `03 02` (Show Logo)       | `0B 63` (Show Logo)          |
| Serial report ID     | `0x06`                    | `0x03`                        |
| FW version report    | `0x05`                    | `0x04`                        |
| Feature report size  | 32 bytes                  | 17 bytes                      |

---

## Practical Code Examples

### Node.js / TypeScript (using node-hid)

```typescript
import HID from 'node-hid';

const VID = 0x0FD9;
const PID = 0x006C; // Stream Deck XL

const device = new HID.HID(VID, PID);

// Set brightness to 80%
const brightnessReport = Buffer.alloc(32);
brightnessReport[0] = 0x03; // Report ID
brightnessReport[1] = 0x08; // Command: Set Brightness
brightnessReport[2] = 0x50; // 80 decimal = 0x50
device.sendFeatureReport(Array.from(brightnessReport));

// Show Elgato logo (clear all keys)
const showLogoReport = Buffer.alloc(32);
showLogoReport[0] = 0x03;
showLogoReport[1] = 0x02;
device.sendFeatureReport(Array.from(showLogoReport));

// Get firmware version (AP2)
const fwReport = device.getFeatureReport(0x05, 32);
const fwVersion = Buffer.from(fwReport.slice(6))
  .toString('ascii')
  .replace(/\0/g, '');

// Get serial number
const serialReport = device.getFeatureReport(0x06, 32);
const serialNumber = Buffer.from(serialReport.slice(2))
  .toString('ascii')
  .replace(/\0/g, '');

// Get unit information
const infoReport = device.getFeatureReport(0x08, 32);
const rows = infoReport[1];
const cols = infoReport[2];
const keyWidth = infoReport[3] | (infoReport[4] << 8);
const keyHeight = infoReport[5] | (infoReport[6] << 8);

// Set sleep timer to 5 minutes (300 seconds)
const sleepReport = Buffer.alloc(32);
sleepReport[0] = 0x03;
sleepReport[1] = 0x0D;
sleepReport.writeInt32LE(300, 2); // 300 seconds
device.sendFeatureReport(Array.from(sleepReport));

// Fill entire LCD blue
const fillReport = Buffer.alloc(32);
fillReport[0] = 0x03;
fillReport[1] = 0x05;
fillReport[2] = 0x00; // R
fillReport[3] = 0x00; // G
fillReport[4] = 0xFF; // B
device.sendFeatureReport(Array.from(fillReport));
```

### Python (using hidapi)

```python
import hid

VID = 0x0FD9
PID = 0x006C  # Stream Deck XL

device = hid.device()
device.open(VID, PID)

# Set brightness to 50%
brightness_report = [0x03, 0x08, 0x32] + [0x00] * 29
device.send_feature_report(brightness_report)

# Show logo
logo_report = [0x03, 0x02] + [0x00] * 30
device.send_feature_report(logo_report)

# Get firmware version
fw = device.get_feature_report(0x05, 32)
version = bytes(fw[6:14]).decode('ascii').rstrip('\x00')

# Get serial number
sn = device.get_feature_report(0x06, 32)
serial = bytes(sn[2:16]).decode('ascii').rstrip('\x00')
```

---

## Sources

1. [Elgato Stream Deck HID API -- Introduction](https://docs.elgato.com/streamdeck/hid/intro/)
2. [Elgato Stream Deck HID API -- Main Protocol: General Reference](https://docs.elgato.com/streamdeck/hid/general/)
3. [Elgato Stream Deck HID API -- Stream Deck Classic](https://docs.elgato.com/streamdeck/hid/stream-deck-classic/)
4. [Elgato Stream Deck HID API -- Stream Deck XL](https://docs.elgato.com/streamdeck/hid/stream-deck-xl/)
5. [Elgato Stream Deck HID API -- Stream Deck Mini Protocol](https://docs.elgato.com/streamdeck/hid/mini/)
6. [Elgato Stream Deck HID API -- Module 15 and 32 Keys](https://docs.elgato.com/streamdeck/hid/module-15_32/)
7. [Notes on the Stream Deck HID protocol (Cliff Rowley, GitHub Gist)](https://gist.github.com/cliffrowley/d18a9c4569537b195f2b1eb6c68469e0)
8. [python-elgato-streamdeck -- StreamDeckXL.py source](https://github.com/abcminiuser/python-elgato-streamdeck/blob/master/src/StreamDeck/Devices/StreamDeckXL.py)
9. [python-elgato-streamdeck -- Device modules documentation](https://python-elgato-streamdeck.readthedocs.io/en/stable/modules/devices.html)
10. [node-elgato-stream-deck (Julusian, GitHub)](https://github.com/Julusian/node-elgato-stream-deck)
11. [Stream Deck HID packet discussion (dh1tw/streamdeck#6)](https://github.com/dh1tw/streamdeck/issues/6)
12. [USB Device Hunt -- VID 0x0FD9 PID 0x006C](https://devicehunt.com/view/type/usb/vendor/0FD9/device/006C)
13. [StreamDeckSharp -- Wake from sleep discussion](https://github.com/OpenMacroBoard/StreamDeckSharp/issues/64)
