# Smart Home and IoT

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Novel Use Cases & Ideas

---

## Key Findings

- Home Assistant is the strongest integration point; its WebSocket API enables real-time state subscriptions and service calls from Node.js via the official `home-assistant-js-websocket` library
- Philips Hue lights can be controlled directly via `node-hue-api` without Home Assistant -- bridge discovery, auth, and light state changes are all supported
- MQTT (via `mqtt.js`) is the universal glue for IoT -- Zigbee2MQTT, Tasmota, ESPHome, and DIY sensors all speak it natively
- Stream Deck buttons can display live device status by subscribing to state changes and rendering dynamic SVG icons with `sharp`
- Camera snapshots from Home Assistant's `/api/camera_proxy/` endpoint can be fetched, resized to 96x96, and displayed on keys
- macOS Shortcuts can be triggered via `child_process.exec('shortcuts run "Name"')`, bridging into Apple HomeKit
- Scene buttons are straightforward: one press fires `scene.turn_on` or an MQTT publish sequence

---

## 1. Home Assistant Integration

Home Assistant aggregates hundreds of device types behind a unified API, making it the single best integration target.

### 1.1 REST API

All requests need a long-lived access token (generate at `http://YOUR_HA:8123/profile`).

| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/api/states/<entity_id>` | Get entity state |
| POST | `/api/services/<domain>/<service>` | Call a service |
| GET | `/api/camera_proxy/<entity_id>` | Camera snapshot JPEG |

```typescript
const HA_URL = 'http://homeassistant.local:8123';
const TOKEN = process.env.HA_TOKEN;
const headers = { 'Content-Type': 'application/json', 'Authorization': `Bearer ${TOKEN}` };

// Toggle a light
await fetch(`${HA_URL}/api/services/light/toggle`, {
  method: 'POST', headers,
  body: JSON.stringify({ entity_id: 'light.kitchen' }),
});

// Get entity state
const res = await fetch(`${HA_URL}/api/states/light.kitchen`, { headers });
const { state, attributes } = await res.json();
// state: "on"/"off", attributes: { brightness, color_temp, friendly_name, ... }
```

### 1.2 WebSocket API (Preferred)

The WebSocket API avoids polling and pushes state changes in real time. The official library handles reconnection and resubscription automatically.

```bash
pnpm add home-assistant-js-websocket
```

```typescript
import {
  createConnection, createLongLivedTokenAuth,
  subscribeEntities, callService,
} from 'home-assistant-js-websocket';

const auth = createLongLivedTokenAuth('http://homeassistant.local:8123', TOKEN);
const connection = await createConnection({ auth });

// Real-time state updates for ALL entities
subscribeEntities(connection, (entities) => {
  const light = entities['light.living_room'];
  if (light) updateButtonIcon(0, light); // push to Stream Deck
});

// Call services
await callService(connection, 'light', 'turn_on',
  { brightness_pct: 80 },
  { entity_id: 'light.living_room' }
);

// Activate a scene
await callService(connection, 'scene', 'turn_on', {},
  { entity_id: 'scene.movie_night' }
);
```

---

## 2. Philips Hue Direct Control

For Hue-only setups without Home Assistant, `node-hue-api` talks directly to the bridge.

```bash
pnpm add node-hue-api
```

```typescript
const v3 = require('node-hue-api').v3;
const { LightState } = v3.lightStates;

// One-time setup: discover bridge, create user (press Link button first)
const [bridge] = await v3.discovery.nupnpSearch();
const unauthApi = await v3.api.createLocal(bridge.ipaddress).connect();
const user = await unauthApi.users.createUser('streamdeck', 'my-desk');

// Connect with saved credentials
const api = await v3.api.createLocal(bridge.ipaddress).connect(user.username);

// Control lights
await api.lights.setLightState(1, new LightState().on().brightness(80).ct(350));
await api.lights.setLightState(1, new LightState().on().rgb(255, 0, 100));
await api.lights.setLightState(1, new LightState().off());

// Read state
const light = await api.lights.getLight(1);
console.log(`On: ${light.state.on}, Brightness: ${light.state.bri}`);

