use crate::domain::{BarcodeScanner, BarcodeScannerError};
use crate::infrastructure::ScanAccumulator;
use derive_getters::Getters;
use thiserror::Error;
use tokio::{
    io::AsyncReadExt,
    sync::{mpsc, Mutex},
};
use tokio_serial::{SerialPort as _, SerialPortBuilderExt, SerialPortInfo, SerialPortType, UsbPortInfo};

/// Errors that can occur when parsing scanner information from serial port data.
#[derive(Debug, Error)]
pub enum MertechScannerInformationParsingError {
    #[error("Unsupported connection type - only USB connections are supported")]
    UnsupportedConnection,
}

/// Contains identifying information for a Mertech scanner device.
#[derive(Debug, Getters, Clone, PartialEq)]
pub struct MertechScannerInformation {
    /// USB Vendor ID
    vid: u16,
    /// USB Product ID
    pid: u16,
    /// Device serial number
    serial_number: Option<String>,
    /// USB manufacturer string
    manufacturer: Option<String>,
    /// Product name string
    product: Option<String>,
}

impl TryFrom<SerialPortInfo> for MertechScannerInformation {
    type Error = MertechScannerInformationParsingError;

    fn try_from(port_info: SerialPortInfo) -> Result<Self, Self::Error> {
        match port_info.port_type {
            SerialPortType::UsbPort(usb_port_info) => Ok(Self {
                vid: usb_port_info.vid,
                pid: usb_port_info.pid,
                serial_number: usb_port_info.serial_number,
                manufacturer: usb_port_info.manufacturer,
                product: usb_port_info.product,
            }),
            _ => {
                tracing::warn!(
                    "Attempted to create MertechScannerInformation from unsupported port type: {:?}",
                    port_info.port_type
                );
                Err(MertechScannerInformationParsingError::UnsupportedConnection)
            }
        }
    }
}

