use byteorder::{LittleEndian, ReadBytesExt};
use log::*;
use std::io::Cursor;

pub use poe_dat::dat_reader::DatFile;

use crate::traversal::value::Value;
use crate::traversal::value::Value::U64;

use super::specification::FieldSpec;
use super::specification::FileSpec;

// ── Spec-driven field reading (poe-query extension) ─────────────────────────

/// Extension trait that adds schema-driven field reading to `DatFile`.
///
/// This is the poe-query layer: it reads fields based on `FieldSpec` type
/// strings (e.g., "ref|string", "list|u64", "bool") and returns `Value` enums.
/// The core binary reading lives in poe-dat's `DatFile`.
pub trait DatFileQueryExt {
    fn valid(&self, spec: &FileSpec);
    fn read_field(&self, row: u64, field: &FieldSpec) -> Value;
    fn read_value(&self, offset: u64, data_type: &str) -> Value;
    fn read_list(&self, offset: u64, len: u64, data_type: &str) -> Vec<Value>;
}

impl DatFileQueryExt for DatFile {
    fn valid(&self, spec: &FileSpec) {
        debug!("Validating using specification '{}'", spec);
        let last_field = spec.file_fields.last();
        if let Some(field) = last_field {
            let spec_row_size = field.field_offset + FileSpec::field_size(field);
            if self.row_size > spec_row_size {
                warn!("Spec for '{}' missing {} bytes", spec.file_name, self.row_size - spec_row_size);
            }
            if spec_row_size > self.row_size {
                warn!("Spec for '{}' overflows by {} bytes", spec.file_name, spec_row_size - self.row_size);
            }
        } else {
            warn!("Spec for {} does not contain fields", spec.file_name);
        }
    }

    fn read_field(&self, row: u64, field: &FieldSpec) -> Value {
        let row_offset = self.rows_begin + row as usize * self.row_size;
        let exact_offset = row_offset + field.field_offset;

        if field.field_offset > self.row_size {
            return Value::Empty;
        }

        let mut cursor = Cursor::new(&self.bytes[exact_offset..]);

        let mut parts = field.field_type.split('|');
        let prefix = parts.next();
        let result = if let Some(enum_spec) = &field.enum_name {
            match cursor.u32() {
                Value::U64(v) => Value::Str(enum_spec.value(v as usize)),
                Value::Empty => Value::Empty,
                x => panic!("reading {} from row {} - got {:?}", field, row, x)
            }
        } else if prefix.filter(|&dtype| "list" == dtype).is_some() {
            let length = cursor.u64();
            let offset = cursor.u64();
            match (offset, length) {
                (Value::U64(o), Value::U64(len)) => Value::List(self.read_list(o, len, parts.next().unwrap())),
                _ => Value::Empty
            }
        } else if prefix.filter(|&dtype| "ref" == dtype).is_some() {
            match cursor.u64() {
                Value::U64(offset) => self.read_value(offset, parts.next().unwrap()),
                Value::Empty => Value::Empty,
                x => panic!("reading {} from row {} - got {:?}", field, row, x)
            }
        } else {
            cursor.read_value(field.field_type.as_str())
        };
        debug!("Result {}[{}] = {:?}", field, row, result);
        result
    }

    fn read_value(&self, offset: u64, data_type: &str) -> Value {
        let exact_offset = self.data_section + offset as usize;
        if !self.check_offset(exact_offset) {
            error!("Offset {} exceeds file size {}", exact_offset, self.total_size);
            return Value::Empty;
        }

        let mut cursor = Cursor::new(&self.bytes[exact_offset..]);
        cursor.read_value(data_type)
    }

    fn read_list(&self, offset: u64, len: u64, data_type: &str) -> Vec<Value> {
        let exact_offset = self.data_section + offset as usize;
        if !self.check_offset(exact_offset) {
            error!("Offset {} exceeds file size {}", exact_offset, self.total_size);
            return Vec::new();
        }

        let mut cursor = Cursor::new(&self.bytes[exact_offset..]);
        (0..len).map(|_| {
            match data_type {
                "string" | "path" => {
                    let U64(offset) = cursor.u64() else { panic!("Unable to read offset to string list element") };
                    let mut text_cursor = Cursor::new(&self.bytes[(self.data_section + offset as usize)..]);
                    text_cursor.read_value(data_type)

                },
                _ => cursor.read_value(data_type)
            }
        }).collect()
    }
}

