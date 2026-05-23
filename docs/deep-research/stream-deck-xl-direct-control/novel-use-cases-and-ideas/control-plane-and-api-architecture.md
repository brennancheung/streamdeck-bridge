# Control Plane and API Architecture

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Novel Use Cases & Ideas

---

## Key Findings

- The OBS WebSocket v5 protocol is the strongest prior art: opcode-based JSON messages, SHA256 challenge-response auth, request batching, and event subscriptions -- all applicable to a Stream Deck control plane
- Elgato's own SDK already uses WebSocket internally between plugins and the Stream Deck app, with a message format built around `event`, `context`, `action`, and `payload` fields
- Bitfocus Companion exposes a TCP/WebSocket Satellite API on ports 16622/16623, proving that network-accessible Stream Deck control is production-viable
- A hybrid REST + WebSocket architecture is the right design: REST for CRUD operations (read button state, set images, configure layouts), WebSocket for real-time events (button presses, connection lifecycle)
- Multi-client control requires a reservation/ownership model -- buttons are assigned to clients via "claims," and unclaimed buttons fall through to a default layer
- Image transfer over WebSocket is best done as binary frames containing JPEG data (the device natively consumes JPEG), with a reference-by-hash caching layer to avoid redundant transfers
- Service discovery via mDNS/Bonjour (advertising `_streamdeck._tcp.local`) lets clients auto-discover the control plane on the LAN without hardcoded addresses
- The 96x96 JPEG images used by the XL are roughly 3-8 KB each; at 32 buttons, a full-device refresh is under 256 KB -- well within WebSocket frame limits

---

## 1. Prior Art Survey

### 1.1 OBS WebSocket v5 Protocol

The OBS WebSocket protocol (v5, shipped with OBS Studio 28+) is the gold standard for remote-controlling creative software. Its design directly informs this architecture.

**Connection lifecycle:**
1. Client opens WebSocket connection
2. Server sends `Hello` (OpCode 0) with auth challenge
3. Client sends `Identify` (OpCode 1) with auth response + desired event subscriptions
4. Server sends `Identified` (OpCode 2) confirming the session
5. Bidirectional messages flow: requests (OpCode 6), responses (OpCode 7), events (OpCode 5)

**Message envelope:**
```json
{
  "op": 6,
  "d": {
    "requestType": "SetCurrentProgramScene",
    "requestId": "f819dcf0-89cc-11eb-8f0e-382c4ac93b9c",
    "requestData": {
      "sceneName": "Scene 2"
    }
  }
}
```

**Key design patterns borrowed:**
- Opcode-based message discrimination (integer, not string -- fast to parse)
- Client-generated `requestId` for correlating responses to requests
- Event subscription bitmask in the `Identify` message (clients opt into event categories)
- Request batching (OpCode 8/9) for atomic multi-step operations
- SHA256 challenge-response authentication (no plaintext passwords on the wire)
- Support for both JSON and MessagePack serialization

### 1.2 Elgato Stream Deck Plugin SDK WebSocket Protocol

Elgato's own plugin architecture uses WebSocket internally. Each plugin connects to a local WebSocket server run by the Stream Deck application.

**Registration handshake:**
```json
{
  "event": "registerPlugin",
  "uuid": "com.elgato.example"
}
```

**Receiving a key press:**
```json
{
  "action": "com.elgato.example.action1",
  "event": "keyDown",
  "context": "ABC123",
  "device": "DEV456",
  "payload": {
    "settings": {},
    "coordinates": { "column": 3, "row": 1 },
    "state": 0,
    "isInMultiAction": false
  }
}
```

**Setting a button image:**
```json
{
  "event": "setImage",
  "context": "ABC123",
  "payload": {
    "image": "data:image/png;base64,iVBORw0KGgo...",
    "target": 0
  }
}
```

**Lessons learned:**
- The `context` field acts as a handle to a specific button instance -- essential for multi-button routing
- Images are transferred as base64 data URIs -- functional but inefficient (33% overhead vs binary)
- The `target` field distinguishes hardware display vs software display -- a concept worth preserving
- String-based event discrimination (`"event": "keyDown"`) is more readable than opcodes but slower to parse

### 1.3 Bitfocus Companion Satellite API

Companion proves that network-accessible Stream Deck control works in production. Its Satellite API lets remote devices (including physical Stream Decks on other machines) connect over the network.

**Protocol details:**
- Transport: TCP (port 16622) or WebSocket (port 16623, added in Companion 3.5)
- Format: ASCII line protocol, not JSON -- `COMMAND ARG1=VAL1 ARG2=VAL2\n`
- Handshake: server sends `BEGIN CompanionVersion=3.x.x ApiVersion=1.0.0\n` on connect

**Example commands:**
```
DEVICEID=myDevice MODEL=STREAMDECK_XL COLS=8 ROWS=4\n
KEY-PRESS DEVICEID=myDevice KEY=5 PRESSED=true\n
KEY-PRESS DEVICEID=myDevice KEY=5 PRESSED=false\n
```

**Image transfer:**
```
KEY-DRAW DEVICEID=myDevice KEY=5 FORMAT=png DATA=base64:<base64data>\n
```

**Takeaways:**
- ASCII line protocol is simple but limited (no nested structures, awkward for binary data)
- The `DEVICEID` field supports multiple devices on a single connection -- good for multi-device setups
- Companion's model proves demand for this kind of network bridge
- Companion lacks fine-grained auth or multi-client arbitration -- an area to improve

### 1.4 Home Assistant WebSocket API

Home Assistant's API model is relevant because it solves the same class of problem: many clients subscribing to events from a shared device/state system.

**Authentication flow:**
```json
// Server sends:
{"type": "auth_required", "ha_version": "2025.5.0"}

// Client sends:
{"type": "auth", "access_token": "eyJ0eXAiOiJK..."}

// Server confirms:
{"type": "auth_ok", "ha_version": "2025.5.0"}
```

**Command with incrementing ID:**
```json
{"id": 18, "type": "subscribe_events", "event_type": "state_changed"}
```

**Event delivery:**
```json
{
  "id": 18,
  "type": "event",
  "event": {
    "event_type": "state_changed",
    "data": { "entity_id": "light.living_room", "new_state": { "state": "on" } }
  }
}
```

