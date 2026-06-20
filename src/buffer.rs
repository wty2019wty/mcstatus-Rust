/// An in-memory byte buffer with a read/write position cursor.
///
/// Used both for packet construction (writing) and as a mock connection in tests
/// (where data is pre-loaded for reading).
#[derive(Debug, Clone)]
pub struct Buffer {
    data: Vec<u8>,
    pos: usize,
}

impl Buffer {
    /// Creates a new empty buffer.
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            pos: 0,
        }
    }

    /// Creates a buffer pre-filled with the given data.
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }

    /// Creates a buffer pre-filled with hex-encoded bytes.
    ///
    /// Panics if the hex string is invalid.
    #[cfg(test)]
    pub fn from_hex(hex: &str) -> Self {
        let data = hex::decode(hex).expect("Invalid hex string");
        Self { data, pos: 0 }
    }

    /// Returns the number of bytes written so far (the length of the buffer).
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the number of bytes remaining to be read.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Returns a slice of all the data in the buffer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Consumes the buffer and returns the underlying data.
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Resets the read position to the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Clears all data and resets the position.
    pub fn clear(&mut self) {
        self.data.clear();
        self.pos = 0;
    }

    // === Write operations (used for packet construction) ===

    /// Appends raw bytes to the buffer.
    pub fn write_bytes(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    /// Appends a single unsigned byte to the buffer.
    pub fn write_ubyte(&mut self, value: u8) {
        self.data.push(value);
    }

    /// Appends a big-endian unsigned short (2 bytes) to the buffer.
    pub fn write_ushort(&mut self, value: u16) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    /// Appends a big-endian unsigned int (4 bytes) to the buffer.
    pub fn write_uint(&mut self, value: u32) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    /// Appends a big-endian long long (8 bytes) to the buffer.
    pub fn write_ulonglong(&mut self, value: u64) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    /// Appends a big-endian signed byte to the buffer.
    pub fn write_byte(&mut self, value: i8) {
        self.data.push(value as u8);
    }

    /// Appends a big-endian signed short (2 bytes) to the buffer.
    pub fn write_short(&mut self, value: i16) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    /// Appends a boolean as a single byte (0 or 1).
    pub fn write_bool(&mut self, value: bool) {
        self.data.push(if value { 1 } else { 0 });
    }

    // === Read operations ===

    /// Reads `length` bytes from the current position, advancing the cursor.
    ///
    /// Returns an error if there are not enough bytes remaining.
    pub fn read_bytes(&mut self, length: usize) -> std::io::Result<Vec<u8>> {
        if length == 0 {
            return Ok(Vec::new());
        }
        if self.pos + length > self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "Not enough bytes to read: requested {length}, available {}",
                    self.remaining()
                ),
            ));
        }
        let bytes = self.data[self.pos..self.pos + length].to_vec();
        self.pos += length;
        Ok(bytes)
    }

    /// Reads a single unsigned byte.
    pub fn read_ubyte(&mut self) -> std::io::Result<u8> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0])
    }

    /// Reads a big-endian unsigned short (2 bytes).
    pub fn read_ushort(&mut self) -> std::io::Result<u16> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Reads a big-endian unsigned int (4 bytes).
    pub fn read_uint(&mut self) -> std::io::Result<u32> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Reads a big-endian long long (8 bytes).
    pub fn read_ulonglong(&mut self) -> std::io::Result<u64> {
        let bytes = self.read_bytes(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Reads a bool (single byte, 0 = false, anything else = true).
    pub fn read_bool(&mut self) -> std::io::Result<bool> {
        Ok(self.read_ubyte()? != 0)
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

// ── std::io::Write implementation ────────────────────────────────────────────

impl std::io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// ── std::io::Read implementation ─────────────────────────────────────────────

impl std::io::Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let available = self.data.len().saturating_sub(self.pos);
        let to_read = buf.len().min(available);
        if to_read == 0 {
            return Ok(0);
        }
        buf[..to_read].copy_from_slice(&self.data[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Ok(to_read)
    }
}

/// Re-export of hex for test usage.
/// In dev/test builds, we use a simple hex implementation.
/// For production, this is only used internally.
#[cfg(test)]
mod hex {
    pub fn decode(hex: &str) -> Result<Vec<u8>, String> {
        let hex: String = hex
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();

        if hex.len() % 2 != 0 {
            return Err("Hex string must have an even number of characters".into());
        }

        (0..hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&hex[i..i + 2], 16)
                    .map_err(|e| format!("Invalid hex string: {e}"))
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_decode_hex() {
            assert_eq!(decode("ff00ab").unwrap(), vec![0xff, 0x00, 0xab]);
            assert_eq!(decode("FF 00 AB").unwrap(), vec![0xff, 0x00, 0xab]);
        }
    }
}
