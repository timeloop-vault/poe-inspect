//! Reader for `PoE` `.datc64` binary files.
//!
//! Based on poe-query's `dat/file.rs` (proven implementation). Adapted to
//! return native Rust types instead of the generic `Value` enum, and to use
//! typed field accessors instead of spec-driven `read_field()`.
//!
//! The datc64 format:
//! - Bytes 0..4: u32 LE row count
//! - Bytes 4..marker: fixed-size row data (`row_count` * `row_size`)
//! - 8-byte marker: `0xBBBBBBBBBBBBBBBB`
//! - After marker: variable-length data section (strings, lists)
//!
//! All multi-byte values are little-endian. Strings are UTF-16LE null-terminated.
//! String/list offsets in row data are relative to the marker position.
//! Foreign key references are 16 bytes: u64 row index + u64 key hash.
//! Null sentinel: `0xFEFEFEFEFEFEFEFE`.

use std::fmt;
use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt};

// ── Constants ────────────────────────────────────────────────────────────────

/// Sentinel value for null foreign keys and empty fields.
const NULL_U64: u64 = 0xFEFE_FEFE_FEFE_FEFE;

/// The 8-byte marker that separates fixed rows from variable data.
const DATA_SECTION_MARKER: &[u8; 8] = &[0xBB; 8];

// ── DatFile ──────────────────────────────────────────────────────────────────

/// A parsed datc64 file, ready for field extraction.
pub struct DatFile {
    pub bytes: Vec<u8>,
    pub total_size: usize,
    pub rows_begin: usize,
    /// Offset of the data section marker within `bytes`.
    /// String and list offsets stored in rows are relative to this position.
    pub data_section: usize,
    pub row_count: u32,
    pub row_size: usize,
}

impl fmt::Debug for DatFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DatFile({} rows, {} bytes/row, {} total bytes)",
            self.row_count, self.row_size, self.total_size
        )
    }
}

impl DatFile {
    /// Parse a datc64 file from raw bytes.
    ///
    /// Adapted from poe-query `DatFile::from_bytes`.
    ///
    /// # Errors
    ///
    /// Returns `DatError` if the file is empty, too short, or missing the data section marker.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, DatError> {
        if bytes.is_empty() {
            return Err(DatError::Empty);
        }