**Patterns adopted:**
- Incrementing integer IDs for request/response correlation
- Subscription model for events (subscribe once, receive many)
- Dual API surface: REST for simple reads/writes, WebSocket for real-time subscriptions
- Long-lived access tokens for machine-to-machine auth

### 1.5 OSC (Open Sound Control) and RTP-MIDI

OSC and RTP-MIDI represent the live-performance world's answer to networked device control.

**OSC key concepts:**
- Address-pattern routing: `/streamdeck/button/5/press` maps naturally to a button topology
- UDP transport (low latency, no connection overhead)
- Bundles with timestamps for synchronized multi-button updates
- Wildcard addressing: `/streamdeck/button/*/brightness` to set all at once

**RTP-MIDI key concepts:**
- Session management with mDNS/Bonjour discovery
- Clock synchronization between endpoints
- Journal-based recovery for lost packets (no retransmission latency)
- Apple's implementation is built into macOS Core MIDI

These protocols validate that hardware control over networks is a solved problem in the audio/visual world. The Stream Deck control plane should feel familiar to developers in this ecosystem.

---

## 2. Architecture Overview

### 2.1 System Topology

```
                                        +-------------------+
                                        |  Stream Deck XL   |
                                        |  (USB HID Device) |
                                        +--------+----------+
                                                 |
                                           USB HID
                                                 |
+-------------------+     +-----------+----------+----------+
| CLI Tool          |     |                                 |
| (streamdeck-ctl)  +---->|    Stream Deck Control Plane    |
+-------------------+     |                                 |
                     REST |  +--------+  +---------------+  |
+-------------------+     |  | Device |  | Client        |  |
| Web Dashboard     +---->|  | Driver |  | Manager       |  |
+-------------------+  WS |  +--------+  +-------+-------+  |
                          |  +--------+  +--------+------+  |
+-------------------+     |  | Image  |  | Event         |  |
| Monitoring App    +---->|  | Cache  |  | Router        |  |
+-------------------+  WS |  +--------+  +---------------+  |
                          |  +--------+  +---------------+  |
+-------------------+     |  | mDNS   |  | Auth          |  |
| CI/CD Pipeline    +---->|  | Advert |  | Provider      |  |
+-------------------+ REST|  +--------+  +---------------+  |
                          +---------------------------------+
```

### 2.2 Core Components

| Component         | Responsibility                                                       |
|-------------------|----------------------------------------------------------------------|
| Device Driver     | USB HID communication; image encoding; key state polling             |
| Client Manager    | WebSocket connection lifecycle; client registration; claim tracking   |
| Event Router      | Dispatches button events to the correct client based on claims       |
| Image Cache       | Content-addressed image store; dedup; format conversion              |
| mDNS Advertiser   | Bonjour service advertisement for LAN discovery                      |
| Auth Provider     | API key validation; token issuance; permission scoping               |
| REST API          | HTTP endpoints for CRUD operations on device state                   |
| WebSocket API     | Real-time bidirectional channel for events and streaming updates      |

### 2.3 Design Principles

1. **USB is an implementation detail.** Clients never see HID reports. The control plane abstracts the device into a logical model of buttons, images, and events.
2. **WebSocket-first, REST-friendly.** Real-time events flow over WebSocket. But every state mutation is also available as a REST endpoint for simple integrations (curl, webhooks, scripts).
3. **Clients own buttons, not the whole device.** The claim system lets multiple applications coexist on a single Stream Deck.
4. **Images are content-addressed.** An image is identified by its SHA256 hash. Upload once, reference many times. The cache prevents redundant USB transfers.
5. **Zero-config on the LAN.** mDNS discovery means a client on the same network can find the control plane without configuration.

---

## 3. WebSocket Protocol Specification

### 3.1 Connection Lifecycle

```
Client                                Server
  |                                      |
  |  ---- WebSocket CONNECT ---------->  |
  |                                      |
  |  <--- OpCode 0: Hello -------------- |  (server version, auth challenge)
  |                                      |
  |  ---- OpCode 1: Identify --------->  |  (auth response, client metadata, subscriptions)
  |                                      |
  |  <--- OpCode 2: Identified --------- |  (session ID, device info, negotiated capabilities)
  |                                      |
  |  ---- OpCode 6: Request ---------->  |  (claim buttons, set images, etc.)
  |  <--- OpCode 7: RequestResponse ---  |
  |                                      |
  |  <--- OpCode 5: Event -------------- |  (button presses, device connect/disconnect)
  |                                      |
  |  ---- OpCode 3: Heartbeat -------->  |  (keepalive, every 30s)
  |  <--- OpCode 4: Heartbeat --------- |
  |                                      |
  |  ---- OpCode 9: Disconnect ------->  |  (graceful close)
```

### 3.2 OpCode Definitions

| OpCode | Name             | Direction        | Description                                  |
|--------|------------------|------------------|----------------------------------------------|
| 0      | Hello            | Server -> Client | Initial handshake with auth challenge         |
| 1      | Identify         | Client -> Server | Auth response, client info, subscriptions     |
| 2      | Identified       | Server -> Client | Session confirmed, device capabilities        |
| 3      | Heartbeat        | Bidirectional    | Keepalive ping/pong                           |
| 4      | HeartbeatAck     | Bidirectional    | Keepalive response                            |
| 5      | Event            | Server -> Client | Button press, device state change, etc.       |
| 6      | Request          | Client -> Server | Client command (set image, claim button, etc.)|
| 7      | RequestResponse  | Server -> Client | Response to a client request                  |
| 8      | RequestBatch     | Client -> Server | Multiple requests in one message              |
| 9      | Disconnect       | Client -> Server | Graceful connection teardown                  |

### 3.3 Message Envelope

Every WebSocket message is a JSON object with this structure:

```json
{
  "op": <integer>,
  "d": <object>
}
```

Binary frames are reserved for image data transfers (see Section 5.3).

### 3.4 Hello (OpCode 0)

Sent by the server immediately after the WebSocket connection is established.

