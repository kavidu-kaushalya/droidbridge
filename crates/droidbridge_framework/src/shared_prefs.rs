use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum PrefValue {
    String(String),
    Int(i32),
    Boolean(bool),
    Long(i64),
    Float(f32),
}

pub struct SharedPreferences {
    file_path: PathBuf,
    data: HashMap<String, PrefValue>,
}

impl SharedPreferences {
    pub fn open<P: AsRef<Path>>(file_path: P) -> Self {
        let path = file_path.as_ref().to_path_buf();
        let mut data = HashMap::new();

        if path.exists() {
            if let Ok(mut file) = File::open(&path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    data = Self::parse_simple_json(&content);
                }
            }
        }

        Self { file_path: path, data }
    }

    pub fn get_string<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        match self.data.get(key) {
            Some(PrefValue::String(s)) => s.as_str(),
            _ => default,
        }
    }

    pub fn get_int(&self, key: &str, default: i32) -> i32 {
        match self.data.get(key) {
            Some(PrefValue::Int(v)) => *v,
            _ => default,
        }
    }

    pub fn get_boolean(&self, key: &str, default: bool) -> bool {
        match self.data.get(key) {
            Some(PrefValue::Boolean(v)) => *v,
            _ => default,
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn edit(&self) -> Editor {
        Editor {
            file_path: self.file_path.clone(),
            changes: HashMap::new(),
            current_data: self.data.clone(),
        }
    }

    fn parse_simple_json(json: &str) -> HashMap<String, PrefValue> {
        let mut map = HashMap::new();
        // Lightweight parser for "key": "value" or "key": 123 or "key": true
        for line in json.lines() {
            let line = line.trim();
            if let Some(colon_pos) = line.find(':') {
                let key_part = line[..colon_pos].trim().trim_matches('"');
                let val_part = line[colon_pos + 1..].trim().trim_end_matches(',').trim();

                if val_part == "true" {
                    map.insert(key_part.to_string(), PrefValue::Boolean(true));
                } else if val_part == "false" {
                    map.insert(key_part.to_string(), PrefValue::Boolean(false));
                } else if val_part.starts_with('"') && val_part.ends_with('"') {
                    let str_val = val_part.trim_matches('"');
                    map.insert(key_part.to_string(), PrefValue::String(str_val.to_string()));
                } else if let Ok(int_val) = val_part.parse::<i32>() {
                    map.insert(key_part.to_string(), PrefValue::Int(int_val));
                }
            }
        }
        map
    }
}

pub struct Editor {
    file_path: PathBuf,
    changes: HashMap<String, Option<PrefValue>>,
    current_data: HashMap<String, PrefValue>,
}

impl Editor {
    pub fn put_string(mut self, key: &str, value: &str) -> Self {
        self.changes.insert(key.to_string(), Some(PrefValue::String(value.to_string())));
        self
    }

    pub fn put_int(mut self, key: &str, value: i32) -> Self {
        self.changes.insert(key.to_string(), Some(PrefValue::Int(value)));
        self
    }

    pub fn put_boolean(mut self, key: &str, value: bool) -> Self {
        self.changes.insert(key.to_string(), Some(PrefValue::Boolean(value)));
        self
    }

    pub fn remove(mut self, key: &str) -> Self {
        self.changes.insert(key.to_string(), None);
        self
    }

    pub fn commit(mut self) -> bool {
        for (k, v) in self.changes {
            match v {
                Some(val) => { self.current_data.insert(k, val); }
                None => { self.current_data.remove(&k); }
            }
        }

        // Serialize to simple JSON
        let mut json = String::from("{\n");
        let len = self.current_data.len();
        for (i, (k, v)) in self.current_data.iter().enumerate() {
            let comma = if i + 1 < len { "," } else { "" };
            match v {
                PrefValue::String(s) => json.push_str(&format!("  \"{}\": \"{}\"{}\n", k, s, comma)),
                PrefValue::Int(n) => json.push_str(&format!("  \"{}\": {}{}\n", k, n, comma)),
                PrefValue::Boolean(b) => json.push_str(&format!("  \"{}\": {}{}\n", k, b, comma)),
                PrefValue::Long(n) => json.push_str(&format!("  \"{}\": {}{}\n", k, n, comma)),
                PrefValue::Float(f) => json.push_str(&format!("  \"{}\": {}{}\n", k, f, comma)),
            }
        }
        json.push_str("}\n");

        if let Some(parent) = self.file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(mut f) = File::create(&self.file_path) {
            f.write_all(json.as_bytes()).is_ok()
        } else {
            false
        }
    }

    pub fn apply(self) {
        let _ = self.commit();
    }
}
