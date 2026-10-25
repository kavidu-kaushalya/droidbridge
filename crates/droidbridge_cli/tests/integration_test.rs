use std::fs::File;
use std::io::Write;
use droidbridge_loader::{ApkPackage, CompatibilityVerdict};
use droidbridge_core::{DexVm, ExecutionResult, Value};

/// Helper to generate a minimal valid classes.dex binary
fn generate_minimal_dex() -> Vec<u8> {
    let mut dex = vec![0u8; 112];
    dex[0..8].copy_from_slice(b"dex\n035\0");
    dex[8..12].copy_from_slice(&0xdeadbeefu32.to_le_bytes());
    dex[32..36].copy_from_slice(&(112u32).to_le_bytes());
    dex[36..40].copy_from_slice(&0x70u32.to_le_bytes()); // header_size = 112
    dex[40..44].copy_from_slice(&0x12345678u32.to_le_bytes()); // endian_tag
    dex
}

/// Helper to generate a minimal valid Android Binary XML (AXML)
fn generate_minimal_axml(package_name: &str) -> Vec<u8> {
    let mut axml = Vec::new();

    let pkg_utf16: Vec<u16> = package_name.encode_utf16().collect();
    let manifest_str: Vec<u16> = "manifest".encode_utf16().collect();
    let package_attr_str: Vec<u16> = "package".encode_utf16().collect();

    let strings_data: Vec<Vec<u16>> = vec![manifest_str, package_attr_str, pkg_utf16];

    let mut string_offsets = Vec::new();
    let mut string_bytes = Vec::new();

    for s in &strings_data {
        string_offsets.push(string_bytes.len() as u32);
        string_bytes.extend_from_slice(&(s.len() as u16).to_le_bytes());
        for c in s {
            string_bytes.extend_from_slice(&c.to_le_bytes());
        }
        string_bytes.extend_from_slice(&0u16.to_le_bytes());
    }

    while string_bytes.len() % 4 != 0 {
        string_bytes.push(0);
    }

    let string_count = strings_data.len() as u32;
    let pool_header_size = 28u32;
    let strings_start = pool_header_size + string_count * 4;
    let pool_total_size = strings_start + string_bytes.len() as u32;

    let mut pool_chunk = Vec::new();
    pool_chunk.extend_from_slice(&0x0001u16.to_le_bytes());
    pool_chunk.extend_from_slice(&0x001cu16.to_le_bytes());
    pool_chunk.extend_from_slice(&pool_total_size.to_le_bytes());
    pool_chunk.extend_from_slice(&string_count.to_le_bytes());
    pool_chunk.extend_from_slice(&0u32.to_le_bytes());
    pool_chunk.extend_from_slice(&0u32.to_le_bytes());
    pool_chunk.extend_from_slice(&strings_start.to_le_bytes());
    pool_chunk.extend_from_slice(&0u32.to_le_bytes());

    for off in string_offsets {
        pool_chunk.extend_from_slice(&off.to_le_bytes());
    }
    pool_chunk.extend_from_slice(&string_bytes);

    let attr_count = 1u16;
    let elem_header_size = 16u16;
    let elem_total_size = 36u32 + 20u32;

    let mut elem_chunk = Vec::new();
    elem_chunk.extend_from_slice(&0x0102u16.to_le_bytes());
    elem_chunk.extend_from_slice(&elem_header_size.to_le_bytes());
    elem_chunk.extend_from_slice(&elem_total_size.to_le_bytes());
    elem_chunk.extend_from_slice(&1u32.to_le_bytes());
    elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    elem_chunk.extend_from_slice(&0u32.to_le_bytes());
    elem_chunk.extend_from_slice(&20u16.to_le_bytes());
    elem_chunk.extend_from_slice(&20u16.to_le_bytes());
    elem_chunk.extend_from_slice(&attr_count.to_le_bytes());
    elem_chunk.extend_from_slice(&0u16.to_le_bytes());
    elem_chunk.extend_from_slice(&0u16.to_le_bytes());
    elem_chunk.extend_from_slice(&0u16.to_le_bytes());

    elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    elem_chunk.extend_from_slice(&1u32.to_le_bytes());
    elem_chunk.extend_from_slice(&2u32.to_le_bytes());
    elem_chunk.extend_from_slice(&8u16.to_le_bytes());
    elem_chunk.push(0);
    elem_chunk.push(0x03);
    elem_chunk.extend_from_slice(&2u32.to_le_bytes());

    let mut end_elem_chunk = Vec::new();
    end_elem_chunk.extend_from_slice(&0x0103u16.to_le_bytes());
    end_elem_chunk.extend_from_slice(&16u16.to_le_bytes());
    end_elem_chunk.extend_from_slice(&24u32.to_le_bytes());
    end_elem_chunk.extend_from_slice(&1u32.to_le_bytes());
    end_elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    end_elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    end_elem_chunk.extend_from_slice(&0u32.to_le_bytes());

    let xml_total_size = 8u32 + pool_chunk.len() as u32 + elem_chunk.len() as u32 + end_elem_chunk.len() as u32;
    axml.extend_from_slice(&0x0003u16.to_le_bytes());
    axml.extend_from_slice(&8u16.to_le_bytes());
    axml.extend_from_slice(&xml_total_size.to_le_bytes());

    axml.extend_from_slice(&pool_chunk);
    axml.extend_from_slice(&elem_chunk);
    axml.extend_from_slice(&end_elem_chunk);

    axml
}