```json
{
  "op": 0,
  "d": {
    "serverVersion": "1.0.0",
    "protocolVersion": 1,
    "authentication": {
      "challenge": "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6",
      "salt": "x9y8z7w6v5u4t3s2"
    },
    "devices": [
      {
        "id": "AL44H1A02803",
        "model": "STREAMDECK_XL",
        "firmware": "1.02.008",
        "rows": 4,
        "columns": 8,
        "buttonCount": 32,
        "imageSize": 96,
        "imageFormat": "jpeg"
      }
    ]
  }
}
```

### 3.5 Identify (OpCode 1)

Client responds with authentication and session preferences.

```json
{
  "op": 1,
  "d": {
    "authentication": "hashed_auth_string_here",
    "clientName": "monitoring-dashboard",
    "clientVersion": "2.1.0",
    "eventSubscriptions": 31,
    "capabilities": ["images", "text-overlay"]
  }
}
```

**Event subscription bitmask:**

| Bit | Category          | Events included                           |
|-----|-------------------|-------------------------------------------|
| 0   | ButtonEvents      | buttonDown, buttonUp, buttonHold          |
| 1   | DeviceEvents      | deviceConnected, deviceDisconnected       |
| 2   | ClaimEvents       | buttonClaimed, buttonReleased, claimConflict |
| 3   | ImageEvents       | imageSet, imageCacheHit, imageCacheMiss   |
| 4   | SystemEvents      | brightnessChanged, sleepStateChanged      |

To subscribe to ButtonEvents + DeviceEvents: `eventSubscriptions = 0b00011 = 3`.

**Authentication computation (SHA256 challenge-response):**
```
base64(SHA256(base64(SHA256(apiKey + salt)) + challenge))
```

### 3.6 Identified (OpCode 2)

Server confirms the session.

```json
{
  "op": 2,
  "d": {
    "sessionId": "sess_7f3a2b1c",
    "negotiatedCapabilities": ["images", "text-overlay"],
    "heartbeatInterval": 30000,
    "maxRequestsPerSecond": 100
  }
}
```

### 3.7 Request / RequestResponse (OpCode 6 / 7)

**Request:**
```json
{
  "op": 6,
  "d": {
    "requestType": "SetButtonImage",
    "requestId": "req_001",
    "requestData": {
      "deviceId": "AL44H1A02803",
      "button": 5,
      "imageHash": "sha256:a1b2c3d4..."
    }
  }
}
```

**Response:**
```json
{
  "op": 7,
  "d": {
    "requestType": "SetButtonImage",
    "requestId": "req_001",
    "requestStatus": "ok",
    "responseData": {
      "button": 5,
      "previousImageHash": "sha256:e5f6a7b8...",
      "appliedAt": 1716480000000
    }
  }
}
```

**Error response:**
```json
{
  "op": 7,
  "d": {
    "requestType": "SetButtonImage",
    "requestId": "req_001",
    "requestStatus": "error",
    "error": {
      "code": "BUTTON_NOT_CLAIMED",
      "message": "Button 5 is claimed by client 'ci-dashboard'. Use ForceSetButtonImage or claim the button first."
    }
  }
}
```

### 3.8 Event (OpCode 5)

```json
{
  "op": 5,
  "d": {
    "eventType": "buttonDown",
    "eventData": {
      "deviceId": "AL44H1A02803",
      "button": 5,
      "coordinates": { "column": 5, "row": 0 },
      "timestamp": 1716480000123,
      "claimedBy": "monitoring-dashboard"
    }
  }
}
```

### 3.9 RequestBatch (OpCode 8)

For atomic multi-button updates (e.g., refreshing an entire page of buttons simultaneously):

```json
{
  "op": 8,
  "d": {
    "batchId": "batch_001",
    "haltOnFailure": false,
    "requests": [
      {
        "requestType": "SetButtonImage",
        "requestId": "req_b1",
        "requestData": { "button": 0, "imageHash": "sha256:aaa..." }
      },
      {
        "requestType": "SetButtonImage",
        "requestId": "req_b2",
        "requestData": { "button": 1, "imageHash": "sha256:bbb..." }
      },
      {
        "requestType": "SetBrightness",
        "requestId": "req_b3",
        "requestData": { "brightness": 80 }
      }
    ]
  }
}
```

---

## 4. REST API Specification

The REST API provides a stateless interface for simple integrations. Every operation available over WebSocket is also available via REST.

### 4.1 Base URL and Versioning

```
http://<host>:9120/api/v1/
```

Port 9120 chosen to avoid conflicts with common development ports. The `/v1/` prefix enables future breaking changes without disrupting existing clients.

### 4.2 Authentication

All REST requests require an `Authorization` header:

```
Authorization: Bearer <api-key>
```

API keys are generated via the control plane's management interface or CLI.

### 4.3 Endpoint Reference

#### Device Information

```
GET /api/v1/devices
```

**Response:**
```json
{
  "devices": [
    {
      "id": "AL44H1A02803",
      "model": "STREAMDECK_XL",
      "firmware": "1.02.008",
      "rows": 4,
      "columns": 8,
      "buttonCount": 32,
      "imageSize": 96,
      "imageFormat": "jpeg",
      "brightness": 75,
      "isAwake": true,
      "connectedAt": "2026-05-23T10:30:00Z"
    }
  ]
}
```

```
GET /api/v1/devices/:deviceId
```

Returns a single device object. 404 if not found.

#### Button State

```
GET /api/v1/devices/:deviceId/buttons
```

**Response:**
```json
{
  "buttons": [
    {
      "index": 0,
      "coordinates": { "column": 0, "row": 0 },
      "imageHash": "sha256:a1b2c3d4...",
      "imageUrl": "/api/v1/images/sha256:a1b2c3d4...",
      "claimedBy": "monitoring-dashboard",
      "label": "CPU Usage",
      "lastPressed": "2026-05-23T10:45:12Z"
    },
    {
      "index": 1,
      "coordinates": { "column": 1, "row": 0 },
      "imageHash": null,
      "imageUrl": null,
      "claimedBy": null,
      "label": null,
      "lastPressed": null
    }
  ]
}
```

```
GET /api/v1/devices/:deviceId/buttons/:buttonIndex
```

Returns a single button object.

#### Set Button Image

