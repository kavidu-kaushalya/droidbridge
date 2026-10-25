use std::collections::HashMap;
use std::ffi::c_void;
use crate::bionic::BionicSymbolTable;
use crate::elf::{ElfFile, PT_LOAD};

#[link(name = "kernel32")]
extern "system" {
    fn VirtualAlloc(
        lp_address: *mut c_void,
        dw_size: usize,
        fl_allocation_type: u32,
        fl_protect: u32,
    ) -> *mut c_void;
    fn VirtualFree(lp_address: *mut c_void, dw_size: usize, dw_free_type: u32) -> i32;
}

const MEM_COMMIT: u32 = 0x00001000;
const MEM_RESERVE: u32 = 0x00002000;
const MEM_RELEASE: u32 = 0x00008000;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;

const R_X86_64_RELATIVE: u32 = 8;
const R_X86_64_GLOB_DAT: u32 = 6;
const R_X86_64_JUMP_SLOT: u32 = 7;

pub struct LoadedElfLibrary {
    pub base_address: usize,
    pub total_size: usize,
    pub symbols: HashMap<String, usize>,
    pub needed_libraries: Vec<String>,
}

impl Drop for LoadedElfLibrary {
    fn drop(&mut self) {
        if self.base_address != 0 {
            unsafe {
                VirtualFree(self.base_address as *mut c_void, 0, MEM_RELEASE);
            }
        }
    }
}

pub struct ElfLoader;

impl ElfLoader {
    pub fn load(elf_bytes: &[u8], bionic: &BionicSymbolTable) -> Result<LoadedElfLibrary, String> {
        let elf = ElfFile::parse(elf_bytes).map_err(|e| format!("ELF parse error: {}", e))?;

        // 1. Calculate virtual memory span across all PT_LOAD segments
        let mut min_vaddr = u64::MAX;
        let mut max_vaddr = 0u64;

        for ph in &elf.program_headers {
            if ph.p_type == PT_LOAD {
                if ph.p_vaddr < min_vaddr {
                    min_vaddr = ph.p_vaddr;
                }
                let end = ph.p_vaddr + ph.p_memsz;
                if end > max_vaddr {
                    max_vaddr = end;
                }
            }
        }

        if min_vaddr > max_vaddr {
            return Err("ELF has no valid PT_LOAD segments".to_string());
        }

        let total_size = (max_vaddr - min_vaddr) as usize;

        // 2. Allocate executable memory on Windows host via VirtualAlloc
        let base_ptr = unsafe {
            VirtualAlloc(
                std::ptr::null_mut(),
                total_size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        };

        if base_ptr.is_null() {
            return Err("VirtualAlloc failed to allocate memory for ELF segments".to_string());
        }

        let base_addr = base_ptr as usize;

        // 3. Copy segments into allocated memory
        for ph in &elf.program_headers {
            if ph.p_type == PT_LOAD {
                let dest_offset = (ph.p_vaddr - min_vaddr) as usize;
                let dest = (base_addr + dest_offset) as *mut u8;

                let src_offset = ph.p_offset as usize;
                let filesz = ph.p_filesz as usize;
                let memsz = ph.p_memsz as usize;

                if src_offset + filesz <= elf_bytes.len() {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            elf_bytes[src_offset..src_offset + filesz].as_ptr(),
                            dest,
                            filesz,
                        );
                        // Zero remaining BSS bytes
                        if memsz > filesz {
                            std::ptr::write_bytes(dest.add(filesz), 0, memsz - filesz);
                        }
                    }
                }
            }
        }

        // 4. Resolve Dynamic Relocations
        for rel in &elf.relocations {
            let target_addr = (base_addr as u64 + rel.offset) as *mut usize;
            match rel.rel_type {
                R_X86_64_RELATIVE => {
                    // Fixup: *target = base + addend
                    let relocated_val = (base_addr as i64 + rel.addend) as usize;
                    unsafe {
                        *target_addr = relocated_val;
                    }
                }
                R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                    if let Some(sym) = elf.symbols.get(rel.symbol_idx as usize) {
                        if let Some(shim_addr) = bionic.resolve(&sym.name) {
                            unsafe {
                                *target_addr = shim_addr;
                            }
                        } else {
                            println!("[ElfLoader Warning] Unresolved external symbol: {}", sym.name);
                        }
                    }
                }
                _ => {}
            }
        }

        // 5. Index Exported Symbols
        let mut symbols = HashMap::new();
        for sym in elf.symbols {
            if !sym.name.is_empty() && sym.value > 0 {
                let sym_addr = base_addr + (sym.value - min_vaddr) as usize;
                symbols.insert(sym.name, sym_addr);
            }
        }

        Ok(LoadedElfLibrary {
            base_address: base_addr,
            total_size,
            symbols,
            needed_libraries: elf.needed_libraries,
        })
    }
}