// Stream Deck toggle pattern
streamDeck.on('down', async (keyIndex: number) => {
  const lightId = buttonLightMap[keyIndex];
  if (lightId === undefined) return;
  const light = await api.lights.getLight(lightId);
  const newState = light.state.on ? new LightState().off() : new LightState().on().brightness(80);
  await api.lights.setLightState(lightId, newState);
});
```

---

## 3. MQTT for Generic IoT

MQTT is the lingua franca of IoT. Zigbee2MQTT, Tasmota, ESPHome, Shelly, and DIY ESP32 projects all use it.

```bash
pnpm add mqtt
```

```typescript
import mqtt from 'mqtt';

const client = mqtt.connect('mqtt://your-broker:1883', {
  clientId: `streamdeck_${Math.random().toString(16).slice(3)}`,
  username: 'user', password: 'password',
  reconnectPeriod: 5000,
});

// Subscribe to Zigbee2MQTT device states
client.subscribe('zigbee2mqtt/+');
client.on('message', (topic, payload) => {
  const state = JSON.parse(payload.toString());
  // state: { state: "ON", brightness: 200, color_temp: 350 }
});

// Control a Zigbee2MQTT light
client.publish('zigbee2mqtt/living_room_light/set', JSON.stringify({
  state: 'ON', brightness: 200, color_temp: 350, transition: 2,
}));

// Toggle
client.publish('zigbee2mqtt/living_room_light/set', JSON.stringify({ state: 'TOGGLE' }));

// Tasmota smart plug
client.publish('cmnd/coffee_maker/POWER', 'ON');

// ESP32 temperature sensor
client.subscribe('home/office/temperature');
// payload arrives as a float string, e.g. "22.5"
```

**MQTT-only scene** (no Home Assistant needed):

```typescript
const movieScene = [
  { topic: 'zigbee2mqtt/living_room_light/set', payload: { state: 'ON', brightness: 30, color_temp: 400 } },
  { topic: 'zigbee2mqtt/ceiling_light/set', payload: { state: 'OFF' } },
  { topic: 'zigbee2mqtt/led_strip/set', payload: { state: 'ON', brightness: 50, color: { r: 80, g: 0, b: 120 } } },
];

streamDeck.on('down', (keyIndex: number) => {
  if (keyIndex === 24) {
    movieScene.forEach(a => client.publish(a.topic, JSON.stringify(a.payload)));
  }
});
```

---

## 4. Matter and HomeKit

### Matter via matter.js

`@matter/main` is a full TypeScript Matter protocol implementation. It can act as a controller to commission and control Matter devices (on/off, level, color clusters). However, commissioning is complex -- for most users, routing Matter through Home Assistant is simpler.

```bash
pnpm add @matter/main
```

### HomeKit via macOS Shortcuts

The most practical HomeKit path on macOS. Create Shortcuts that use the "Control Home" action, then trigger them from Node.js:

```typescript
import { exec } from 'child_process';
import { promisify } from 'util';
const execAsync = promisify(exec);

async function runShortcut(name: string) {
  const { stdout } = await execAsync(`shortcuts run "${name}"`);
  return stdout.trim();
}

// Bind to Stream Deck buttons
const shortcutButtons: Record<number, string> = {
  16: 'Turn Off All Lights',
  17: 'Arriving Home',
  18: 'Leaving Home',
  19: 'Bedtime',
};

streamDeck.on('down', async (keyIndex: number) => {
  const name = shortcutButtons[keyIndex];
  if (name) await runShortcut(name);
});
```

### Homebridge

Homebridge is a Node.js server that bridges non-HomeKit devices into HomeKit. From v2, it also exposes devices over Matter. Primarily an accessory server (exposing devices TO controllers), not a control client.

---

## 5. Displaying Device Status on Buttons

The key advantage over physical switches: buttons show live state. Use `sharp` to render SVG into raw pixel buffers.

```bash
pnpm add sharp
```

```typescript
import sharp from 'sharp';

async function createStatusIcon(label: string, isOn: boolean, value?: string): Promise<Buffer> {
  const bg = isOn ? '#1a5c2a' : '#3a3a3a';
  const accent = isOn ? '#4ade80' : '#6b7280';
  const svg = `
    <svg width="96" height="96" xmlns="http://www.w3.org/2000/svg">
      <rect width="96" height="96" rx="8" fill="${bg}"/>
      <circle cx="48" cy="30" r="10" fill="${accent}"/>
      <text x="48" y="62" text-anchor="middle" font-family="sans-serif"
            font-size="13" fill="white">${label}</text>
      <text x="48" y="82" text-anchor="middle" font-family="sans-serif"
            font-size="12" fill="${accent}">${value ?? (isOn ? 'ON' : 'OFF')}</text>
    </svg>`;
  return sharp(Buffer.from(svg)).resize(96, 96).raw().toBuffer();
}