```
PUT /api/v1/devices/:deviceId/buttons/:buttonIndex/image
```

**Option A -- Upload image directly:**
```
PUT /api/v1/devices/AL44H1A02803/buttons/5/image
Content-Type: image/jpeg

<raw JPEG bytes>
```

**Option B -- Reference a cached image:**
```json
PUT /api/v1/devices/AL44H1A02803/buttons/5/image
Content-Type: application/json

{
  "imageHash": "sha256:a1b2c3d4..."
}
```

**Option C -- Render text to image server-side:**
```json
PUT /api/v1/devices/AL44H1A02803/buttons/5/image
Content-Type: application/json

{
  "render": {
    "text": "CPU\n87%",
    "fontSize": 18,
    "fontColor": "#FFFFFF",
    "backgroundColor": "#CC0000",
    "icon": "sha256:icon_hash..."
  }
}
```

**Response (all options):**
```json
{
  "button": 5,
  "imageHash": "sha256:a1b2c3d4...",
  "appliedAt": "2026-05-23T10:50:00Z"
}
```

#### Set Button Image by Coordinates

```
PUT /api/v1/devices/:deviceId/buttons/at/:column/:row/image
```

Same request/response body as above. Allows addressing by grid position instead of linear index.

#### Bulk Image Update

```
POST /api/v1/devices/:deviceId/buttons/batch
Content-Type: application/json

{
  "updates": [
    { "button": 0, "imageHash": "sha256:aaa..." },
    { "button": 1, "imageHash": "sha256:bbb..." },
    { "button": 2, "render": { "text": "LIVE", "backgroundColor": "#FF0000" } }
  ]
}
```

#### Brightness

```
GET /api/v1/devices/:deviceId/brightness
PUT /api/v1/devices/:deviceId/brightness

{ "brightness": 80 }
```

#### Image Cache

```
POST /api/v1/images
Content-Type: image/jpeg

<raw JPEG bytes>
```

**Response:**
```json
{
  "hash": "sha256:a1b2c3d4...",
  "size": 4096,
  "width": 96,
  "height": 96,
  "format": "jpeg"
}
```

```
GET /api/v1/images/:hash
```

Returns the raw image bytes. Useful for clients that want to verify what is on a button.

```
GET /api/v1/images
```

Lists all cached images with metadata.

#### Claims

```
POST /api/v1/claims
Content-Type: application/json

{
  "clientName": "monitoring-dashboard",
  "deviceId": "AL44H1A02803",
  "buttons": [0, 1, 2, 3, 4, 5, 6, 7],
  "priority": 100
}
```

**Response:**
```json
{
  "claimId": "claim_8a7b6c5d",
  "clientName": "monitoring-dashboard",
  "buttons": [0, 1, 2, 3, 4, 5, 6, 7],
  "priority": 100,
  "grantedAt": "2026-05-23T10:55:00Z"
}
```

```
DELETE /api/v1/claims/:claimId
```

Releases the claim. Buttons revert to default/unclaimed state.

```
GET /api/v1/claims
```

Lists all active claims across all clients.

#### Webhooks

```
POST /api/v1/webhooks
Content-Type: application/json

{
  "url": "https://my-app.example.com/streamdeck-events",
  "events": ["buttonDown", "buttonUp"],
  "buttons": [5, 6, 7],
  "secret": "webhook_signing_secret_here"
}
```

**Response:**
```json
{
  "webhookId": "wh_1a2b3c4d",
  "url": "https://my-app.example.com/streamdeck-events",
  "events": ["buttonDown", "buttonUp"],
  "buttons": [5, 6, 7],
  "createdAt": "2026-05-23T11:00:00Z"
}
```

**Webhook delivery payload:**
```json
POST https://my-app.example.com/streamdeck-events
X-StreamDeck-Signature: sha256=<HMAC of body using secret>
Content-Type: application/json

{
  "webhookId": "wh_1a2b3c4d",
  "eventType": "buttonDown",
  "eventData": {
    "deviceId": "AL44H1A02803",
    "button": 5,
    "coordinates": { "column": 5, "row": 0 },
    "timestamp": "2026-05-23T11:05:30.123Z"
  },
  "deliveryId": "del_9e8f7a6b"
}
```

---

## 5. Multi-Client Architecture

### 5.1 The Problem

A single Stream Deck XL has 32 buttons. Multiple applications want to use it simultaneously:

- A CI/CD dashboard showing build status on buttons 0-7
- A monitoring tool showing server health on buttons 8-15
- A personal productivity app on buttons 16-23
- Buttons 24-31 reserved for quick-launch shortcuts

Without arbitration, the last writer wins, and button presses go to all listeners (or none).

### 5.2 Claim Model

Each client can **claim** a set of buttons. A claim grants:
- **Write access**: only the claim holder can set images on claimed buttons
- **Event routing**: button presses on claimed buttons are delivered only to the claim holder
- **Priority**: higher-priority claims override lower-priority ones on the same button

**Claim resolution rules:**
1. A button can have multiple claims at different priority levels
2. The highest-priority claim is the **active** claim -- its image is displayed, its client gets events
3. When the active claim is released, the next-highest claim becomes active (its image is restored)
4. An unclaimed button shows the default image (blank or a configurable default)
5. A client can request a claim on an already-claimed button; it succeeds if its priority is higher
6. Priority ties are broken by timestamp (first claim wins)

```
Priority Stack for Button 5:

  Priority 200: [emergency-alerts]    <-- ACTIVE (highest)
  Priority 100: [monitoring-dashboard]
  Priority  50: [default-layout]
```

If `emergency-alerts` releases its claim, `monitoring-dashboard` automatically becomes active, and its cached image is restored to the button.

### 5.3 Claim Lifecycle

