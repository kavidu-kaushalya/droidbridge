use std::collections::HashMap;
use std::io::{Read, Seek};
use crate::dex::{ClassInfo, DexError, DexFile, MethodInfo};
use crate::zip::ZipArchive;

#[derive(Default)]
pub struct MultiDexArchive {
    pub dex_data: Vec<Vec<u8>>,
    pub classes: Vec<ClassInfo>,
    pub class_map: HashMap<String, usize>, // normalized class descriptor -> index in `classes`
    pub method_map: HashMap<String, (usize, usize)>, // "Lcom/pkg/Class;->method(sig)" -> (class_idx, method_idx)
}

impl MultiDexArchive {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_zip<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Self, DexError> {
        let mut multi = Self::new();
        let entries = archive.entries().to_vec();

        // 1. Gather all classes*.dex entries
        let mut dex_names: Vec<String> = entries
            .into_iter()
            .filter(|name| {
                if name == "classes.dex" {
                    true
                } else if name.starts_with("classes") && name.ends_with(".dex") {
                    let num_part = &name[7..name.len() - 4];
                    num_part.chars().all(|c| c.is_ascii_digit())
                } else {
                    false
                }
            })
            .collect();

        // Sort so classes.dex is first, followed by classes2.dex, classes3.dex ...
        dex_names.sort_by(|a, b| {
            let num_a = if a == "classes.dex" { 1 } else { a[7..a.len() - 4].parse::<u32>().unwrap_or(999) };
            let num_b = if b == "classes.dex" { 1 } else { b[7..b.len() - 4].parse::<u32>().unwrap_or(999) };
            num_a.cmp(&num_b)
        });

        println!("[MultiDex] Found {} DEX archive(s) in APK", dex_names.len());

        // 2. Parse each DEX file and index classes and methods
        for dex_name in dex_names {
            if let Ok(bytes) = archive.read_file(&dex_name) {
                if let Ok(parsed) = DexFile::parse(&bytes) {
                    println!("    -> Loaded {}: {} classes, {} methods", dex_name, parsed.classes.len(), parsed.methods.len());
                    for class in parsed.classes {
                        let class_idx = multi.classes.len();
                        let normalized_name = Self::normalize_descriptor(&class.class_name);
                        multi.class_map.insert(normalized_name.clone(), class_idx);

                        for (method_idx, method) in class.methods.iter().enumerate() {
                            let key = format!("{}->{}{}", normalized_name, method.method_name, method.signature);
                            multi.method_map.insert(key, (class_idx, method_idx));

                            // Also index simple form: "com.pkg.Class.methodName"
                            let dot_key = format!("{}.{}", normalized_name.trim_start_matches('L').trim_end_matches(';').replace('/', "."), method.method_name);
                            multi.method_map.insert(dot_key, (class_idx, method_idx));
                        }

                        multi.classes.push(class);
                    }
                    multi.dex_data.push(bytes);
                }
            }
        }

        println!("[MultiDex] Indexed {} total classes across multi-DEX files", multi.classes.len());
        Ok(multi)
    }

    pub fn find_class(&self, name: &str) -> Option<&ClassInfo> {
        let norm = Self::normalize_descriptor(name);
        if let Some(&idx) = self.class_map.get(&norm) {
            return self.classes.get(idx);
        }
        // Try dot notation fallback
        let dot_norm = norm.trim_start_matches('L').trim_end_matches(';').replace('/', ".");
        for (k, &idx) in &self.class_map {
            let k_dot = k.trim_start_matches('L').trim_end_matches(';').replace('/', ".");
            if k_dot == dot_norm {
                return self.classes.get(idx);
            }
        }
        None
    }

    pub fn find_method(&self, class_name: &str, method_name: &str) -> Option<&MethodInfo> {
        if let Some(class) = self.find_class(class_name) {
            for method in &class.methods {
                if method.method_name == method_name {
                    return Some(method);
                }
            }
        }
        None
    }

    fn normalize_descriptor(desc: &str) -> String {
        let trimmed = desc.trim();
        if trimmed.starts_with('L') && trimmed.ends_with(';') {
            trimmed.to_string()
        } else if trimmed.contains('.') {
            format!("L{};", trimmed.replace('.', "/"))
        } else if trimmed.contains('/') {
            format!("L{};", trimmed)
        } else {
            trimmed.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_descriptor() {
        assert_eq!(
            MultiDexArchive::normalize_descriptor("com.whatsapp.Main"),
            "Lcom/whatsapp/Main;"
        );
        assert_eq!(
            MultiDexArchive::normalize_descriptor("Lcom/whatsapp/Main;"),
            "Lcom/whatsapp/Main;"
        );
        assert_eq!(
            MultiDexArchive::normalize_descriptor("com/whatsapp/Main"),
            "Lcom/whatsapp/Main;"
        );
    }
}
