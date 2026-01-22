use chrono::Local;
use datamatrix::{DataMatrix, SymbolList};
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during PDF generation.
#[derive(Debug, Error)]
pub enum PdfGeneratorError {
    #[error("Failed to encode DataMatrix: {0}")]
    DataMatrixEncoding(String),

    #[error("Failed to create PDF: {0}")]
    PdfCreation(String),

    #[error("Failed to write PDF file: {0}")]
    FileWrite(#[from] std::io::Error),

    #[error("Failed to create output directory: {0}")]
    DirectoryCreation(String),
}

/// Information about a generated PDF file.
#[derive(Debug, Clone)]
pub struct GeneratedPdf {
    /// Path to the generated PDF file
    pub path: PathBuf,
    /// Number of codes/pages in the PDF
    pub code_count: usize,
    /// Generation timestamp
    pub created_at: chrono::DateTime<Local>,
}

/// PDF generator for Honest Sign DataMatrix labels.
///
/// Generates PDF files with DataMatrix barcodes on 58mm x 40mm thermal labels.
/// Each page contains one barcode, maintaining the scan order.
pub struct PdfGenerator {
    /// Output directory for generated PDFs
    output_dir: PathBuf,
}

impl PdfGenerator {
    /// Creates a new PDF generator.
    ///
    /// # Arguments
    /// * `output_dir` - Directory where PDFs will be saved
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Generates a PDF with DataMatrix barcodes for the given codes.
    ///
    /// # Arguments
    /// * `codes` - Raw Honest Sign code data (as received from scanner/validator)
    ///
    /// # Returns
    /// Information about the generated PDF file.
    pub fn generate(&self, codes: &[Vec<u8>]) -> Result<GeneratedPdf, PdfGeneratorError> {
        if codes.is_empty() {
            return Err(PdfGeneratorError::PdfCreation(
                "No codes provided".to_string(),
            ));
        }

        // Ensure output directory exists
        std::fs::create_dir_all(&self.output_dir).map_err(|e| {
            PdfGeneratorError::DirectoryCreation(format!(
                "Failed to create {}: {}",
                self.output_dir.display(),
                e
            ))
        })?;

        // Generate filename with timestamp
        let timestamp = Local::now();
        let filename = format!("labels_{}.pdf", timestamp.format("%Y%m%d_%H%M%S"));
        let filepath = self.output_dir.join(&filename);

        // Create PDF document
        // Page size: 58mm x 40mm (thermal label)
        let (doc, page_index, layer_index) =
            PdfDocument::new("Honest Sign Labels", Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Page 1");

        let mut current_layer = doc.get_page(page_index).get_layer(layer_index);

        // Generate first page
        Self::add_datamatrix_to_layer(&mut current_layer, &codes[0])?;

        // Generate additional pages
        let doc = doc;
        for (i, code) in codes.iter().enumerate().skip(1) {
            let (page_index, layer_index) = doc.add_page(
                Mm(PAGE_WIDTH_MM),
                Mm(PAGE_HEIGHT_MM),
                format!("Page {}", i + 1),
            );
            let mut layer = doc.get_page(page_index).get_layer(layer_index);
            Self::add_datamatrix_to_layer(&mut layer, code)?;
        }

        // Save PDF
        let file = File::create(&filepath)?;
        let writer = BufWriter::new(file);
        doc.save(&mut BufWriter::new(writer))
            .map_err(|e| PdfGeneratorError::PdfCreation(e.to_string()))?;

        tracing::info!(
            "Generated PDF with {} codes: {}",
            codes.len(),
            filepath.display()
        );

        Ok(GeneratedPdf {
            path: filepath,
            code_count: codes.len(),
            created_at: timestamp,
        })
    }

    /// Adds a DataMatrix barcode to a PDF layer.
    fn add_datamatrix_to_layer(
        layer: &mut PdfLayerReference,
        code: &[u8],
    ) -> Result<(), PdfGeneratorError> {
        // Encode DataMatrix with GS1 format
        // The code already contains GS separators (0x1D), encode_gs1 adds FNC1 at start
        let dm = DataMatrix::encode_gs1(code, SymbolList::default()).map_err(|e| {
            PdfGeneratorError::DataMatrixEncoding(format!("Failed to encode: {:?}", e))
        })?;

        let bitmap = dm.bitmap();
        let width = bitmap.width();
        let height = bitmap.height();

        tracing::debug!(
            "Generated DataMatrix: {}x{} modules for {} bytes",
            width,
            height,
            code.len()
        );

        // Calculate module size to fit within the label
        // Target barcode size: ~15mm (to leave margins)
        let target_size_mm = DATAMATRIX_SIZE_MM;
        let module_size_mm = target_size_mm / width.max(height) as f32;

        let barcode_width_mm = width as f32 * module_size_mm;
        let barcode_height_mm = height as f32 * module_size_mm;

        // Center the barcode on the page
        let x_offset = (PAGE_WIDTH_MM - barcode_width_mm) / 2.0;
        let y_offset = (PAGE_HEIGHT_MM - barcode_height_mm) / 2.0;

        // Draw each black module as a rectangle
        // pixels() returns (x, y) coordinates for all black modules
        for (x, y) in bitmap.pixels() {
            let rect_x = x_offset + (x as f32 * module_size_mm);
            // PDF coordinates are from bottom-left, so invert Y
            let rect_y = y_offset + ((height - 1 - y) as f32 * module_size_mm);

            let rect = Rect::new(
                Mm(rect_x),
                Mm(rect_y),
                Mm(rect_x + module_size_mm),
                Mm(rect_y + module_size_mm),
            );

            layer.add_rect(rect);
        }

        // Set fill color to black
        layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));

        Ok(())
    }

    /// Returns the path to the output directory.
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Lists all generated PDF files in the output directory.
    pub fn list_pdfs(&self) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut pdfs = Vec::new();

        if self.output_dir.exists() {
            for entry in std::fs::read_dir(&self.output_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "pdf").unwrap_or(false) {
                    pdfs.push(path);
                }
            }
        }

        // Sort by modification time (newest first)
        pdfs.sort_by(|a, b| {
            let a_time = std::fs::metadata(a)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let b_time = std::fs::metadata(b)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            b_time.cmp(&a_time)
        });

        Ok(pdfs)
    }

    /// Deletes all PDF files in the output directory.
    pub fn clear_all(&self) -> Result<usize, std::io::Error> {
        let pdfs = self.list_pdfs()?;
        let count = pdfs.len();

        for pdf in pdfs {
            std::fs::remove_file(pdf)?;
        }

        tracing::info!("Cleared {} PDF files from output directory", count);
        Ok(count)
    }
}

impl Default for PdfGenerator {
    fn default() -> Self {
        Self::new("output")
    }
}

// Page dimensions for 58mm x 40mm thermal label
const PAGE_WIDTH_MM: f32 = 58.0;
const PAGE_HEIGHT_MM: f32 = 40.0;

// Target DataMatrix size in mm (fits within label with margins)
const DATAMATRIX_SIZE_MM: f32 = 18.0;