```json
// 1. Client claims buttons
{
  "op": 6,
  "d": {
    "requestType": "ClaimButtons",
    "requestId": "req_010",
    "requestData": {
      "deviceId": "AL44H1A02803",
      "buttons": [0, 1, 2, 3, 4, 5, 6, 7],
      "priority": 100,
      "restoreOnRelease": true
    }
  }
}

// 2. Server confirms
{
  "op": 7,
  "d": {
    "requestType": "ClaimButtons",
    "requestId": "req_010",
    "requestStatus": "ok",
    "responseData": {
      "claimId": "claim_8a7b6c5d",
      "granted": [0, 1, 2, 3, 4, 5, 6, 7],
      "conflicts": []
    }
  }
}

// 3. Conflict notification (sent to displaced client)
{
  "op": 5,
  "d": {
    "eventType": "claimOverridden",
    "eventData": {
      "buttons": [5],
      "overriddenBy": "emergency-alerts",
      "yourClaimId": "claim_old123",
      "yourPriority": 100,
      "theirPriority": 200
    }
  }
}
```

### 5.4 Page Model (Virtual Layers)

Beyond claims, the control plane supports **pages** -- virtual button layouts that can be swapped instantly. This is similar to how the official Stream Deck app organizes buttons into folders.

```json
{
  "op": 6,
  "d": {
    "requestType": "CreatePage",
    "requestId": "req_020",
    "requestData": {
      "pageId": "monitoring-page",
      "buttons": {
        "0": { "imageHash": "sha256:cpu_icon...", "label": "CPU" },
        "1": { "imageHash": "sha256:mem_icon...", "label": "Memory" },
        "2": { "imageHash": "sha256:disk_icon...", "label": "Disk" }
      }
    }
  }
}

// Switch to page
{
  "op": 6,
  "d": {
    "requestType": "ActivatePage",
    "requestId": "req_021",
    "requestData": {
      "pageId": "monitoring-page",
      "deviceId": "AL44H1A02803"
    }
  }
}
```

---

## 6. Image Management

### 6.1 Image Pipeline

The Stream Deck XL requires 96x96 JPEG images, rotated 180 degrees, delivered over USB HID in 1024-byte output reports. The control plane abstracts all of this.

```
Client Image (any format/size)
       |
       v
  [ Image Ingestion ]
       |  - Accept JPEG, PNG, SVG, BMP, GIF (first frame)
       |  - Accept any resolution (will be resized)
       v
  [ Normalization ]
       |  - Resize to 96x96 with aspect-fill
       |  - Convert to JPEG
       |  - Apply 180-degree rotation (device requirement)
       v
  [ Content-Addressed Cache ]
       |  - SHA256 hash of normalized JPEG bytes
       |  - Store in memory + optional disk persistence
       v
  [ Device Transfer ]
       |  - Split into 1024-byte HID output reports
       |  - Include report header (key index, part number, total parts)
       v
  [ Stream Deck XL Hardware ]
```

### 6.2 Image Transfer over WebSocket

For high-frequency image updates (e.g., live metrics), the control plane supports binary WebSocket frames to avoid the 33% overhead of base64 encoding.

**Binary frame format:**

```
Byte 0:    Frame type (0x01 = image upload)
Bytes 1-4: Button index (uint32, big-endian)
Bytes 5-N: Raw JPEG data
```

The server responds with a JSON text frame:

```json
{
  "op": 7,
  "d": {
    "requestType": "SetButtonImage",
    "requestId": "binary_5_1716480000",
    "requestStatus": "ok",
    "responseData": {
      "button": 5,
      "imageHash": "sha256:a1b2c3d4...",
      "cached": true
    }
  }
}
```

### 6.3 Server-Side Rendering

For simple status indicators, clients should not need to generate images. The control plane includes a server-side renderer:

```json
{
  "op": 6,
  "d": {
    "requestType": "RenderAndSetImage",
    "requestId": "req_030",
    "requestData": {
      "deviceId": "AL44H1A02803",
      "button": 5,
      "render": {
        "backgroundColor": "#1a1a2e",
        "layers": [
          {
            "type": "icon",
            "src": "sha256:cpu_icon_hash...",
            "x": 24, "y": 8,
            "width": 48, "height": 48
          },
          {
            "type": "text",
            "text": "87%",
            "fontSize": 16,
            "fontWeight": "bold",
            "color": "#FF6B6B",
            "x": 48, "y": 76,
            "anchor": "center"
          },
          {
            "type": "text",
            "text": "CPU",
            "fontSize": 10,
            "color": "#8888AA",
            "x": 48, "y": 90,
            "anchor": "center"
          }
        ]
      }
    }
  }
}
```

This eliminates the need for every client to bundle an image rendering library.

### 6.4 Image Cache Semantics

The cache uses content-addressing (SHA256 of the normalized JPEG bytes). This means:

- Two clients uploading the same image get the same hash -- the device receives it only once
- A client can set an image by hash without uploading -- if the hash exists in cache, zero bytes cross the network
- Cache entries include a reference count; eviction happens only when no button references the image
- The `GET /api/v1/images/:hash` endpoint lets clients verify cached content

---

## 7. Service Discovery

### 7.1 mDNS/Bonjour Advertisement

The control plane advertises itself on the local network using mDNS:

```
Service Type: _streamdeck._tcp.local
Instance Name: StreamDeck-XL-02803._streamdeck._tcp.local
Port: 9120
TXT Records:
  v=1                              (protocol version)
  model=STREAMDECK_XL              (device model)
  buttons=32                       (total button count)
  api=/api/v1                      (REST API path)
  ws=/ws                           (WebSocket path)
  id=AL44H1A02803                  (device serial)
```

### 7.2 Client Discovery Flow

