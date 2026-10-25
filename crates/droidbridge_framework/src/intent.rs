use std::collections::HashSet;
use crate::bundle::Bundle;

#[derive(Debug, Clone, PartialEq)]
pub struct Intent {
    pub action: Option<String>,
    pub categories: HashSet<String>,
    pub data_uri: Option<String>,
    pub component_package: Option<String>,
    pub component_class: Option<String>,
    pub extras: Bundle,
    pub flags: u32,
}

impl Intent {
    pub const ACTION_MAIN: &'static str = "android.intent.action.MAIN";
    pub const ACTION_VIEW: &'static str = "android.intent.action.VIEW";
    pub const CATEGORY_LAUNCHER: &'static str = "android.intent.category.LAUNCHER";
    pub const CATEGORY_DEFAULT: &'static str = "android.intent.category.DEFAULT";

    pub fn new() -> Self {
        Self {
            action: None,
            categories: HashSet::new(),
            data_uri: None,
            component_package: None,
            component_class: None,
            extras: Bundle::new(),
            flags: 0,
        }
    }

    pub fn with_action(action: &str) -> Self {
        let mut intent = Self::new();
        intent.action = Some(action.to_string());
        intent
    }

    pub fn with_component(package: &str, class_name: &str) -> Self {
        let mut intent = Self::new();
        intent.component_package = Some(package.to_string());
        intent.component_class = Some(class_name.to_string());
        intent
    }

    pub fn add_category(&mut self, category: &str) {
        self.categories.insert(category.to_string());
    }

    pub fn set_data(&mut self, uri: &str) {
        self.data_uri = Some(uri.to_string());
    }

    pub fn put_extra_string(&mut self, key: &str, value: &str) {
        self.extras.put_string(key, value);
    }

    pub fn put_extra_int(&mut self, key: &str, value: i32) {
        self.extras.put_int(key, value);
    }

    pub fn matches_filter(&self, action: Option<&str>, categories: &[String]) -> bool {
        if let Some(act) = action {
            if self.action.as_deref() != Some(act) {
                return false;
            }
        }
        for cat in &self.categories {
            if !categories.contains(cat) {
                return false;
            }
        }
        true
    }
}

impl Default for Intent {
    fn default() -> Self {
        Self::new()
    }
}
