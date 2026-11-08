//! Named Binary Tag (NBT) Serialization and Deserialization.
//!
//! Replicates Minecraft's binary NBT format with big-endian byte ordering.
//! Has NO graphics dependencies and runs entirely headless.

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::io::{self, Read, Write};

pub const TAG_END: u8 = 0;
pub const TAG_BYTE: u8 = 1;
pub const TAG_SHORT: u8 = 2;
pub const TAG_INT: u8 = 3;
pub const TAG_LONG: u8 = 4;
pub const TAG_FLOAT: u8 = 5;
pub const TAG_DOUBLE: u8 = 6;
pub const TAG_BYTE_ARRAY: u8 = 7;
pub const TAG_STRING: u8 = 8;
pub const TAG_LIST: u8 = 9;
pub const TAG_COMPOUND: u8 = 10;
pub const TAG_INT_ARRAY: u8 = 11;

#[derive(Debug, Clone, PartialEq)]
pub enum NbtTag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(Vec<NbtTag>),
    Compound(HashMap<String, NbtTag>),
    IntArray(Vec<i32>),
}

impl NbtTag {
    pub fn type_id(&self) -> u8 {
        match self {
            NbtTag::End => TAG_END,
            NbtTag::Byte(_) => TAG_BYTE,
            NbtTag::Short(_) => TAG_SHORT,
            NbtTag::Int(_) => TAG_INT,
            NbtTag::Long(_) => TAG_LONG,
            NbtTag::Float(_) => TAG_FLOAT,
            NbtTag::Double(_) => TAG_DOUBLE,
            NbtTag::ByteArray(_) => TAG_BYTE_ARRAY,
            NbtTag::String(_) => TAG_STRING,
            NbtTag::List(_) => TAG_LIST,
            NbtTag::Compound(_) => TAG_COMPOUND,
            NbtTag::IntArray(_) => TAG_INT_ARRAY,
        }
    }