```typescript
import { Bonjour } from 'bonjour-service';

const bonjour = new Bonjour();

bonjour.find({ type: 'streamdeck' }, (service) => {
  console.log(`Found: ${service.name}`);
  console.log(`  Host: ${service.host}:${service.port}`);
  console.log(`  Model: ${service.txt.model}`);
  console.log(`  Buttons: ${service.txt.buttons}`);

  // Connect via WebSocket
  const ws = new WebSocket(`ws://${service.host}:${service.port}/ws`);
});
```

### 7.3 Multi-Device Discovery

When multiple Stream Decks are connected (or multiple control plane instances exist), each advertises as a separate mDNS service. The `id` TXT record lets clients identify specific devices.

---

## 8. Authentication and Authorization

### 8.1 Auth Model

The control plane supports two authentication modes:

**Mode 1: API Key (simple)**
- Generated via CLI: `streamdeck-ctl auth create-key --name "ci-dashboard" --scope buttons:write`
- Passed as `Authorization: Bearer <key>` (REST) or in the `Identify` message (WebSocket)
- Suitable for trusted LAN environments

**Mode 2: No auth (localhost-only)**
- When the server is bound to `127.0.0.1`, authentication is disabled
- Useful for single-machine development

### 8.2 Permission Scopes

| Scope            | Grants                                                     |
|------------------|-------------------------------------------------------------|
| `device:read`    | Read device info, button state, brightness                  |
| `device:write`   | Set brightness, reset device                                |
| `buttons:read`   | Read button state, images, claims                           |
| `buttons:write`  | Set images, claim/release buttons                           |
| `events:subscribe` | Subscribe to WebSocket events                            |
| `webhooks:manage`| Create/delete webhook registrations                         |
| `admin`          | All permissions, including key management                   |

### 8.3 API Key Storage

Keys are stored as SHA256 hashes (never plaintext) in a local SQLite database or JSON file. The plaintext key is shown only once at creation time.

```json
// ~/.config/streamdeck-control-plane/keys.json
{
  "keys": [
    {
      "id": "key_1a2b3c",
      "name": "ci-dashboard",
      "hash": "sha256:...",
      "scopes": ["buttons:read", "buttons:write", "events:subscribe"],
      "createdAt": "2026-05-23T10:00:00Z",
      "lastUsed": "2026-05-23T11:30:00Z"
    }
  ]
}
```

---

## 9. CLI Tool Design

A companion CLI tool provides quick access to the control plane from the terminal.

### 9.1 Command Reference

```bash
# Device info
streamdeck-ctl devices                              # List connected devices
streamdeck-ctl devices AL44H1A02803                  # Show device details
streamdeck-ctl brightness 80                         # Set brightness

# Button control
streamdeck-ctl set-image --button 5 --file icon.png  # Upload and set image
streamdeck-ctl set-image --button 5 --hash sha256:... # Set by cached hash
streamdeck-ctl set-text --button 5 --text "LIVE" --bg red
streamdeck-ctl clear --button 5                       # Clear a button
streamdeck-ctl clear --all                            # Clear all buttons

# Batch operations
streamdeck-ctl load-layout layout.json               # Apply a full layout
streamdeck-ctl save-layout > current.json             # Dump current state

# Claims
streamdeck-ctl claim --buttons 0-7 --name my-app     # Claim buttons
streamdeck-ctl release --claim claim_8a7b6c5d         # Release claim
streamdeck-ctl claims                                 # List active claims

# Events (streaming)
streamdeck-ctl listen                                 # Stream all events to stdout
streamdeck-ctl listen --buttons 5,6,7 --json          # Filtered, JSON output
streamdeck-ctl listen --exec "notify-send 'Button {}'" # Run command on press

# Webhooks
streamdeck-ctl webhook add --url http://... --events buttonDown
streamdeck-ctl webhook list
streamdeck-ctl webhook remove wh_1a2b3c4d

# Auth
streamdeck-ctl auth create-key --name ci --scope buttons:write
streamdeck-ctl auth list-keys
streamdeck-ctl auth revoke-key key_1a2b3c

# Service discovery
streamdeck-ctl discover                               # Find control planes on LAN
```

### 9.2 Layout File Format

```json
{
  "version": 1,
  "device": "STREAMDECK_XL",
  "brightness": 75,
  "buttons": [
    {
      "index": 0,
      "image": "./icons/cpu.png",
      "label": "CPU",
      "onPress": {
        "webhook": "http://localhost:8080/actions/cpu-detail"
      }
    },
    {
      "index": 1,
      "render": {
        "text": "MEM\n64%",
        "backgroundColor": "#2d3436",
        "fontColor": "#74b9ff",
        "fontSize": 16
      }
    },
    {
      "index": 5,
      "image": "https://example.com/status-badge.png",
      "refresh": 30
    }
  ]
}
```

The `refresh` field causes the control plane to re-fetch the URL at the specified interval (seconds) and update the button. This enables dynamic status badges without any client-side code.

---

## 10. Integration Patterns

### 10.1 CI/CD Build Status

```bash
#!/bin/bash
# Triggered by GitHub Actions webhook on build status change

STATUS=$1  # "success", "failure", "pending"
BUTTON=0

case $STATUS in
  success)
    streamdeck-ctl set-text --button $BUTTON \
      --text "BUILD\nPASS" --bg "#00b894" --fg white
    ;;
  failure)
    streamdeck-ctl set-text --button $BUTTON \
      --text "BUILD\nFAIL" --bg "#d63031" --fg white
    ;;
  pending)
    streamdeck-ctl set-text --button $BUTTON \
      --text "BUILD\n..." --bg "#fdcb6e" --fg black
    ;;
esac
```

### 10.2 Server Monitoring Dashboard

```typescript
// TypeScript client that polls Prometheus and updates Stream Deck buttons
import { StreamDeckClient } from 'streamdeck-control-plane-client';

const client = new StreamDeckClient({
  url: 'ws://streamdeck-host:9120/ws',
  apiKey: process.env.STREAMDECK_API_KEY,
  clientName: 'prometheus-monitor',
});

await client.connect();
await client.claimButtons([0, 1, 2, 3, 4, 5, 6, 7], { priority: 100 });

async function updateMetrics() {
  const metrics = await fetchPrometheusMetrics();

  await client.batchUpdate([
    { button: 0, render: { text: `CPU\n${metrics.cpu}%`, backgroundColor: colorForPercent(metrics.cpu) } },
    { button: 1, render: { text: `MEM\n${metrics.mem}%`, backgroundColor: colorForPercent(metrics.mem) } },
    { button: 2, render: { text: `DISK\n${metrics.disk}%`, backgroundColor: colorForPercent(metrics.disk) } },
    { button: 3, render: { text: `NET\n${metrics.netMbps}M`, backgroundColor: '#2d3436' } },
  ]);
}

// Update every 5 seconds
setInterval(updateMetrics, 5000);

