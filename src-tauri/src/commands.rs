use crate::audio::SoundType;
use crate::pdf::{printer, LabelData, PdfGenerator};
use crate::ui::state::{AppMode, AppSettings, PdfRecord, PrinterInfo, ScannedCode};
use std::collections::HashSet;
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::{mpsc, Mutex};

/// Shared scanned codes buffer.
pub struct AppScannedCodes(pub Arc<Mutex<Vec<ScannedCode>>>);

/// PDF generator.
pub struct AppPdfGenerator(pub Arc<PdfGenerator>);

/// Audio feedback channel.
pub struct AppAudio(pub mpsc::UnboundedSender<SoundType>);

/// Shared application settings (mode + printer).
pub struct AppSettingsState(pub Arc<Mutex<AppSettings>>);

/// Tracks scanned code raw strings for duplicate detection (runtime only, not persisted).
pub struct AppScanHistory(pub Arc<Mutex<HashSet<String>>>);

/// Shared log buffer for dev tools.
pub struct AppLogBuffer(pub crate::log_buffer::SharedLogBuffer);

fn codes_to_labels(codes: &[ScannedCode], with_index: bool) -> Vec<LabelData> {
    codes
        .iter()
        .enumerate()
        .map(|(i, c)| LabelData {
            raw_code: c.code.raw.clone(),
            vendor_code: c.vendor_code.clone(),
            expire_date: c.expire_date.clone(),
            index: if with_index { Some(i + 1) } else { None },
        })
        .collect()
}

fn generated_to_record(generated: &crate::pdf::GeneratedPdf) -> PdfRecord {
    let filename = generated
        .path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    PdfRecord {
        path: generated.path.to_string_lossy().to_string(),
        filename,
        created_at: chrono::Local::now().format("%d.%m.%Y %H:%M").to_string(),
        code_count: generated.code_count,
    }
}

/// Regenerate the buffered PDF from current codes and emit event.
pub async fn regenerate_buffered_pdf(
    codes: &[ScannedCode],
    pdf_gen: &PdfGenerator,
    handle: &tauri::AppHandle,
) {
    if codes.is_empty() {
        let _ = handle.emit("pdf-regenerated", Option::<PdfRecord>::None);
        return;
    }

    let labels = codes_to_labels(codes, true);
    match pdf_gen.generate(&labels) {
        Ok(generated) => {
            let record = generated_to_record(&generated);
            let _ = handle.emit("pdf-regenerated", Some(record));
        }
        Err(e) => {
            tracing::error!("Failed to regenerate buffered PDF: {}", e);
            let _ = handle.emit("error", format!("Ошибка генерации PDF: {}", e));
        }
    }
}

#[tauri::command]
pub async fn generate_pdf(
    codes_state: State<'_, AppScannedCodes>,
    pdf_generator: State<'_, AppPdfGenerator>,
    audio: State<'_, AppAudio>,
) -> Result<PdfRecord, String> {
    let codes = codes_state.0.lock().await;
    if codes.is_empty() {
        return Err("Нет кодов для генерации PDF".to_string());
    }

    let labels = codes_to_labels(&codes, true);
    drop(codes);

    match pdf_generator.0.generate(&labels) {
        Ok(generated) => {
            let _ = audio.0.send(SoundType::Success);
            Ok(generated_to_record(&generated))
        }
        Err(e) => {
            let _ = audio.0.send(SoundType::Error);
            Err(format!("Ошибка генерации PDF: {}", e))
        }
    }
}

#[tauri::command]
pub async fn clear_buffer(
    codes_state: State<'_, AppScannedCodes>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut codes = codes_state.0.lock().await;
    codes.clear();
    let _ = app_handle.emit("pdf-regenerated", Option::<PdfRecord>::None);
    Ok(())
}

#[tauri::command]
pub async fn remove_code(
    index: usize,
    codes_state: State<'_, AppScannedCodes>,
    pdf_generator: State<'_, AppPdfGenerator>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut codes = codes_state.0.lock().await;
    if index >= codes.len() {
        return Err("Индекс за пределами буфера".to_string());
    }
    codes.remove(index);

    // Regenerate PDF
    regenerate_buffered_pdf(&codes, &pdf_generator.0, &app_handle).await;
    Ok(())
}

