# Stream Deck XL — Direct Control

Custom software for the **Elgato Stream Deck XL** (32-button, 8x4 grid, 96x96px per-button LCD) — bypassing Elgato's official software for full programmatic control over USB HID.

## Research

Comprehensive deep research covering hardware protocol, open-source libraries, macOS automation, performance characteristics, and novel use cases.

**Start here:** [`docs/deep-research/stream-deck-xl-direct-control/overview.md`](docs/deep-research/stream-deck-xl-direct-control/overview.md)

### Topics Covered

| Area | Documents | Key Finding |
|------|-----------|-------------|
| **Hardware & HID Protocol** | 4 docs | Full 32-key rollover, state bitmap model, 96x96 JPEG, official HID docs now published |
| **Open-Source Libraries** | 4 docs | `@elgato-stream-deck/node` is the definitive TypeScript library; WebHID enables browser control |
| **Applications & Architecture** | 3 docs | No existing app exposes a REST/WebSocket API — clear gap to fill |
| **macOS Automation** | 4 docs | Volume, media, keyboard, window management, system monitoring all accessible from Node.js |
| **Image Performance** | 2 docs | 5-15ms per button, 50-200ms full refresh, no firmware rate limit |
| **Novel Use Cases** | 6 docs | Chord combos, mode layers, control plane API, dashboards, smart home, dev productivity |

### Recommended Stack

- **Device**: `@elgato-stream-deck/node` + `@julusian/jpeg-turbo`
- **Rendering**: `sharp` + `@napi-rs/canvas`
- **API**: `fastify` + `ws` + `bonjour-service`
- **Automation**: `systeminformation`, `nut.js`, `nowplaying-cli`

## Project Direction

See [`AGENTS.md`](AGENTS.md) for project context and research expectations.
