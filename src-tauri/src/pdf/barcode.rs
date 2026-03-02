use std::path::{Path, PathBuf};

/// Finds a barcode PDF by vendor_code in the given directory.
/// Case-insensitive matching: vendor_code "SN264" matches "sn264.pdf" or "SN264.pdf".
pub fn find_barcode_pdf(directory: &str, vendor_code: &str) -> Option<PathBuf> {
    let dir = Path::new(directory);
    if !dir.is_dir() {
        tracing::warn!("Barcode directory does not exist: {}", directory);
        return None;
    }

    let target = format!("{}.pdf", vendor_code.to_lowercase());

    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let filename = entry.file_name().to_string_lossy().to_lowercase();
        if filename == target {
            return Some(entry.path());
        }
    }

    tracing::warn!(
        "Barcode PDF not found for vendor_code '{}' in '{}'",
        vendor_code,
        directory
    );
    None
}

/// Prints a barcode PDF N times to the given printer (auto-size, no custom media override).
pub fn print_barcode(path: &Path, printer_name: &str, copies: u32) -> Result<(), String> {
    let path_str = path.to_string_lossy();
    for i in 0..copies {
        super::printer::print_pdf_auto_size(&path_str, printer_name)?;
        tracing::info!(
            "Printed barcode copy {}/{} to {}",
            i + 1,
            copies,
            printer_name
        );
    }
    Ok(())
}
