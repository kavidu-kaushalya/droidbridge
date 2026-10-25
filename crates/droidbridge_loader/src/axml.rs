use std::collections::HashMap;

#[derive(Debug)]
pub enum AxmlError {
    UnexpectedEof,
    InvalidChunkType(u16),
    InvalidHeaderSize,
    StringPoolCorrupted,
    MalformedElement,
}

impl std::fmt::Display for AxmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AxmlError::UnexpectedEof => write!(f, "Unexpected EOF while reading Android Binary XML"),
            AxmlError::InvalidChunkType(t) => write!(f, "Invalid chunk type: 0x{:04x}", t),
            AxmlError::InvalidHeaderSize => write!(f, "Invalid chunk header size"),
            AxmlError::StringPoolCorrupted => write!(f, "Corrupted string pool in AXML"),
            AxmlError::MalformedElement => write!(f, "Malformed XML element chunk"),
        }
    }
}

impl std::error::Error for AxmlError {}

#[derive(Debug, Clone, PartialEq)]
pub enum AxmlAttributeValue {
    String(String),
    Reference(u32),
    Color(u32),
    Boolean(bool),
    Dimension(f32, String),
    Integer(i32),
    Raw { data_type: u8, data: u32 },
}

#[derive(Debug, Clone, Default)]
pub struct AxmlNode {
    pub tag: String,
    pub attributes: HashMap<String, AxmlAttributeValue>,
    pub children: Vec<AxmlNode>,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityManifestInfo {
    pub name: String,
    pub label: Option<String>,
    pub exported: bool,
    pub is_launcher: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AndroidManifest {
    pub package_name: String,
    pub version_code: Option<u32>,
    pub version_name: Option<String>,
    pub min_sdk_version: Option<u32>,
    pub target_sdk_version: Option<u32>,
    pub permissions: Vec<String>,
    pub activities: Vec<ActivityManifestInfo>,
    pub application_label: Option<String>,
}

pub struct AxmlDecoder<'a> {
    data: &'a [u8],
    strings: Vec<String>,
}

impl<'a> AxmlDecoder<'a> {
    const RES_XML_TYPE: u16 = 0x0003;
    const RES_STRING_POOL_TYPE: u16 = 0x0001;
    const RES_XML_RESOURCE_MAP_TYPE: u16 = 0x0180;
    const RES_XML_START_NAMESPACE_TYPE: u16 = 0x0100;
    const RES_XML_END_NAMESPACE_TYPE: u16 = 0x0101;
    const RES_XML_START_ELEMENT_TYPE: u16 = 0x0102;
    const RES_XML_END_ELEMENT_TYPE: u16 = 0x0103;
    const RES_XML_CDATA_TYPE: u16 = 0x0104;

    pub fn decode(data: &'a [u8]) -> Result<AndroidManifest, AxmlError> {
        let mut decoder = Self {
            data,
            strings: Vec::new(),
        };
        decoder.parse()
    }

    fn read_u16(slice: &[u8], offset: usize) -> Result<u16, AxmlError> {
        if offset + 2 > slice.len() {
            return Err(AxmlError::UnexpectedEof);
        }
        Ok(u16::from_le_bytes(slice[offset..offset + 2].try_into().unwrap()))
    }

    fn read_u32(slice: &[u8], offset: usize) -> Result<u32, AxmlError> {
        if offset + 4 > slice.len() {
            return Err(AxmlError::UnexpectedEof);
        }
        Ok(u32::from_le_bytes(slice[offset..offset + 4].try_into().unwrap()))
    }