// Thermostat variant
async function createThermostatIcon(current: number, target: number, heating: boolean): Promise<Buffer> {
  const color = heating ? '#f97316' : '#3b82f6';
  const svg = `
    <svg width="96" height="96" xmlns="http://www.w3.org/2000/svg">
      <rect width="96" height="96" rx="8" fill="#1e1e2e"/>
      <text x="48" y="40" text-anchor="middle" font-family="sans-serif"
            font-size="28" font-weight="bold" fill="${color}">${current.toFixed(0)}°</text>
      <text x="48" y="62" text-anchor="middle" font-family="sans-serif"
            font-size="12" fill="#9ca3af">Target: ${target}°</text>
      <text x="48" y="82" text-anchor="middle" font-family="sans-serif"
            font-size="11" fill="${color}">${heating ? 'HEATING' : 'IDLE'}</text>
    </svg>`;
  return sharp(Buffer.from(svg)).resize(96, 96).raw().toBuffer();
}
```

### Live Updates via Home Assistant WebSocket

```typescript
const keyEntityMap: Record<number, string> = {
  0: 'light.living_room',
  1: 'light.kitchen',
  2: 'climate.thermostat',
  3: 'lock.front_door',
};

subscribeEntities(connection, async (entities) => {
  for (const [key, entityId] of Object.entries(keyEntityMap)) {
    const entity = entities[entityId];
    if (!entity) continue;
    const idx = Number(key);
    const domain = entityId.split('.')[0];

    if (domain === 'light') {
      const bri = entity.attributes.brightness;
      const pct = bri ? `${Math.round((bri / 255) * 100)}%` : undefined;
      await streamDeck.fillImage(idx, await createStatusIcon(
        entity.attributes.friendly_name, entity.state === 'on', pct));
    }
    if (domain === 'climate') {
      await streamDeck.fillImage(idx, await createThermostatIcon(
        entity.attributes.current_temperature,
        entity.attributes.temperature,
        entity.state === 'heating'));
    }
  }
});
```

---

## 6. Security Camera Snapshots on Buttons

### From Home Assistant

```typescript
async function displayCameraOnButton(keyIndex: number, entityId: string) {
  const res = await fetch(`${HA_URL}/api/camera_proxy/${entityId}`, { headers });
  const snapshot = Buffer.from(await res.arrayBuffer());
  const resized = await sharp(snapshot).resize(96, 96, { fit: 'cover' }).raw().toBuffer();
  await streamDeck.fillImage(keyIndex, resized);
}

// Refresh every 10 seconds
setInterval(() => {
  displayCameraOnButton(30, 'camera.front_door');
  displayCameraOnButton(31, 'camera.backyard');
}, 10_000);
```

### From RTSP Cameras (via ffmpeg)

```typescript
async function getRtspSnapshot(rtspUrl: string): Promise<Buffer> {
  const { stdout } = await execAsync(
    `ffmpeg -i "${rtspUrl}" -frames:v 1 -f image2pipe -vcodec mjpeg pipe:1`,
    { encoding: 'buffer', maxBuffer: 5 * 1024 * 1024 },
  );
  return stdout;
}
// Common snapshot URLs: Hikvision /ISAPI/Streaming/channels/101/picture
// Dahua/Amcrest /cgi-bin/snapshot.cgi, Generic /snap.jpg
```

---

## 7. Scene Buttons

A scene button has three parts: trigger (button press), feedback (icon), and definition (HA scene or MQTT sequence).

```typescript
const sceneButtons: Record<number, { scene: string; label: string; color: string }> = {
  24: { scene: 'scene.movie_night',  label: 'Movie',   color: '#7c3aed' },
  25: { scene: 'scene.dinner',       label: 'Dinner',  color: '#f59e0b' },
  26: { scene: 'scene.bright_work',  label: 'Work',    color: '#3b82f6' },
  27: { scene: 'scene.goodnight',    label: 'Night',   color: '#1e3a5f' },
  28: { scene: 'scene.all_off',      label: 'All Off', color: '#ef4444' },
};