    /// Read a named root NBT tag (usually a TAG_Compound) from reader.
    pub fn read<R: Read>(reader: &mut R) -> io::Result<(String, Self)> {
        let tag_id = reader.read_u8()?;
        if tag_id == TAG_END {
            return Ok((String::new(), NbtTag::End));
        }

        let name_len = reader.read_u16::<BigEndian>()? as usize;
        let mut name_bytes = vec![0u8; name_len];
        reader.read_exact(&mut name_bytes)?;
        let name = String::from_utf8(name_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let tag = Self::read_payload(reader, tag_id)?;
        Ok((name, tag))
    }

    fn read_payload<R: Read>(reader: &mut R, tag_id: u8) -> io::Result<Self> {
        match tag_id {
            TAG_END => Ok(NbtTag::End),
            TAG_BYTE => Ok(NbtTag::Byte(reader.read_i8()?)),
            TAG_SHORT => Ok(NbtTag::Short(reader.read_i16::<BigEndian>()?)),
            TAG_INT => Ok(NbtTag::Int(reader.read_i32::<BigEndian>()?)),
            TAG_LONG => Ok(NbtTag::Long(reader.read_i64::<BigEndian>()?)),
            TAG_FLOAT => Ok(NbtTag::Float(reader.read_f32::<BigEndian>()?)),
            TAG_DOUBLE => Ok(NbtTag::Double(reader.read_f64::<BigEndian>()?)),
            TAG_BYTE_ARRAY => {
                let len = reader.read_i32::<BigEndian>()?;
                if len < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Negative byte array length",
                    ));
                }
                let mut data = vec![0u8; len as usize];
                reader.read_exact(&mut data)?;
                Ok(NbtTag::ByteArray(data))
            }
            TAG_STRING => {
                let len = reader.read_u16::<BigEndian>()? as usize;
                let mut str_bytes = vec![0u8; len];
                reader.read_exact(&mut str_bytes)?;
                let s = String::from_utf8(str_bytes)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(NbtTag::String(s))
            }
            TAG_LIST => {
                let elem_type = reader.read_u8()?;
                let len = reader.read_i32::<BigEndian>()?;
                if len < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Negative list length",
                    ));
                }
                let mut list = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    list.push(Self::read_payload(reader, elem_type)?);
                }
                Ok(NbtTag::List(list))
            }
            TAG_COMPOUND => {
                let mut map = HashMap::new();
                loop {
                    let child_id = reader.read_u8()?;
                    if child_id == TAG_END {
                        break;
                    }
                    let key_len = reader.read_u16::<BigEndian>()? as usize;
                    let mut key_bytes = vec![0u8; key_len];
                    reader.read_exact(&mut key_bytes)?;
                    let key = String::from_utf8(key_bytes)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                    let child_tag = Self::read_payload(reader, child_id)?;
                    map.insert(key, child_tag);
                }
                Ok(NbtTag::Compound(map))
            }
            TAG_INT_ARRAY => {
                let len = reader.read_i32::<BigEndian>()?;
                if len < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Negative int array length",
                    ));
                }
                let mut array = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    array.push(reader.read_i32::<BigEndian>()?);
                }
                Ok(NbtTag::IntArray(array))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown NBT tag type: {}", tag_id),
            )),
        }
    }

    /// Write named NBT tag to writer.
    pub fn write<W: Write>(&self, writer: &mut W, name: &str) -> io::Result<()> {
        let tag_id = self.type_id();
        writer.write_u8(tag_id)?;
        if tag_id == TAG_END {
            return Ok(());
        }

        let name_bytes = name.as_bytes();
        writer.write_u16::<BigEndian>(name_bytes.len() as u16)?;
        writer.write_all(name_bytes)?;

        self.write_payload(writer)?;
        Ok(())
    }

    fn write_payload<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            NbtTag::End => Ok(()),
            NbtTag::Byte(v) => writer.write_i8(*v),
            NbtTag::Short(v) => writer.write_i16::<BigEndian>(*v),
            NbtTag::Int(v) => writer.write_i32::<BigEndian>(*v),
            NbtTag::Long(v) => writer.write_i64::<BigEndian>(*v),
            NbtTag::Float(v) => writer.write_f32::<BigEndian>(*v),
            NbtTag::Double(v) => writer.write_f64::<BigEndian>(*v),
            NbtTag::ByteArray(arr) => {
                writer.write_i32::<BigEndian>(arr.len() as i32)?;
                writer.write_all(arr)
            }
            NbtTag::String(s) => {
                let bytes = s.as_bytes();
                writer.write_u16::<BigEndian>(bytes.len() as u16)?;
                writer.write_all(bytes)
            }
            NbtTag::List(list) => {
                let elem_type = if list.is_empty() {
                    TAG_END
                } else {
                    list[0].type_id()
                };
                writer.write_u8(elem_type)?;
                writer.write_i32::<BigEndian>(list.len() as i32)?;
                for item in list {
                    item.write_payload(writer)?;
                }
                Ok(())
            }
            NbtTag::Compound(map) => {
                for (key, tag) in map {
                    tag.write(writer, key)?;
                }
                writer.write_u8(TAG_END)?;
                Ok(())
            }
            NbtTag::IntArray(arr) => {
                writer.write_i32::<BigEndian>(arr.len() as i32)?;
                for &v in arr {
                    writer.write_i32::<BigEndian>(v)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nbt_roundtrip() {
        let mut map = HashMap::new();
        map.insert("ByteVal".to_string(), NbtTag::Byte(42));
        map.insert("ShortVal".to_string(), NbtTag::Short(1337));
        map.insert("IntVal".to_string(), NbtTag::Int(12345678));
        map.insert("LongVal".to_string(), NbtTag::Long(98765432101234));
        map.insert("FloatVal".to_string(), NbtTag::Float(3.125));
        map.insert("DoubleVal".to_string(), NbtTag::Double(123.4567890123));
        map.insert(
            "StringVal".to_string(),
            NbtTag::String("Hello Minecraft 1.0".to_string()),
        );
        map.insert(
            "ByteArrayVal".to_string(),
            NbtTag::ByteArray(vec![1, 2, 3, 4, 5, 255]),
        );

        let mut child_map = HashMap::new();
        child_map.insert(
            "NestedKey".to_string(),
            NbtTag::String("NestedValue".to_string()),
        );
        map.insert("CompoundVal".to_string(), NbtTag::Compound(child_map));

        let root = NbtTag::Compound(map);

        let mut buf = Vec::new();
        root.write(&mut buf, "Root").unwrap();

        let mut cursor = io::Cursor::new(buf);
        let (name, loaded) = NbtTag::read(&mut cursor).unwrap();

        assert_eq!(name, "Root");
        assert_eq!(root, loaded);
    }
}