    fn parse(&mut self) -> Result<AndroidManifest, AxmlError> {
        if self.data.len() < 8 {
            return Err(AxmlError::UnexpectedEof);
        }

        let root_type = Self::read_u16(self.data, 0)?;
        if root_type != Self::RES_XML_TYPE {
            return Err(AxmlError::InvalidChunkType(root_type));
        }

        let mut offset = 8;
        let mut manifest = AndroidManifest::default();
        let mut current_tag_stack: Vec<String> = Vec::new();
        let mut current_activity: Option<ActivityManifestInfo> = None;
        let mut inside_launcher_intent_filter = false;
        let mut has_main_action = false;
        let mut has_launcher_category = false;

        while offset + 8 <= self.data.len() {
            let chunk_type = Self::read_u16(self.data, offset)?;
            let _header_size = Self::read_u16(self.data, offset + 2)? as usize;
            let chunk_size = Self::read_u32(self.data, offset + 4)? as usize;

            if chunk_size < 8 || offset + chunk_size > self.data.len() {
                break;
            }

            match chunk_type {
                Self::RES_STRING_POOL_TYPE => {
                    self.parse_string_pool(&self.data[offset..offset + chunk_size])?;
                }
                Self::RES_XML_RESOURCE_MAP_TYPE => {
                    // Resource ID mapping chunk (skip for basic manifest parsing)
                }
                Self::RES_XML_START_NAMESPACE_TYPE | Self::RES_XML_END_NAMESPACE_TYPE => {
                    // Namespace declaration events
                }
                Self::RES_XML_START_ELEMENT_TYPE => {
                    let element_slice = &self.data[offset..offset + chunk_size];
                    let name_idx = Self::read_u32(element_slice, 20)? as usize;
                    let tag_name = self.get_string(name_idx).unwrap_or("unknown").to_string();

                    let attr_count = Self::read_u16(element_slice, 28)? as usize;
                    let attr_start = 36; // 36 is the standard byte offset for attributes in ResXMLTree_attrExt

                    let mut attributes = HashMap::new();
                    for i in 0..attr_count {
                        let a_off = attr_start + i * 20;
                        if a_off + 20 <= element_slice.len() {
                            let a_name_idx = Self::read_u32(element_slice, a_off + 4)? as usize;
                            let a_raw_idx = Self::read_u32(element_slice, a_off + 8)? as i32;
                            let a_type = element_slice[a_off + 15];
                            let a_data = Self::read_u32(element_slice, a_off + 16)?;

                            let attr_name = self.get_string(a_name_idx).unwrap_or("").to_string();
                            let attr_val = if a_raw_idx >= 0 {
                                self.get_string(a_raw_idx as usize).unwrap_or("").to_string()
                            } else {
                                match a_type {
                                    0x10 | 0x11 => a_data.to_string(), // Int
                                    0x12 => if a_data != 0 { "true" } else { "false" }.to_string(), // Boolean
                                    _ => a_data.to_string(),
                                }
                            };
                            attributes.insert(attr_name, attr_val);
                        }
                    }

                    match tag_name.as_str() {
                        "manifest" => {
                            if let Some(pkg) = attributes.get("package") {
                                manifest.package_name = pkg.clone();
                            }
                            if let Some(vc) = attributes.get("versionCode") {
                                manifest.version_code = vc.parse().ok();
                            }
                            if let Some(vn) = attributes.get("versionName") {
                                manifest.version_name = Some(vn.clone());
                            }
                        }
                        "uses-sdk" => {
                            if let Some(min) = attributes.get("minSdkVersion") {
                                manifest.min_sdk_version = min.parse().ok();
                            }
                            if let Some(target) = attributes.get("targetSdkVersion") {
                                manifest.target_sdk_version = target.parse().ok();
                            }
                        }
                        "uses-permission" => {
                            if let Some(perm) = attributes.get("name") {
                                manifest.permissions.push(perm.clone());
                            }
                        }
                        "application" => {
                            if let Some(lbl) = attributes.get("label") {
                                manifest.application_label = Some(lbl.clone());
                            }
                        }
                        "activity" => {
                            let act_name = attributes.get("name").cloned().unwrap_or_default();
                            let act_label = attributes.get("label").cloned();
                            let exported = attributes.get("exported").map(|v| v == "true").unwrap_or(false);

                            current_activity = Some(ActivityManifestInfo {
                                name: act_name,
                                label: act_label,
                                exported,
                                is_launcher: false,
                            });
                            inside_launcher_intent_filter = false;
                            has_main_action = false;
                            has_launcher_category = false;
                        }
                        "intent-filter" => {
                            inside_launcher_intent_filter = true;
                            has_main_action = false;
                            has_launcher_category = false;
                        }
                        "action" => {
                            if inside_launcher_intent_filter {
                                if let Some(n) = attributes.get("name") {
                                    if n == "android.intent.action.MAIN" {
                                        has_main_action = true;
                                    }
                                }
                            }
                        }
                        "category" => {
                            if inside_launcher_intent_filter {
                                if let Some(n) = attributes.get("name") {
                                    if n == "android.intent.category.LAUNCHER" {
                                        has_launcher_category = true;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }

                    current_tag_stack.push(tag_name);
                }
                Self::RES_XML_END_ELEMENT_TYPE => {
                    if let Some(tag) = current_tag_stack.pop() {
                        if tag == "intent-filter" {
                            if has_main_action && has_launcher_category {
                                if let Some(act) = &mut current_activity {
                                    act.is_launcher = true;
                                }
                            }
                            inside_launcher_intent_filter = false;
                        } else if tag == "activity" {
                            if let Some(act) = current_activity.take() {
                                manifest.activities.push(act);
                            }
                        }
                    }
                }
                Self::RES_XML_CDATA_TYPE => {}
                _ => {}
            }

            offset += chunk_size;
        }

        Ok(manifest)
    }

    pub fn decode_tree(data: &'a [u8]) -> Result<AxmlNode, AxmlError> {
        let mut decoder = Self {
            data,
            strings: Vec::new(),
        };
        decoder.parse_tree()
    }

    fn parse_tree(&mut self) -> Result<AxmlNode, AxmlError> {
        if self.data.len() < 8 {
            return Err(AxmlError::UnexpectedEof);
        }

        let root_type = Self::read_u16(self.data, 0)?;
        if root_type != Self::RES_XML_TYPE {
            return Err(AxmlError::InvalidChunkType(root_type));
        }

        let mut offset = 8;
        let mut node_stack: Vec<AxmlNode> = Vec::new();
        let mut root_node: Option<AxmlNode> = None;

        while offset + 8 <= self.data.len() {
            let chunk_type = Self::read_u16(self.data, offset)?;
            let chunk_size = Self::read_u32(self.data, offset + 4)? as usize;

            if chunk_size < 8 || offset + chunk_size > self.data.len() {
                break;
            }

            match chunk_type {
                Self::RES_STRING_POOL_TYPE => {
                    self.parse_string_pool(&self.data[offset..offset + chunk_size])?;
                }
                Self::RES_XML_START_ELEMENT_TYPE => {
                    let element_slice = &self.data[offset..offset + chunk_size];
                    let name_idx = Self::read_u32(element_slice, 20)? as usize;
                    let tag_name = self.get_string(name_idx).unwrap_or("unknown").to_string();

                    let attr_count = Self::read_u16(element_slice, 28)? as usize;
                    let attr_start = 36;

                    let mut attributes = HashMap::new();
                    for i in 0..attr_count {
                        let a_off = attr_start + i * 20;
                        if a_off + 20 <= element_slice.len() {
                            let a_name_idx = Self::read_u32(element_slice, a_off + 4)? as usize;
                            let a_raw_idx = Self::read_u32(element_slice, a_off + 8)? as i32;
                            let a_type = element_slice[a_off + 15];
                            let a_data = Self::read_u32(element_slice, a_off + 16)?;

                            let attr_name = self.get_string(a_name_idx).unwrap_or("").to_string();
                            let attr_val = if a_raw_idx >= 0 {
                                AxmlAttributeValue::String(self.get_string(a_raw_idx as usize).unwrap_or("").to_string())
                            } else {
                                match a_type {
                                    0x01 => AxmlAttributeValue::Reference(a_data),
                                    0x03 => {
                                        let s = self.get_string(a_data as usize).unwrap_or("").to_string();
                                        AxmlAttributeValue::String(s)
                                    }
                                    0x10 | 0x11 => AxmlAttributeValue::Integer(a_data as i32),
                                    0x12 => AxmlAttributeValue::Boolean(a_data != 0),
                                    0x1c..=0x1f => AxmlAttributeValue::Color(a_data),
                                    0x05 => {
                                        let unit_idx = (a_data & 0x0f) as usize;
                                        let units = ["px", "dp", "sp", "pt", "in", "mm"];
                                        let unit = if unit_idx < units.len() { units[unit_idx] } else { "px" };
                                        let val = ((a_data >> 8) as f32) / 256.0;
                                        AxmlAttributeValue::Dimension(val, unit.to_string())
                                    }
                                    _ => AxmlAttributeValue::Raw { data_type: a_type, data: a_data },
                                }
                            };
                            attributes.insert(attr_name, attr_val);
                        }
                    }

                    let node = AxmlNode {
                        tag: tag_name,
                        attributes,
                        children: Vec::new(),
                    };
                    node_stack.push(node);
                }
                Self::RES_XML_END_ELEMENT_TYPE => {
                    if let Some(finished_node) = node_stack.pop() {
                        if let Some(parent) = node_stack.last_mut() {
                            parent.children.push(finished_node);
                        } else {
                            root_node = Some(finished_node);
                        }
                    }
                }
                _ => {}
            }

            offset += chunk_size;
        }

        root_node.ok_or(AxmlError::MalformedElement)
    }

    fn parse_string_pool(&mut self, chunk: &[u8]) -> Result<(), AxmlError> {
        if chunk.len() < 28 {
            return Err(AxmlError::StringPoolCorrupted);
        }

        let string_count = Self::read_u32(chunk, 8)? as usize;
        let flags = Self::read_u32(chunk, 16)?;
        let is_utf8 = (flags & 0x00000100) != 0;
        let strings_start = Self::read_u32(chunk, 20)? as usize;

        let offsets_start = 28;
        let mut offsets = Vec::with_capacity(string_count);
        for i in 0..string_count {
            let off = Self::read_u32(chunk, offsets_start + i * 4)? as usize;
            offsets.push(off);
        }

        let string_data = &chunk[strings_start..];
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

        self.strings = strings;
        Ok(())
    }

    fn decode_utf8_string(slice: &[u8]) -> String {
        let mut p = 0;
        // Skip UTF-8 length markers
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
        if p <= end && end <= slice.len() {
            String::from_utf8_lossy(&slice[p..end]).to_string()
        } else {
            String::new()
        }
    }

    fn decode_utf16_string(slice: &[u8]) -> String {
        let mut p = 0;
        if slice.len() < 2 {
            return String::new();
        }
        let len = u16::from_le_bytes(slice[0..2].try_into().unwrap()) as usize;
        p += 2;
        if (len & 0x8000) != 0 {
            if slice.len() >= 4 {
                let high = u16::from_le_bytes(slice[2..4].try_into().unwrap()) as usize;
                let _full_len = ((len & 0x7fff) << 16) | high;
                p += 2;
            }
        }

        let mut u16_chars = Vec::new();
        while p + 2 <= slice.len() {
            let c = u16::from_le_bytes(slice[p..p + 2].try_into().unwrap());
            if c == 0 {
                break;
            }
            u16_chars.push(c);
            p += 2;
        }

        String::from_utf16_lossy(&u16_chars)
    }

    pub fn get_string(&self, index: usize) -> Option<&str> {
        self.strings.get(index).map(|s| s.as_str())
    }
}
