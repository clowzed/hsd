mod api;
mod audio;
mod commands;
mod domain;
mod infrastructure;
pub mod pdf;
mod services;
mod ui;

use audio::{AudioPlayer, SoundType};
use commands::*;
use pdf::{printer, LabelData, PdfGenerator};
use services::{HonestSignValidator, ScannerManager, ValidationError};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::{mpsc, Mutex};
use ui::state::{AppMode, AppSettings, ScannedCode, ScannerStatus};

/// Event payload for a successful scan.
#[derive(Clone, serde::Serialize)]
struct ScanSuccessPayload {
    code: ScannedCode,
    response: api::CrptResponse,
}

/// Event payload for a scan error.
#[derive(Clone, serde::Serialize)]
struct ScanErrorPayload {
    message: String,
    explanation: String,
}

/// Event payload for instant print success.
#[derive(Clone, serde::Serialize)]
struct InstantPrintPayload {
    filename: String,
    printer: String,
}

fn error_to_message(e: &ValidationError) -> (String, String) {
    match e {
        ValidationError::TooShort { len, min } => (
            "Неполный код".to_string(),
            format!("Получено {} байт, минимум {}", len, min),
        ),
        ValidationError::InvalidStart => (
            "Неверный формат".to_string(),
            "Код должен начинаться с '01'".to_string(),
        ),
        ValidationError::InvalidGtin => (
            "Неверный GTIN".to_string(),
            "GTIN должен содержать только цифры".to_string(),
        ),
        ValidationError::GtinChecksumFailed => (
            "Ошибка контрольной суммы".to_string(),
            "GTIN содержит ошибку, отсканируйте заново".to_string(),
        ),
        ValidationError::MissingSerialMarker => (
            "Неверный формат".to_string(),
            "Отсутствует маркер серийного номера".to_string(),
        ),
        ValidationError::CodeNotFound => (
            "Код не найден".to_string(),
            "Код не зарегистрирован в системе".to_string(),
        ),
        ValidationError::InvalidStatus {
            status,
            explanation,
        } => (status.clone(), explanation.clone()),
        ValidationError::NetworkError(e) => (
            "Ошибка сети".to_string(),
            format!("Не удалось связаться с сервером: {}", e),
        ),
        ValidationError::ApiError(e) => ("Ошибка API".to_string(), e.clone()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Honest Sign Scanner (Tauri)");

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();

            // --- Audio: dedicated non-Send thread ---
            let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<SoundType>();
            std::thread::spawn(move || {
                let player = AudioPlayer::new();
                while let Some(sound) = audio_rx.blocking_recv() {
                    if let Some(ref p) = player {
                        p.play(sound);
                    }
                }
            });

            // --- Initialize settings with system default printer ---
            let printers = printer::list_printers();
            let default_printer = printers
                .iter()
                .find(|p| p.is_default)
                .map(|p| p.name.clone());

            let settings = Arc::new(Mutex::new(AppSettings {
                mode: AppMode::Buffered,
                selected_printer: default_printer,
            }));

            // --- Shared state ---
            let pdf_generator = Arc::new(PdfGenerator::default());
            let scanned_codes: Arc<Mutex<Vec<ScannedCode>>> =
                Arc::new(Mutex::new(Vec::new()));

            app.manage(AppPdfGenerator(pdf_generator.clone()));
            app.manage(AppScannedCodes(scanned_codes.clone()));
            app.manage(AppAudio(audio_tx.clone()));
            app.manage(AppSettingsState(settings.clone()));

            // --- Scanner + validation pipeline ---
            let (scan_tx, mut scan_rx) = mpsc::channel::<Vec<u8>>(100);
            let audio_tx_scan = audio_tx.clone();
            let handle_scanner = handle.clone();
            let scanned_codes_pipeline = scanned_codes.clone();
            let settings_pipeline = settings.clone();
            let pdf_gen_pipeline = pdf_generator.clone();

            // Spawn the scanner manager and status listener
            let handle_status = handle.clone();
            tauri::async_runtime::spawn(async move {
                let _ = handle_status.emit("scanner-status-changed", ScannerStatus::Connecting);

                let (manager, mut status_rx) = ScannerManager::new(scan_tx);
                let manager = Arc::new(manager);

                // Status listener task
                let handle_status_inner = handle_status.clone();
                tokio::spawn(async move {
                    while let Ok(status) = status_rx.recv().await {
                        let ui_status = match status {
                            services::ScannerStatus::Connected => ScannerStatus::Connected,
                            services::ScannerStatus::Connecting => ScannerStatus::Connecting,
                            services::ScannerStatus::Disconnected => ScannerStatus::Disconnected,
                            services::ScannerStatus::Error(e) => ScannerStatus::Error(e),
                        };
                        tracing::debug!("Scanner status update: {:?}", ui_status);
                        let _ = handle_status_inner.emit("scanner-status-changed", &ui_status);
                    }
                });

                // Start scanner (runs forever)
                tracing::info!("Starting scanner manager...");
                manager.start().await;
            });

            // Spawn the validation pipeline
            tauri::async_runtime::spawn(async move {
                let validator = HonestSignValidator::new();

                while let Some(raw_code) = scan_rx.recv().await {
                    tracing::info!("Processing scan: {} bytes", raw_code.len());

                    // Notify frontend that validation started
                    let _ = handle_scanner.emit("scan-started", ());

                    match validator.validate(&raw_code).await {
                        Ok(result) => {
                            let scanned_code = ScannedCode {
                                code: result.code.clone(),
                                product_name: result
                                    .response
                                    .product_name
                                    .clone()
                                    .unwrap_or_else(|| "Неизвестный товар".to_string()),
                                gtin: result.code.gtin.clone(),
                                produced_date: result.response.formatted_produced_date(),
                                expire_date: result.response.formatted_expire_date(),
                                vendor_code: result.response.vendor_code(),
                            };

                            let current_settings = settings_pipeline.lock().await.clone();

                            match current_settings.mode {
                                AppMode::Instant => {
                                    // Instant mode: generate single label + print immediately
                                    if let Some(printer_name) = &current_settings.selected_printer {
                                        let label = LabelData {
                                            raw_code: scanned_code.code.raw.clone(),
                                            vendor_code: scanned_code.vendor_code.clone(),
                                            expire_date: scanned_code.expire_date.clone(),
                                            index: None,
                                        };

                                        match pdf_gen_pipeline.generate(&[label]) {
                                            Ok(generated) => {
                                                let path = generated.path.to_string_lossy().to_string();
                                                match printer::print_pdf(&path, printer_name) {
                                                    Ok(()) => {
                                                        let filename = generated.path
                                                            .file_name()
                                                            .map(|f| f.to_string_lossy().to_string())
                                                            .unwrap_or_default();
                                                        let _ = handle_scanner.emit(
                                                            "instant-print-success",
                                                            InstantPrintPayload {
                                                                filename,
                                                                printer: printer_name.to_string(),
                                                            },
                                                        );
                                                    }
                                                    Err(e) => {
                                                        let _ = handle_scanner.emit(
                                                            "scan-error",
                                                            ScanErrorPayload {
                                                                message: "Ошибка печати".to_string(),
                                                                explanation: e,
                                                            },
                                                        );
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let _ = handle_scanner.emit(
                                                    "scan-error",
                                                    ScanErrorPayload {
                                                        message: "Ошибка генерации PDF".to_string(),
                                                        explanation: e.to_string(),
                                                    },
                                                );
                                            }
                                        }
                                    } else {
                                        let _ = handle_scanner.emit(
                                            "scan-error",
                                            ScanErrorPayload {
                                                message: "Принтер не выбран".to_string(),
                                                explanation: "Выберите принтер для мгновенной печати".to_string(),
                                            },
                                        );
                                    }

                                    // Still emit scan-success for UI display
                                    let _ = handle_scanner.emit(
                                        "scan-success",
                                        ScanSuccessPayload {
                                            code: scanned_code,
                                            response: result.response,
                                        },
                                    );
                                    let _ = audio_tx_scan.send(SoundType::Success);
                                }
                                AppMode::Buffered => {
                                    // Buffered mode: add to buffer + auto-regenerate PDF
                                    {
                                        let mut codes = scanned_codes_pipeline.lock().await;
                                        codes.push(scanned_code.clone());
                                    }

                                    let _ = handle_scanner.emit(
                                        "scan-success",
                                        ScanSuccessPayload {
                                            code: scanned_code,
                                            response: result.response,
                                        },
                                    );
                                    let _ = audio_tx_scan.send(SoundType::Success);

                                    // Auto-regenerate buffered PDF
                                    let codes = scanned_codes_pipeline.lock().await;
                                    regenerate_buffered_pdf(
                                        &codes,
                                        &pdf_gen_pipeline,
                                        &handle_scanner,
                                    )
                                    .await;
                                }
                            }
                        }
                        Err(e) => {
                            let (message, explanation) = error_to_message(&e);
                            let _ = handle_scanner.emit(
                                "scan-error",
                                ScanErrorPayload {
                                    message: message.clone(),
                                    explanation: explanation.clone(),
                                },
                            );
                            let _ = audio_tx_scan.send(SoundType::Error);
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            generate_pdf,
            clear_buffer,
            remove_code,
            list_pdfs,
            clear_pdf_history,
            open_pdf,
            list_printers,
            print_pdf,
            set_mode,
            set_printer,
            get_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