// Render colored scene buttons
for (const [key, { label, color }] of Object.entries(sceneButtons)) {
  const svg = `<svg width="96" height="96" xmlns="http://www.w3.org/2000/svg">
    <rect width="96" height="96" rx="10" fill="${color}"/>
    <text x="48" y="55" text-anchor="middle" font-family="sans-serif"
          font-size="14" font-weight="bold" fill="white">${label}</text></svg>`;
  await streamDeck.fillImage(Number(key), await sharp(Buffer.from(svg)).resize(96, 96).raw().toBuffer());
}

// One-press scene activation with flash feedback
streamDeck.on('down', async (keyIndex: number) => {
  const cfg = sceneButtons[keyIndex];
  if (!cfg) return;
  await callService(connection, 'scene', 'turn_on', {}, { entity_id: cfg.scene });
  await streamDeck.fillKeyColor(keyIndex, 255, 255, 255); // flash white
  setTimeout(() => { /* re-render normal icon */ }, 200);
});
```

---

## 8. Recommended Architecture

### Hub-Based (Recommended)

```
Stream Deck XL  -->  Node.js Controller  -->  Home Assistant (WebSocket)
                                          |       |-- Zigbee, Z-Wave, Matter
                                          |       |-- Wi-Fi (Hue, LIFX, TP-Link)
                                          |       |-- Cameras, Scenes
                                          |-->  macOS Shortcuts --> HomeKit
```

Home Assistant normalizes every device behind one API. The controller needs a single integration.

### Direct Control (Simpler Setups)

```
Node.js Controller  -->  node-hue-api --> Hue Bridge
                     -->  mqtt.js --> MQTT Broker --> Zigbee2MQTT, Tasmota, ESPHome
                     -->  @matter/main --> Matter devices
                     -->  HTTP --> IP camera snapshots
                     -->  shortcuts CLI --> macOS HomeKit
```

### Key npm Packages

| Package | Purpose |
|---------|---------|
| `home-assistant-js-websocket` | Official HA WebSocket client |
| `node-hue-api` | Direct Philips Hue bridge control |
| `mqtt` | MQTT client for IoT devices |
| `@matter/main` | Matter protocol controller |
| `sharp` | Dynamic button icon generation |
| `@elgato-stream-deck/node` | Direct USB Stream Deck control |

---

## Sources

1. [Home Assistant WebSocket API Docs](https://developers.home-assistant.io/docs/api/websocket/)
2. [Home Assistant REST API Docs](https://developers.home-assistant.io/docs/api/rest/)
3. [home-assistant-js-websocket (Official JS Client)](https://github.com/home-assistant/home-assistant-js-websocket)
4. [node-hue-api (Philips Hue Node.js Library)](https://github.com/peter-murray/node-hue-api)
5. [MQTT.js (Node.js MQTT Client)](https://github.com/mqttjs)
6. [matter.js (Matter Protocol for JS/TS)](https://github.com/project-chip/matter.js/)
7. [Homebridge (HomeKit Bridge)](https://github.com/homebridge/homebridge)
8. [Zigbee2MQTT MQTT Topics and Messages](https://www.zigbee2mqtt.io/guide/usage/mqtt_topics_and_messages.html)
9. [streamdeck-homeassistant (Existing HA Plugin)](https://github.com/cgiesche/streamdeck-homeassistant)
10. [home-assistant-streamdeck-yaml](https://github.com/basnijholt/home-assistant-streamdeck-yaml/)
11. [Node.js MQTT Tutorial (EMQX)](https://www.emqx.com/en/blog/how-to-use-mqtt-in-nodejs)
12. [node-elgato-stream-deck (Direct USB Control)](https://github.com/Julusian/node-elgato-stream-deck)
13. [rtsp-ffmpeg (Camera Snapshot)](https://www.npmjs.com/package/rtsp-ffmpeg)
14. [macOS Shortcuts CLI (Apple Support)](https://support.apple.com/guide/shortcuts-mac/run-shortcuts-from-the-command-line-apd455c82f02/mac)
15. [Home Assistant Camera Proxy](https://www.home-assistant.io/integrations/proxy/)
16. [Sharp (Image Processing)](https://github.com/lovell/sharp)
