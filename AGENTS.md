# Stream Deck Bridge

Rust binary that provides USB HID control of the Elgato Stream Deck XL, exposed via a WebSocket API on `localhost:9001`.

## Architecture

Three layers, all in this repo:

1. **USB HID Driver** — Raw HID communication via `elgato-streamdeck` crate
2. **Device Abstraction** — Image cache (32 slots), button state diffing, JPEG encoding
3. **WebSocket Protocol** — JSON text frames for commands/events, binary frames for images

Higher-level logic (modes, rendering, automation) lives in a separate Phoenix app repo.

See `docs/architecture.md` for full layer definitions and decision records.

## Build & Run

```bash
cargo build --release
./target/release/streamdeck-bridge --port 9001
```

Requires `hidapi` system library (installed via `brew install hidapi` on macOS).

## Key decisions

- **Rust** over Swift/Node — single static binary, existing HID crate, mature async ecosystem (D-1)
- **WebSocket** over stdin/stdout BEAM Port — standalone, testable, multi-client, binary frames (D-2)
- **Phoenix** for the client app in a separate repo (D-3)

## Code structure

```
src/
├── main.rs        — CLI args, tokio runtime, channel wiring
├── protocol.rs    — Command/Event JSON types (serde)
├── device.rs      — Device manager: connect, button polling, image cache, command dispatch
├── server.rs      — WebSocket server, per-client handler
```

## WebSocket protocol

See `docs/requirement-gathering.md` section R-5 for the full protocol spec.

## Research

Comprehensive deep research in `docs/deep-research/stream-deck-xl-direct-control/`.