// Listen for button presses to drill down
client.on('buttonDown', (event) => {
  if (event.button === 0) openGrafanaDashboard('cpu');
  if (event.button === 1) openGrafanaDashboard('memory');
});
```

### 10.3 Chat/Notification Integration

```typescript
// Slack bot that shows unread message count on a Stream Deck button
client.on('buttonDown', async (event) => {
  if (event.button === 10) {
    // Button press opens Slack in the browser
    exec('open https://app.slack.com');
  }
});

slackClient.on('message', async (msg) => {
  unreadCount++;
  await client.renderAndSet({
    button: 10,
    render: {
      backgroundColor: unreadCount > 0 ? '#e74c3c' : '#2d3436',
      layers: [
        { type: 'icon', src: 'sha256:slack_icon...', x: 24, y: 12, width: 48, height: 48 },
        { type: 'text', text: `${unreadCount}`, fontSize: 14, color: '#fff', x: 48, y: 82, anchor: 'center' },
      ],
    },
  });
});
```

### 10.4 Button Press to Webhook Pipeline

The control plane can act as a bridge between physical button presses and HTTP webhooks, requiring zero client code:

```
Stream Deck Button 5 Pressed
       |
       v
  Control Plane Event Router
       |
       v
  Webhook Registry: Button 5 -> POST https://n8n.example.com/webhook/deploy
       |
       v
  HTTP POST with signed payload
       |
       v
  n8n/Zapier/custom endpoint triggers deployment
```

This pattern turns the Stream Deck into a physical webhook trigger board.

---

## 11. Technology Recommendations

### 11.1 Server Stack

| Layer             | Recommendation        | Rationale                                       |
|-------------------|-----------------------|-------------------------------------------------|
| HTTP Server       | Fastify               | Faster than Express, first-class TypeScript, JSON schema validation built-in |
| WebSocket         | ws (via fastify-websocket) | Low-level, high-performance, binary frame support |
| Image Processing  | sharp                 | Fast native JPEG resize/convert, handles rotation |
| USB HID           | @elgato-stream-deck/node | Official maintained library for device communication |
| mDNS              | bonjour-service       | Pure JS, no native dependencies                 |
| Database          | better-sqlite3        | Embedded, no server, fast for key/config storage |
| CLI               | Commander.js           | Mature, TypeScript support, subcommands          |
| Schema Validation | Zod                   | Runtime validation with TypeScript type inference |

### 11.2 Why Not tRPC or gRPC?

**tRPC** is excellent for TypeScript-to-TypeScript communication but requires a TypeScript client. The control plane should be accessible from any language -- curl, Python, Go, shell scripts. REST + WebSocket is universally supported.

**gRPC** adds complexity (protobuf compilation, HTTP/2 requirement) without clear benefit. The message rates for a 32-button device are low (tens of messages per second at peak). gRPC's streaming and binary efficiency advantages do not justify the integration friction for this use case.

### 11.3 Why Not Socket.IO?

Socket.IO adds a layer of abstraction over WebSocket (automatic reconnection, rooms, namespaces) that is useful for web applications but adds weight to this API. The control plane should be protocol-level compatible with any WebSocket client. Socket.IO's custom handshake and encoding prevent raw WebSocket clients from connecting. Use the bare `ws` library and implement reconnection logic in the client SDK.

### 11.4 Server-Sent Events (SSE) Consideration

SSE is a viable alternative to WebSocket for clients that only need to **receive** events (no bidirectional communication). The control plane should offer an SSE endpoint as a lightweight alternative:

```
GET /api/v1/events?subscribe=buttonDown,buttonUp&buttons=0,1,2,3
Accept: text/event-stream
Authorization: Bearer <key>
```

```
event: buttonDown
data: {"deviceId":"AL44H1A02803","button":5,"timestamp":"2026-05-23T11:10:00Z"}

