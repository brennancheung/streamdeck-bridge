use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use crate::device::{DeviceCommand, DeviceState};
use crate::protocol::{Command, Event};

pub async fn run(
    port: u16,
    cmd_tx: mpsc::Sender<DeviceCommand>,
    event_tx: broadcast::Sender<Event>,
    state_rx: watch::Receiver<DeviceState>,
) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await.expect("failed to bind WebSocket port");
    info!("WebSocket server listening on ws://localhost:{}", port);

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("Accept error: {}", e);
                continue;
            }
        };

        let ws = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                warn!("WebSocket upgrade failed from {}: {}", peer, e);
                continue;
            }
        };

        info!("Client connected: {}", peer);
        let cmd_tx = cmd_tx.clone();
        let event_rx = event_tx.subscribe();
        let state_rx = state_rx.clone();

        tokio::spawn(async move {
            handle_client(ws, peer, cmd_tx, event_rx, state_rx).await;
            info!("Client disconnected: {}", peer);
        });
    }
}

async fn handle_client(
    ws: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    peer: SocketAddr,
    cmd_tx: mpsc::Sender<DeviceCommand>,
    mut event_rx: broadcast::Receiver<Event>,
    state_rx: watch::Receiver<DeviceState>,
) {
    let (mut sink, mut stream) = ws.split();

    let initial_state = state_rx.borrow().clone();
    if let DeviceState::Connected(info) = initial_state {
        let event = Event::DeviceConnected {
            serial: info.serial.clone(),
            model: info.model.clone(),
            firmware: info.firmware.clone(),
            keys: info.keys,
            rows: info.rows,
            cols: info.cols,
            icon_size: info.icon_size,
        };
        if let Ok(json) = serde_json::to_string(&event) {
            let _ = sink.send(Message::Text(json)).await;
        }
    }

    let mut pending_cmd: Option<Command> = None;

    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<Command>(&text) {
                            Ok(cmd) => {
                                if expects_binary(&cmd) {
                                    pending_cmd = Some(cmd);
                                } else {
                                    let _ = cmd_tx.send(DeviceCommand {
                                        command: cmd,
                                        binary_data: None,
                                    }).await;
                                }
                            }
                            Err(e) => {
                                let err = Event::Error {
                                    message: format!("invalid command: {}", e),
                                };
                                if let Ok(json) = serde_json::to_string(&err) {
                                    let _ = sink.send(Message::Text(json)).await;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        if let Some(cmd) = pending_cmd.take() {
                            let _ = cmd_tx.send(DeviceCommand {
                                command: cmd,
                                binary_data: Some(data),
                            }).await;
                        } else {
                            let err = Event::Error {
                                message: "unexpected binary frame".into(),
                            };
                            if let Ok(json) = serde_json::to_string(&err) {
                                let _ = sink.send(Message::Text(json)).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        warn!("WebSocket error from {}: {}", peer, e);
                        break;
                    }
                }
            }
            event = event_rx.recv() => {
                match event {
                    Ok(event) => {
                        if let Ok(json) = serde_json::to_string(&event) {
                            if sink.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Client {} lagged, dropped {} events", peer, n);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

fn expects_binary(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::SetImage { .. } | Command::SetImageJpeg { .. } | Command::SetPanelImage { .. }
    )
}
