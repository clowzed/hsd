use crate::api::{CrptClient, CrptError, CrptResponse};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Represents a parsed and validated Honest Sign code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HonestSignCode {
    /// Raw code as received from scanner
    pub raw: Vec<u8>,
    /// Raw code as string (for API calls)
    pub raw_string: String,
    /// GTIN (14-digit product code)
    pub gtin: String,
    /// Serial number (if extracted)
    pub serial: Option<String>,
    /// Crypto verification code (if extracted)
    pub crypto: Option<String>,
}

/// Validation errors for Honest Sign codes.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Код слишком короткий: {len} байт, минимум {min}")]
    TooShort { len: usize, min: usize },

    #[error("Код не начинается с '01'")]
    InvalidStart,

    #[error("Неверный формат GTIN - должен содержать только цифры")]
    InvalidGtin,

    #[error("Ошибка контрольной суммы GTIN")]
    GtinChecksumFailed,

    #[error("Отсутствует маркер серийного номера '21'")]
    MissingSerialMarker,

    #[error("Код не найден в системе")]
    CodeNotFound,

    #[error("{status}: {explanation}")]
    InvalidStatus {
        status: String,
        explanation: String,
    },

    #[error("Ошибка сети: {0}")]
    NetworkError(String),

    #[error("Ошибка API: {0}")]
    ApiError(String),
}

/// Result of code validation including API response.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Parsed Honest Sign code
    pub code: HonestSignCode,
    /// API response with product information
    pub response: CrptResponse,
}

/// Validator for Honest Sign codes.
///
/// Performs both local format validation and API validation against CRPT.
pub struct HonestSignValidator {
    api_client: CrptClient,
}

impl HonestSignValidator {
    /// Creates a new validator.
    pub fn new() -> Self {
        Self {
            api_client: CrptClient::new(),
        }
    }

    /// Validates a raw barcode scan.
    ///
    /// # Process
    /// 1. Parse and validate local format (GTIN, serial, crypto)
    /// 2. Validate GTIN checksum
    /// 3. Call CRPT API to verify code
    /// 4. Check that outerStatus is IN_CIRCULATION
    ///
    /// # Arguments
    /// * `raw_data` - Raw bytes from scanner
    ///
    /// # Returns
    /// Validation result with parsed code and API response, or error.
    pub async fn validate(&self, raw_data: &[u8]) -> Result<ValidationResult, ValidationError> {
        // Step 1: Parse local format
        let code = Self::parse_code(raw_data)?;

        tracing::info!(
            "Parsed code - GTIN: {}, Serial: {:?}, Crypto: {:?}",
            code.gtin,
            code.serial,
            code.crypto
        );

        // Step 2: Call API
        let response = self
            .api_client
            .check_code(&code.raw_string)
            .await
            .map_err(|e| match e {
                CrptError::Network(e) => ValidationError::NetworkError(e.to_string()),
                CrptError::ApiError(e) => ValidationError::ApiError(e),
                CrptError::ParseError(e) => ValidationError::ApiError(e),
            })?;

        // Log full API response for debugging
        tracing::info!(
            "API response: code_founded={}, outer_status={:?}, status={:?}, status_v2={:?}, product_name={:?}",
            response.code_founded,
            response.outer_status,
            response.status,
            response.status_v2,
            response.product_name
        );

        // Step 3: Check if code was found
        if !response.code_founded {
            return Err(ValidationError::CodeNotFound);
        }

        // Step 4: Check status is acceptable for label printing (currently IN_CIRCULATION only)
        if !response.is_acceptable_for_label() {
            tracing::warn!(
                "Code not acceptable for label: status={:?}, message={}",
                response.outer_status,
                response.status_message_ru()
            );
            return Err(ValidationError::InvalidStatus {
                status: response.status_message_ru(),
                explanation: response.status_explanation_ru(),
            });
        }

        tracing::info!(
            "Code validated successfully: {} - {}",
            code.gtin,
            response.product_name.as_deref().unwrap_or("Unknown")
        );

        Ok(ValidationResult { code, response })
    }

