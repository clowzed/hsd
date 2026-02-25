use chrono::Local;
use datamatrix::{DataMatrixBuilder, EncodationType, SymbolList};
use image::{ImageBuffer, Rgb};
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};
use std::path::{Path, PathBuf};
use thiserror::Error;

// Page dimensions for 58mm x 40mm thermal label (in PDF points)
const PAGE_WIDTH_PT: f32 = 58.0 * 2.83464567;
const PAGE_HEIGHT_PT: f32 = 40.0 * 2.83464567;

// DataMatrix scale factor (each module becomes scale×scale pixels)
const DM_SCALE: usize = 2;

// Embedded background image
const MARK_PNG: &[u8] = include_bytes!("../../assets/mark.png");

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

    #[error("Failed to load background image: {0}")]
    ImageError(#[from] image::ImageError),
}

/// Data for a single label page.
pub struct LabelData {
    pub raw_code: Vec<u8>,
    pub vendor_code: Option<String>,
    pub expire_date: Option<String>,
    pub index: Option<usize>,
}

/// Information about a generated PDF file.
#[derive(Debug, Clone)]
pub struct GeneratedPdf {
    pub path: PathBuf,
    pub code_count: usize,
    pub created_at: chrono::DateTime<Local>,
}

/// PDF generator for Honest Sign DataMatrix labels.
pub struct PdfGenerator {
    output_dir: PathBuf,
}

impl PdfGenerator {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Generates a multi-page PDF with one label per page.
    pub fn generate(&self, labels: &[LabelData]) -> Result<GeneratedPdf, PdfGeneratorError> {
        if labels.is_empty() {
            return Err(PdfGeneratorError::PdfCreation(
                "No codes provided".to_string(),
            ));
        }

        std::fs::create_dir_all(&self.output_dir).map_err(|e| {
            PdfGeneratorError::DirectoryCreation(format!(
                "Failed to create {}: {}",
                self.output_dir.display(),
                e
            ))
        })?;

        let timestamp = Local::now();
        let filename = format!("labels_{}.pdf", timestamp.format("%Y%m%d_%H%M%S"));
        let filepath = self.output_dir.join(&filename);

        let pdf_bytes = self.build_pdf(labels)?;
        std::fs::write(&filepath, pdf_bytes)?;

        tracing::info!(
            "Generated PDF with {} labels: {}",
            labels.len(),
            filepath.display()
        );

        Ok(GeneratedPdf {
            path: filepath,
            code_count: labels.len(),
            created_at: timestamp,
        })
    }

    fn build_pdf(&self, labels: &[LabelData]) -> Result<Vec<u8>, PdfGeneratorError> {
        let n = labels.len();

        // Load background image once
        let mark_img = image::load_from_memory(MARK_PNG)?.to_rgb8();
        let (mark_w, mark_h) = mark_img.dimensions();
        let mark_raw = mark_img.as_raw();

        let mut pdf = Pdf::new();
        let page_rect = Rect::new(0.0, 0.0, PAGE_WIDTH_PT, PAGE_HEIGHT_PT);

        // Ref allocation:
        // 1 = catalog, 2 = page tree, 3 = mark image, 4 = font
        // Per page i: 5 + i*3 = page, 6 + i*3 = content, 7 + i*3 = dm image
        let catalog_id = Ref::new(1);
        let page_tree_id = Ref::new(2);
        let mark_image_id = Ref::new(3);
        let font_id = Ref::new(4);

        let page_ids: Vec<Ref> = (0..n).map(|i| Ref::new((5 + i * 3) as i32)).collect();
        let content_ids: Vec<Ref> = (0..n).map(|i| Ref::new((6 + i * 3) as i32)).collect();
        let dm_image_ids: Vec<Ref> = (0..n).map(|i| Ref::new((7 + i * 3) as i32)).collect();

        // Catalog + page tree
        pdf.catalog(catalog_id).pages(page_tree_id);
        pdf.pages(page_tree_id)
            .kids(page_ids.clone())
            .count(n as i32);

        // Shared background image XObject
        let mut mark_xobj = pdf.image_xobject(mark_image_id, mark_raw);
        mark_xobj.width(mark_w as i32);
        mark_xobj.height(mark_h as i32);
        mark_xobj.color_space().device_rgb();
        mark_xobj.bits_per_component(8);
        mark_xobj.finish();

        // Shared font (Helvetica — built-in Type1, works for ASCII text)
        pdf.type1_font(font_id).base_font(Name(b"Helvetica"));

        // Generate each page
        for (i, label) in labels.iter().enumerate() {
            let dm_img = self.render_datamatrix(&label.raw_code)?;
            let (dm_w, dm_h) = dm_img.dimensions();
            let dm_raw = dm_img.as_raw();

            // DataMatrix image XObject
            let mut dm_xobj = pdf.image_xobject(dm_image_ids[i], dm_raw);
            dm_xobj.width(dm_w as i32);
            dm_xobj.height(dm_h as i32);
            dm_xobj.color_space().device_rgb();
            dm_xobj.bits_per_component(8);
            dm_xobj.finish();

            // Page
            let mut page = pdf.page(page_ids[i]);
            page.media_box(page_rect);
            page.parent(page_tree_id);
            page.contents(content_ids[i]);

            let mut resources = page.resources();
            resources
                .x_objects()
                .pair(Name(b"Bg"), mark_image_id)
                .pair(Name(b"DM"), dm_image_ids[i]);
            resources.fonts().pair(Name(b"F1"), font_id);
            resources.finish();
            page.finish();

            // Content stream
            let content_bytes = self.build_page_content(
                dm_w as f32,
                dm_h as f32,
                &label.vendor_code,
                &label.expire_date,
                label.index,
            );
            pdf.stream(content_ids[i], &content_bytes);
        }

        Ok(pdf.finish())
    }

