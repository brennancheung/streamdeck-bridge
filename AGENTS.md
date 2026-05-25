# Stream Deck Bridge

Rust binary that turns the Elgato Stream Deck XL into a network-accessible device via WebSocket on `localhost:9001`.

## Architecture

Three layers, all in this repo:

1. **USB HID Driver** — Raw HID communication via the `elgato-streamdeck` crate
2. **Device Abstraction** — Image cache (32 slots), button state diffing, JPEG encoding, dedup
3. **WebSocket Protocol** — JSON text frames for commands/events, binary frames for images

Client applications connect over WebSocket and implement their own logic (modes, rendering, automation). The bridge has no opinions about what's displayed or what button presses mean.

## Code structure

```
src/
├── main.rs        — CLI args, tokio runtime, channel wiring
├── protocol.rs    — Command/Event JSON types (serde)
├── device.rs      — Device manager: connect, button polling, image cache, command dispatch
├── server.rs      — WebSocket server, per-client handler
```

## Key design decisions

- **Rust** over Swift/Node — single binary, existing HID crate, mature async ecosystem
- **WebSocket** over stdin/stdout — standalone, testable, multi-client, binary frames

## Build & run

```bash
cargo build --release
./target/release/streamdeck-bridge --port 9001
```

## Documentation

- [`docs/protocol.md`](docs/protocol.md) — Full WebSocket protocol reference
- [`docs/architecture.md`](docs/architecture.md) — Layer definitions and design decisions
- [`docs/vision.md`](docs/vision.md) — Design principles and scope
- [`docs/design.md`](docs/design.md) — Use cases, requirements, user stories
- [`docs/deep-research/`](docs/deep-research/) — Hardware/protocol/ecosystem research