    /// Parses and validates the local format of a Honest Sign code.
    ///
    /// Honest Sign codes can have various formats:
    /// - Standard: 01{GTIN:14}21{serial}GS91{key}GS92{signature}
    /// - Short: 01{GTIN:14}21{serial}GS93{crypto:4}
    /// - With weight: 01{GTIN:14}21{serial}GS3103{weight:6}GS93{crypto:4}
    ///
    /// The GS character (0x1D) is used as Application Identifier separator.
    /// Application Identifiers: 01=GTIN, 21=serial, 91=key, 92=signature, 93=verification, 3103=weight
    fn parse_code(raw_data: &[u8]) -> Result<HonestSignCode, ValidationError> {
        // Strip optional symbology prefixes from scanner
        // ]d2 = GS1 DataMatrix, ]C1 = Code 128, etc.
        let data = if raw_data.starts_with(b"]d2") || raw_data.starts_with(b"]C1") {
            &raw_data[3..]
        } else if raw_data.starts_with(b"]d1") || raw_data.starts_with(b"]Q3") {
            &raw_data[3..]
        } else {
            raw_data
        };

        // Log raw data for debugging
        tracing::debug!(
            "Parsing code: {} bytes, hex: {:02X?}",
            data.len(),
            &data[..std::cmp::min(50, data.len())]
        );

        // Minimum structure: 01{14}21{1} = 17 bytes (very minimal)
        // We're lenient here - let the API do the final validation
        if data.len() < 17 {
            return Err(ValidationError::TooShort {
                len: data.len(),
                min: 17,
            });
        }

        // Must start with "01" (GTIN Application Identifier)
        if !data.starts_with(b"01") {
            return Err(ValidationError::InvalidStart);
        }

        // GTIN must be 14 digits (positions 2-15)
        let gtin = &data[2..16];
        if !gtin.iter().all(|&b| b.is_ascii_digit()) {
            return Err(ValidationError::InvalidGtin);
        }

        // Validate GTIN check digit
        if !Self::validate_gtin_checksum(gtin) {
            return Err(ValidationError::GtinChecksumFailed);
        }

        // Must have "21" at position 16 (serial number AI)
        if data.len() < 18 || &data[16..18] != b"21" {
            return Err(ValidationError::MissingSerialMarker);
        }

        // Parse Application Identifiers to extract serial and crypto
        let (serial, crypto) = Self::parse_application_identifiers(data, 18);

        // Convert to string, preserving raw bytes for API
        // Replace GS (0x1D) with ASCII 29 representation for API
        let raw_string = String::from_utf8_lossy(data).to_string();

        tracing::debug!(
            "Parsed - GTIN: {}, Serial: {:?}, Crypto: {:?}",
            String::from_utf8_lossy(gtin),
            serial,
            crypto
        );

        Ok(HonestSignCode {
            raw: data.to_vec(),
            raw_string,
            gtin: String::from_utf8_lossy(gtin).to_string(),
            serial,
            crypto,
        })
    }

    /// Parses Application Identifiers from position after "21" marker.
    ///
    /// Returns (serial, crypto) where serial is the value after AI 21,
    /// and crypto is the verification code if found.
    fn parse_application_identifiers(data: &[u8], start: usize) -> (Option<String>, Option<String>) {
        let mut serial: Option<String> = None;
        let mut crypto: Option<String> = None;
        let mut pos = start;

        // Find the end of serial number (ends at GS or at next AI)
        let mut serial_end = data.len();
        for i in start..data.len() {
            // GS separator (0x1D)
            if data[i] == 0x1D {
                serial_end = i;
                break;
            }
            // Check for known 2-digit AIs that might appear without GS
            // AI 91, 92, 93 (verification codes)
            if i + 2 <= data.len() && i > start + 4 {
                let potential_ai = &data[i..i + 2];
                if potential_ai == b"91" || potential_ai == b"92" || potential_ai == b"93" {
                    serial_end = i;
                    break;
                }
            }
            // Check for 4-digit AIs (like 3103 for weight)
            if i + 4 <= data.len() && i > start + 4 {
                let potential_ai = &data[i..i + 4];
                if potential_ai == b"3103" || potential_ai == b"3102" || potential_ai == b"3100" {
                    serial_end = i;
                    break;
                }
            }
        }

        if serial_end > start {
            serial = Some(String::from_utf8_lossy(&data[start..serial_end]).to_string());
        }

        // Continue parsing remaining AIs
        pos = serial_end;
        while pos < data.len() {
            // Skip GS separator
            if data[pos] == 0x1D {
                pos += 1;
                continue;
            }

            // Check for 2-digit AIs
            if pos + 2 <= data.len() {
                let ai = &data[pos..pos + 2];

                // AI 91 - Verification key (variable length, up to 90 chars)
                // AI 92 - Internal verification (variable length)
                // AI 93 - Verification code (typically 4 chars for short format)
                if ai == b"91" || ai == b"92" || ai == b"93" {
                    pos += 2;
                    let value_start = pos;

                    // Find end of value (next GS or end of data or next AI)
                    while pos < data.len() && data[pos] != 0x1D {
                        // Check if we hit another AI
                        if pos + 2 <= data.len() && pos > value_start {
                            let next = &data[pos..pos + 2];
                            if next == b"91" || next == b"92" || next == b"93" {
                                break;
                            }
                        }
                        if pos + 4 <= data.len() {
                            let next = &data[pos..pos + 4];
                            if next == b"3103" || next == b"3102" || next == b"3100" {
                                break;
                            }
                        }
                        pos += 1;
                    }

                    if pos > value_start {
                        let value = String::from_utf8_lossy(&data[value_start..pos]).to_string();
                        // Store crypto from AI 93, or from 91/92 if no 93 found
                        if ai == b"93" || crypto.is_none() {
                            crypto = Some(value);
                        }
                    }
                    continue;
                }
            }

            // Check for 4-digit AIs
            if pos + 4 <= data.len() {
                let ai = &data[pos..pos + 4];

                // AI 3103 - weight in grams (6 digits)
                // AI 3102 - weight with 2 decimal places
                // AI 3100 - weight with 0 decimal places
                if ai == b"3103" || ai == b"3102" || ai == b"3100" {
                    pos += 4;
                    // Weight is always 6 digits
                    pos += std::cmp::min(6, data.len() - pos);
                    continue;
                }
            }

            // Unknown data, skip one byte
            pos += 1;
        }

        (serial, crypto)
    }