/// Errors that can occur during Mertech scanner operations.
#[derive(Debug, Error)]
pub enum MertechScannerError {
    #[error("Serial port error: {0}")]
    Serial(#[from] tokio_serial::Error),

    #[error("Background listener task terminated - scanner may be disconnected")]
    ChannelClosed,

    #[error("Failed to set Data Terminal Ready (DTR) signal: {0}")]
    DataTerminalReady(#[source] tokio_serial::Error),

    #[error("Background listener task failure: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error("Failed to enumerate available serial ports")]
    PortEnumerationFailed,

    #[allow(dead_code)]
    #[error("Port enumeration timed out")]
    PortEnumerationTimeout,

    #[error("No compatible Mertech scanner found on available ports")]
    ScannerNotFound,

    #[error("Invalid scanner information: {0}")]
    InvalidScannerInformation(#[from] MertechScannerInformationParsingError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Port {port} is busy - another application may be using it")]
    PortBusy { port: String },

    #[error("Connection timed out")]
    ConnectionTimeout,
}

impl BarcodeScannerError for MertechScannerError {
    fn is_scanner_disconnected(&self) -> bool {
        matches!(
            self,
            MertechScannerError::Serial(_)
                | MertechScannerError::ChannelClosed
                | MertechScannerError::Io(_)
        )
    }
}

/// Supported connection types for Mertech scanners.
#[derive(Default, Debug, Clone, Copy)]
pub enum MertechScannerConnectionType {
    #[default]
    Usb,
}

/// Mertech barcode scanner implementation with CR+LF terminator detection.
///
/// # Architecture
/// Uses a dedicated background task to continuously read from the serial port.
/// Raw data is accumulated until CR+LF terminator is detected, then complete
/// codes are forwarded via an MPSC channel.
///
/// # Fragmentation Handling
/// The scanner sometimes sends incomplete data that only completes on the next
/// scan button press. This is handled by the internal `ScanAccumulator` which
/// buffers data until a complete code (terminated by CR+LF) is detected.
#[derive(Debug, Getters)]
pub struct MertechScanner {
    /// Scanner identification information
    #[allow(dead_code)]
    information: MertechScannerInformation,

    /// Receiver for complete barcode data
    #[getter(skip)]
    code_receiver: Mutex<mpsc::Receiver<Vec<u8>>>,

    /// Background task handle
    #[getter(skip)]
    _listener_task: tokio::task::JoinHandle<()>,
}

impl MertechScanner {
    /// Automatically discovers and connects to an available Mertech scanner.
    ///
    /// # Process Flow
    /// 1. Enumerates all available serial ports
    /// 2. Identifies Mertech scanners by manufacturer name
    /// 3. Establishes serial connection
    /// 4. Configures port settings (DTR signal)
    /// 5. Spawns background data listener task with CR+LF accumulator
    #[tracing::instrument]
    pub async fn auto_connect(
        _connection_type: MertechScannerConnectionType,
    ) -> Result<Self, MertechScannerError> {
        let serial_port_info = discover_scanner_port().await?;
        let port_name = serial_port_info.port_name.clone();

        tracing::info!("Opening serial port: {}", port_name);

        // Open serial port with timeout
        let port_name_clone = port_name.clone();
        let open_result = tokio::time::timeout(
            std::time::Duration::from_secs(PORT_OPEN_TIMEOUT_SECS),
            tokio::task::spawn_blocking(move || {
                tracing::debug!("spawn_blocking: opening port {}", port_name_clone);
                let result = tokio_serial::new(port_name_clone, MERTECH_DEFAULT_BAUD_RATE).open_native_async();
                tracing::debug!("spawn_blocking: port open returned");
                result
            })
        ).await;

        let mut port = match open_result {
            Ok(Ok(Ok(p))) => p,
            Ok(Ok(Err(e))) => {
                let error_str = e.to_string().to_lowercase();
                tracing::error!("Failed to open port {}: {}", port_name, e);

                // Check for common "port busy" errors
                if error_str.contains("busy")
                    || error_str.contains("in use")
                    || error_str.contains("resource busy")
                    || error_str.contains("permission denied")
                    || error_str.contains("access denied")
                    || error_str.contains("exclusive")
                {
                    return Err(MertechScannerError::PortBusy { port: port_name });
                }
                return Err(MertechScannerError::Serial(e));
            }
            Ok(Err(e)) => {
                tracing::error!("spawn_blocking join error: {}", e);
                return Err(MertechScannerError::TaskJoin(e));
            }
            Err(_) => {
                tracing::error!("Port open timed out after {} seconds - port may be busy", PORT_OPEN_TIMEOUT_SECS);
                return Err(MertechScannerError::ConnectionTimeout);
            }
        };

        tracing::info!("Serial port opened successfully");

        port.write_data_terminal_ready(true)
            .map_err(MertechScannerError::DataTerminalReady)?;

        let scanner_information = MertechScannerInformation::try_from(serial_port_info)?;

        // Channel for complete codes (after CR+LF detection)
        let (code_sender, code_receiver) = mpsc::channel(SCANNER_CHANNEL_BUFFER_SIZE);

        // Spawn listener task with accumulator
        let _listener_task = tokio::task::spawn(async move {
            let mut buffer = [0u8; SERIAL_READ_BUFFER_SIZE];
            let mut accumulator = ScanAccumulator::new();

            loop {
                match port.read(&mut buffer).await {
                    Ok(bytes_read) if bytes_read > 0 => {
                        let chunk = &buffer[..bytes_read];

                        tracing::trace!(
                            "Received {} bytes from scanner",
                            bytes_read
                        );

                        // Process chunk through accumulator
                        let complete_codes = accumulator.process_chunk(chunk);

                        // Send all complete codes
                        for code in complete_codes {
                            if code_sender.send(code).await.is_err() {
                                tracing::warn!("Code receiver dropped, stopping listener");
                                return;
                            }
                        }
                    }
                    Ok(_) => {
                        // Zero bytes read, continue
                        continue;
                    }
                    Err(e) => {
                        tracing::error!("Serial port read error: {}", e);
                        break;
                    }
                }
            }

            tracing::info!("Scanner listener task terminated");
        });

        tracing::info!(
            "Connected to Mertech scanner: {:?}",
            scanner_information.product()
        );

        Ok(Self {
            information: scanner_information,
            code_receiver: Mutex::new(code_receiver),
            _listener_task,
        })
    }
}

impl BarcodeScanner for MertechScanner {
    type Error = MertechScannerError;

    /// Retrieves the next complete barcode scan from the scanner.
    ///
    /// This method waits for a complete code (one that was terminated by CR+LF).
    /// Fragmented codes are automatically assembled by the internal accumulator.
    async fn scan(&self) -> Result<Vec<u8>, Self::Error> {
        let code = self
            .code_receiver
            .lock()
            .await
            .recv()
            .await
            .ok_or(MertechScannerError::ChannelClosed)?;

        tracing::debug!("Received complete scan: {} bytes", code.len());

        Ok(code)
    }
}

/// Discovers Mertech scanners by enumerating available serial ports.
///
/// Uses spawn_blocking to avoid blocking the async runtime on Apple Silicon
/// where port enumeration can be slow. Includes timeout to prevent indefinite hangs.
async fn discover_scanner_port() -> Result<SerialPortInfo, MertechScannerError> {
    tracing::info!("Starting port enumeration...");

    // Run port enumeration in a blocking thread with timeout
    let enumeration_result = tokio::time::timeout(
        std::time::Duration::from_secs(PORT_ENUMERATION_TIMEOUT_SECS),
        tokio::task::spawn_blocking(|| {
            tracing::debug!("spawn_blocking: calling available_ports()");
            let result = tokio_serial::available_ports();
            tracing::debug!("spawn_blocking: available_ports() returned");
            result
        })
    ).await;

    let available_ports = match enumeration_result {
        Ok(Ok(Ok(ports))) => ports,
        Ok(Ok(Err(e))) => {
            tracing::error!("Port enumeration error: {}", e);
            return Err(MertechScannerError::PortEnumerationFailed);
        }
        Ok(Err(e)) => {
            tracing::error!("spawn_blocking join error: {}", e);
            return Err(MertechScannerError::PortEnumerationFailed);
        }
        Err(_) => {
            tracing::error!("Port enumeration timed out after {} seconds", PORT_ENUMERATION_TIMEOUT_SECS);
            return Err(MertechScannerError::PortEnumerationFailed);
        }
    };

    tracing::info!("Found {} serial ports", available_ports.len());

    for port in &available_ports {
        tracing::debug!("Port: {} - {:?}", port.port_name, port.port_type);
    }

    // First try to find by known Mertech VID/PID (faster)
    let mertech_port = available_ports
        .iter()
        .find(|port| match &port.port_type {
            SerialPortType::UsbPort(UsbPortInfo { vid, pid, .. }) => {
                let found = MERTECH_VID_PID_PAIRS.contains(&(*vid, *pid));
                if found {
                    tracing::debug!("VID/PID match: {:04X}:{:04X}", vid, pid);
                }
                found
            }
            _ => false,
        })
        .cloned();

    if let Some(port) = mertech_port {
        tracing::info!("Found Mertech scanner by VID/PID at {}", port.port_name);
        return Ok(port);
    }

    // Fallback: try to find by manufacturer name
    let mertech_port = available_ports
        .into_iter()
        .find(|port| match &port.port_type {
            SerialPortType::UsbPort(UsbPortInfo { manufacturer, .. }) => {
                manufacturer.as_ref().map(|m| {
                    let m_lower = m.to_lowercase();
                    let found = m_lower.contains("mertech") || m_lower.contains("superlead");
                    if found {
                        tracing::debug!("Manufacturer match: {}", m);
                    }
                    found
                }).unwrap_or(false)
            }
            _ => false,
        })
        .ok_or(MertechScannerError::ScannerNotFound)?;

    tracing::info!("Found Mertech scanner by manufacturer at {}", mertech_port.port_name);
    Ok(mertech_port)
}

const PORT_ENUMERATION_TIMEOUT_SECS: u64 = 10;
const PORT_OPEN_TIMEOUT_SECS: u64 = 5;

const SERIAL_READ_BUFFER_SIZE: usize = 2048;
const SCANNER_CHANNEL_BUFFER_SIZE: usize = 100;
const MERTECH_DEFAULT_BAUD_RATE: u32 = 9600;

/// Known Mertech scanner USB Vendor ID / Product ID pairs
/// Add more pairs here as needed for different scanner models
const MERTECH_VID_PID_PAIRS: &[(u16, u16)] = &[
    (0x0483, 0x5750), // Mertech N200 / N300
    (0x0483, 0x5740), // Mertech N100
    (0x1FC9, 0x2016), // Mertech Superlead
    (0x1FC9, 0x2017), // Mertech Superlead variant
    // Silicon Labs CP210x (common USB-UART chip used in scanners)
    (0x10C4, 0xEA60),
    // FTDI FT232 (another common USB-UART chip)
    (0x0403, 0x6001),
];
