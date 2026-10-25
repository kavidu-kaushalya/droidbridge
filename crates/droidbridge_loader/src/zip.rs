use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug)]
pub enum ZipError {
    Io(io::Error),
    InvalidSignature(&'static str),
    EocdNotFound,
    DecompressionFailed(String),
    UnsupportedCompression(u16),
    PathTraversalDetected(String),
}

impl From<io::Error> for ZipError {
    fn from(e: io::Error) -> Self {
        ZipError::Io(e)
    }
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZipError::Io(e) => write!(f, "I/O error: {}", e),
            ZipError::InvalidSignature(sig) => write!(f, "Invalid ZIP signature: {}", sig),
            ZipError::EocdNotFound => write!(f, "End of Central Directory (EOCD) not found"),
            ZipError::DecompressionFailed(msg) => write!(f, "Decompression error: {}", msg),
            ZipError::UnsupportedCompression(m) => write!(f, "Unsupported compression method: {}", m),
            ZipError::PathTraversalDetected(p) => write!(f, "Zip-Slip path traversal detected: {}", p),
        }
    }
}

impl std::error::Error for ZipError {}

#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub name: String,
    pub compression_method: u16,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub local_header_offset: u64,
    pub is_directory: bool,
}

pub struct ZipArchive<R> {
    reader: R,
    entries: HashMap<String, ZipEntry>,
    entry_list: Vec<String>,
}

impl ZipArchive<File> {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, ZipError> {
        let file = File::open(path)?;
        Self::new(file)
    }
}

impl<R: Read + Seek> ZipArchive<R> {
    const EOCD_SIGNATURE: u32 = 0x06054b50;
    const CENTRAL_HEADER_SIGNATURE: u32 = 0x02014b50;
    const LOCAL_HEADER_SIGNATURE: u32 = 0x04034b50;

    pub fn new(mut reader: R) -> Result<Self, ZipError> {
        let (cd_offset, cd_entries_count) = Self::find_eocd(&mut reader)?;

        reader.seek(SeekFrom::Start(cd_offset))?;

        let mut entries = HashMap::new();
        let mut entry_list = Vec::new();

        for _ in 0..cd_entries_count {
            let mut header = [0u8; 46];
            reader.read_exact(&mut header)?;

            let sig = u32::from_le_bytes(header[0..4].try_into().unwrap());
            if sig != Self::CENTRAL_HEADER_SIGNATURE {
                return Err(ZipError::InvalidSignature("Central Directory Header"));
            }

            let compression_method = u16::from_le_bytes(header[10..12].try_into().unwrap());
            let compressed_size = u32::from_le_bytes(header[20..24].try_into().unwrap()) as u64;
            let uncompressed_size = u32::from_le_bytes(header[24..28].try_into().unwrap()) as u64;
            let name_len = u16::from_le_bytes(header[28..30].try_into().unwrap()) as usize;
            let extra_len = u16::from_le_bytes(header[30..32].try_into().unwrap()) as usize;
            let comment_len = u16::from_le_bytes(header[32..34].try_into().unwrap()) as usize;
            let local_header_offset = u32::from_le_bytes(header[42..46].try_into().unwrap()) as u64;

            let mut name_buf = vec![0u8; name_len];
            reader.read_exact(&mut name_buf)?;
            let name = String::from_utf8_lossy(&name_buf).to_string();

            // Zip-Slip security validation:
            if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
                return Err(ZipError::PathTraversalDetected(name));
            }

            // Skip extra field and comment
            reader.seek(SeekFrom::Current((extra_len + comment_len) as i64))?;

            let is_directory = name.ends_with('/') || name.ends_with('\\');

            let entry = ZipEntry {
                name: name.clone(),
                compression_method,
                compressed_size,
                uncompressed_size,
                local_header_offset,
                is_directory,
            };

            entry_list.push(name.clone());
            entries.insert(name, entry);
        }

        Ok(Self {
            reader,
            entries,
            entry_list,
        })
    }

    fn find_eocd(reader: &mut R) -> Result<(u64, u16), ZipError> {
        let total_size = reader.seek(SeekFrom::End(0))?;
        if total_size < 22 {
            return Err(ZipError::EocdNotFound);
        }

        let max_search_size = std::cmp::min(total_size, 65535 + 22);
        let search_start = total_size - max_search_size;
        reader.seek(SeekFrom::Start(search_start))?;

        let mut buf = vec![0u8; max_search_size as usize];
        reader.read_exact(&mut buf)?;

        for i in (0..=(buf.len() - 22)).rev() {
            let sig = u32::from_le_bytes(buf[i..i + 4].try_into().unwrap());
            if sig == Self::EOCD_SIGNATURE {
                let cd_entries = u16::from_le_bytes(buf[i + 10..i + 12].try_into().unwrap());
                let cd_offset = u32::from_le_bytes(buf[i + 16..i + 20].try_into().unwrap()) as u64;
                return Ok((cd_offset, cd_entries));
            }
        }

        Err(ZipError::EocdNotFound)
    }

    pub fn entries(&self) -> &[String] {
        &self.entry_list
    }

    pub fn get_entry(&self, name: &str) -> Option<&ZipEntry> {
        self.entries.get(name)
    }

    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>, ZipError> {
        let entry = self
            .entries
            .get(name)
            .cloned()
            .ok_or_else(|| ZipError::Io(io::Error::new(io::ErrorKind::NotFound, format!("File '{}' not in archive", name))))?;

        self.reader.seek(SeekFrom::Start(entry.local_header_offset))?;

        let mut local_header = [0u8; 30];
        self.reader.read_exact(&mut local_header)?;

        let sig = u32::from_le_bytes(local_header[0..4].try_into().unwrap());
        if sig != Self::LOCAL_HEADER_SIGNATURE {
            return Err(ZipError::InvalidSignature("Local File Header"));
        }

        let name_len = u16::from_le_bytes(local_header[26..28].try_into().unwrap()) as usize;
        let extra_len = u16::from_le_bytes(local_header[28..30].try_into().unwrap()) as usize;

        // Skip filename and extra field
        self.reader.seek(SeekFrom::Current((name_len + extra_len) as i64))?;

        let mut compressed_data = vec![0u8; entry.compressed_size as usize];
        self.reader.read_exact(&mut compressed_data)?;

        match entry.compression_method {
            0 => Ok(compressed_data), // Stored
            8 => {
                // Deflate
                miniz_oxide::inflate::decompress_to_vec_zlib(&compressed_data)
                    .or_else(|_| miniz_oxide::inflate::decompress_to_vec(&compressed_data))
                    .map_err(|e| ZipError::DecompressionFailed(format!("{:?}", e)))
            }
            m => Err(ZipError::UnsupportedCompression(m)),
        }
    }
}
