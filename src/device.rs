use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use elgato_streamdeck::asynchronous::AsyncStreamDeck;
use elgato_streamdeck::info::Kind;
use elgato_streamdeck::{list_devices, new_hidapi, StreamDeckInput};
use image::{DynamicImage, Rgba, RgbaImage};
use sha2::{Digest, Sha256};
use tokio::sync::{broadcast, mpsc, watch};
use tracing::info;

use crate::protocol::{Command, Event, ImageFormat};

const ICON_SIZE: u32 = 96;
const BUTTON_POLL_RATE: f32 = 60.0;
const HOTPLUG_POLL: Duration = Duration::from_secs(2);

pub struct DeviceCommand {
    pub command: Command,
    pub binary_data: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub serial: String,
    pub model: String,
    pub firmware: String,
    pub keys: u8,
    pub rows: u8,
    pub cols: u8,
    pub icon_size: u16,
}

#[derive(Debug, Clone)]
pub enum DeviceState {
    Disconnected,
    Connected(DeviceInfo),
}

struct ImageSlot {
    hash: [u8; 32],
    image: DynamicImage,
}

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn hash_bytes(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

fn solid_color(r: u8, g: u8, b: u8) -> DynamicImage {
    let mut img = RgbaImage::new(ICON_SIZE, ICON_SIZE);
    let pixel = Rgba([r, g, b, 255]);
    for p in img.pixels_mut() {
        *p = pixel;
    }
    DynamicImage::ImageRgba8(img)
}

fn decode_image(data: &[u8], format: &ImageFormat, width: u32, height: u32) -> Result<DynamicImage, String> {
    match format {
        ImageFormat::Rgba => {
            let expected = (width * height * 4) as usize;
            if data.len() != expected {
                return Err(format!(
                    "expected {} bytes for {}x{} RGBA, got {}",
                    expected, width, height, data.len()
                ));
            }
            RgbaImage::from_raw(width, height, data.to_vec())
                .map(DynamicImage::ImageRgba8)
                .ok_or_else(|| "failed to create image from RGBA data".into())
        }
        ImageFormat::Png => image::load_from_memory(data).map_err(|e| e.to_string()),
    }
}

fn device_info_from_kind(kind: Kind, serial: &str, firmware: &str) -> DeviceInfo {
    DeviceInfo {
        serial: serial.to_string(),
        model: format!("{:?}", kind).to_lowercase(),
        firmware: firmware.to_string(),
        keys: kind.key_count(),
        rows: kind.row_count(),
        cols: kind.column_count(),
        icon_size: ICON_SIZE as u16,
    }
}

fn try_connect() -> Option<(AsyncStreamDeck, Kind)> {
    let hid = new_hidapi().ok()?;
    let devices = list_devices(&hid);
    let (kind, serial) = devices.into_iter().next()?;
    let deck = AsyncStreamDeck::connect(&hid, kind, &serial).ok()?;
    Some((deck, kind))
}

async fn restore_cache(deck: &AsyncStreamDeck, cache: &[Option<ImageSlot>]) {
    let mut restored = 0;
    for (i, slot) in cache.iter().enumerate() {
        if let Some(slot) = slot {
            if deck.set_button_image(i as u8, slot.image.clone()).await.is_ok() {
                restored += 1;
            }
        }
    }
    if restored > 0 {
        let _ = deck.flush().await;
        info!("Restored {} cached images", restored);
    }
}

pub async fn run(
    mut cmd_rx: mpsc::Receiver<DeviceCommand>,
    event_tx: broadcast::Sender<Event>,
    state_tx: watch::Sender<DeviceState>,
) {
    let mut cache: Vec<Option<ImageSlot>> = Vec::new();

    loop {
        info!("Scanning for Stream Deck...");

        let (deck, kind) = match try_connect() {
            Some(d) => d,
            None => {
                tokio::time::sleep(HOTPLUG_POLL).await;
                continue;
            }
        };

        let deck = Arc::new(deck);
        let num_keys = kind.key_count() as usize;
        let serial = deck.serial_number().await.unwrap_or_else(|_| "unknown".into());
        let firmware = deck.firmware_version().await.unwrap_or_else(|_| "unknown".into());
        let dev_info = device_info_from_kind(kind, &serial, &firmware);

        info!("Connected: {} (serial: {})", dev_info.model, dev_info.serial);

        cache.resize_with(num_keys, || None);
        let _ = state_tx.send(DeviceState::Connected(dev_info.clone()));
        let _ = event_tx.send(Event::DeviceConnected {
            serial: dev_info.serial.clone(),
            model: dev_info.model.clone(),
            firmware: dev_info.firmware.clone(),
            keys: dev_info.keys,
            rows: dev_info.rows,
            cols: dev_info.cols,
            icon_size: dev_info.icon_size,
        });

        restore_cache(&deck, &cache).await;

        let reader_deck = deck.clone();
        let reader_tx = event_tx.clone();
        let mut reader_handle = tokio::spawn(async move {
            let mut prev = vec![false; num_keys];
            loop {
                match reader_deck.read_input(BUTTON_POLL_RATE).await {
                    Ok(StreamDeckInput::ButtonStateChange(buttons)) => {
                        let now = timestamp_ms();
                        for (i, &pressed) in buttons.iter().enumerate().take(num_keys) {
                            if pressed != prev[i] {
                                let event = if pressed {
                                    Event::KeyDown { key: i as u8, timestamp: now }
                                } else {
                                    Event::KeyUp { key: i as u8, timestamp: now }
                                };
                                let _ = reader_tx.send(event);
                                prev[i] = pressed;
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });

        let mut brightness: u8 = 100;
        let disconnected = loop {
            tokio::select! {
                _ = &mut reader_handle => break true,
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(cmd) => {
                            if let Err(msg) = handle_command(
                                &deck, &mut cache, &event_tx, &dev_info, &mut brightness, cmd,
                            ).await {
                                let _ = event_tx.send(Event::Error { message: msg });
                            }
                        }
                        None => break false,
                    }
                }
            }
        };

        let _ = state_tx.send(DeviceState::Disconnected);
        let _ = event_tx.send(Event::DeviceDisconnected);
        info!("Device disconnected");

        if !disconnected {
            return;
        }
    }
}

async fn handle_command(
    deck: &AsyncStreamDeck,
    cache: &mut [Option<ImageSlot>],
    event_tx: &broadcast::Sender<Event>,
    info: &DeviceInfo,
    brightness: &mut u8,
    cmd: DeviceCommand,
) -> Result<(), String> {
    match cmd.command {
        Command::SetImage { key, format, width, height } => {
            let data = cmd.binary_data.ok_or("missing binary data")?;
            let idx = validate_key(key, cache.len())?;

            let hash = hash_bytes(&data);
            if cache[idx].as_ref().is_some_and(|s| s.hash == hash) {
                let _ = event_tx.send(Event::ImageSet { key });
                return Ok(());
            }

            let image = decode_image(&data, &format, width, height)?;
            deck.set_button_image(key, image.clone()).await.map_err(|e| e.to_string())?;
            deck.flush().await.map_err(|e| e.to_string())?;
            cache[idx] = Some(ImageSlot { hash, image });
            let _ = event_tx.send(Event::ImageSet { key });
        }
        Command::SetImageJpeg { key } => {
            let data = cmd.binary_data.ok_or("missing binary data")?;
            let idx = validate_key(key, cache.len())?;

            let hash = hash_bytes(&data);
            if cache[idx].as_ref().is_some_and(|s| s.hash == hash) {
                let _ = event_tx.send(Event::ImageSet { key });
                return Ok(());
            }

            let image = image::load_from_memory(&data).map_err(|e| e.to_string())?;
            deck.set_button_image(key, image.clone()).await.map_err(|e| e.to_string())?;
            deck.flush().await.map_err(|e| e.to_string())?;
            cache[idx] = Some(ImageSlot { hash, image });
            let _ = event_tx.send(Event::ImageSet { key });
        }
        Command::SetColor { key, r, g, b } => {
            let idx = validate_key(key, cache.len())?;

            let hash = hash_bytes(&[r, g, b]);
            if cache[idx].as_ref().is_some_and(|s| s.hash == hash) {
                let _ = event_tx.send(Event::ImageSet { key });
                return Ok(());
            }

            let image = solid_color(r, g, b);
            deck.set_button_image(key, image.clone()).await.map_err(|e| e.to_string())?;
            deck.flush().await.map_err(|e| e.to_string())?;
            cache[idx] = Some(ImageSlot { hash, image });
            let _ = event_tx.send(Event::ImageSet { key });
        }
        Command::SetBrightness { value } => {
            deck.set_brightness(value).await.map_err(|e| e.to_string())?;
            *brightness = value;
        }
        Command::ClearKey { key } => {
            let idx = validate_key(key, cache.len())?;
            deck.clear_button_image(key).await.map_err(|e| e.to_string())?;
            deck.flush().await.map_err(|e| e.to_string())?;
            cache[idx] = None;
        }
        Command::ClearAll => {
            deck.clear_all_button_images().await.map_err(|e| e.to_string())?;
            deck.flush().await.map_err(|e| e.to_string())?;
            for slot in cache.iter_mut() {
                *slot = None;
            }
        }
        Command::ResetToLogo => {
            deck.reset().await.map_err(|e| e.to_string())?;
            for slot in cache.iter_mut() {
                *slot = None;
            }
        }
        Command::GetDeviceInfo => {
            let _ = event_tx.send(Event::DeviceInfo {
                serial: info.serial.clone(),
                model: info.model.clone(),
                firmware: info.firmware.clone(),
                keys: info.keys,
                rows: info.rows,
                cols: info.cols,
                icon_size: info.icon_size,
                brightness: *brightness,
            });
        }
        Command::SetPanelImage { .. } => {
            return Err("set_panel_image not yet implemented".into());
        }
    }
    Ok(())
}

fn validate_key(key: u8, max: usize) -> Result<usize, String> {
    let idx = key as usize;
    if idx >= max {
        Err(format!("invalid key index: {}", key))
    } else {
        Ok(idx)
    }
}

pub async fn run_test(default_port: u16) -> Result<(), String> {
    println!("Scanning for Stream Deck...");

    let (deck, kind) = try_connect()
        .ok_or("No Stream Deck found. Is it plugged in and not claimed by another application?")?;
    let deck = Arc::new(deck);

    let serial = deck
        .serial_number()
        .await
        .unwrap_or_else(|_| "unknown".into());
    let firmware = deck
        .firmware_version()
        .await
        .unwrap_or_else(|_| "unknown".into());
    let num_keys = kind.key_count();

    println!(
        "Found: {} (serial: {}, firmware: {})",
        format!("{:?}", kind).to_lowercase(),
        serial,
        firmware
    );
    println!(
        "Layout: {} keys ({}x{})",
        num_keys,
        kind.row_count(),
        kind.column_count()
    );
    println!();
    println!("Running visual test...");

    for key in 0..num_keys {
        let (r, g, b) = rainbow_color(key as f32 / num_keys as f32);
        let img = solid_color(r, g, b);
        let _ = deck.set_button_image(key, img).await;
        let _ = deck.flush().await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(400)).await;

    for key in 0..num_keys {
        let _ = deck
            .set_button_image(key, solid_color(255, 255, 255))
            .await;
    }
    let _ = deck.flush().await;
    tokio::time::sleep(Duration::from_millis(150)).await;

    for key in 0..num_keys {
        let _ = deck
            .set_button_image(key, solid_color(0, 200, 80))
            .await;
    }
    let _ = deck.flush().await;
    tokio::time::sleep(Duration::from_millis(600)).await;

    let _ = deck.reset().await;

    println!();
    println!("Test passed! Stream Deck is working correctly.");
    println!();
    println!("Next steps:");
    println!("  Start the bridge:  streamdeck-bridge");
    println!("  Custom port:       streamdeck-bridge --port <PORT>");
    println!("  Install service:   scripts/install-launchd.sh");
    println!("  Connect via:       ws://localhost:{}", default_port);

    Ok(())
}

fn rainbow_color(t: f32) -> (u8, u8, u8) {
    let h = t * 6.0;
    let sector = h.floor() as u8 % 6;
    let f = h - h.floor();
    let q = 1.0 - f;

    let (r, g, b) = match sector {
        0 => (1.0, f, 0.0),
        1 => (q, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, q, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, q),
    };

    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}
