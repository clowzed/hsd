pub mod barcode;
mod generator;
pub mod printer;

pub use generator::{GeneratedPdf, LabelData, PdfGenerator};