#[tauri::command]
pub async fn list_pdfs(
    pdf_generator: State<'_, AppPdfGenerator>,
) -> Result<Vec<PdfRecord>, String> {
    pdf_generator
        .0
        .list_pdfs()
        .map(|paths| {
            paths
                .into_iter()
                .filter_map(|path| {
                    let filename = path.file_name()?.to_string_lossy().to_string();
                    let metadata = std::fs::metadata(&path).ok()?;
                    let modified = metadata.modified().ok()?;
                    let datetime: chrono::DateTime<chrono::Local> = modified.into();
                    Some(PdfRecord {
                        path: path.to_string_lossy().to_string(),
                        filename,
                        created_at: datetime.format("%d.%m.%Y %H:%M").to_string(),
                        code_count: 0,
                    })
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_pdf_history(
    pdf_generator: State<'_, AppPdfGenerator>,
) -> Result<usize, String> {
    pdf_generator.0.clear_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_pdf(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn list_printers() -> Result<Vec<PrinterInfo>, String> {
    Ok(printer::list_printers())
}

#[tauri::command]
pub async fn print_pdf(path: String, printer_name: String) -> Result<(), String> {
    printer::print_pdf(&path, &printer_name)
}

#[tauri::command]
pub async fn set_mode(
    mode: AppMode,
    settings: State<'_, AppSettingsState>,
) -> Result<(), String> {
    let mut s = settings.0.lock().await;
    s.mode = mode;
    let _ = crate::ui::persistence::save_settings(&s);
    Ok(())
}

#[tauri::command]
pub async fn set_printer(
    printer_name: String,
    settings: State<'_, AppSettingsState>,
) -> Result<(), String> {
    let mut s = settings.0.lock().await;
    s.selected_printer = Some(printer_name);
    let _ = crate::ui::persistence::save_settings(&s);
    Ok(())
}

#[tauri::command]
pub async fn get_settings(
    settings: State<'_, AppSettingsState>,
) -> Result<AppSettings, String> {
    let s = settings.0.lock().await;
    Ok(s.clone())
}

#[tauri::command]
pub async fn set_barcode_settings(
    enabled: bool,
    copies: u32,
    active_preset: Option<String>,
    settings: State<'_, AppSettingsState>,
) -> Result<(), String> {
    let mut s = settings.0.lock().await;
    s.barcode_enabled = enabled;
    s.barcode_copies = copies;
    s.barcode_active_preset = active_preset;
    crate::ui::persistence::save_settings(&s)
}

#[tauri::command]
pub async fn set_barcode_preset_directory(
    preset_name: String,
    directory: String,
    settings: State<'_, AppSettingsState>,
) -> Result<(), String> {
    let mut s = settings.0.lock().await;
    if let Some(preset) = s.barcode_presets.iter_mut().find(|p| p.name == preset_name) {
        preset.directory = directory;
    } else {
        return Err(format!("Пресет '{}' не найден", preset_name));
    }
    crate::ui::persistence::save_settings(&s)
}

#[tauri::command]
pub async fn print_buffered_barcodes(
    codes_state: State<'_, AppScannedCodes>,
    settings: State<'_, AppSettingsState>,
    pdf_generator: State<'_, AppPdfGenerator>,
) -> Result<u32, String> {
    let s = settings.0.lock().await;
    if !s.barcode_enabled {
        return Ok(0);
    }
    let preset_name = s
        .barcode_active_preset
        .clone()
        .ok_or("Пресет штрихкодов не выбран")?;
    let preset = s
        .barcode_presets
        .iter()
        .find(|p| p.name == preset_name)
        .ok_or(format!("Пресет '{}' не найден", preset_name))?
        .clone();
    let printer_name = s
        .selected_printer
        .clone()
        .ok_or("Принтер не выбран")?;
    let copies = s.barcode_copies;
    drop(s);

    let codes = codes_state.0.lock().await;

    // Collect all barcode PDFs with their copy counts
    let mut barcode_inputs: Vec<(std::path::PathBuf, u32)> = Vec::new();
    for code in codes.iter() {
        if let Some(ref vc) = code.vendor_code {
            if let Some(barcode_path) =
                crate::pdf::barcode::find_barcode_pdf(&preset.directory, vc)
            {
                barcode_inputs.push((barcode_path, copies));
            }
        }
    }

    if barcode_inputs.is_empty() {
        return Ok(0);
    }

    let printed = barcode_inputs.len() as u32;

    // Also include the honest signs PDF if available
    let honest_signs_pdf = {
        let labels = codes_to_labels(&codes, true);
        pdf_generator.0.generate(&labels).ok()
    };
    drop(codes);

    // Build merge inputs: honest signs first, then all barcodes
    let mut merge_inputs: Vec<(&std::path::Path, u32)> = Vec::new();
    if let Some(ref hs) = honest_signs_pdf {
        merge_inputs.push((hs.path.as_path(), 1));
    }
    for (path, cp) in &barcode_inputs {
        merge_inputs.push((path.as_path(), *cp));
    }

    let merged_bytes = crate::pdf::merge::merge_pdfs(&merge_inputs)?;
    let output_dir = pdf_generator.0.output_dir();
    let merged_path = output_dir.join(format!(
        "combined_barcodes_{}.pdf",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ));
    std::fs::write(&merged_path, merged_bytes)
        .map_err(|e| format!("Failed to write merged PDF: {}", e))?;

    printer::print_pdf_auto_size(&merged_path.to_string_lossy(), &printer_name)?;
    tracing::info!(
        "Printed combined PDF ({} honest signs + {} barcodes) to {}",
        honest_signs_pdf.as_ref().map_or(0, |hs| hs.code_count),
        printed,
        printer_name
    );

    Ok(printed)
}

#[tauri::command]
pub async fn select_directory() -> Result<Option<String>, String> {
    Ok(rfd::FileDialog::new()
        .pick_folder()
        .map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn set_duplicate_detection(
    buffered: bool,
    instant: bool,
    settings: State<'_, AppSettingsState>,
) -> Result<(), String> {
    let mut s = settings.0.lock().await;
    s.duplicate_detection_buffered = buffered;
    s.duplicate_detection_instant = instant;
    crate::ui::persistence::save_settings(&s)
}

#[tauri::command]
pub async fn clear_scan_history(
    scan_history: State<'_, AppScanHistory>,
) -> Result<(), String> {
    let mut history = scan_history.0.lock().await;
    history.clear();
    Ok(())
}

#[tauri::command]
pub async fn get_recent_logs(
    log_buffer: State<'_, AppLogBuffer>,
) -> Result<Vec<String>, String> {
    Ok(log_buffer.0.get_entries())
}
