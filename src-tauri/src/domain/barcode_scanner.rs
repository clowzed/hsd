use std::future::Future;

/// Trait for barcode scanner error types.
/// Allows checking if an error indicates scanner disconnection.
pub trait BarcodeScannerError: std::error::Error + Send + Sync {
    /// Returns true if this error indicates the scanner has been disconnected.
    fn is_scanner_disconnected(&self) -> bool;
}

/// Trait defining the interface for barcode scanners.
///
/// This abstraction allows for different scanner implementations
/// while maintaining a consistent API for the application.
pub trait BarcodeScanner: Send + Sync {
    type Error: BarcodeScannerError;

    /// Retrieves the next available barcode scan from the scanner.
    ///
    /// This method should return complete, validated barcode data.
    /// It may block until a complete scan is available.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: Complete barcode data as bytes
    /// - `Err(Self::Error)`: Scanner error (disconnection, timeout, etc.)
    fn scan(&self) -> impl Future<Output = Result<Vec<u8>, Self::Error>> + Send;
}
