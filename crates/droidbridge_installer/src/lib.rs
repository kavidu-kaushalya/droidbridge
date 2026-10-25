pub mod installer;
pub mod registry;

pub use installer::PackageInstaller;
pub use registry::{AppRegistry, InstalledApp};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_save_load() {
        let mut registry = AppRegistry::load();
        let app = InstalledApp {
            package_name: "org.droidbridge.testapp".to_string(),
            app_label: "Test App".to_string(),
            version_name: "1.0".to_string(),
            version_code: 1,
            main_activity: "MainActivity".to_string(),
            apk_path: std::path::PathBuf::from(r"C:\test\base.apk"),
            data_dir: std::path::PathBuf::from(r"C:\test\data"),
            native_lib_dir: std::path::PathBuf::from(r"C:\test\lib"),
            install_time: "2026-10-25".to_string(),
            size_bytes: 1024,
        };

        registry.register(app.clone()).expect("Failed to register");
        assert!(registry.find("org.droidbridge.testapp").is_some());

        let removed = registry.unregister("org.droidbridge.testapp").expect("Failed to unregister");
        assert!(removed);
    }
}
