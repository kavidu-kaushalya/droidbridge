#[derive(Debug)]
pub enum ElfError {
    InvalidMagic,
    Not64Bit,
    NotLittleEndian,
    UnsupportedMachine(u16),
    UnexpectedEof,
    CorruptedHeader(&'static str),
}

impl std::fmt::Display for ElfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfError::InvalidMagic => write!(f, "Invalid ELF magic number"),
            ElfError::Not64Bit => write!(f, "ELF is not 64-bit (ELFCLASS64 required)"),
            ElfError::NotLittleEndian => write!(f, "ELF is not Little-Endian"),
            ElfError::UnsupportedMachine(m) => write!(f, "Unsupported machine architecture: 0x{:04x}", m),
            ElfError::UnexpectedEof => write!(f, "Unexpected end of file while reading ELF"),
            ElfError::CorruptedHeader(msg) => write!(f, "Corrupted ELF header: {}", msg),
        }
    }
}

impl std::error::Error for ElfError {}

pub const EM_X86_64: u16 = 0x003e;
pub const EM_AARCH64: u16 = 0x00b7;
pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const DT_NEEDED: u64 = 1;
pub const DT_STRTAB: u64 = 5;
pub const DT_SYMTAB: u64 = 6;
pub const DT_RELA: u64 = 7;
pub const DT_RELASZ: u64 = 8;
pub const DT_RELAENT: u64 = 9;

#[derive(Debug, Clone)]
pub struct ElfProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

#[derive(Debug, Clone)]
pub struct ElfSymbol {
    pub name: String,
    pub value: u64,
    pub size: u64,
    pub binding: u8,
    pub sym_type: u8,
}

#[derive(Debug, Clone)]
pub struct ElfRelocation {
    pub offset: u64,
    pub rel_type: u32,
    pub symbol_idx: u32,
    pub addend: i64,
}

pub struct ElfFile<'a> {
    pub data: &'a [u8],
    pub machine: u16,
    pub entry: u64,
    pub program_headers: Vec<ElfProgramHeader>,
    pub needed_libraries: Vec<String>,
    pub symbols: Vec<ElfSymbol>,
    pub relocations: Vec<ElfRelocation>,
}

