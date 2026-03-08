//! Minimal reader for PoE `.datc64` binary files.
//!
//! The datc64 format:
//! - Bytes 0..4: u32 LE row count
//! - Bytes 4..marker: fixed-size row data (row_count * row_size)
//! - 8-byte marker: `0xBBBBBBBBBBBBBBBB`
//! - After marker: variable-length data section (strings, lists)
//!
//! All multi-byte values are little-endian. Strings are UTF-16LE null-terminated.
//! Foreign key references are u64 (row index into another table, or `0xFEFEFEFEFEFEFEFE` for null).
//! Lists are (u64 length, u64 offset) pointing into the variable data section.

use std::fmt;

/// A parsed datc64 file, ready for field extraction.
pub struct DatFile {
    bytes: Vec<u8>,
    pub row_count: u32,
    pub row_size: usize,
    rows_start: usize,
    data_section: usize,
}

impl fmt::Debug for DatFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DatFile({} rows, {} bytes/row, {} total bytes)",
            self.row_count,
            self.row_size,
            self.bytes.len()
        )
    }
}

/// Sentinel value for null foreign keys and empty fields.
const NULL_FK: u64 = 0xFEFE_FEFE_FEFE_FEFE;

/// The 8-byte marker that separates fixed rows from variable data.
const DATA_SECTION_MARKER: [u8; 8] = [0xBB; 8];

impl DatFile {
    /// Parse a datc64 file from raw bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, DatError> {
        if bytes.len() < 4 {
            return Err(DatError::TooShort);
        }

        let row_count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let rows_start = 4;

        // Find the data section marker
        let data_section = find_marker(&bytes, &DATA_SECTION_MARKER)
            .ok_or(DatError::NoMarker)?;

        let rows_total = data_section - rows_start;
        if row_count == 0 {
            return Ok(DatFile {
                bytes,
                row_count: 0,
                row_size: 0,
                rows_start,
                data_section,
            });
        }

        let row_size = rows_total / row_count as usize;

        Ok(DatFile {
            bytes,
            row_count,
            row_size,
            rows_start,
            data_section, // string/list offsets are relative to the marker position
        })
    }

    /// Read a u32 field from a row at the given byte offset within the row.
    pub fn read_u32(&self, row: u32, offset: usize) -> Option<u32> {
        let pos = self.field_pos(row, offset)?;
        if pos + 4 > self.bytes.len() {
            return None;
        }
        Some(u32::from_le_bytes([
            self.bytes[pos],
            self.bytes[pos + 1],
            self.bytes[pos + 2],
            self.bytes[pos + 3],
        ]))
    }

    /// Read an i32 field.
    pub fn read_i32(&self, row: u32, offset: usize) -> Option<i32> {
        let pos = self.field_pos(row, offset)?;
        if pos + 4 > self.bytes.len() {
            return None;
        }
        Some(i32::from_le_bytes([
            self.bytes[pos],
            self.bytes[pos + 1],
            self.bytes[pos + 2],
            self.bytes[pos + 3],
        ]))
    }

    /// Read a u64 field (used for foreign keys).
    /// Returns `None` for null FK (`0xFEFEFEFEFEFEFEFE`).
    pub fn read_fk(&self, row: u32, offset: usize) -> Option<u64> {
        let pos = self.field_pos(row, offset)?;
        if pos + 8 > self.bytes.len() {
            return None;
        }
        let val = u64::from_le_bytes([
            self.bytes[pos],
            self.bytes[pos + 1],
            self.bytes[pos + 2],
            self.bytes[pos + 3],
            self.bytes[pos + 4],
            self.bytes[pos + 5],
            self.bytes[pos + 6],
            self.bytes[pos + 7],
        ]);
        if val == NULL_FK { None } else { Some(val) }
    }

    /// Read a bool field (1 byte, 0 = false, anything else = true).
    pub fn read_bool(&self, row: u32, offset: usize) -> Option<bool> {
        let pos = self.field_pos(row, offset)?;
        if pos >= self.bytes.len() {
            return None;
        }
        Some(self.bytes[pos] != 0)
    }

    /// Read a string field. The row contains a `ref|string` (u64 offset into data section).
    /// The string is UTF-16LE null-terminated in the data section.
    pub fn read_string(&self, row: u32, offset: usize) -> Option<String> {
        let str_offset = self.read_fk(row, offset)?;
        self.read_string_at(str_offset)
    }

    /// Read a UTF-16LE null-terminated string at a data section offset.
    fn read_string_at(&self, offset: u64) -> Option<String> {
        let pos = self.data_section + offset as usize;
        if pos >= self.bytes.len() {
            return None;
        }
        let mut chars = Vec::new();
        let mut i = pos;
        while i + 1 < self.bytes.len() {
            let c = u16::from_le_bytes([self.bytes[i], self.bytes[i + 1]]);
            if c == 0 {
                break;
            }
            chars.push(c);
            i += 2;
        }
        String::from_utf16(&chars).ok()
    }

    /// Read a list of u64 values (typically FK indices).
    /// The row contains (u64 length, u64 offset) at the given offset.
    pub fn read_list_u64(&self, row: u32, offset: usize) -> Vec<u64> {
        let Some((len, list_offset)) = self.read_list_header(row, offset) else {
            return Vec::new();
        };
        let pos = self.data_section + list_offset as usize;
        (0..len)
            .filter_map(|i| {
                let p = pos + i as usize * 8;
                if p + 8 > self.bytes.len() {
                    return None;
                }
                let val = u64::from_le_bytes([
                    self.bytes[p],
                    self.bytes[p + 1],
                    self.bytes[p + 2],
                    self.bytes[p + 3],
                    self.bytes[p + 4],
                    self.bytes[p + 5],
                    self.bytes[p + 6],
                    self.bytes[p + 7],
                ]);
                Some(val)
            })
            .collect()
    }

    /// Read a list of i32 values.
    pub fn read_list_i32(&self, row: u32, offset: usize) -> Vec<i32> {
        let Some((len, list_offset)) = self.read_list_header(row, offset) else {
            return Vec::new();
        };
        let pos = self.data_section + list_offset as usize;
        (0..len)
            .filter_map(|i| {
                let p = pos + i as usize * 4;
                if p + 4 > self.bytes.len() {
                    return None;
                }
                Some(i32::from_le_bytes([
                    self.bytes[p],
                    self.bytes[p + 1],
                    self.bytes[p + 2],
                    self.bytes[p + 3],
                ]))
            })
            .collect()
    }

    /// Read the (length, offset) header for a list field.
    fn read_list_header(&self, row: u32, offset: usize) -> Option<(u64, u64)> {
        let pos = self.field_pos(row, offset)?;
        if pos + 16 > self.bytes.len() {
            return None;
        }
        let len = u64::from_le_bytes([
            self.bytes[pos],
            self.bytes[pos + 1],
            self.bytes[pos + 2],
            self.bytes[pos + 3],
            self.bytes[pos + 4],
            self.bytes[pos + 5],
            self.bytes[pos + 6],
            self.bytes[pos + 7],
        ]);
        let off = u64::from_le_bytes([
            self.bytes[pos + 8],
            self.bytes[pos + 9],
            self.bytes[pos + 10],
            self.bytes[pos + 11],
            self.bytes[pos + 12],
            self.bytes[pos + 13],
            self.bytes[pos + 14],
            self.bytes[pos + 15],
        ]);
        if len == NULL_FK {
            return None;
        }
        Some((len, off))
    }

    /// Compute the byte position of a field in the raw data.
    fn field_pos(&self, row: u32, offset: usize) -> Option<usize> {
        if row >= self.row_count || offset >= self.row_size {
            return None;
        }
        Some(self.rows_start + row as usize * self.row_size + offset)
    }
}

