use crate::ui::state::PrinterInfo;
use printers::common::converters::Converter;
use printers::common::base::job::PrinterJobOptions;
use printers::{get_printer_by_name, get_printers};

/// Lists available printers using the `printers` crate (CUPS on macOS).
pub fn list_printers() -> Vec<PrinterInfo> {
    get_printers()
        .into_iter()
        .map(|p| PrinterInfo {
            is_default: p.is_default,
            name: p.name.clone(),
        })
        .collect()
}

/// Prints a PDF file to the specified printer with 58x40mm label media size.
pub fn print_pdf(path: &str, printer_name: &str) -> Result<(), String> {
    let printer = get_printer_by_name(printer_name)
        .ok_or_else(|| format!("Принтер '{}' не найден", printer_name))?;

    printer
        .print_file(
            path,
            PrinterJobOptions {
                name: Some("HonestSign Label"),
                raw_properties: &[("media", "Custom.58x40mm")],
                converter: Converter::None,
            },
        )
        .map_err(|e| format!("Ошибка печати: {}", e.message))?;

    tracing::info!("Printed {} to printer {}", path, printer_name);
    Ok(())
}