// ── Cursor → Value conversion (poe-query specific) ─────────────────────────

trait ReadBytesToValue {
    fn read_value(&mut self, tag: &str) -> Value;
    fn bool(&mut self) -> Value;
    fn u8(&mut self) -> Value;
    fn u32(&mut self) -> Value;
    fn i32(&mut self) -> Value;
    fn f32(&mut self) -> Value;
    fn u64(&mut self) -> Value;
    fn utf16(&mut self) -> String;
    fn utf8(&mut self) -> String;
}

impl ReadBytesToValue for Cursor<&[u8]> {

    fn read_value(&mut self, tag: &str) -> Value {
        match tag {
            "bool" => self.bool(),
            "u8"   => self.u8(),
            "u32"  => self.u32(),
            "i32"  => self.i32(),
            "f32"  => self.f32(),
            "ptr"  => self.u64(),
            "u64"  => self.u64(),
            "string" => Value::Str(self.utf16()),
            "path" => Value::Str(self.utf8()),
            "_" => Value::Empty,
            value => panic!("Unsupported type in specification. {}", value),
        }
    }

    fn bool(&mut self) -> Value {
        match self.read_u8() {
            Ok(0) => Value::Bool(false),
            Ok(1) => Value::Bool(true),
            Ok(255) => Value::Bool(true),
            Ok(value) => {
                warn!("Expected boolean value got {}", value);
                Value::Bool(true)
            },
            _ => panic!("Unable to read bool"),
        }
    }

    fn u8(&mut self) -> Value {
        match self.read_u8() {
            Ok(value) => Value::Byte(value),
            Err(_)=> panic!("Unable to read u8"),
        }
    }

    fn u32(&mut self) -> Value {
        match self.read_u32::<LittleEndian>() {
            Ok(value) => u32_to_enum(value),
            Err(_) => panic!("Unable to read u32"),
        }
    }

    fn i32(&mut self) -> Value {
        match self.read_i32::<LittleEndian>() {
            Ok(value) => i32_to_enum(value),
            Err(_) => panic!("Unable to read u32"),
        }
    }

    fn f32(&mut self) -> Value {
        match self.read_f32::<LittleEndian>() {
            Ok(value) => f32_to_enum(value),
            Err(_) => panic!("Unable to read f32"),
        }
    }

    fn u64(&mut self) -> Value {
        match self.read_u64::<LittleEndian>() {
            Ok(value) => u64_to_enum(value),
            Err(_) => panic!("Unable to read u64"),
        }
    }

    fn utf16(&mut self) -> String {
        let raw = (0..)
            .map(|_| self.read_u16::<LittleEndian>().unwrap())
            .take_while(|&x| x != 0u16)
            .collect::<Vec<u16>>();
        String::from_utf16(&raw).expect("Unable to decode as UTF-16 String")
    }

    fn utf8(&mut self) -> String {
        let raw = (0..)
            .map(|_| self.read_u16::<LittleEndian>().unwrap())
            .take_while(|&x| x != 0u16)
            .map(|x| x as u8)
            .collect::<Vec<u8>>();
        String::from_utf8(raw).expect("Unable to decode as UTF-8 String")
    }
}

fn u64_to_enum(value: u64) -> Value {
    if value == 0xFEFEFEFEFEFEFEFE {
        return Value::Empty;
    }
    Value::U64(value)
}

fn u32_to_enum(value: u32) -> Value {
    if value == 0xFEFEFEFE {
        return Value::Empty;
    }
    Value::U64(value as u64)
}

fn i32_to_enum(value: i32) -> Value {
    Value::I64(value as i64)
}

fn f32_to_enum(value: f32) -> Value {
    Value::F32(value)
}