fn write_zip_file(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip_data = Vec::new();
    let mut cd_entries = Vec::new();

    for (name, data) in entries {
        let local_header_offset = zip_data.len() as u32;

        zip_data.extend_from_slice(&0x04034b50u32.to_le_bytes());
        zip_data.extend_from_slice(&20u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u32.to_le_bytes());
        zip_data.extend_from_slice(&(data.len() as u32).to_le_bytes());
        zip_data.extend_from_slice(&(data.len() as u32).to_le_bytes());
        zip_data.extend_from_slice(&(name.len() as u16).to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(name.as_bytes());
        zip_data.extend_from_slice(data);

        cd_entries.push((name, data.len() as u32, local_header_offset));
    }

    let cd_offset = zip_data.len() as u32;

    for (name, size, local_off) in &cd_entries {
        zip_data.extend_from_slice(&0x02014b50u32.to_le_bytes());
        zip_data.extend_from_slice(&20u16.to_le_bytes());
        zip_data.extend_from_slice(&20u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u32.to_le_bytes());
        zip_data.extend_from_slice(&size.to_le_bytes());
        zip_data.extend_from_slice(&size.to_le_bytes());
        zip_data.extend_from_slice(&(name.len() as u16).to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u32.to_le_bytes());
        zip_data.extend_from_slice(&local_off.to_le_bytes());
        zip_data.extend_from_slice(name.as_bytes());
    }

    let cd_size = (zip_data.len() as u32) - cd_offset;

    zip_data.extend_from_slice(&0x06054b50u32.to_le_bytes());
    zip_data.extend_from_slice(&0u16.to_le_bytes());
    zip_data.extend_from_slice(&0u16.to_le_bytes());
    zip_data.extend_from_slice(&(cd_entries.len() as u16).to_le_bytes());
    zip_data.extend_from_slice(&(cd_entries.len() as u16).to_le_bytes());
    zip_data.extend_from_slice(&cd_size.to_le_bytes());
    zip_data.extend_from_slice(&cd_offset.to_le_bytes());
    zip_data.extend_from_slice(&0u16.to_le_bytes());

    zip_data
}

#[test]
fn test_end_to_end_apk_lifecycle_and_execution() {
    let manifest_bytes = generate_minimal_axml("org.droidbridge.demo");
    let dex_bytes = generate_minimal_dex();

    let apk_bytes = write_zip_file(&[
        ("AndroidManifest.xml", &manifest_bytes),
        ("classes.dex", &dex_bytes),
    ]);

    let temp_dir = std::env::temp_dir().join("droidbridge_test");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let apk_path = temp_dir.join("demo_app.apk");

    let mut file = File::create(&apk_path).unwrap();
    file.write_all(&apk_bytes).unwrap();
    drop(file);

    // 1. Inspect and open APK
    let apk = ApkPackage::open(&apk_path).expect("Failed to open synthetic APK");
    assert_eq!(apk.manifest.package_name, "org.droidbridge.demo");
    assert_eq!(apk.compatibility.verdict, CompatibilityVerdict::SupportedPureManaged);

    // 2. Parse DEX directly from APK
    let dex = apk.parse_primary_dex().expect("Failed to parse primary DEX");
    assert_eq!(&dex.header.magic, b"dex\n035\0");

    // 3. Execute synthetic Dalvik method on host DEX VM without emulator or VM
    let vm = DexVm::new();
    let insns = vec![
        0x7012,         // const/4 v0, #7
        0x3112,         // const/4 v1, #3
        0x0291, 0x0100, // sub-int v2, v0, v1 (dst=2, src1=0, src2=1)
        0x020f,         // return v2
    ];
    let result = vm.execute_bytecode(&insns, 3, 0, &[]);
    assert_eq!(result, ExecutionResult::Returned(Some(Value::Int(4))));

    // Cleanup
    let _ = std::fs::remove_file(apk_path);
}