/// Errors that can occur when reading a datc64 file.
#[derive(Debug, thiserror::Error)]
pub enum DatError {
    #[error("file too short to contain row count header")]
    TooShort,
    #[error("data section marker (0xBB*8) not found")]
    NoMarker,
}

/// Find the position of an 8-byte marker in a byte slice.
fn find_marker(bytes: &[u8], marker: &[u8; 8]) -> Option<usize> {
    bytes
        .windows(8)
        .position(|w| w == marker)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_dat() -> Vec<u8> {
        // 2 rows, each 12 bytes: u32 + u64(ref|string)
        // String offsets are relative to marker position (marker = 8 bytes,
        // so first string after marker is at offset 8).
        let mut bytes = Vec::new();

        // Row count
        bytes.extend_from_slice(&2u32.to_le_bytes());

        // Row 0: u32=42, string offset=8 (right after the 8-byte marker)
        bytes.extend_from_slice(&42u32.to_le_bytes());
        bytes.extend_from_slice(&8u64.to_le_bytes());

        // Row 1: u32=99, string offset=14 (8 + "Hi\0" = 8 + 6 bytes)
        bytes.extend_from_slice(&99u32.to_le_bytes());
        bytes.extend_from_slice(&14u64.to_le_bytes());

        // Data section marker
        bytes.extend_from_slice(&DATA_SECTION_MARKER);

        // Variable data: "Hi\0" then "Ok\0" in UTF-16LE
        for c in &[b'H', b'i'] {
            bytes.push(*c);
            bytes.push(0);
        }
        bytes.push(0);
        bytes.push(0); // null terminator

        for c in &[b'O', b'k'] {
            bytes.push(*c);
            bytes.push(0);
        }
        bytes.push(0);
        bytes.push(0); // null terminator

        bytes
    }

    #[test]
    fn read_basic_fields() {
        let dat = DatFile::from_bytes(make_test_dat()).unwrap();
        assert_eq!(dat.row_count, 2);
        assert_eq!(dat.row_size, 12);
        assert_eq!(dat.read_u32(0, 0), Some(42));
        assert_eq!(dat.read_u32(1, 0), Some(99));
        assert_eq!(dat.read_string(0, 4), Some("Hi".to_string()));
        assert_eq!(dat.read_string(1, 4), Some("Ok".to_string()));
    }
}
