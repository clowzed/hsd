use crate::audio::{AudioPlayer, SoundType};
use crate::pdf::PdfGenerator;
use crate::services::{HonestSignValidator, ScannerManager, ValidationError};
use crate::ui::state::*;
use eframe::egui::{self, Color32, RichText, Stroke};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct App {
    /// Tokio runtime for async operations
    runtime: tokio::runtime::Runtime,

    /// UI state
    state: UiState,

    /// Channel receiver for UI updates from background tasks
    ui_rx: mpsc::UnboundedReceiver<UiMessage>,

    /// Channel sender for UI updates (cloned to background tasks)
    ui_tx: mpsc::UnboundedSender<UiMessage>,

    /// Channel to send scan data from scanner to validator
    scan_tx: mpsc::Sender<Vec<u8>>,

    /// PDF generator
    pdf_generator: Arc<PdfGenerator>,

    /// Audio player (kept in Option because it's not Send)
    audio_player: Option<AudioPlayer>,

    /// Flag to track if scanner manager was started
    scanner_started: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Runtime) -> Self {
        let (ui_tx, ui_rx) = mpsc::unbounded_channel();
        let (scan_tx, _scan_rx) = mpsc::channel(100);

        let pdf_generator = Arc::new(PdfGenerator::default());

        // Load initial PDF history
        let initial_history = load_pdf_history(&pdf_generator);

        let mut state = UiState::default();
        state.pdf_history = initial_history;

        Self {
            runtime,
            state,
            ui_rx,
            ui_tx,
            scan_tx,
            pdf_generator,
            audio_player: AudioPlayer::new(),
            scanner_started: false,
        }
    }

    fn start_scanner_manager(&mut self) {
        if self.scanner_started {
            return;
        }
        self.scanner_started = true;

        let ui_tx = self.ui_tx.clone();
        let (scan_tx, mut scan_rx) = mpsc::channel::<Vec<u8>>(100);
        self.scan_tx = scan_tx.clone();

        // Spawn scanner manager
        let ui_tx_scanner = ui_tx.clone();
        self.runtime.spawn(async move {
            // Send immediate "Connecting" status so UI updates right away
            let _ = ui_tx_scanner.send(UiMessage::ScannerStatusChanged(ScannerStatus::Connecting));

            let (manager, mut status_rx) = ScannerManager::new(scan_tx);
            let manager = Arc::new(manager);

            // Spawn status listener
            let ui_tx_status = ui_tx_scanner.clone();
            tokio::spawn(async move {
                while let Ok(status) = status_rx.recv().await {
                    let ui_status = match status {
                        crate::services::ScannerStatus::Connected => ScannerStatus::Connected,
                        crate::services::ScannerStatus::Connecting => ScannerStatus::Connecting,
                        crate::services::ScannerStatus::Disconnected => ScannerStatus::Disconnected,
                        crate::services::ScannerStatus::Error(e) => ScannerStatus::Error(e),
                    };
                    tracing::debug!("Scanner status update: {:?}", ui_status);
                    let _ = ui_tx_status.send(UiMessage::ScannerStatusChanged(ui_status));
                }
            });

            // Start scanner (this runs forever)
            tracing::info!("Starting scanner manager...");
            manager.start().await;
        });

        // Spawn scan processor
        let ui_tx_scan = ui_tx.clone();
        self.runtime.spawn(async move {
            let validator = HonestSignValidator::new();

            while let Some(raw_code) = scan_rx.recv().await {
                tracing::info!("Processing scan: {} bytes", raw_code.len());

                // Notify UI that validation started
                let _ = ui_tx_scan.send(UiMessage::ScanStarted);

                // Validate the code
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
                        };

                        let _ = ui_tx_scan.send(UiMessage::ScanSuccess {
                            code: scanned_code,
                            response: result.response,
                        });
                    }
                    Err(e) => {
                        let (message, explanation) = error_to_message(&e);
                        let _ = ui_tx_scan.send(UiMessage::ScanError {
                            message,
                            explanation,
                        });
                    }
                }
            }
        });
    }

    fn process_messages(&mut self) {
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UiMessage::ScannerStatusChanged(status) => {
                    tracing::info!("Scanner status: {:?}", status);
                    self.state.scanner_status = status;
                }
                UiMessage::ScanStarted => {
                    self.state.last_scan_result = LastScanResult::Validating;
                    self.state.current_error = None;
                }
                UiMessage::ScanSuccess { code, response } => {
                    tracing::info!("Scan success: {}", code.product_name);
                    self.state.scanned_codes.push(code.clone());
                    self.state.last_scan_result = LastScanResult::Success { code, response };
                    self.state.current_error = None;
                    self.play_sound(SoundType::Success);
                }
                UiMessage::ScanError { message, explanation } => {
                    tracing::warn!("Scan error: {} - {}", message, explanation);
                    self.state.last_scan_result = LastScanResult::Error {
                        message: message.clone(),
                        explanation: explanation.clone(),
                    };
                    self.state.current_error = Some(format!("{}: {}", message, explanation));
                    self.play_sound(SoundType::Error);
                }
                UiMessage::PdfGenerated(record) => {
                    self.state.pdf_history.insert(0, record);
                    self.state.scanned_codes.clear();
                    self.state.last_scan_result = LastScanResult::None;
                    self.play_sound(SoundType::Success);
                }
                UiMessage::PdfHistoryLoaded(history) => {
                    self.state.pdf_history = history;
                }
                UiMessage::Error(e) => {
                    self.state.current_error = Some(e);
                    self.play_sound(SoundType::Error);
                }
                UiMessage::ClearError => {
                    self.state.current_error = None;
                }
            }
        }
    }

    fn play_sound(&self, sound_type: SoundType) {
        if let Some(ref player) = self.audio_player {
            player.play(sound_type);
        }
    }

    fn generate_pdf(&mut self) {
        if self.state.scanned_codes.is_empty() {
            self.state.current_error = Some("Нет кодов для генерации PDF".to_string());
            return;
        }

        let codes: Vec<Vec<u8>> = self
            .state
            .scanned_codes
            .iter()
            .map(|sc| sc.code.raw.clone())
            .collect();

        match self.pdf_generator.generate(&codes) {
            Ok(generated) => {
                tracing::info!(
                    "PDF generated: {} with {} codes",
                    generated.path.display(),
                    generated.code_count
                );

                let record = PdfRecord {
                    path: generated.path.clone(),
                    filename: generated
                        .path
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    created_at: chrono::Local::now().format("%d.%m.%Y %H:%M").to_string(),
                    code_count: generated.code_count,
                };

                self.state.pdf_history.insert(0, record);
                self.state.scanned_codes.clear();
                self.state.last_scan_result = LastScanResult::None;
                self.state.current_error = None;
                self.play_sound(SoundType::Success);
            }
            Err(e) => {
                let error_msg = format!("Ошибка генерации PDF: {}", e);
                self.state.current_error = Some(error_msg);
                self.play_sound(SoundType::Error);
            }
        }
    }

    fn clear_buffer(&mut self) {
        self.state.scanned_codes.clear();
        self.state.last_scan_result = LastScanResult::None;
        self.state.current_error = None;
    }

    fn open_pdf(&self, path: &PathBuf) {
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(path).spawn();
        }

        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", &path.to_string_lossy()])
                .spawn();
        }

        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
    }

    fn clear_pdf_history(&mut self) {
        if let Err(e) = self.pdf_generator.clear_all() {
            self.state.current_error = Some(format!("Ошибка удаления PDF: {}", e));
        } else {
            self.state.pdf_history.clear();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Start scanner manager on first frame
        self.start_scanner_manager();

        // Process messages from background tasks
        self.process_messages();

        // Request repaint to keep UI responsive
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // Top panel - Header
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // Scanner status
                ui.horizontal(|ui| {
                    let (color, text) = match &self.state.scanner_status {
                        ScannerStatus::Connected => (Color32::GREEN, "● Подключен"),
                        ScannerStatus::Connecting => (Color32::YELLOW, "◐ Подключение..."),
                        ScannerStatus::Disconnected => (Color32::RED, "○ Отключен"),
                        ScannerStatus::Error(e) => (Color32::RED, "✗ Ошибка"),
                    };

                    ui.label(RichText::new("Сканер:").strong());
                    ui.label(RichText::new(text).color(color));

                    if let ScannerStatus::Error(e) = &self.state.scanner_status {
                        ui.label(RichText::new(format!("({})", e)).small().color(Color32::GRAY));
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Generate PDF button
                    let can_generate = !self.state.scanned_codes.is_empty();
                    if ui
                        .add_enabled(can_generate, egui::Button::new("📄 Создать PDF"))
                        .clicked()
                    {
                        self.generate_pdf();
                    }

                    // Clear buffer button
                    if ui
                        .add_enabled(
                            can_generate,
                            egui::Button::new(format!("🗑 Очистить ({})", self.state.scanned_codes.len())),
                        )
                        .clicked()
                    {
                        self.clear_buffer();
                    }
                });
            });
            ui.add_space(8.0);
        });

        // Right panel - PDF History
        egui::SidePanel::right("pdf_history")
            .min_width(220.0)
            .max_width(280.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("История PDF");
                ui.separator();

                if self.state.pdf_history.is_empty() {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Нет созданных PDF").color(Color32::GRAY));
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let mut pdf_to_open: Option<PathBuf> = None;

                        for record in &self.state.pdf_history {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(&record.filename).strong().small());
                                        ui.label(
                                            RichText::new(&record.created_at)
                                                .small()
                                                .color(Color32::GRAY),
                                        );
                                    });

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("↗").on_hover_text("Открыть").clicked() {
                                                pdf_to_open = Some(record.path.clone());
                                            }
                                        },
                                    );
                                });
                            });
                            ui.add_space(4.0);
                        }

                        if let Some(path) = pdf_to_open {
                            self.open_pdf(&path);
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    if ui.button("🗑 Очистить историю").clicked() {
                        self.clear_pdf_history();
                    }
                }
            });

        // Central panel - Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);

            // Error display
            if let Some(error) = &self.state.current_error.clone() {
                let mut should_clear = false;
                egui::Frame::none()
                    .fill(Color32::from_rgb(255, 200, 200))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(200, 50, 50)))
                    .inner_margin(12.0)
                    .outer_margin(4.0)
                    .rounding(6.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⚠").color(Color32::from_rgb(200, 50, 50)));
                            ui.label(error);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("✕").clicked() {
                                    should_clear = true;
                                }
                            });
                        });
                    });
                if should_clear {
                    self.state.current_error = None;
                }
                ui.add_space(8.0);
            }

            // Last scan result
            ui.heading("Последний скан");
            ui.add_space(8.0);

            match &self.state.last_scan_result {
                LastScanResult::None => {
                    egui::Frame::none()
                        .fill(Color32::from_gray(240))
                        .inner_margin(20.0)
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    RichText::new("Отсканируйте код товара")
                                        .color(Color32::GRAY)
                                        .size(16.0),
                                );
                            });
                        });
                }
                LastScanResult::Validating => {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(255, 250, 220))
                        .inner_margin(20.0)
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(RichText::new("Проверка кода...").size(16.0));
                                });
                            });
                        });
                }
                LastScanResult::Success { code, response } => {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(220, 255, 220))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(50, 180, 50)))
                        .inner_margin(16.0)
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("✓ В ОБОРОТЕ")
                                        .color(Color32::from_rgb(50, 150, 50))
                                        .strong(),
                                );
                            });
                            ui.add_space(8.0);
                            ui.label(RichText::new(&code.product_name).strong().size(15.0));
                            ui.add_space(8.0);

                            egui::Grid::new("scan_details").show(ui, |ui| {
                                ui.label(RichText::new("GTIN:").color(Color32::GRAY));
                                ui.label(&code.gtin);
                                ui.end_row();

                                if let Some(date) = &code.produced_date {
                                    ui.label(RichText::new("Произведено:").color(Color32::GRAY));
                                    ui.label(date);
                                    ui.end_row();
                                }

                                if let Some(expire) = response.formatted_expire_date() {
                                    ui.label(RichText::new("Годен до:").color(Color32::GRAY));
                                    ui.label(expire);
                                    ui.end_row();
                                }
                            });
                        });
                }
                LastScanResult::Error { message, explanation } => {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(255, 220, 220))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(200, 50, 50)))
                        .inner_margin(16.0)
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("✗ {}", message))
                                    .color(Color32::from_rgb(200, 50, 50))
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            ui.label(RichText::new(explanation).color(Color32::GRAY));
                        });
                }
            }

            ui.add_space(16.0);

            // Code buffer
            ui.heading(format!("Буфер кодов ({})", self.state.scanned_codes.len()));
            ui.add_space(8.0);

            if self.state.scanned_codes.is_empty() {
                egui::Frame::none()
                    .fill(Color32::from_gray(240))
                    .inner_margin(20.0)
                    .rounding(8.0)
                    .show(ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new("Отсканированные коды появятся здесь")
                                    .color(Color32::GRAY),
                            );
                        });
                    });
            } else {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for (i, code) in self.state.scanned_codes.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{}.", i + 1)).color(Color32::GRAY),
                                );
                                ui.label(RichText::new(&code.gtin).monospace().strong());
                                ui.label(
                                    RichText::new(truncate(&code.product_name, 40))
                                        .color(Color32::GRAY),
                                );
                            });
                            ui.separator();
                        }
                    });
            }
        });
    }
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
        ValidationError::InvalidStatus { status, explanation } => (status.clone(), explanation.clone()),
        ValidationError::NetworkError(e) => (
            "Ошибка сети".to_string(),
            format!("Не удалось связаться с сервером: {}", e),
        ),
        ValidationError::ApiError(e) => ("Ошибка API".to_string(), e.clone()),
    }
}

fn load_pdf_history(generator: &PdfGenerator) -> Vec<PdfRecord> {
    match generator.list_pdfs() {
        Ok(pdfs) => pdfs
            .into_iter()
            .filter_map(|path| {
                let filename = path.file_name()?.to_string_lossy().to_string();
                let metadata = std::fs::metadata(&path).ok()?;
                let modified = metadata.modified().ok()?;
                let datetime: chrono::DateTime<chrono::Local> = modified.into();

                Some(PdfRecord {
                    path,
                    filename,
                    created_at: datetime.format("%d.%m.%Y %H:%M").to_string(),
                    code_count: 0,
                })
            })
            .collect(),
        Err(e) => {
            tracing::warn!("Failed to list PDFs: {}", e);
            Vec::new()
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}
