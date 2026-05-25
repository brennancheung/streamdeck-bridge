# Stream Deck Bridge — Agent Context

Rust binary that turns the Elgato Stream Deck XL into a WebSocket-accessible device on `localhost:9001`. This file is for AI agents working on this repo.

## How it works

The bridge is an actor system with three communication channels:

```
WebSocket clients ──→ mpsc::channel<DeviceCommand> ──→ Device actor
                                                          │
WebSocket clients ←── broadcast::channel<Event> ──────────┘
                                                          
New clients read ←── watch::channel<DeviceState> (current connection state)
```

1. The **device actor** (`device::run`) owns the USB connection and image cache. It runs in a dedicated tokio task with two concurrent loops: button polling (spawned subtask at 60Hz) and command processing (via `tokio::select!`).
2. The **WebSocket server** (`server::run`) accepts connections and spawns a task per client. Each client task forwards JSON commands to the device actor and relays broadcast events back to the client.
3. The **protocol** (`protocol.rs`) defines all JSON types via serde-tagged enums. `Command` uses `#[serde(tag = "cmd")]` and `Event` uses `#[serde(tag = "event")]`.

## Code map

| File | Role | Key types/functions |
|------|------|-------------------|
| `src/main.rs` | Entry point. CLI args via clap, creates channels, spawns device actor, runs server. | `Args` |
| `src/protocol.rs` | JSON wire types. No logic. | `Command`, `Event`, `ImageFormat` |
| `src/device.rs` | Device actor. Connects to hardware, polls buttons, processes commands, manages image cache. | `run()`, `handle_command()`, `DeviceCommand`, `DeviceState`, `DeviceInfo` |
| `src/server.rs` | WebSocket server. Accepts clients, routes messages, handles binary frame protocol. | `run()`, `handle_client()`, `expects_binary()` |

## Internal flow for a command

Example: client sends `{"cmd": "set_color", "key": 5, "r": 255, "g": 0, "b": 0}`

1. `server::handle_client` receives text frame, deserializes to `Command::SetColor`
2. `expects_binary()` returns false → command sent immediately via `cmd_tx`
3. `device::run` receives `DeviceCommand` in the `select!` loop
4. `handle_command()` creates a 96x96 solid-color `DynamicImage`, hashes the input `[r, g, b]`
5. Checks image cache — if hash matches slot 5, skips USB transfer
6. Otherwise: calls `deck.set_button_image(5, image)` then `deck.flush()`
7. Updates cache slot 5, sends `Event::ImageSet { key: 5 }` via broadcast
8. All connected clients receive the event as a JSON text frame

For image commands (`set_image`, `set_image_jpeg`), the flow has an extra step: `expects_binary()` returns true, so the server stores the command as `pending_cmd` and waits for the next binary frame before sending the `DeviceCommand`.

## Image cache

- `Vec<Option<ImageSlot>>` sized to the device's key count (32 for XL)
- Each slot stores a SHA-256 hash of the input bytes and the decoded `DynamicImage`
- Dedup: if the hash matches, skip the USB transfer entirely
- Restore: on device reconnect, all cached images are re-pushed via `set_button_image` + `flush`
- Cache is cleared by `clear_key`, `clear_all`, and `reset_to_logo`

## Concurrency notes

- `AsyncStreamDeck` uses interior mutability — safe to share via `Arc` between the button reader task and command handler
- Button reader runs in a spawned task; its `JoinHandle` is polled in the main `select!` loop to detect disconnection
- `read_input()` uses `block_in_place` internally — requires tokio `rt-multi-thread` (set by `#[tokio::main]`)
- The `watch::Ref` from `state_rx.borrow()` is not `Send` — must be cloned before any `.await`

## Adding a new command

1. Add the variant to `Command` in `protocol.rs` (serde handles JSON deserialization)
2. If it needs binary data, add it to `expects_binary()` in `server.rs`
3. Handle it in the `match cmd.command` block in `device::handle_command()`
4. If it produces a response, send an `Event` via `event_tx`

## Adding a new event

1. Add the variant to `Event` in `protocol.rs` (serde handles JSON serialization)
2. Send it via `event_tx.send()` wherever appropriate in `device.rs`
3. It automatically broadcasts to all connected WebSocket clients

## Build and test

```bash
cargo build --release
RUST_LOG=info ./target/release/streamdeck-bridge
```

Testing requires a physical Stream Deck connected via USB. There are no mock/simulated device tests yet. Use any WebSocket client (Python `websockets`, `websocat`, browser console) to exercise the protocol.

## Not yet implemented

- `set_panel_image` — the `Command` variant exists in protocol.rs but returns an error in device.rs
- CLI mode for one-shot commands (planned: clap subcommands)
- Panel-wide image slicing (768x384 → 32 individual 96x96 keys)

## Key dependencies

| Crate | Purpose |
|-------|---------|
| `elgato-streamdeck` (0.12, `async` feature) | USB HID communication, image encoding, device detection |
| `tokio` (`full` features) | Async runtime, channels, timers |
| `tokio-tungstenite` (0.23) | WebSocket server |
| `image` (0.25) | Image decoding (PNG, JPEG) and manipulation |
| `serde` / `serde_json` | JSON serialization for the protocol |
| `sha2` | Image dedup hashing |
| `clap` (4, `derive` feature) | CLI argument parsing |
| `tracing` / `tracing-subscriber` | Structured logging |
