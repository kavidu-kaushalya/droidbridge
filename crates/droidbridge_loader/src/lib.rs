pub mod arsc;
pub mod axml;
pub mod dex;
pub mod validator;
pub mod zip;

use std::fs::File;
use std::path::Path;

pub use arsc::{ArscError, ResEntry, ResPackage, ResValue, ResourceTable};
pub use axml::{ActivityManifestInfo, AndroidManifest, AxmlAttributeValue, AxmlDecoder, AxmlError, AxmlNode};
pub use dex::{ClassInfo, CodeItem, DexError, DexFile, DexHeader, MethodInfo};
pub use validator::{ApkCompatibilityReport, CompatibilityValidator, CompatibilityVerdict, NativeLibraryInfo};
pub use zip::{ZipArchive, ZipEntry, ZipError};

#[derive(Debug)]
pub enum ApkLoadError {
    Zip(ZipError),
    Axml(AxmlError),
    Dex(DexError),
    Arsc(ArscError),
    MissingManifest,
    MissingDex,
}

impl From<ZipError> for ApkLoadError {
    fn from(e: ZipError) -> Self {
        ApkLoadError::Zip(e)
    }
}

impl From<AxmlError> for ApkLoadError {
    fn from(e: AxmlError) -> Self {
        ApkLoadError::Axml(e)
    }
}

impl From<DexError> for ApkLoadError {
    fn from(e: DexError) -> Self {
        ApkLoadError::Dex(e)
    }
}

impl From<ArscError> for ApkLoadError {
    fn from(e: ArscError) -> Self {
        ApkLoadError::Arsc(e)
    }
}

impl std::fmt::Display for ApkLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApkLoadError::Zip(e) => write!(f, "ZIP error: {}", e),
            ApkLoadError::Axml(e) => write!(f, "AXML error: {}", e),
            ApkLoadError::Dex(e) => write!(f, "DEX error: {}", e),
            ApkLoadError::Arsc(e) => write!(f, "ARSC error: {}", e),
            ApkLoadError::MissingManifest => write!(f, "APK does not contain AndroidManifest.xml"),
            ApkLoadError::MissingDex => write!(f, "APK does not contain classes.dex"),
        }
    }
}

impl std::error::Error for ApkLoadError {}

pub struct ApkPackage {
    pub manifest: AndroidManifest,
    pub compatibility: ApkCompatibilityReport,
    pub dex_data: Vec<u8>,
    pub resources: Option<ResourceTable>,
    pub arsc_data: Option<Vec<u8>>,
}

impl ApkPackage {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, ApkLoadError> {
        let file = File::open(path).map_err(ZipError::Io)?;
        let mut archive = ZipArchive::new(file)?;

        // Read and parse AndroidManifest.xml
        let manifest_bytes = archive
            .read_file("AndroidManifest.xml")
            .map_err(|_| ApkLoadError::MissingManifest)?;
        let manifest = AxmlDecoder::decode(&manifest_bytes)?;

        // Evaluate feature compatibility
        let entries = archive.entries().to_vec();
        let compatibility = CompatibilityValidator::evaluate(&manifest, &entries);

        // Read primary classes.dex
        let dex_data = archive
            .read_file("classes.dex")
            .map_err(|_| ApkLoadError::MissingDex)?;

        // Attempt to read resources.arsc if present
        let mut resources = None;
        let mut arsc_data = None;
        if let Ok(arsc_bytes) = archive.read_file("resources.arsc") {
            if let Ok(parsed_table) = ResourceTable::parse(&arsc_bytes) {
                resources = Some(parsed_table);
            }
            arsc_data = Some(arsc_bytes);
        }

        Ok(Self {
            manifest,
            compatibility,
            dex_data,
            resources,
            arsc_data,
        })
    }

    pub fn parse_primary_dex(&self) -> Result<DexFile<'_>, DexError> {
        DexFile::parse(&self.dex_data)
    }
}
