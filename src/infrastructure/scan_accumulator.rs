/// Accumulates scanner data chunks until CR+LF terminator is received.
///
/// Mertech scanners send CR+LF (0x0D 0x0A) after each complete scan.
/// This accumulator buffers incoming data and extracts complete codes
/// when terminators are found.
///
/// # Fragmentation Handling
///
/// Sometimes the scanner doesn't send all data until the next scan button press:
/// - Scan 1 button press → sends partial data (no CR+LF)
/// - Scan 2 button press → sends remaining data from scan 1 + CR+LF + scan 2 data + CR+LF
///
/// This accumulator handles this by:
/// 1. Buffering all incoming data in `pending`
/// 2. Only returning complete codes when CR+LF is found
/// 3. Keeping any trailing data for the next code
#[derive(Debug, Default)]
pub struct ScanAccumulator {
    /// Data waiting for CR+LF terminator
    pending: Vec<u8>,
}

impl ScanAccumulator {
    /// Creates a new empty accumulator.
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Process incoming chunk of data from the scanner.
    ///
    /// Returns a vector of complete codes (terminated by CR+LF).
    /// May return multiple codes if one chunk contains several terminators.
    /// May return empty vector if no complete codes are found yet.
    ///
    /// # Arguments
    /// * `chunk` - Raw bytes received from the scanner
    ///
    /// # Returns
    /// Vector of complete barcode data (without CR+LF terminators)
    pub fn process_chunk(&mut self, chunk: &[u8]) -> Vec<Vec<u8>> {
        self.pending.extend_from_slice(chunk);

        let mut complete_codes = Vec::new();

        loop {
            // Look for CR+LF (0x0D 0x0A) or just LF (0x0A) in pending data
            // Some configurations might send only LF
            let terminator_pos = self.find_terminator();

            match terminator_pos {
                Some((pos, terminator_len)) => {
                    // Extract code (everything before terminator)
                    let code = self.pending[..pos].to_vec();

                    // Remove processed data including terminator
                    self.pending = self.pending[pos + terminator_len..].to_vec();

                    // Only add non-empty codes
                    if !code.is_empty() {
                        tracing::debug!(
                            "Extracted complete code: {} bytes",
                            code.len()
                        );
                        complete_codes.push(code);
                    }
                }
                None => break, // No more complete codes
            }
        }

        if !complete_codes.is_empty() {
            tracing::info!(
                "Extracted {} complete code(s), {} bytes pending",
                complete_codes.len(),
                self.pending.len()
            );
        }

        complete_codes
    }

    /// Find the position of the next terminator (CR+LF or just LF).
    /// Returns the position and the length of the terminator.
    fn find_terminator(&self) -> Option<(usize, usize)> {
        // First, look for CR+LF (most common)
        if let Some(pos) = self
            .pending
            .windows(2)
            .position(|w| w == [0x0D, 0x0A])
        {
            return Some((pos, 2));
        }

        // Fall back to just LF (some configurations)
        if let Some(pos) = self.pending.iter().position(|&b| b == 0x0A) {
            return Some((pos, 1));
        }

        // Also check for just CR (rare but possible)
        if let Some(pos) = self.pending.iter().position(|&b| b == 0x0D) {
            // Make sure it's not followed by LF (would be CR+LF handled above)
            if pos + 1 >= self.pending.len() || self.pending[pos + 1] != 0x0A {
                return Some((pos, 1));
            }
        }

        None
    }

    /// Check if there's incomplete data waiting for terminator.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Get the number of pending bytes (for debugging/logging).
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Clear all pending data.
    /// Use when resetting connection or discarding corrupted data.
    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_complete_code() {
        let mut acc = ScanAccumulator::new();
        let codes = acc.process_chunk(b"01046070253998102155p=nN\"\r\n");
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0], b"01046070253998102155p=nN\"");
    }

    #[test]
    fn test_fragmented_code() {
        let mut acc = ScanAccumulator::new();

        // First chunk - incomplete
        let codes1 = acc.process_chunk(b"01046070253998102155p=nN");
        assert!(codes1.is_empty());
        assert!(acc.has_pending());

        // Second chunk - completes first code and starts new one
        let codes2 = acc.process_chunk(b"\"\r\n0104607025399811\r\n");
        assert_eq!(codes2.len(), 2);
        assert_eq!(codes2[0], b"01046070253998102155p=nN\"");
        assert_eq!(codes2[1], b"0104607025399811");
    }

    #[test]
    fn test_multiple_codes_single_chunk() {
        let mut acc = ScanAccumulator::new();
        let codes = acc.process_chunk(b"code1\r\ncode2\r\ncode3\r\n");
        assert_eq!(codes.len(), 3);
        assert_eq!(codes[0], b"code1");
        assert_eq!(codes[1], b"code2");
        assert_eq!(codes[2], b"code3");
    }

    #[test]
    fn test_lf_only_terminator() {
        let mut acc = ScanAccumulator::new();
        let codes = acc.process_chunk(b"code1\ncode2\n");
        assert_eq!(codes.len(), 2);
    }

    #[test]
    fn test_empty_lines_ignored() {
        let mut acc = ScanAccumulator::new();
        let codes = acc.process_chunk(b"\r\ncode1\r\n\r\n");
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0], b"code1");
    }
}
