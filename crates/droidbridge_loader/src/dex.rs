#[derive(Debug)]
pub enum DexError {
    InvalidMagic,
    UnexpectedEof,
    InvalidHeaderSize(u32),
    CorruptedData(&'static str),
}

impl std::fmt::Display for DexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DexError::InvalidMagic => write!(f, "Invalid DEX magic signature"),
            DexError::UnexpectedEof => write!(f, "Unexpected end of file while reading DEX"),
            DexError::InvalidHeaderSize(s) => write!(f, "Invalid DEX header size: {}", s),
            DexError::CorruptedData(msg) => write!(f, "Corrupted DEX data: {}", msg),
        }
    }
}

impl std::error::Error for DexError {}

#[derive(Debug, Clone)]
pub struct DexHeader {
    pub magic: [u8; 8],
    pub checksum: u32,
    pub signature: [u8; 20],
    pub file_size: u32,
    pub header_size: u32,
    pub endian_tag: u32,
    pub link_size: u32,
    pub link_off: u32,
    pub map_off: u32,
    pub string_ids_size: u32,
    pub string_ids_off: u32,
    pub type_ids_size: u32,
    pub type_ids_off: u32,
    pub proto_ids_size: u32,
    pub proto_ids_off: u32,
    pub field_ids_size: u32,
    pub field_ids_off: u32,
    pub method_ids_size: u32,
    pub method_ids_off: u32,
    pub class_defs_size: u32,
    pub class_defs_off: u32,
    pub data_size: u32,
    pub data_off: u32,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub class_name: String,
    pub method_name: String,
    pub signature: String,
    pub access_flags: u32,
    pub code_item: Option<CodeItem>,
}

#[derive(Debug, Clone)]
pub struct CodeItem {
    pub registers_size: u16,
    pub ins_size: u16,
    pub outs_size: u16,
    pub insns: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub class_name: String,
    pub superclass_name: Option<String>,
    pub access_flags: u32,
    pub methods: Vec<MethodInfo>,
}

pub struct DexFile<'a> {
    pub data: &'a [u8],
    pub header: DexHeader,
    pub strings: Vec<String>,
    pub types: Vec<String>,
    pub methods: Vec<MethodInfo>,
    pub classes: Vec<ClassInfo>,
}

