use droidbridge_loader::dex::DexFile;
use droidbridge_loader::validator::{CompatibilityValidator, CompatibilityVerdict};
use droidbridge_loader::axml::AndroidManifest;

#[test]
fn test_dex_header_parsing_and_endian() {
    // Construct a minimal valid 112-byte DEX header
    let mut dex_bytes = vec![0u8; 128];
    // Magic: "dex\n035\0"
    dex_bytes[0..8].copy_from_slice(b"dex\n035\0");
    // Checksum: 0x12345678
    dex_bytes[8..12].copy_from_slice(&0x12345678u32.to_le_bytes());
    // File size: 128
    dex_bytes[32..36].copy_from_slice(&128u32.to_le_bytes());
    // Header size: 0x70 = 112
    dex_bytes[36..40].copy_from_slice(&112u32.to_le_bytes());
    // Endian tag: 0x12345678
    dex_bytes[40..44].copy_from_slice(&0x12345678u32.to_le_bytes());

    let dex = DexFile::parse(&dex_bytes).expect("Failed to parse valid minimal DEX header");
    assert_eq!(&dex.header.magic, b"dex\n035\0");
    assert_eq!(dex.header.file_size, 128);
    assert_eq!(dex.header.header_size, 112);
}

#[test]
fn test_compatibility_validator_pure_managed() {
    let manifest = AndroidManifest {
        package_name: "com.example.pureapp".to_string(),
        version_code: Some(1),
        version_name: Some("1.0".to_string()),
        min_sdk_version: Some(26),
        target_sdk_version: Some(34),
        permissions: vec!["android.permission.INTERNET".to_string()],
        activities: Vec::new(),
        application_label: Some("Pure App".to_string()),
    };

    let entries = vec![
        "AndroidManifest.xml".to_string(),
        "classes.dex".to_string(),
        "resources.arsc".to_string(),
        "res/layout/main.xml".to_string(),
    ];

    let report = CompatibilityValidator::evaluate(&manifest, &entries);
    assert_eq!(report.dex_count, 1);
    assert_eq!(report.verdict, CompatibilityVerdict::SupportedPureManaged);
    assert!(report.native_libraries.is_empty());
}

#[test]
fn test_compatibility_validator_x86_64_native() {
    let manifest = AndroidManifest {
        package_name: "com.example.nativeapp".to_string(),
        ..Default::default()
    };

    let entries = vec![
        "AndroidManifest.xml".to_string(),
        "classes.dex".to_string(),
        "lib/x86_64/libapp.so".to_string(),
        "lib/arm64-v8a/libapp.so".to_string(),
    ];

    let report = CompatibilityValidator::evaluate(&manifest, &entries);
    assert_eq!(report.verdict, CompatibilityVerdict::SupportedWithNativeX64);
    assert!(report.supported_architectures.contains("x86_64"));
}

#[test]
fn test_compatibility_validator_arm_only_native() {
    let manifest = AndroidManifest {
        package_name: "com.example.armapp".to_string(),
        ..Default::default()
    };

    let entries = vec![
        "AndroidManifest.xml".to_string(),
        "classes.dex".to_string(),
        "lib/arm64-v8a/libgame.so".to_string(),
    ];

    let report = CompatibilityValidator::evaluate(&manifest, &entries);
    match report.verdict {
        CompatibilityVerdict::BlockedArmOnlyNative(libs) => {
            assert_eq!(libs.len(), 1);
            assert_eq!(libs[0], "arm64-v8a/libgame.so");
        }
        _ => panic!("Expected BlockedArmOnlyNative verdict"),
    }
}
