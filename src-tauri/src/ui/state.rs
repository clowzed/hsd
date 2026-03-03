use crate::api::CrptResponse;
use crate::services::HonestSignCode;
use serde::{Deserialize, Serialize};

/// Scanner connection status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
pub enum ScannerStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl Default for ScannerStatus {
    fn default() -> Self {
        Self::Connecting
    }
}

/// Represents a validated and stored code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedCode {
    pub code: HonestSignCode,
    pub product_name: String,
    pub gtin: String,
    pub produced_date: Option<String>,
    pub expire_date: Option<String>,
    pub vendor_code: Option<String>,
    pub barcode_exists: bool,
}

/// Result of the last scan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code, clippy::large_enum_variant)]
pub enum LastScanResult {
    Success {
        code: ScannedCode,
        response: CrptResponse,
    },
    Error {
        message: String,
        explanation: String,
    },
    Validating,
    None,
}

impl Default for LastScanResult {
    fn default() -> Self {
        Self::None
    }
}

/// Record of a generated PDF file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfRecord {
    pub path: String,
    pub filename: String,
    pub created_at: String,
    pub code_count: usize,
}

/// App operating mode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppMode {
    Buffered,
    Instant,
}

impl Default for AppMode {
    fn default() -> Self {
        Self::Buffered
    }
}

/// Information about an available printer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
}

/// A barcode printing preset (e.g., Ozon, WB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodePreset {
    pub name: String,
    pub directory: String,
    pub default_copies: u32,
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub mode: AppMode,
    pub selected_printer: Option<String>,
    pub barcode_enabled: bool,
    pub barcode_copies: u32,
    pub barcode_active_preset: Option<String>,
    pub barcode_presets: Vec<BarcodePreset>,
    pub duplicate_detection_buffered: bool,
    pub duplicate_detection_instant: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mode: AppMode::Buffered,
            selected_printer: None,
            barcode_enabled: false,
            barcode_copies: 1,
            barcode_active_preset: None,
            duplicate_detection_buffered: true,
            duplicate_detection_instant: false,
            barcode_presets: vec![
                BarcodePreset {
                    name: "Ozon".to_string(),
                    directory: String::new(),
                    default_copies: 2,
                },
                BarcodePreset {
                    name: "WB".to_string(),
                    directory: String::new(),
                    default_copies: 1,
                },
            ],
        }
    }
}
