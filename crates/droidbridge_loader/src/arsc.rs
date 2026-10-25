use std::collections::HashMap;

#[derive(Debug)]
pub enum ArscError {
    UnexpectedEof,
    InvalidChunkType(u16),
    InvalidHeaderSize,
    StringPoolCorrupted,
    MalformedPackageChunk,
}

impl std::fmt::Display for ArscError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArscError::UnexpectedEof => write!(f, "Unexpected EOF while parsing resources.arsc"),
            ArscError::InvalidChunkType(t) => write!(f, "Invalid ARSC chunk type: 0x{:04x}", t),
            ArscError::InvalidHeaderSize => write!(f, "Invalid ARSC chunk header size"),
            ArscError::StringPoolCorrupted => write!(f, "Corrupted string pool in resources.arsc"),
            ArscError::MalformedPackageChunk => write!(f, "Malformed package chunk in resources.arsc"),
        }
    }
}

impl std::error::Error for ArscError {}

#[derive(Debug, Clone, PartialEq)]
pub enum ResValue {
    String(String),
    Color(u32),
    Boolean(bool),
    Reference(u32),
    Dimension(f32, String),
    Integer(i32),
    Raw { data_type: u8, data: u32 },
}

#[derive(Debug, Clone)]
pub struct ResEntry {
    pub res_id: u32,
    pub type_name: String,
    pub key_name: String,
    pub value: ResValue,
}

#[derive(Debug, Clone, Default)]
pub struct ResPackage {
    pub id: u32,
    pub name: String,
    pub type_strings: Vec<String>,
    pub key_strings: Vec<String>,
    pub entries: HashMap<u32, ResEntry>,
    pub name_to_id: HashMap<String, u32>,
}

#[derive(Debug, Default)]
pub struct ResourceTable {
    pub global_strings: Vec<String>,
    pub packages: Vec<ResPackage>,
}

impl ResourceTable {
    const RES_STRING_POOL_TYPE: u16 = 0x0001;
    const RES_TABLE_TYPE: u16 = 0x0002;
    const RES_TABLE_PACKAGE_TYPE: u16 = 0x0200;
    const RES_TABLE_TYPE_TYPE: u16 = 0x0201;
    #[allow(dead_code)]
    const RES_TABLE_TYPE_SPEC_TYPE: u16 = 0x0202;

    pub fn parse(data: &[u8]) -> Result<Self, ArscError> {
        if data.len() < 12 {
            return Err(ArscError::UnexpectedEof);
        }

        let root_type = Self::read_u16(data, 0)?;
        if root_type != Self::RES_TABLE_TYPE {
            return Err(ArscError::InvalidChunkType(root_type));
        }

        let header_size = Self::read_u16(data, 2)? as usize;
        let chunk_size = Self::read_u32(data, 4)? as usize;
        let _package_count = Self::read_u32(data, 8)? as usize;

        if chunk_size > data.len() || header_size < 12 {
            return Err(ArscError::InvalidHeaderSize);
        }

        let mut table = ResourceTable::default();
        let mut offset = header_size;

        while offset + 8 <= data.len() {
            let chunk_type = Self::read_u16(data, offset)?;
            let _c_header_size = Self::read_u16(data, offset + 2)? as usize;
            let c_chunk_size = Self::read_u32(data, offset + 4)? as usize;

            if c_chunk_size < 8 || offset + c_chunk_size > data.len() {
                break;
            }

            let chunk_slice = &data[offset..offset + c_chunk_size];

            match chunk_type {
                Self::RES_STRING_POOL_TYPE => {
                    if table.global_strings.is_empty() {
                        table.global_strings = Self::parse_string_pool(chunk_slice)?;
                    }
                }
                Self::RES_TABLE_PACKAGE_TYPE => {
                    let pkg = Self::parse_package(chunk_slice, &table.global_strings)?;
                    table.packages.push(pkg);
                }
                _ => {}
            }

            offset += c_chunk_size;
        }

        Ok(table)
    }