    fn render_datamatrix(
        &self,
        raw_code: &[u8],
    ) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, PdfGeneratorError> {
        let code = DataMatrixBuilder::new()
            .with_encodation_types(EncodationType::Ascii)
            .with_macros(false)
            .with_fnc1_start(true)
            .with_symbol_list(SymbolList::default())
            .encode(raw_code)
            .map_err(|e| PdfGeneratorError::DataMatrixEncoding(format!("{:?}", e)))?;

        let bitmap = code.bitmap();
        let pixels: Vec<(usize, usize)> = bitmap.pixels().collect();
        let scaled = resize_black_pixels(&pixels, DM_SCALE);

        let width = (bitmap.width() * DM_SCALE) as u32;
        let height = (bitmap.height() * DM_SCALE) as u32;

        let mut img = ImageBuffer::from_pixel(width, height, Rgb([255u8, 255u8, 255u8]));
        for (x, y) in scaled {
            if x < width && y < height {
                img.put_pixel(x, y, Rgb([0u8, 0u8, 0u8]));
            }
        }

        Ok(img)
    }

    fn build_page_content(
        &self,
        dm_w: f32,
        dm_h: f32,
        vendor_code: &Option<String>,
        expire_date: &Option<String>,
        index: Option<usize>,
    ) -> Vec<u8> {
        let mut content = Content::new();

        // Draw background (covers entire page)
        content.save_state();
        content.transform([PAGE_WIDTH_PT, 0.0, 0.0, PAGE_HEIGHT_PT, 0.0, 0.0]);
        content.x_object(Name(b"Bg"));
        content.restore_state();

        // Draw DataMatrix centered with slight right offset
        let dm_x = ((PAGE_WIDTH_PT - dm_w) / 2.0) + 20.0;
        let dm_y = (PAGE_HEIGHT_PT - dm_h) / 2.0;
        content.save_state();
        content.transform([dm_w, 0.0, 0.0, dm_h, dm_x, dm_y]);
        content.x_object(Name(b"DM"));
        content.restore_state();

        // Scan index in top-left corner (small, for internal use)
        if let Some(idx) = index {
            content.begin_text();
            content.set_font(Name(b"F1"), 5.0);
            content.next_line(3.0, PAGE_HEIGHT_PT - 8.0);
            let idx_text = format!("#{}", idx);
            content.show(Str(idx_text.as_bytes()));
            content.end_text();
        }

        // Info text below DataMatrix (vendor code + expiration date)
        let has_info = vendor_code.is_some() || expire_date.is_some();
        if has_info {
            let text_x = dm_x;
            let mut text_y = dm_y - 8.0;

            content.begin_text();
            content.set_font(Name(b"F1"), 5.5);

            // Vendor code
            if let Some(ref vc) = vendor_code {
                content.next_line(text_x, text_y);
                content.show(Str(vc.as_bytes()));
                text_y -= 7.0;
            }

            // Expiration date
            if let Some(ref exp) = expire_date {
                let exp_text = format!("exp: {}", exp);
                let is_first = vendor_code.is_none();
                content.next_line(
                    if is_first { text_x } else { 0.0 },
                    if is_first { text_y } else { -7.0 },
                );
                content.show(Str(exp_text.as_bytes()));
            }

            content.end_text();
        }

        content.finish()
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

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
        let output_dir = dirs::document_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| std::env::temp_dir().into()))
            .join("HonestSignScanner")
            .join("pdfs");

        Self::new(output_dir)
    }
}

/// Scales pixel coordinates by a given factor (each pixel becomes scale×scale).
fn resize_black_pixels(pixels: &[(usize, usize)], scale: usize) -> Vec<(u32, u32)> {
    let mut resized = Vec::with_capacity(pixels.len() * scale * scale);
    for &(x, y) in pixels {
        for i in 0..scale {
            for j in 0..scale {
                resized.push(((x * scale + i) as u32, (y * scale + j) as u32));
            }
        }
    }
    resized
}