event: buttonUp
data: {"deviceId":"AL44H1A02803","button":5,"timestamp":"2026-05-23T11:10:00.150Z"}
```

SSE is particularly useful for:
- Monitoring dashboards that only display events
- Serverless functions that cannot maintain WebSocket connections
- Browser-based clients (SSE has native browser support via `EventSource`)

---

## 12. Performance and Latency Budget

### 12.1 Latency Targets

| Path                              | Budget    | Notes                                      |
|-----------------------------------|-----------|--------------------------------------------|
| USB HID report write              | 1-3 ms    | Hardware-limited                           |
| Image normalization (sharp)       | 5-15 ms   | 96x96 JPEG encode/resize is trivial        |
| WebSocket message transit (LAN)   | < 1 ms    | Local network, sub-millisecond             |
| REST request round-trip (LAN)     | 1-5 ms    | HTTP overhead minimal on LAN               |
| Button press to client delivery   | < 10 ms   | Target: imperceptible                      |
| Image upload to display on device | < 50 ms   | Full pipeline: receive + normalize + USB   |
| Full 32-button refresh            | < 500 ms  | 32 images at ~5 KB each = ~160 KB          |

### 12.2 Throughput Estimates

The Stream Deck XL's USB HID interface is the bottleneck, not the network:

- USB HID output reports: 1024 bytes per report, max ~1000 reports/second
- A single 96x96 JPEG image: ~3-8 KB = 3-8 reports
- Theoretical max image throughput: ~125-333 images/second
- Practical max with overhead: ~50-100 images/second
- At 32 buttons: ~1.5-3 full refreshes per second

The WebSocket and REST APIs will never be the bottleneck. Even on WiFi, the network can deliver data faster than the USB interface can consume it.

---

## 13. Error Handling and Resilience

### 13.1 Error Code Reference

| Code                    | HTTP Status | Description                                       |
|-------------------------|-------------|---------------------------------------------------|
| `DEVICE_NOT_FOUND`      | 404         | Device ID does not match any connected device      |
| `BUTTON_OUT_OF_RANGE`   | 400         | Button index exceeds device button count           |
| `BUTTON_NOT_CLAIMED`    | 403         | Client has not claimed the target button           |
| `CLAIM_CONFLICT`        | 409         | Button claimed by higher-priority client           |
| `IMAGE_NOT_FOUND`       | 404         | Referenced image hash not in cache                 |
| `IMAGE_TOO_LARGE`       | 413         | Source image exceeds 10 MB upload limit            |
| `IMAGE_INVALID`         | 422         | Image could not be decoded                         |
| `AUTH_REQUIRED`         | 401         | Missing or invalid authentication                  |
| `INSUFFICIENT_SCOPE`    | 403         | API key lacks required permission scope            |
| `RATE_LIMITED`          | 429         | Exceeded max requests per second                   |
| `DEVICE_ASLEEP`         | 409         | Device is in sleep mode; wake it first             |
| `INTERNAL_ERROR`        | 500         | Unexpected server error                            |

### 13.2 Device Disconnect Handling

When the Stream Deck is physically disconnected:

1. The control plane emits a `deviceDisconnected` event to all subscribed WebSocket clients
2. All claims remain in memory (they are logical, not physical)
3. Image state is preserved in the cache
4. Requests targeting the disconnected device return `DEVICE_NOT_FOUND` errors
5. When the device is reconnected, a `deviceConnected` event fires
6. Clients with active claims can re-apply their images (the control plane can optionally auto-restore from cached state)

### 13.3 WebSocket Reconnection

The client SDK should implement exponential backoff reconnection:

```
Attempt 1: wait 1s
Attempt 2: wait 2s
Attempt 3: wait 4s
Attempt 4: wait 8s
Attempt 5+: wait 15s (cap)
```

On reconnection, the client must re-send `Identify` and re-establish claims. The server preserves claim state for a configurable grace period (default: 60 seconds) to allow seamless reconnection without losing button ownership.

---

## 14. Comparison with Alternative Protocols

### 14.1 Protocol Suitability Matrix

| Protocol     | Bidirectional | Binary Data | Browser Support | Multi-language | Latency | Complexity |
|--------------|---------------|-------------|-----------------|----------------|---------|------------|
| REST/HTTP    | No            | Yes (body)  | Native          | Universal      | Medium  | Low        |
| WebSocket    | Yes           | Yes (frames)| Native          | Universal      | Low     | Medium     |
| SSE          | Server->Client| No          | Native          | Good           | Low     | Low        |
| gRPC         | Yes           | Yes (native)| Via grpc-web    | Good           | Low     | High       |
| tRPC         | Yes           | Via adapter | Via client       | TypeScript only| Low     | Medium     |
| OSC/UDP      | Yes           | Yes         | No              | Good           | Very Low| Medium     |
| MQTT         | Pub/Sub       | Yes         | Via client       | Good           | Low     | Medium     |

### 14.2 Recommendation

The proposed architecture uses **REST + WebSocket + SSE** because:

1. **REST** is the universal integration language -- every programming language and tool can make HTTP requests
2. **WebSocket** provides the real-time bidirectional channel needed for button events and live image updates
3. **SSE** offers a lightweight read-only alternative for monitoring use cases
4. All three are natively supported in browsers, enabling web-based dashboards without additional dependencies

OSC/UDP could be added as an optional transport for live-performance integrations where sub-millisecond latency matters and reliability is less important.

---

## 15. Future Considerations

### 15.1 Multi-Device Coordination

When multiple Stream Decks are connected, the control plane could support:
- Treating N devices as a single extended surface (e.g., two XLs = 64 buttons in an 8x8 grid)
- Mirror mode (same content on all devices)
- Device groups with shared claim namespaces

### 15.2 Plugin System

A plugin system could allow the control plane to run server-side logic:
- Timer plugins (clock display, countdown)
- Polling plugins (fetch URL every N seconds, render result to button)
- Animation plugins (rotate through images on a button)
- Integration plugins (native Slack/GitHub/Home Assistant adapters)

### 15.3 State Persistence

On restart, the control plane could restore:
- Last known button images from the image cache
- Active claims (if clients reconnect within the grace period)
- Webhook registrations
- Brightness and device preferences

### 15.4 Remote Access (Beyond LAN)

For access outside the local network:
- Cloudflare Tunnel or Tailscale for secure exposure
- OAuth2/OIDC for production-grade authentication
- Rate limiting and abuse prevention
- End-to-end encryption for image data

---

## Sources

1. [OBS WebSocket v5 Protocol Specification](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md)
2. [OBS WebSocket Message Types and OpCodes (DeepWiki)](https://deepwiki.com/obsproject/obs-websocket/2.1-message-types-and-opcodes)
3. [Bitfocus Companion Satellite API](https://github.com/bitfocus/companion/wiki/Satellite-API)
4. [Bitfocus Companion Satellite Protocol over WebSocket](https://github.com/bitfocus/companion/issues/3101)
5. [Elgato Stream Deck Plugin SDK - WebSocket Reference](https://docs.elgato.com/streamdeck/sdk/references/websocket/plugin/)
6. [Elgato Stream Deck SDK - WebSocket API Changes](https://docs.elgato.com/streamdeck/sdk/references/websocket/changelog/)
7. [Home Assistant WebSocket API](https://developers.home-assistant.io/docs/api/websocket/)
8. [Home Assistant REST and WebSocket APIs (DeepWiki)](https://deepwiki.com/home-assistant/developers.home-assistant/6.2-rest-and-websocket-apis)
9. [Open Sound Control 1.0 Specification](https://hangar.org/wp-content/uploads/2012/01/The-Open-Sound-Control-1.0-Specification-opensoundcontrol.org_.pdf)
10. [RTP-MIDI (Wikipedia)](https://en.wikipedia.org/wiki/RTP-MIDI)
11. [Apple MIDI Network Driver Protocol](https://developer.apple.com/library/archive/documentation/Audio/Conceptual/MIDINetworkDriverProtocol/MIDI/MIDI.html)
12. [StreamDeckWS - WebSocket proxy for Stream Deck to Node-RED](https://github.com/ybizeul/StreamDeckWS)
13. [StreamDeckProductionController - Node.js Stream Deck controller](https://github.com/josephdadams/StreamDeckProductionController)
14. [@elgato-stream-deck/node (npm)](https://www.npmjs.com/package/elgato-stream-deck)
15. [bonjour-service (npm) - mDNS/Bonjour for Node.js](https://www.npmjs.com/package/bonjour)
16. [tRPC vs REST vs WebSocket API Design Comparison](https://medium.com/@hnwagba/choosing-your-frontend-api-weapon-rest-graphql-trpc-or-websocket-personal-rant-edition-b64854ed3f17)
