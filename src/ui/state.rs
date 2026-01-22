use crate::api::CrptResponse;
use crate::services::HonestSignCode;
use std::path::PathBuf;

/// Scanner connection status
#[derive(Debug, Clone, PartialEq)]
pub enum ScannerStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl Default for ScannerStatus {
    fn default() -> Self {
        Self::Connecting  // Start as "Connecting" since we auto-connect on startup
    }
}

/// Represents a validated and stored code
#[derive(Debug, Clone)]
pub struct ScannedCode {
    pub code: HonestSignCode,
    pub product_name: String,
    pub gtin: String,
    pub produced_date: Option<String>,
}

/// Result of the last scan operation
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct PdfRecord {
    pub path: PathBuf,
    pub filename: String,
    pub created_at: String,
    pub code_count: usize,
}

/// UI state that gets updated from background tasks
#[derive(Default)]
pub struct UiState {
    pub scanner_status: ScannerStatus,
    pub scanned_codes: Vec<ScannedCode>,
    pub last_scan_result: LastScanResult,
    pub pdf_history: Vec<PdfRecord>,
    pub current_error: Option<String>,
}

/// Messages sent from background tasks to update UI
#[derive(Debug, Clone)]
pub enum UiMessage {
    ScannerStatusChanged(ScannerStatus),
    ScanStarted,
    ScanSuccess {
        code: ScannedCode,
        response: CrptResponse,
    },
    ScanError {
        message: String,
        explanation: String,
    },
    PdfGenerated(PdfRecord),
    PdfHistoryLoaded(Vec<PdfRecord>),
    Error(String),
    ClearError,
}
