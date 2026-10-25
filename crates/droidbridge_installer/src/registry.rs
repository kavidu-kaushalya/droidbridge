use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct InstalledApp {
    pub package_name: String,
    pub app_label: String,
    pub version_name: String,
    pub version_code: u32,
    pub main_activity: String,
    pub apk_path: PathBuf,
    pub data_dir: PathBuf,
    pub native_lib_dir: PathBuf,
    pub install_time: String,
    pub size_bytes: u64,
}

pub struct AppRegistry {
    registry_file: PathBuf,
    pub apps: Vec<InstalledApp>,
}

impl AppRegistry {
    pub fn get_default_storage_root() -> PathBuf {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| r"C:\Users\Default\AppData\Local".to_string());
        PathBuf::from(local_app_data).join("DroidBridge")
    }

    pub fn load() -> Self {
        let root = Self::get_default_storage_root();
        let registry_file = root.join("installed_apps.json");
        let mut apps = Vec::new();

        if registry_file.exists() {
            if let Ok(mut f) = File::open(&registry_file) {
                let mut content = String::new();
                if f.read_to_string(&mut content).is_ok() {
                    apps = Self::parse_apps_json(&content);
                }
            }
        }

        Self {
            registry_file,
            apps,
        }
    }

    pub fn register(&mut self, app: InstalledApp) -> Result<(), String> {
        self.apps.retain(|a| a.package_name != app.package_name);
        self.apps.push(app);
        self.save()
    }

    pub fn unregister(&mut self, package_name: &str) -> Result<bool, String> {
        let prev_len = self.apps.len();
        self.apps.retain(|a| a.package_name != package_name);
        let removed = self.apps.len() < prev_len;
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    pub fn find(&self, package_name: &str) -> Option<&InstalledApp> {
        self.apps.iter().find(|a| a.package_name == package_name)
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.registry_file.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut json = String::from("[\n");
        let len = self.apps.len();
        for (i, app) in self.apps.iter().enumerate() {
            let comma = if i + 1 < len { "," } else { "" };
            json.push_str("  {\n");
            json.push_str(&format!("    \"package_name\": \"{}\",\n", app.package_name));
            json.push_str(&format!("    \"app_label\": \"{}\",\n", app.app_label));
            json.push_str(&format!("    \"version_name\": \"{}\",\n", app.version_name));
            json.push_str(&format!("    \"version_code\": {},\n", app.version_code));
            json.push_str(&format!("    \"main_activity\": \"{}\",\n", app.main_activity));
            json.push_str(&format!("    \"apk_path\": \"{}\",\n", app.apk_path.to_string_lossy().replace('\\', "\\\\")));
            json.push_str(&format!("    \"data_dir\": \"{}\",\n", app.data_dir.to_string_lossy().replace('\\', "\\\\")));
            json.push_str(&format!("    \"native_lib_dir\": \"{}\",\n", app.native_lib_dir.to_string_lossy().replace('\\', "\\\\")));
            json.push_str(&format!("    \"install_time\": \"{}\",\n", app.install_time));
            json.push_str(&format!("    \"size_bytes\": {}\n", app.size_bytes));
            json.push_str(&format!("  }}{}\n", comma));
        }
        json.push_str("]\n");

        let mut f = File::create(&self.registry_file).map_err(|e| format!("Cannot save registry: {}", e))?;
        f.write_all(json.as_bytes()).map_err(|e| format!("Cannot write registry: {}", e))?;
        Ok(())
    }

    fn parse_apps_json(json: &str) -> Vec<InstalledApp> {
        let mut apps = Vec::new();
        // Parse simple JSON objects
        let blocks: Vec<&str> = json.split('{').collect();
        for block in blocks.iter().skip(1) {
            let block_str = block.split('}').next().unwrap_or("");
            let mut pkg = String::new();
            let mut label = String::new();
            let mut ver_name = String::new();
            let mut ver_code = 1u32;
            let mut act = String::new();
            let mut apk_p = PathBuf::new();
            let mut data_d = PathBuf::new();
            let mut lib_d = PathBuf::new();
            let mut inst_time = String::new();
            let mut size = 0u64;

            for line in block_str.lines() {
                let line = line.trim().trim_end_matches(',');
                if let Some(pos) = line.find(':') {
                    let k = line[..pos].trim().trim_matches('"');
                    let v = line[pos + 1..].trim().trim_matches('"');
                    match k {
                        "package_name" => pkg = v.to_string(),
                        "app_label" => label = v.to_string(),
                        "version_name" => ver_name = v.to_string(),
                        "version_code" => ver_code = v.parse().unwrap_or(1),
                        "main_activity" => act = v.to_string(),
                        "apk_path" => apk_p = PathBuf::from(v.replace("\\\\", "\\")),
                        "data_dir" => data_d = PathBuf::from(v.replace("\\\\", "\\")),
                        "native_lib_dir" => lib_d = PathBuf::from(v.replace("\\\\", "\\")),
                        "install_time" => inst_time = v.to_string(),
                        "size_bytes" => size = v.parse().unwrap_or(0),
                        _ => {}
                    }
                }
            }

            if !pkg.is_empty() {
                apps.push(InstalledApp {
                    package_name: pkg,
                    app_label: label,
                    version_name: ver_name,
                    version_code: ver_code,
                    main_activity: act,
                    apk_path: apk_p,
                    data_dir: data_d,
                    native_lib_dir: lib_d,
                    install_time: inst_time,
                    size_bytes: size,
                });
            }
        }

        apps
    }
}
