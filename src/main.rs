mod device;
mod protocol;
mod server;

use clap::Parser;
use tokio::sync::{broadcast, mpsc, watch};

use crate::device::DeviceState;

#[derive(Parser)]
#[command(name = "streamdeck-bridge", about = "Stream Deck XL USB-to-WebSocket bridge")]
struct Args {
    #[arg(short, long, default_value_t = 9001)]
    port: u16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let (cmd_tx, cmd_rx) = mpsc::channel(256);
    let (event_tx, _) = broadcast::channel(256);
    let (state_tx, state_rx) = watch::channel(DeviceState::Disconnected);

    let event_tx_device = event_tx.clone();
    tokio::spawn(async move {
        device::run(cmd_rx, event_tx_device, state_tx).await;
    });

    server::run(args.port, cmd_tx, event_tx, state_rx).await;
}