    /// Validates GTIN-14 check digit.
    ///
    /// GTIN check digit algorithm:
    /// 1. Multiply digits at odd positions (1,3,5...) by 3
    /// 2. Multiply digits at even positions (2,4,6...) by 1
    /// 3. Sum all products
    /// 4. Check digit makes sum divisible by 10
    fn validate_gtin_checksum(gtin: &[u8]) -> bool {
        if gtin.len() != 14 {
            return false;
        }

        let digits: Vec<u32> = gtin
            .iter()
            .filter_map(|&b| (b as char).to_digit(10))
            .collect();

        if digits.len() != 14 {
            return false;
        }

        let sum: u32 = digits
            .iter()
            .enumerate()
            .map(|(i, &d)| if i % 2 == 0 { d * 3 } else { d })
            .sum();

        sum % 10 == 0
    }
}

impl Default for HonestSignValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gtin_checksum_valid() {
        // Example valid GTIN-14
        assert!(HonestSignValidator::validate_gtin_checksum(b"04607025399810"));
    }

    #[test]
    fn test_gtin_checksum_invalid() {
        // Changed last digit
        assert!(!HonestSignValidator::validate_gtin_checksum(b"04607025399811"));
    }

    #[test]
    fn test_parse_code_basic_with_gs93() {
        // Code with GS + 93 marker
        let code = b"0104607025399810215p=nN\"\x1D93abcd";
        let result = HonestSignValidator::parse_code(code);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.gtin, "04607025399810");
        assert_eq!(parsed.serial, Some("5p=nN\"".to_string()));
        assert_eq!(parsed.crypto, Some("abcd".to_string()));
    }

    #[test]
    fn test_parse_code_with_91_92() {
        // Code with AI 91 and 92 (long format verification)
        let code = b"0104607025399810215p=nN\"\x1D91abcd\x1D92efghijklmnop";
        let result = HonestSignValidator::parse_code(code);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.gtin, "04607025399810");
        assert_eq!(parsed.serial, Some("5p=nN\"".to_string()));
        // Crypto should be extracted from AI 91 or 92
        assert!(parsed.crypto.is_some());
    }

    #[test]
    fn test_parse_code_with_weight() {
        // Code with weight (AI 3103)
        let code = b"0104607025399810215p=nN\"\x1D3103000500\x1D93abcd";
        let result = HonestSignValidator::parse_code(code);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.gtin, "04607025399810");
        assert_eq!(parsed.serial, Some("5p=nN\"".to_string()));
        assert_eq!(parsed.crypto, Some("abcd".to_string()));
    }

    #[test]
    fn test_parse_code_with_prefix() {
        let code = b"]d20104607025399810215p=nN\"\x1D93abcd";
        let result = HonestSignValidator::parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_code_too_short() {
        let code = b"01046070";
        let result = HonestSignValidator::parse_code(code);
        assert!(matches!(result, Err(ValidationError::TooShort { .. })));
    }

    #[test]
    fn test_parse_code_invalid_start() {
        let code = b"0204607025399810215p=nN\"\x1D93abcd";
        let result = HonestSignValidator::parse_code(code);
        assert!(matches!(result, Err(ValidationError::InvalidStart)));
    }

    #[test]
    fn test_parse_long_code_no_explicit_93() {
        // Long code that might not have explicit 93 marker
        // This tests the lenient parsing
        let code = b"04607025399810215p=nN\"some_long_verification_data";
        let result = HonestSignValidator::parse_code(code);
        // Should fail because doesn't start with "01"
        assert!(result.is_err());

        // With proper "01" prefix
        let code = b"0104607025399810215p=nN\"some_verification";
        let result = HonestSignValidator::parse_code(code);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.gtin, "04607025399810");
        // Serial should include everything until a separator is found
        assert!(parsed.serial.is_some());
    }
}