    fn parse_package(data: &[u8], global_strings: &[String]) -> Result<ResPackage, ArscError> {
        if data.len() < 288 {
            return Err(ArscError::MalformedPackageChunk);
        }

        let pkg_id = Self::read_u32(data, 8)?;
        let name_slice = &data[12..12 + 256];
        let mut pkg_name_u16 = Vec::new();
        for i in 0..128 {
            let ch = u16::from_le_bytes(name_slice[i * 2..i * 2 + 2].try_into().unwrap());
            if ch == 0 {
                break;
            }
            pkg_name_u16.push(ch);
        }
        let pkg_name = String::from_utf16_lossy(&pkg_name_u16);

        let type_strings_offset = Self::read_u32(data, 268)? as usize;
        let key_strings_offset = Self::read_u32(data, 276)? as usize;

        let mut type_strings = Vec::new();
        if type_strings_offset > 0 && type_strings_offset < data.len() {
            let sub = &data[type_strings_offset..];
            if sub.len() >= 8 && Self::read_u16(sub, 0)? == Self::RES_STRING_POOL_TYPE {
                let pool_size = Self::read_u32(sub, 4)? as usize;
                if pool_size <= sub.len() {
                    type_strings = Self::parse_string_pool(&sub[..pool_size]).unwrap_or_default();
                }
            }
        }

        let mut key_strings = Vec::new();
        if key_strings_offset > 0 && key_strings_offset < data.len() {
            let sub = &data[key_strings_offset..];
            if sub.len() >= 8 && Self::read_u16(sub, 0)? == Self::RES_STRING_POOL_TYPE {
                let pool_size = Self::read_u32(sub, 4)? as usize;
                if pool_size <= sub.len() {
                    key_strings = Self::parse_string_pool(&sub[..pool_size]).unwrap_or_default();
                }
            }
        }

        let mut package = ResPackage {
            id: pkg_id,
            name: pkg_name,
            type_strings,
            key_strings,
            entries: HashMap::new(),
            name_to_id: HashMap::new(),
        };

        // Walk sub-chunks inside package
        let mut offset = 288;
        while offset + 8 <= data.len() {
            let chunk_type = Self::read_u16(data, offset)?;
            let c_header_size = Self::read_u16(data, offset + 2)? as usize;
            let c_chunk_size = Self::read_u32(data, offset + 4)? as usize;

            if c_chunk_size < 8 || offset + c_chunk_size > data.len() {
                break;
            }

            let sub = &data[offset..offset + c_chunk_size];

            if chunk_type == Self::RES_TABLE_TYPE_TYPE && sub.len() >= 24 {
                let type_id = sub[8] as u32; // 1-based index into type_strings
                let entry_count = Self::read_u32(sub, 12)? as usize;
                let entries_start = Self::read_u32(sub, 16)? as usize;

                let type_name = if type_id > 0 && (type_id as usize) <= package.type_strings.len() {
                    package.type_strings[(type_id - 1) as usize].clone()
                } else {
                    format!("type_{}", type_id)
                };

                // Inspect configuration: default/any or English vs localized overrides
                let is_default_or_en = if sub.len() >= 30 {
                    let lang = [sub[28], sub[29]];
                    lang == [0, 0] || &lang == b"en"
                } else {
                    true
                };

                let offsets_start = c_header_size;
                for i in 0..entry_count {
                    let off_pos = offsets_start + i * 4;
                    if off_pos + 4 > sub.len() {
                        break;
                    }
                    let entry_rel = Self::read_u32(sub, off_pos)?;
                    if entry_rel == 0xffffffff {
                        continue;
                    }

                    let entry_abs = entries_start + (entry_rel as usize);
                    if entry_abs + 8 > sub.len() {
                        continue;
                    }

                    let _entry_size = Self::read_u16(sub, entry_abs)?;
                    let flags = Self::read_u16(sub, entry_abs + 2)?;
                    let key_idx = Self::read_u32(sub, entry_abs + 4)? as usize;

                    let key_name = if key_idx < package.key_strings.len() {
                        package.key_strings[key_idx].clone()
                    } else {
                        format!("entry_{}", i)
                    };

                    let res_id = (pkg_id << 24) | (type_id << 16) | (i as u32);

                    // If not complex, value is at entry_abs + 8
                    if (flags & 0x0001) == 0 && entry_abs + 16 <= sub.len() {
                        let val_slice = &sub[entry_abs + 8..entry_abs + 16];
                        let data_type = val_slice[3];
                        let val_data = Self::read_u32(val_slice, 4)?;

                        let res_val = match data_type {
                            0x03 => {
                                let s = if (val_data as usize) < global_strings.len() {
                                    global_strings[val_data as usize].clone()
                                } else {
                                    String::new()
                                };
                                ResValue::String(s)
                            }
                            0x01 => ResValue::Reference(val_data),
                            0x12 => ResValue::Boolean(val_data != 0),
                            0x10 | 0x11 => ResValue::Integer(val_data as i32),
                            0x1c..=0x1f => ResValue::Color(val_data),
                            0x05 => {
                                let unit_idx = (val_data & 0x0f) as usize;
                                let units = ["px", "dp", "sp", "pt", "in", "mm"];
                                let unit = if unit_idx < units.len() { units[unit_idx] } else { "px" };
                                let val = ((val_data >> 8) as f32) / 256.0;
                                ResValue::Dimension(val, unit.to_string())
                            }
                            _ => ResValue::Raw { data_type, data: val_data },
                        };

                        let should_insert = match package.entries.get(&res_id) {
                            None => true,
                            Some(existing) => {
                                match (&existing.value, &res_val) {
                                    (ResValue::String(old_s), ResValue::String(new_s)) => {
                                        let old_is_ascii = !old_s.is_empty() && old_s.is_ascii();
                                        let new_is_ascii = !new_s.is_empty() && new_s.is_ascii();
                                        if is_default_or_en && !old_is_ascii {
                                            true
                                        } else if !is_default_or_en && old_is_ascii {
                                            false
                                        } else if new_is_ascii && !old_is_ascii {
                                            true
                                        } else if !new_is_ascii && old_is_ascii {
                                            false
                                        } else {
                                            is_default_or_en
                                        }
                                    }
                                    _ => is_default_or_en,
                                }
                            }
                        };

                        if should_insert {
                            let entry = ResEntry {
                                res_id,
                                type_name: type_name.clone(),
                                key_name: key_name.clone(),
                                value: res_val,
                            };

                            package.name_to_id.insert(format!("{}/{}", type_name, key_name), res_id);
                            package.entries.insert(res_id, entry);
                        }
                    }
                }
            }

            offset += c_chunk_size;
        }

        Ok(package)
    }

