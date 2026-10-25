pub mod bundle;
pub mod intent;
pub mod logger;
pub mod shared_prefs;

pub use bundle::{Bundle, BundleValue};
pub use intent::Intent;
pub use logger::{Log, LogPriority};
pub use shared_prefs::{Editor, PrefValue, SharedPreferences};

#[cfg(test)]
mod tests {
    use super::*;

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
