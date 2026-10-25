use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum BundleValue {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    Boolean(bool),
    Bundle(Box<Bundle>),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bundle {
    data: HashMap<String, BundleValue>,
}

impl Bundle {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn put_string(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), BundleValue::String(value.to_string()));
    }

    pub fn get_string<'a>(&'a self, key: &str) -> Option<&'a str> {
        match self.data.get(key) {
            Some(BundleValue::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn put_int(&mut self, key: &str, value: i32) {
        self.data.insert(key.to_string(), BundleValue::Int(value));
    }

    pub fn get_int(&self, key: &str, default: i32) -> i32 {
        match self.data.get(key) {
            Some(BundleValue::Int(v)) => *v,
            _ => default,
        }
    }

    pub fn put_boolean(&mut self, key: &str, value: bool) {
        self.data.insert(key.to_string(), BundleValue::Boolean(value));
    }

    pub fn get_boolean(&self, key: &str, default: bool) -> bool {
        match self.data.get(key) {
            Some(BundleValue::Boolean(v)) => *v,
            _ => default,
        }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}