    pub fn resolve_string(&self, res_id: u32) -> Option<&str> {
        let pkg_id = (res_id >> 24) & 0xff;
        for pkg in &self.packages {
            if pkg.id == pkg_id {
                if let Some(entry) = pkg.entries.get(&res_id) {
                    if let ResValue::String(ref s) = entry.value {
                        return Some(s.as_str());
                    }
                }
            }
        }
        None
    }

    pub fn resolve_entry(&self, res_id: u32) -> Option<&ResEntry> {
        let pkg_id = (res_id >> 24) & 0xff;
        for pkg in &self.packages {
            if pkg.id == pkg_id {
                return pkg.entries.get(&res_id);
            }
        }
        None
    }

    pub fn find_by_name(&self, type_name: &str, key_name: &str) -> Option<&ResEntry> {
        let query = format!("{}/{}", type_name, key_name);
        for pkg in &self.packages {
            if let Some(&res_id) = pkg.name_to_id.get(&query) {
                return pkg.entries.get(&res_id);
            }
        }
        None
    }

    pub fn resolve_resource_path(&self, res_id: u32) -> Option<&str> {
        if let Some(entry) = self.resolve_entry(res_id) {
            if entry.type_name == "layout" || entry.type_name == "drawable" {
                if let ResValue::String(ref path) = entry.value {
                    return Some(path.as_str());
                }
            }
        }
        None
    }

