use crate::ui::state::PrinterInfo;
use std::process::Command;

/// Lists available printers via macOS CUPS `lpstat`.
pub fn list_printers() -> Vec<PrinterInfo> {
    let default_printer = get_default_printer();

    let output = match Command::new("lpstat").arg("-p").output() {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("Failed to run lpstat: {}", e);
            return Vec::new();
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    stdout
        .lines()
        .filter_map(|line| {
            // Format: "printer PrinterName is idle. ..." or "printer PrinterName disabled ..."
            let line = line.strip_prefix("printer ")?;
            let name = line.split_whitespace().next()?;
            Some(PrinterInfo {
                is_default: default_printer.as_deref() == Some(name),
                name: name.to_string(),
            })
        })
        .collect()
}

/// Gets the system default printer name.
fn get_default_printer() -> Option<String> {
    let output = Command::new("lpstat").arg("-d").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Format: "system default destination: PrinterName"
    stdout
        .trim()
        .strip_prefix("system default destination: ")
        .map(|s| s.trim().to_string())
}

/// Prints a PDF file to the specified printer with 58x40mm label media size.
pub fn print_pdf(path: &str, printer: &str) -> Result<(), String> {
    let output = Command::new("lp")
        .args(["-d", printer, "-o", "media=Custom.58x40mm", path])
        .output()
        .map_err(|e| format!("Не удалось запустить lp: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Ошибка печати: {}", stderr.trim()));
    }

    tracing::info!("Printed {} to printer {}", path, printer);
    Ok(())
}