        let mut cursor = Cursor::new(&bytes);
        let row_count = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| DatError::TooShort)?;

        let rows_begin = 4;
        let data_section = search_for(&bytes, DATA_SECTION_MARKER).ok_or(DatError::NoMarker)?;

        let rows_total_size = data_section - rows_begin;
        let row_size = if row_count == 0 {
            0
        } else {
            rows_total_size / row_count as usize
        };

        Ok(DatFile {
            total_size: bytes.len(),
            bytes,
            rows_begin,
            data_section,
            row_count,
            row_size,
        })
    }

    /// Check that an absolute offset is within the file bounds.
    ///
    /// From poe-query `DatFile::check_offset`.
    pub fn check_offset(&self, offset: usize) -> bool {
        offset <= self.total_size
    }

    // ── Typed field accessors ────────────────────────────────────────────

    /// Read a u32 field from a row at the given byte offset within the row.
    pub fn read_u32(&self, row: u32, offset: usize) -> Option<u32> {
        let pos = self.field_pos(row, offset)?;
        let slice = self.bytes.get(pos..pos + 4)?;
        let mut cursor = Cursor::new(slice);
        let val = cursor.read_u32::<LittleEndian>().ok()?;
        if val == 0xFEFE_FEFE { None } else { Some(val) }
    }

    /// Read an i32 field.
    pub fn read_i32(&self, row: u32, offset: usize) -> Option<i32> {
        let pos = self.field_pos(row, offset)?;
        let slice = self.bytes.get(pos..pos + 4)?;
        let mut cursor = Cursor::new(slice);
        cursor.read_i32::<LittleEndian>().ok()
    }

    /// Read a u64 field (used for foreign keys).
    /// Returns `None` for null FK (`0xFEFEFEFEFEFEFEFE`).
    ///
    /// Null check from poe-query `u64_to_enum`.
    pub fn read_fk(&self, row: u32, offset: usize) -> Option<u64> {
        let pos = self.field_pos(row, offset)?;
        let slice = self.bytes.get(pos..pos + 8)?;
        let mut cursor = Cursor::new(slice);
        let val = cursor.read_u64::<LittleEndian>().ok()?;
        if val == NULL_U64 { None } else { Some(val) }
    }

    /// Read a bool field (1 byte).
    ///
    /// Matches poe-query's bool handling: 0 = false, 1/255 = true,
    /// other non-zero values treated as true.
    pub fn read_bool(&self, row: u32, offset: usize) -> Option<bool> {
        let pos = self.field_pos(row, offset)?;
        let val = *self.bytes.get(pos)?;
        Some(val != 0)
    }

    /// Read a string field. The row contains a `ref|string` (u64 offset
    /// into the data section). The string is UTF-16LE null-terminated.
    ///
    /// String reading logic from poe-query `ReadBytesToValue::utf16`.
    pub fn read_string(&self, row: u32, offset: usize) -> Option<String> {
        let str_offset = self.read_fk(row, offset)?;
        self.read_string_at(str_offset)
    }

    /// Read a UTF-16LE null-terminated string at a data section offset.
    ///
    /// Adapted from poe-query `DatFile::read_value` + `utf16()`.
    fn read_string_at(&self, offset: u64) -> Option<String> {
        let exact_offset = self.data_section + offset as usize;
        if !self.check_offset(exact_offset) {
            return None;
        }
        let mut cursor = Cursor::new(&self.bytes[exact_offset..]);
        #[expect(clippy::maybe_infinite_iter)]
        let raw: Vec<u16> = (0..)
            .map(|_| cursor.read_u16::<LittleEndian>().unwrap_or(0))
            .take_while(|&x| x != 0)
            .collect();
        String::from_utf16(&raw).ok()
    }

    /// Read a list of u64 values (typically FK indices).
    /// The row contains (u64 length, u64 offset) at the given offset.
    ///
    /// List reading logic from poe-query `DatFile::read_list`.
    pub fn read_list_u64(&self, row: u32, offset: usize) -> Vec<u64> {
        let Some((len, list_offset)) = self.read_list_header(row, offset) else {
            return Vec::new();
        };
        let exact_offset = self.data_section + list_offset as usize;
        if !self.check_offset(exact_offset) {
            return Vec::new();
        }
        let mut cursor = Cursor::new(&self.bytes[exact_offset..]);
        (0..len)
            .filter_map(|_| cursor.read_u64::<LittleEndian>().ok())
            .collect()
    }

    /// Read a list of i32 values.
    pub fn read_list_i32(&self, row: u32, offset: usize) -> Vec<i32> {
        let Some((len, list_offset)) = self.read_list_header(row, offset) else {
            return Vec::new();
        };
        let exact_offset = self.data_section + list_offset as usize;
        if !self.check_offset(exact_offset) {
            return Vec::new();
        }
        let mut cursor = Cursor::new(&self.bytes[exact_offset..]);
        (0..len)
            .filter_map(|_| cursor.read_i32::<LittleEndian>().ok())
            .collect()
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    /// Read the (length, offset) header for a list field.
    fn read_list_header(&self, row: u32, offset: usize) -> Option<(u64, u64)> {
        let pos = self.field_pos(row, offset)?;
        let slice = self.bytes.get(pos..pos + 16)?;
        let mut cursor = Cursor::new(slice);
        let len = cursor.read_u64::<LittleEndian>().ok()?;
        let off = cursor.read_u64::<LittleEndian>().ok()?;
        if len == NULL_U64 {
            return None;
        }
        Some((len, off))
    }

    /// Compute the byte position of a field in the raw data.
    fn field_pos(&self, row: u32, offset: usize) -> Option<usize> {
        if row >= self.row_count || offset >= self.row_size {
            return None;
        }
        Some(self.rows_begin + row as usize * self.row_size + offset)
    }
}

// ── Errors ───────────────────────────────────────────────────────────────────

/// Errors that can occur when reading a datc64 file.
#[derive(Debug, thiserror::Error)]
pub enum DatError {
    #[error("file is empty")]
    Empty,
    #[error("file too short to contain row count header")]
    TooShort,
    #[error("data section marker (0xBB*8) not found")]
    NoMarker,
}

// ── Utilities ────────────────────────────────────────────────────────────────

/// Find the position of a byte pattern in a slice.
///
/// From poe-query `dat/util.rs`.
fn search_for(data: &[u8], needle: &[u8]) -> Option<usize> {
    data.windows(needle.len())
        .position(|window| window == needle)
}

// ── Tests ────────────────────────────────────────────────────────────────────

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
        bytes.extend_from_slice(DATA_SECTION_MARKER);

        // Variable data: "Hi\0" then "Ok\0" in UTF-16LE
        for c in b"Hi" {
            bytes.push(*c);
            bytes.push(0);
        }
        bytes.push(0);
        bytes.push(0); // null terminator

        for c in b"Ok" {
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
