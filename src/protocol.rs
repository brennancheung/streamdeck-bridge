use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    SetImage {
        key: u8,
        format: ImageFormat,
        width: u32,
        height: u32,
    },
    SetImageJpeg {
        key: u8,
    },
    SetPanelImage {
        format: ImageFormat,
        width: u32,
        height: u32,
    },
    SetColor {
        key: u8,
        r: u8,
        g: u8,
        b: u8,
    },
    SetBrightness {
        value: u8,
    },
    ClearKey {
        key: u8,
    },
    ClearAll,
    ResetToLogo,
    GetDeviceInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    Rgba,
    Png,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    KeyDown {
        key: u8,
        timestamp: u64,
    },
    KeyUp {
        key: u8,
        timestamp: u64,
    },
    DeviceConnected {
        serial: String,
        model: String,
        firmware: String,
        keys: u8,
        rows: u8,
        cols: u8,
        icon_size: u16,
    },
    DeviceDisconnected,
    DeviceInfo {
        serial: String,
        model: String,
        firmware: String,
        keys: u8,
        rows: u8,
        cols: u8,
        icon_size: u16,
        brightness: u8,
    },
    Error {
        message: String,
    },
    ImageSet {
        key: u8,
    },
}
