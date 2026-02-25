use honest_sign_scanner_lib::pdf::{LabelData, PdfGenerator};
use std::path::PathBuf;

fn main() {
    // Real honest sign code from user:
    // 0104610011368387215)EQ0a'fCUHP5 91EE11 925j/4GYBV/q8OtXKcCPNI3cSf4CFlKJSSCB2wf2pvGlg=
    // Spaces represent GS (0x1D) separators between AI fields:
    // AI 01: GTIN = 04610011368387
    // AI 21: serial = 5)EQ0a'fCUHP5
    // AI 91: verification key = EE11
    // AI 92: crypto signature = 5j/4GYBV/q8OtXKcCPNI3cSf4CFlKJSSCB2wf2pvGlg=
    let real_code: Vec<u8> = {
        let mut v = Vec::new();
        v.extend_from_slice(b"0104610011368387215)EQ0a'fCUHP5");
        v.push(0x1D); // GS separator
        v.extend_from_slice(b"91EE11");
        v.push(0x1D); // GS separator
        v.extend_from_slice(b"925j/4GYBV/q8OtXKcCPNI3cSf4CFlKJSSCB2wf2pvGlg=");
        v
    };

    let output_dir = PathBuf::from("/tmp/honest_sign_test_pdfs");
    let _ = std::fs::remove_dir_all(&output_dir);
    let generator = PdfGenerator::new(&output_dir);

    // Test 1: Instant mode (no index)
    let instant_label = vec![LabelData {
        raw_code: real_code.clone(),
        vendor_code: Some("ART-45892".to_string()),
        expire_date: Some("15.06.2026".to_string()),
        index: None,
    }];

    match generator.generate(&instant_label) {
        Ok(result) => println!("Instant label: {}", result.path.display()),
        Err(e) => eprintln!("Error: {}", e),
    }

    std::thread::sleep(std::time::Duration::from_secs(1));

    // Test 2: Buffered mode (3 pages with scan index in corner)
    let buffered_labels = vec![
        LabelData {
            raw_code: real_code.clone(),
            vendor_code: Some("ART-45892".to_string()),
            expire_date: Some("15.06.2026".to_string()),
            index: Some(1),
        },
        LabelData {
            raw_code: real_code.clone(),
            vendor_code: Some("VK-10034".to_string()),
            expire_date: Some("01.12.2025".to_string()),
            index: Some(2),
        },
        LabelData {
            raw_code: real_code.clone(),
            vendor_code: None,
            expire_date: None,
            index: Some(3),
        },
    ];

    match generator.generate(&buffered_labels) {
        Ok(result) => println!("Buffered labels (3 pages): {}", result.path.display()),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\nDone! Files in /tmp/honest_sign_test_pdfs/");
}
