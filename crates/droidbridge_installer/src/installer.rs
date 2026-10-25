use std::fs::{self, File};
use std::path::Path;
use droidbridge_loader::{ApkPackage, ZipArchive};
use crate::registry::{AppRegistry, InstalledApp};

pub struct PackageInstaller;

impl PackageInstaller {
    pub fn install<P: AsRef<Path>>(apk_path: P) -> Result<InstalledApp, String> {
        let apk_p = apk_path.as_ref();
        if !apk_p.exists() {
            return Err(format!("APK file does not exist: {}", apk_p.display()));
        }

        println!("[Installer] Reading APK package: {}", apk_p.display());
        let apk = ApkPackage::open(apk_p).map_err(|e| format!("Invalid APK package: {}", e))?;

        let package_name = apk.manifest.package_name.clone();
        let app_label = if let Some(ref raw_label) = apk.manifest.application_label {
            if let Ok(res_id) = raw_label.parse::<u32>() {
                if let Some(ref res) = apk.resources {
                    res.resolve_string(res_id).unwrap_or(raw_label).to_string()
                } else {
                    raw_label.clone()
                }
            } else {
                raw_label.clone()
            }
        } else {
            package_name.clone()
        };
        let version_name = apk.manifest.version_name.clone().unwrap_or_else(|| "1.0".to_string());
        let version_code = apk.manifest.version_code.unwrap_or(1);

        let main_activity = if let Some(launcher) = apk.manifest.activities.iter().find(|a| a.is_launcher) {
            launcher.name.clone()
        } else if let Some(first) = apk.manifest.activities.first() {
            first.name.clone()
        } else {
            "MainActivity".to_string()
        };

        let file_size = fs::metadata(apk_p).map(|m| m.len()).unwrap_or(0);

        // 1. Establish Scoped Isolated Storage
        let root = AppRegistry::get_default_storage_root();
        let app_root = root.join("Apps").join(&package_name);
        let files_dir = app_root.join("files");
        let cache_dir = app_root.join("cache");
        let shared_prefs_dir = app_root.join("shared_prefs");
        let native_lib_dir = app_root.join("lib");

        fs::create_dir_all(&files_dir).map_err(|e| format!("Cannot create files dir: {}", e))?;
        fs::create_dir_all(&cache_dir).map_err(|e| format!("Cannot create cache dir: {}", e))?;
        fs::create_dir_all(&shared_prefs_dir).map_err(|e| format!("Cannot create prefs dir: {}", e))?;
        fs::create_dir_all(&native_lib_dir).map_err(|e| format!("Cannot create lib dir: {}", e))?;

        // 2. Copy APK to base.apk in target container
        let target_apk_path = app_root.join("base.apk");
        println!("[Installer] Storing sandboxed APK container: {}", target_apk_path.display());
        fs::copy(apk_p, &target_apk_path).map_err(|e| format!("Failed to copy APK container: {}", e))?;

        // 3. Extract Native x86_64 ELF libraries if present
        let mut file = File::open(apk_p).map_err(|e| format!("Cannot open APK: {}", e))?;
        if let Ok(mut archive) = ZipArchive::new(&mut file) {
            let entries = archive.entries().to_vec();
            for entry_name in entries {
                if entry_name.starts_with("lib/x86_64/") && entry_name.ends_with(".so") {
                    let filename = entry_name.split('/').last().unwrap_or("lib.so");
                    let dest_path = native_lib_dir.join(filename);
                    if let Ok(bytes) = archive.read_file(&entry_name) {
                        if fs::write(&dest_path, &bytes).is_ok() {
                            println!("[Installer] Extracted native library: {} ({} bytes)", filename, bytes.len());
                        }
                    }
                }
            }
        }

        let installed_app = InstalledApp {
            package_name: package_name.clone(),
            app_label: app_label.clone(),
            version_name,
            version_code,
            main_activity,
            apk_path: target_apk_path,
            data_dir: app_root,
            native_lib_dir,
            install_time: "2026-10-25".to_string(),
            size_bytes: file_size,
        };

        // 4. Register in system registry
        let mut registry = AppRegistry::load();
        registry.register(installed_app.clone())?;

        println!("[+] Installation completed: {} ({})", app_label, package_name);
        Ok(installed_app)
    }

    pub fn uninstall(package_name: &str) -> Result<bool, String> {
        let mut registry = AppRegistry::load();
        let app = registry.find(package_name).cloned();

        if let Some(app) = app {
            if app.data_dir.exists() {
                let _ = fs::remove_dir_all(&app.data_dir);
            }
            registry.unregister(package_name)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
