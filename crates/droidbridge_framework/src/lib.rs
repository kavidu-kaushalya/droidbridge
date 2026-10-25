pub mod bundle;
pub mod intent;
pub mod logger;
pub mod shared_prefs;
pub mod view;
pub mod activity_thread;

pub use activity_thread::ActivityThread;
pub use bundle::{Bundle, BundleValue};
pub use intent::Intent;
pub use logger::{Log, LogPriority};
pub use shared_prefs::{Editor, PrefValue, SharedPreferences};
pub use view::{Insets, LayoutInflater, LayoutParams, Orientation, Rect, SizeRule, ViewNode, ViewType};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use droidbridge_loader::{AxmlAttributeValue, AxmlNode};

    #[test]
    fn test_layout_inflation_and_measure() {
        let mut root_axml = AxmlNode {
            tag: "LinearLayout".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
        };
        root_axml.attributes.insert("orientation".to_string(), AxmlAttributeValue::String("vertical".to_string()));
        root_axml.attributes.insert("layout_width".to_string(), AxmlAttributeValue::String("match_parent".to_string()));
        root_axml.attributes.insert("layout_height".to_string(), AxmlAttributeValue::String("match_parent".to_string()));

        let mut btn_axml = AxmlNode {
            tag: "Button".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
        };
        btn_axml.attributes.insert("text".to_string(), AxmlAttributeValue::String("Submit".to_string()));
        btn_axml.attributes.insert("layout_width".to_string(), AxmlAttributeValue::String("match_parent".to_string()));
        btn_axml.attributes.insert("layout_height".to_string(), AxmlAttributeValue::Integer(50));

        root_axml.children.push(btn_axml);

        let mut inflated = LayoutInflater::inflate(&root_axml, None);
        assert_eq!(inflated.children.len(), 1);
        if let ViewType::Button { ref text, .. } = inflated.children[0].view_type {
            assert_eq!(text, "Submit");
        } else {
            panic!("Child should be Button");
        }

        // Measure & Layout
        let window_rect = Rect { left: 0, top: 0, right: 400, bottom: 600 };
        LayoutInflater::measure_and_layout(&mut inflated, window_rect);

        assert_eq!(inflated.bounds.width(), 400);
        assert_eq!(inflated.bounds.height(), 600);
        assert_eq!(inflated.children[0].bounds.width(), 400);
        assert_eq!(inflated.children[0].bounds.height(), 50);
    }

    #[test]
    fn test_bundle_operations() {
        let mut bundle = Bundle::new();
        bundle.put_string("user_name", "Nactro");
        bundle.put_int("score", 95);
        bundle.put_boolean("is_verified", true);

        assert_eq!(bundle.get_string("user_name"), Some("Nactro"));
        assert_eq!(bundle.get_int("score", 0), 95);
        assert_eq!(bundle.get_boolean("is_verified", false), true);
        assert_eq!(bundle.get_int("non_existent", 42), 42);
    }

    #[test]
    fn test_intent_matching() {
        let mut intent = Intent::with_action(Intent::ACTION_MAIN);
        intent.add_category(Intent::CATEGORY_LAUNCHER);

        let allowed_categories = vec![Intent::CATEGORY_LAUNCHER.to_string()];
        assert!(intent.matches_filter(Some(Intent::ACTION_MAIN), &allowed_categories));

        assert!(!intent.matches_filter(Some(Intent::ACTION_VIEW), &allowed_categories));
    }

    #[test]
    fn test_shared_preferences() {
        let temp_dir = std::env::temp_dir().join("droidbridge_prefs_test");
        let pref_file = temp_dir.join("settings.json");

        let prefs = SharedPreferences::open(&pref_file);
        let editor = prefs.edit();
        let success = editor
            .put_string("theme", "dark")
            .put_int("volume", 80)
            .put_boolean("notifications", true)
            .commit();

        assert!(success);

        // Reopen and verify persisted values
        let reloaded = SharedPreferences::open(&pref_file);
        assert_eq!(reloaded.get_string("theme", "light"), "dark");
        assert_eq!(reloaded.get_int("volume", 0), 80);
        assert_eq!(reloaded.get_boolean("notifications", false), true);

        // Cleanup
        let _ = std::fs::remove_file(pref_file);
    }
}