    fn read_u16(slice: &[u8], offset: usize) -> Result<u16, ArscError> {
        if offset + 2 > slice.len() {
            return Err(ArscError::UnexpectedEof);
        }
        Ok(u16::from_le_bytes(slice[offset..offset + 2].try_into().unwrap()))
    }

    fn read_u32(slice: &[u8], offset: usize) -> Result<u32, ArscError> {
        if offset + 4 > slice.len() {
            return Err(ArscError::UnexpectedEof);
        }
        Ok(u32::from_le_bytes(slice[offset..offset + 4].try_into().unwrap()))
    }

    fn parse_string_pool(chunk: &[u8]) -> Result<Vec<String>, ArscError> {
        if chunk.len() < 28 {
            return Err(ArscError::StringPoolCorrupted);
        }

        let string_count = Self::read_u32(chunk, 8)? as usize;
        let flags = Self::read_u32(chunk, 16)?;
        let is_utf8 = (flags & 0x00000100) != 0;
        let strings_start = Self::read_u32(chunk, 20)? as usize;

        let offsets_start = 28;
        let mut offsets = Vec::with_capacity(string_count);
        for i in 0..string_count {
            let pos = offsets_start + i * 4;
            if pos + 4 > chunk.len() {
                break;
            }
            offsets.push(Self::read_u32(chunk, pos)? as usize);
        }

        let string_data = if strings_start < chunk.len() {
            &chunk[strings_start..]
        } else {
            &[]
        };

        let mut strings = Vec::with_capacity(string_count);
        for off in offsets {
            if off >= string_data.len() {
                strings.push(String::new());
                continue;
            }

            let slice = &string_data[off..];
            let str_val = if is_utf8 {
                Self::decode_utf8_string(slice)
            } else {
                Self::decode_utf16_string(slice)
            };
            strings.push(str_val);
        }

        Ok(strings)
    }

    fn decode_utf8_string(slice: &[u8]) -> String {
        let mut p = 0;
        if p < slice.len() && (slice[p] & 0x80) != 0 {
            p += 2;
        } else {
            p += 1;
        }
        if p < slice.len() && (slice[p] & 0x80) != 0 {
            p += 2;
        } else {
            p += 1;
        }

        let mut end = p;
        while end < slice.len() && slice[end] != 0 {
            end += 1;
        }

        String::from_utf8_lossy(&slice[p..end]).to_string()
    }

    fn decode_utf16_string(slice: &[u8]) -> String {
        if slice.len() < 2 {
            return String::new();
        }

        let mut p = 0;
        let u16_len = u16::from_le_bytes(slice[p..p + 2].try_into().unwrap());
        p += 2;
        if (u16_len & 0x8000) != 0 && p + 2 <= slice.len() {
            p += 2;
        }

        let mut u16_chars = Vec::new();
        while p + 2 <= slice.len() {
            let ch = u16::from_le_bytes(slice[p..p + 2].try_into().unwrap());
            if ch == 0 {
                break;
            }
            u16_chars.push(ch);
            p += 2;
        }

        String::from_utf16_lossy(&u16_chars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arsc_empty_header() {
        let mut data = vec![0u8; 12];
        data[0..2].copy_from_slice(&0x0002u16.to_le_bytes()); // RES_TABLE_TYPE
        data[2..4].copy_from_slice(&12u16.to_le_bytes());     // header_size
        data[4..8].copy_from_slice(&12u32.to_le_bytes());     // size = 12
        data[8..12].copy_from_slice(&0u32.to_le_bytes());     // package_count = 0

        let table = ResourceTable::parse(&data).expect("Failed to parse minimal ARSC");
        assert_eq!(table.packages.len(), 0);
    }
}
