mod device;
mod protocol;
mod server;

use clap::Parser;
use tokio::sync::{broadcast, mpsc, watch};

use crate::device::DeviceState;

const DEFAULT_PORT: u16 = 9001;

#[derive(Parser)]
#[command(
    name = "streamdeck-bridge",
    version,
    about = "Stream Deck XL USB-to-WebSocket bridge",
    long_about = "Translates Stream Deck USB HID protocol into a WebSocket API on localhost.\n\
                  Any language or application can control the 32-button LCD screen\n\
                  via simple JSON commands over a WebSocket connection.",
    after_help = "\
EXAMPLES:
  streamdeck-bridge              Start with default port (9001)
  streamdeck-bridge -p 8080      Start on custom port
  streamdeck-bridge --test       Verify device connection with a visual test

CONNECTION:
  WebSocket URL: ws://localhost:<port>
  Protocol:      JSON text frames for commands/events, binary frames for images
  See docs/protocol.md for the full protocol reference."
)]
struct Args {
    /// WebSocket server port
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Run a visual test to verify the Stream Deck is connected and working
    #[arg(short, long)]
    test: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.test {
        if let Err(e) = device::run_test(DEFAULT_PORT).await {
            eprintln!("Test failed: {}", e);
            std::process::exit(1);
        }
        return;
    }

    tracing_subscriber::fmt::init();

    let (cmd_tx, cmd_rx) = mpsc::channel(256);
    let (event_tx, _) = broadcast::channel(256);
    let (state_tx, state_rx) = watch::channel(DeviceState::Disconnected);

    let event_tx_device = event_tx.clone();
    tokio::spawn(async move {
        device::run(cmd_rx, event_tx_device, state_tx).await;
    });

    server::run(args.port, cmd_tx, event_tx, state_rx).await;
}
