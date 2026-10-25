pub mod bionic;
pub mod elf;
pub mod jni;
pub mod loader;

pub use bionic::{BionicSymbolTable, shim_android_log_print, shim_malloc, shim_free};
pub use elf::{ElfError, ElfFile, ElfProgramHeader, ElfRelocation, ElfSymbol, EM_AARCH64, EM_X86_64};
pub use jni::{JniBridge, JNIEnv, JNINativeInterface};
pub use loader::{ElfLoader, LoadedElfLibrary};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_header_parsing_minimal() {
        let mut elf_data = vec![0u8; 128];
        // Magic
        elf_data[0..4].copy_from_slice(b"\x7fELF");
        elf_data[4] = 2; // 64-bit
        elf_data[5] = 1; // Little endian
        elf_data[6] = 1; // Version
        // Machine x86_64 = 0x3e
        elf_data[18..20].copy_from_slice(&0x003eu16.to_le_bytes());
        // Entry
        elf_data[24..32].copy_from_slice(&0x1000u64.to_le_bytes());
        // Phoff = 64
        elf_data[32..40].copy_from_slice(&64u64.to_le_bytes());
        // Phentsize = 56
        elf_data[54..56].copy_from_slice(&56u16.to_le_bytes());
        // Phnum = 1
        elf_data[56..58].copy_from_slice(&1u16.to_le_bytes());

        // Program header at offset 64: PT_LOAD
        let ph_off = 64;
        elf_data[ph_off..ph_off + 4].copy_from_slice(&1u32.to_le_bytes()); // PT_LOAD = 1
        elf_data[ph_off + 8..ph_off + 16].copy_from_slice(&0u64.to_le_bytes()); // p_offset = 0
        elf_data[ph_off + 16..ph_off + 24].copy_from_slice(&0x10000u64.to_le_bytes()); // p_vaddr
        elf_data[ph_off + 32..ph_off + 40].copy_from_slice(&128u64.to_le_bytes()); // p_filesz
        elf_data[ph_off + 40..ph_off + 48].copy_from_slice(&256u64.to_le_bytes()); // p_memsz

        let elf = ElfFile::parse(&elf_data).expect("Failed to parse valid minimal ELF");
        assert_eq!(elf.machine, EM_X86_64);
        assert_eq!(elf.program_headers.len(), 1);
        assert_eq!(elf.program_headers[0].p_vaddr, 0x10000);
        assert_eq!(elf.program_headers[0].p_memsz, 256);
    }

    #[test]
    fn test_bionic_symbol_resolution() {
        let bionic = BionicSymbolTable::new();
        assert!(bionic.resolve("__android_log_print").is_some());
        assert!(bionic.resolve("malloc").is_some());
        assert!(bionic.resolve("free").is_some());
        assert!(bionic.resolve("non_existent_function").is_none());
    }

    #[test]
    fn test_jni_symbol_mangling() {
        let symbol = JniBridge::mangle_symbol_name("com/whatsapp/util/NativeUtils", "calculateHash");
        assert_eq!(symbol, "Java_com_whatsapp_util_NativeUtils_calculateHash");
    }
}