impl<'a> DexFile<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, DexError> {
        if data.len() < 112 {
            return Err(DexError::UnexpectedEof);
        }

        // Verify magic: "dex\n035\0" .. "dex\n039\0"
        if &data[0..4] != b"dex\n" || data[7] != 0 {
            return Err(DexError::InvalidMagic);
        }

        let header = Self::parse_header(data)?;
        let strings = Self::parse_strings(data, &header)?;
        let types = Self::parse_types(data, &header, &strings)?;
        let protos = Self::parse_protos(data, &header, &strings, &types)?;
        let methods_meta = Self::parse_methods_meta(data, &header, &strings, &types, &protos)?;
        let classes = Self::parse_classes(data, &header, &types, &methods_meta)?;

        Ok(Self {
            data,
            header,
            strings,
            types,
            methods: methods_meta,
            classes,
        })
    }

    fn read_u16(data: &[u8], offset: usize) -> Result<u16, DexError> {
        if offset + 2 > data.len() {
            return Err(DexError::UnexpectedEof);
        }
        Ok(u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()))
    }

    fn read_u32(data: &[u8], offset: usize) -> Result<u32, DexError> {
        if offset + 4 > data.len() {
            return Err(DexError::UnexpectedEof);
        }
        Ok(u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()))
    }

    pub fn read_uleb128(data: &[u8], offset: &mut usize) -> Result<u32, DexError> {
        let mut result = 0u32;
        let mut shift = 0;
        loop {
            if *offset >= data.len() {
                return Err(DexError::UnexpectedEof);
            }
            let byte = data[*offset];
            *offset += 1;
            result |= ((byte & 0x7f) as u32) << shift;
            if (byte & 0x80) == 0 {
                break;
            }
            shift += 7;
            if shift >= 35 {
                return Err(DexError::CorruptedData("ULEB128 overflow"));
            }
        }
        Ok(result)
    }

    fn parse_header(data: &[u8]) -> Result<DexHeader, DexError> {
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&data[0..8]);

        let checksum = Self::read_u32(data, 8)?;
        let mut signature = [0u8; 20];
        signature.copy_from_slice(&data[12..32]);

        let file_size = Self::read_u32(data, 32)?;
        let header_size = Self::read_u32(data, 36)?;
        if header_size != 0x70 {
            return Err(DexError::InvalidHeaderSize(header_size));
        }

        let endian_tag = Self::read_u32(data, 40)?;
        let link_size = Self::read_u32(data, 44)?;
        let link_off = Self::read_u32(data, 48)?;
        let map_off = Self::read_u32(data, 52)?;
        let string_ids_size = Self::read_u32(data, 56)?;
        let string_ids_off = Self::read_u32(data, 60)?;
        let type_ids_size = Self::read_u32(data, 64)?;
        let type_ids_off = Self::read_u32(data, 68)?;
        let proto_ids_size = Self::read_u32(data, 72)?;
        let proto_ids_off = Self::read_u32(data, 76)?;
        let field_ids_size = Self::read_u32(data, 80)?;
        let field_ids_off = Self::read_u32(data, 84)?;
        let method_ids_size = Self::read_u32(data, 88)?;
        let method_ids_off = Self::read_u32(data, 92)?;
        let class_defs_size = Self::read_u32(data, 96)?;
        let class_defs_off = Self::read_u32(data, 100)?;
        let data_size = Self::read_u32(data, 104)?;
        let data_off = Self::read_u32(data, 108)?;

        Ok(DexHeader {
            magic,
            checksum,
            signature,
            file_size,
            header_size,
            endian_tag,
            link_size,
            link_off,
            map_off,
            string_ids_size,
            string_ids_off,
            type_ids_size,
            type_ids_off,
            proto_ids_size,
            proto_ids_off,
            field_ids_size,
            field_ids_off,
            method_ids_size,
            method_ids_off,
            class_defs_size,
            class_defs_off,
            data_size,
            data_off,
        })
    }

    fn parse_strings(data: &[u8], header: &DexHeader) -> Result<Vec<String>, DexError> {
        let mut strings = Vec::with_capacity(header.string_ids_size as usize);
        for i in 0..header.string_ids_size as usize {
            let string_data_off = Self::read_u32(data, header.string_ids_off as usize + i * 4)? as usize;
            let mut offset = string_data_off;
            let _utf16_size = Self::read_uleb128(data, &mut offset)?;

            let start = offset;
            while offset < data.len() && data[offset] != 0 {
                offset += 1;
            }
            let s = String::from_utf8_lossy(&data[start..offset]).to_string();
            strings.push(s);
        }
        Ok(strings)
    }

    fn parse_types(data: &[u8], header: &DexHeader, strings: &[String]) -> Result<Vec<String>, DexError> {
        let mut types = Vec::with_capacity(header.type_ids_size as usize);
        for i in 0..header.type_ids_size as usize {
            let descriptor_idx = Self::read_u32(data, header.type_ids_off as usize + i * 4)? as usize;
            let t = strings.get(descriptor_idx).cloned().unwrap_or_else(|| "Unknown".to_string());
            types.push(t);
        }
        Ok(types)
    }

    fn parse_protos(
        data: &[u8],
        header: &DexHeader,
        strings: &[String],
        types: &[String],
    ) -> Result<Vec<String>, DexError> {
        let mut protos = Vec::with_capacity(header.proto_ids_size as usize);
        for i in 0..header.proto_ids_size as usize {
            let off = header.proto_ids_off as usize + i * 12;
            let shorty_idx = Self::read_u32(data, off)? as usize;
            let return_type_idx = Self::read_u32(data, off + 4)? as usize;
            let shorty = strings.get(shorty_idx).map(|s| s.as_str()).unwrap_or("V");
            let ret = types.get(return_type_idx).map(|s| s.as_str()).unwrap_or("V");
            protos.push(format!("{} -> {}", shorty, ret));
        }
        Ok(protos)
    }

    fn parse_methods_meta(
        data: &[u8],
        header: &DexHeader,
        strings: &[String],
        types: &[String],
        protos: &[String],
    ) -> Result<Vec<MethodInfo>, DexError> {
        let mut methods = Vec::with_capacity(header.method_ids_size as usize);
        for i in 0..header.method_ids_size as usize {
            let off = header.method_ids_off as usize + i * 8;
            let class_idx = Self::read_u16(data, off)? as usize;
            let proto_idx = Self::read_u16(data, off + 2)? as usize;
            let name_idx = Self::read_u32(data, off + 4)? as usize;

            let class_name = types.get(class_idx).cloned().unwrap_or_default();
            let proto_desc = protos.get(proto_idx).cloned().unwrap_or_default();
            let method_name = strings.get(name_idx).cloned().unwrap_or_default();

            methods.push(MethodInfo {
                class_name,
                method_name,
                signature: proto_desc,
                access_flags: 0,
                code_item: None,
            });
        }
        Ok(methods)
    }

    fn parse_classes(
        data: &[u8],
        header: &DexHeader,
        types: &[String],
        methods_meta: &[MethodInfo],
    ) -> Result<Vec<ClassInfo>, DexError> {
        let mut classes = Vec::with_capacity(header.class_defs_size as usize);
        for i in 0..header.class_defs_size as usize {
            let off = header.class_defs_off as usize + i * 32;
            let class_idx = Self::read_u32(data, off)? as usize;
            let access_flags = Self::read_u32(data, off + 4)?;
            let superclass_idx = Self::read_u32(data, off + 8)? as i32;
            let class_data_off = Self::read_u32(data, off + 24)? as usize;

            let class_name = types.get(class_idx).cloned().unwrap_or_default();
            let superclass_name = if superclass_idx >= 0 {
                types.get(superclass_idx as usize).cloned()
            } else {
                None
            };

            let mut class_methods = Vec::new();

            if class_data_off > 0 && class_data_off < data.len() {
                let mut c_off = class_data_off;
                let static_fields_size = Self::read_uleb128(data, &mut c_off)?;
                let instance_fields_size = Self::read_uleb128(data, &mut c_off)?;
                let direct_methods_size = Self::read_uleb128(data, &mut c_off)?;
                let virtual_methods_size = Self::read_uleb128(data, &mut c_off)?;

                // Skip static fields
                for _ in 0..static_fields_size {
                    let _ = Self::read_uleb128(data, &mut c_off)?;
                    let _ = Self::read_uleb128(data, &mut c_off)?;
                }
                // Skip instance fields
                for _ in 0..instance_fields_size {
                    let _ = Self::read_uleb128(data, &mut c_off)?;
                    let _ = Self::read_uleb128(data, &mut c_off)?;
                }

                // Parse direct & virtual methods
                let mut method_idx = 0usize;
                for _ in 0..(direct_methods_size + virtual_methods_size) {
                    let method_idx_diff = Self::read_uleb128(data, &mut c_off)? as usize;
                    method_idx += method_idx_diff;
                    let m_flags = Self::read_uleb128(data, &mut c_off)?;
                    let code_off = Self::read_uleb128(data, &mut c_off)? as usize;

                    let mut code_item = None;
                    if code_off > 0 && code_off + 16 <= data.len() {
                        let registers_size = Self::read_u16(data, code_off)?;
                        let ins_size = Self::read_u16(data, code_off + 2)?;
                        let outs_size = Self::read_u16(data, code_off + 4)?;
                        let insns_size = Self::read_u32(data, code_off + 12)? as usize;

                        let insns_start = code_off + 16;
                        if insns_start + insns_size * 2 <= data.len() {
                            let mut insns = Vec::with_capacity(insns_size);
                            for ins_i in 0..insns_size {
                                insns.push(Self::read_u16(data, insns_start + ins_i * 2)?);
                            }
                            code_item = Some(CodeItem {
                                registers_size,
                                ins_size,
                                outs_size,
                                insns,
                            });
                        }
                    }

                    if let Some(meta) = methods_meta.get(method_idx) {
                        let mut m = meta.clone();
                        m.access_flags = m_flags;
                        m.code_item = code_item;
                        class_methods.push(m);
                    }
                }
            }

            classes.push(ClassInfo {
                class_name,
                superclass_name,
                access_flags,
                methods: class_methods,
            });
        }

        Ok(classes)
    }
}
