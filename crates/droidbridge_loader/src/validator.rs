use std::collections::HashSet;
use crate::axml::AndroidManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityVerdict {
    /// Pure Dalvik/ART bytecode, zero native libraries. 100% executable natively.
    SupportedPureManaged,
    /// Bundles x86_64 ELF libraries. Supported via user-mode ELF loader and Bionic shim.
    SupportedWithNativeX64,
    /// Bundles only ARM/ARM64 ELF libraries. Requires dynamic binary translation (DBT).
    BlockedArmOnlyNative(Vec<String>),
    /// Relies on unsupported Android hardware/telephony features.
    UnsupportedHardwareFeature(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct NativeLibraryInfo {
    pub architecture: String,
    pub filename: String,
}

#[derive(Debug, Clone)]
pub struct ApkCompatibilityReport {
    pub package_name: String,
    pub dex_count: usize,
    pub native_libraries: Vec<NativeLibraryInfo>,
    pub supported_architectures: HashSet<String>,
    pub permissions: Vec<String>,
    pub verdict: CompatibilityVerdict,
    pub summary: String,
}

pub struct CompatibilityValidator;

impl CompatibilityValidator {
    pub fn evaluate(
        manifest: &AndroidManifest,
        archive_entries: &[String],
    ) -> ApkCompatibilityReport {
        let mut dex_count = 0;
        let mut native_libraries = Vec::new();
        let mut architectures = HashSet::new();

        for entry in archive_entries {
            if entry.ends_with(".dex") && (entry == &"classes.dex" || entry.starts_with("classes")) {
                dex_count += 1;
            } else if entry.starts_with("lib/") && entry.ends_with(".so") {
                let parts: Vec<&str> = entry.split('/').collect();
                if parts.len() >= 3 {
                    let arch = parts[1].to_string();
                    let filename = parts[2].to_string();
                    architectures.insert(arch.clone());
                    native_libraries.push(NativeLibraryInfo {
                        architecture: arch,
                        filename,
                    });
                }
            }
        }

        // Determine verdict
        let (verdict, summary) = if native_libraries.is_empty() {
            (
                CompatibilityVerdict::SupportedPureManaged,
                "100% Pure Managed bytecode (DEX). Full native execution supported without CPU translation.".to_string(),
            )
        } else if architectures.contains("x86_64") {
            (
                CompatibilityVerdict::SupportedWithNativeX64,
                "Contains native x86_64 ELF libraries. Supported via native ELF loader and Bionic shim.".to_string(),
            )
        } else {
            let arm_libs: Vec<String> = native_libraries.iter().map(|l| format!("{}/{}", l.architecture, l.filename)).collect();
            (
                CompatibilityVerdict::BlockedArmOnlyNative(arm_libs.clone()),
                format!(
                    "Blocked: Application contains {} ARM-only native library/libraries ({}). Requires ARM64 instruction translation.",
                    arm_libs.len(),
                    architectures.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            )
        };

        ApkCompatibilityReport {
            package_name: manifest.package_name.clone(),
            dex_count,
            native_libraries,
            supported_architectures: architectures,
            permissions: manifest.permissions.clone(),
            verdict,
            summary,
        }
    }
}
