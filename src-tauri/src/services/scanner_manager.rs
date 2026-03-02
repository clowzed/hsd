use crate::domain::{BarcodeScanner, BarcodeScannerError};
use crate::infrastructure::{MertechScanner, MertechScannerConnectionType, MertechScannerError};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};

/// Scanner connection status.
#[derive(Debug, Clone, PartialEq)]
pub enum ScannerStatus {
    /// Not connected, attempting to find scanner
    Disconnected,
    /// Actively trying to connect
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection error occurred
    Error(String),
}

impl Default for ScannerStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

/// Manages scanner connection with automatic reconnection.
///
/// The `ScannerManager` wraps the `MertechScanner` and provides:
/// - Automatic connection on startup
/// - Automatic reconnection on disconnect
/// - Status broadcasting for UI updates
/// - Complete scan data forwarding
pub struct ScannerManager {
    /// Current scanner instance (None when disconnected)
    scanner: Arc<RwLock<Option<MertechScanner>>>,

    /// Broadcast sender for status updates
    status_tx: broadcast::Sender<ScannerStatus>,

    /// Sender for complete scan data
    scan_tx: mpsc::Sender<Vec<u8>>,
}

impl ScannerManager {
    /// Creates a new scanner manager.
    ///
    /// # Arguments
    /// * `scan_tx` - Channel for forwarding complete scan data
    ///
    /// # Returns
    /// Tuple of (ScannerManager, status receiver)
    pub fn new(scan_tx: mpsc::Sender<Vec<u8>>) -> (Self, broadcast::Receiver<ScannerStatus>) {
        let (status_tx, status_rx) = broadcast::channel(16);

        let manager = Self {
            scanner: Arc::new(RwLock::new(None)),
            status_tx,
            scan_tx,
        };

        (manager, status_rx)
    }

    /// Subscribes to status updates.
    #[allow(dead_code)]
    pub fn subscribe_status(&self) -> broadcast::Receiver<ScannerStatus> {
        self.status_tx.subscribe()
    }

    /// Returns the current status.
    #[allow(dead_code)]
    pub async fn current_status(&self) -> ScannerStatus {
        if self.scanner.read().await.is_some() {
            ScannerStatus::Connected
        } else {
            ScannerStatus::Disconnected
        }
    }

    /// Starts the scanner manager with automatic reconnection.
    ///
    /// This method runs indefinitely, continuously trying to maintain
    /// a connection to the scanner.
    pub async fn start(self: Arc<Self>) {
        tracing::info!("Scanner manager starting...");

        loop {
            // Update status: Connecting
            let _ = self.status_tx.send(ScannerStatus::Connecting);
            tracing::info!("Attempting to connect to scanner...");

            match MertechScanner::auto_connect(MertechScannerConnectionType::Usb).await {
                Ok(scanner) => {
                    // Store scanner instance
                    *self.scanner.write().await = Some(scanner);

                    // Update status: Connected
                    let _ = self.status_tx.send(ScannerStatus::Connected);
                    tracing::info!("Scanner connected successfully");

                    // Run scan loop until disconnect
                    self.scan_loop().await;

                    // Scanner disconnected
                    *self.scanner.write().await = None;
                    let _ = self.status_tx.send(ScannerStatus::Disconnected);
                    tracing::warn!("Scanner disconnected");
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    tracing::warn!("Failed to connect to scanner: {}", error_msg);

                    // Update status: Error with user-friendly Russian messages
                    let status_msg = match &e {
                        MertechScannerError::ScannerNotFound => "Сканер не найден".to_string(),
                        MertechScannerError::PortBusy { port } => {
                            format!("Порт {} занят", port)
                        }
                        MertechScannerError::ConnectionTimeout => {
                            "Таймаут подключения".to_string()
                        }
                        MertechScannerError::PortEnumerationFailed => {
                            "Ошибка поиска портов".to_string()
                        }
                        MertechScannerError::PortEnumerationTimeout => {
                            "Таймаут поиска портов".to_string()
                        }
                        _ => error_msg,
                    };
                    let _ = self.status_tx.send(ScannerStatus::Error(status_msg));
                }
            }

            // Wait before retry
            tracing::debug!("Waiting {} seconds before reconnection attempt", RECONNECT_DELAY_SECS);
            tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
        }
    }

    /// Internal scan loop - reads from scanner until disconnection.
    async fn scan_loop(&self) {
        loop {
            let scan_result = {
                let scanner_guard = self.scanner.read().await;
                if let Some(scanner) = scanner_guard.as_ref() {
                    Some(scanner.scan().await)
                } else {
                    None
                }
            };

            match scan_result {
                Some(Ok(code)) => {
                    tracing::info!("Received scan: {} bytes", code.len());

                    // Forward to application
                    if self.scan_tx.send(code).await.is_err() {
                        tracing::warn!("Scan receiver dropped, stopping scan loop");
                        break;
                    }
                }
                Some(Err(e)) => {
                    if e.is_scanner_disconnected() {
                        tracing::warn!("Scanner disconnected: {}", e);
                        break;
                    } else {
                        tracing::error!("Scanner error: {}", e);
                        // Continue trying for non-disconnect errors
                    }
                }
                None => {
                    // Scanner not available
                    break;
                }
            }
        }
    }
}

const RECONNECT_DELAY_SECS: u64 = 2;