impl<'a> ElfFile<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, ElfError> {
        if data.len() < 64 {
            return Err(ElfError::UnexpectedEof);
        }

        // Check magic: 0x7f 'E' 'L' 'F'
        if &data[0..4] != b"\x7fELF" {
            return Err(ElfError::InvalidMagic);
        }

        // EI_CLASS: 2 = 64-bit
        if data[4] != 2 {
            return Err(ElfError::Not64Bit);
        }

        // EI_DATA: 1 = 2's complement, little endian
        if data[5] != 1 {
            return Err(ElfError::NotLittleEndian);
        }

        let machine = u16::from_le_bytes(data[18..20].try_into().unwrap());
        let entry = u64::from_le_bytes(data[24..32].try_into().unwrap());
        let phoff = u64::from_le_bytes(data[32..40].try_into().unwrap()) as usize;
        let phentsize = u16::from_le_bytes(data[54..56].try_into().unwrap()) as usize;
        let phnum = u16::from_le_bytes(data[56..58].try_into().unwrap()) as usize;

        let mut program_headers = Vec::with_capacity(phnum);
        let mut dynamic_ph: Option<ElfProgramHeader> = None;

        for i in 0..phnum {
            let offset = phoff + i * phentsize;
            if offset + 56 > data.len() {
                return Err(ElfError::UnexpectedEof);
            }

            let p_type = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let p_flags = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let p_offset = u64::from_le_bytes(data[offset + 8..offset + 16].try_into().unwrap());
            let p_vaddr = u64::from_le_bytes(data[offset + 16..offset + 24].try_into().unwrap());
            let p_paddr = u64::from_le_bytes(data[offset + 24..offset + 32].try_into().unwrap());
            let p_filesz = u64::from_le_bytes(data[offset + 32..offset + 40].try_into().unwrap());
            let p_memsz = u64::from_le_bytes(data[offset + 40..offset + 48].try_into().unwrap());
            let p_align = u64::from_le_bytes(data[offset + 48..offset + 56].try_into().unwrap());

            let ph = ElfProgramHeader {
                p_type,
                p_flags,
                p_offset,
                p_vaddr,
                p_paddr,
                p_filesz,
                p_memsz,
                p_align,
            };

            if p_type == PT_DYNAMIC {
                dynamic_ph = Some(ph.clone());
            }

            program_headers.push(ph);
        }

        // Parse Dynamic Section if present
        let mut needed_libraries = Vec::new();
        let mut strtab_offset: Option<usize> = None;
        let mut symtab_offset: Option<usize> = None;
        let mut rela_offset: Option<usize> = None;
        let mut rela_sz = 0usize;
        let mut needed_str_indices = Vec::new();

        if let Some(dyn_ph) = dynamic_ph {
            let dyn_start = dyn_ph.p_offset as usize;
            let dyn_len = dyn_ph.p_filesz as usize;
            let dyn_end = dyn_start + dyn_len;

            if dyn_end <= data.len() {
                let mut curr = dyn_start;
                while curr + 16 <= dyn_end {
                    let d_tag = u64::from_le_bytes(data[curr..curr + 8].try_into().unwrap());
                    let d_val = u64::from_le_bytes(data[curr + 8..curr + 16].try_into().unwrap());

                    if d_tag == 0 {
                        break; // DT_NULL
                    }

                    match d_tag {
                        DT_NEEDED => needed_str_indices.push(d_val as usize),
                        DT_STRTAB => {
                            // Convert vaddr to file offset
                            strtab_offset = Self::vaddr_to_offset(&program_headers, d_val);
                        }
                        DT_SYMTAB => {
                            symtab_offset = Self::vaddr_to_offset(&program_headers, d_val);
                        }
                        DT_RELA => {
                            rela_offset = Self::vaddr_to_offset(&program_headers, d_val);
                        }
                        DT_RELASZ => {
                            rela_sz = d_val as usize;
                        }
                        _ => {}
                    }

                    curr += 16;
                }
            }
        }

        // Extract needed library names from strtab
        if let Some(str_off) = strtab_offset {
            if str_off < data.len() {
                let strtab = &data[str_off..];
                for idx in needed_str_indices {
                    if idx < strtab.len() {
                        let mut end = idx;
                        while end < strtab.len() && strtab[end] != 0 {
                            end += 1;
                        }
                        let name = String::from_utf8_lossy(&strtab[idx..end]).to_string();
                        needed_libraries.push(name);
                    }
                }
            }
        }

        // Parse Dynamic Symbols
        let mut symbols = Vec::new();
        if let (Some(sym_off), Some(str_off)) = (symtab_offset, strtab_offset) {
            let mut curr = sym_off;
            let strtab = if str_off < data.len() { &data[str_off..] } else { &[] };

            // Read symbols until null or end
            while curr + 24 <= data.len() {
                let st_name = u32::from_le_bytes(data[curr..curr + 4].try_into().unwrap()) as usize;
                let st_info = data[curr + 4];
                let _st_other = data[curr + 5];
                let _st_shndx = u16::from_le_bytes(data[curr + 6..curr + 8].try_into().unwrap());
                let st_value = u64::from_le_bytes(data[curr + 8..curr + 16].try_into().unwrap());
                let st_size = u64::from_le_bytes(data[curr + 16..curr + 24].try_into().unwrap());

                let name = if st_name < strtab.len() {
                    let mut end = st_name;
                    while end < strtab.len() && strtab[end] != 0 {
                        end += 1;
                    }
                    String::from_utf8_lossy(&strtab[st_name..end]).to_string()
                } else {
                    String::new()
                };

                // Stop if symbol entries become empty zeros past header
                if symbols.len() > 1 && st_name == 0 && st_value == 0 && st_size == 0 {
                    break;
                }

                symbols.push(ElfSymbol {
                    name,
                    value: st_value,
                    size: st_size,
                    binding: st_info >> 4,
                    sym_type: st_info & 0x0f,
                });

                curr += 24;
            }
        }

        // Parse Relocations (Rela)
        let mut relocations = Vec::new();
        if let Some(r_off) = rela_offset {
            if r_off + rela_sz <= data.len() {
                let mut curr = r_off;
                let end = r_off + rela_sz;
                while curr + 24 <= end {
                    let offset = u64::from_le_bytes(data[curr..curr + 8].try_into().unwrap());
                    let info = u64::from_le_bytes(data[curr + 8..curr + 16].try_into().unwrap());
                    let addend = i64::from_le_bytes(data[curr + 16..curr + 24].try_into().unwrap());

                    let rel_type = (info & 0xffffffff) as u32;
                    let symbol_idx = (info >> 32) as u32;

                    relocations.push(ElfRelocation {
                        offset,
                        rel_type,
                        symbol_idx,
                        addend,
                    });

                    curr += 24;
                }
            }
        }

        Ok(Self {
            data,
            machine,
            entry,
            program_headers,
            needed_libraries,
            symbols,
            relocations,
        })
    }

    fn vaddr_to_offset(headers: &[ElfProgramHeader], vaddr: u64) -> Option<usize> {
        for ph in headers {
            if ph.p_type == PT_LOAD {
                if vaddr >= ph.p_vaddr && vaddr < ph.p_vaddr + ph.p_filesz {
                    let delta = vaddr - ph.p_vaddr;
                    return Some((ph.p_offset + delta) as usize);
                }
            }
        }
        None
    }
}
