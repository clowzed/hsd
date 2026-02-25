use crate::audio::SoundType;
use crate::pdf::{printer, LabelData, PdfGenerator};
use crate::ui::state::{AppMode, AppSettings, PdfRecord, PrinterInfo, ScannedCode};
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
    Ok(())
}

#[tauri::command]
pub async fn set_printer(
    printer_name: String,
    settings: State<'_, AppSettingsState>,
) -> Result<(), String> {
    let mut s = settings.0.lock().await;
    s.selected_printer = Some(printer_name);
    Ok(())
}

#[tauri::command]
pub async fn get_settings(
    settings: State<'_, AppSettingsState>,
) -> Result<AppSettings, String> {
    let s = settings.0.lock().await;
    Ok(s.clone())
}
