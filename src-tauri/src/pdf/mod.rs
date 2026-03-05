pub mod barcode;
mod generator;
pub mod merge;
pub mod printer;

pub use generator::{GeneratedPdf, LabelData, PdfGenerator};
