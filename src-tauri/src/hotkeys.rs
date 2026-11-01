use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterError {
    Invalid,
    Conflict,
}

pub trait HotkeyRegistrar {
    fn register(&mut self, key: &str) -> Result<(), RegisterError>;
    fn unregister(&mut self, key: &str);
}

/// 登録できているホットキー。登録できていないほうは None。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Hotkeys {
    pub toggle: Option<String>,
    pub capture: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum HotkeyError {
    Duplicate,
    Invalid(String),
    Conflict(String),
}

pub fn apply_hotkeys(
    registrar: &mut dyn HotkeyRegistrar,
    current: &Hotkeys,
    toggle: &str,
    capture: &str,
) -> Result<Hotkeys, HotkeyError> {
    let (toggle, capture) = (toggle.trim(), capture.trim());
    for key in [toggle, capture] {
        if key.is_empty() {
            return Err(HotkeyError::Invalid(key.to_string()));
        }
    }
    if toggle.eq_ignore_ascii_case(capture) {
        return Err(HotkeyError::Duplicate);
    }

    let wanted = [(&current.toggle, toggle), (&current.capture, capture)];
    let changed: Vec<(&Option<String>, &str)> =
        wanted.into_iter().filter(|(old, new)| old.as_deref() != Some(*new)).collect();

    for (old, _) in &changed {
        if let Some(old) = old {
            registrar.unregister(old);
        }
    }

    let mut registered: Vec<&str> = vec![];
    for (_, new) in &changed {
        if let Err(error) = registrar.register(new) {
            for key in registered {
                registrar.unregister(key);
            }
            for (old, _) in &changed {
                if let Some(old) = old {
                    let _ = registrar.register(old);
                }
            }
            return Err(match error {
                RegisterError::Conflict => HotkeyError::Conflict(new.to_string()),
                RegisterError::Invalid => HotkeyError::Invalid(new.to_string()),
            });
        }
        registered.push(new);
    }
    Ok(Hotkeys { toggle: Some(toggle.to_string()), capture: Some(capture.to_string()) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};

    #[derive(Default)]
    struct Fake {
        registered: BTreeSet<String>,
        fail: HashMap<String, RegisterError>,
        calls: Vec<String>,
    }

    impl HotkeyRegistrar for Fake {
        fn register(&mut self, key: &str) -> Result<(), RegisterError> {
            self.calls.push(format!("register {key}"));
            if let Some(error) = self.fail.get(key) {
                return Err(error.clone());
            }
            if !self.registered.insert(key.to_string()) {
                return Err(RegisterError::Conflict);
            }
            Ok(())
        }
        fn unregister(&mut self, key: &str) {
            self.calls.push(format!("unregister {key}"));
            self.registered.remove(key);
        }
    }

    fn started_with(toggle: &str, capture: &str) -> (Fake, Hotkeys) {
        let mut fake = Fake::default();
        fake.registered.insert(toggle.to_string());
        fake.registered.insert(capture.to_string());
        let current = Hotkeys { toggle: Some(toggle.to_string()), capture: Some(capture.to_string()) };
        (fake, current)
    }

    fn keys(fake: &Fake) -> Vec<&str> {
        fake.registered.iter().map(String::as_str).collect()
    }

    fn pair(toggle: &str, capture: &str) -> Hotkeys {
        Hotkeys { toggle: Some(toggle.to_string()), capture: Some(capture.to_string()) }
    }

    #[test]
    fn changing_both_keys_registers_the_new_ones_and_releases_the_old_ones() {
        let (mut fake, current) = started_with("F6", "F7");
        let result = apply_hotkeys(&mut fake, &current, "F8", "F9");
        assert_eq!(result, Ok(pair("F8", "F9")));
        assert_eq!(keys(&fake), vec!["F8", "F9"]);
    }

    #[test]
    fn a_key_that_does_not_change_is_left_alone() {
        let (mut fake, current) = started_with("F6", "F7");
        let result = apply_hotkeys(&mut fake, &current, "F6", "F9");
        assert_eq!(result, Ok(pair("F6", "F9")));
        assert_eq!(fake.calls, vec!["unregister F7", "register F9"]);
        assert_eq!(keys(&fake), vec!["F6", "F9"]);
    }

    #[test]
    fn a_conflict_restores_the_old_keys_and_names_the_conflicting_one() {
        let (mut fake, current) = started_with("F6", "F7");
        fake.fail.insert("F9".to_string(), RegisterError::Conflict);
        let result = apply_hotkeys(&mut fake, &current, "F8", "F9");
        assert_eq!(result, Err(HotkeyError::Conflict("F9".to_string())));
        assert_eq!(keys(&fake), vec!["F6", "F7"]);
    }

    #[test]
    fn a_conflict_on_the_first_key_leaves_the_second_untouched_and_restores_both() {
        let (mut fake, current) = started_with("F6", "F7");
        fake.fail.insert("F8".to_string(), RegisterError::Conflict);
        let result = apply_hotkeys(&mut fake, &current, "F8", "F9");
        assert_eq!(result, Err(HotkeyError::Conflict("F8".to_string())));
        assert_eq!(keys(&fake), vec!["F6", "F7"]);
    }

    #[test]
    fn using_the_same_key_for_both_is_refused_before_touching_anything() {
        let (mut fake, current) = started_with("F6", "F7");
        assert_eq!(apply_hotkeys(&mut fake, &current, "F8", "F8"), Err(HotkeyError::Duplicate));
        assert_eq!(apply_hotkeys(&mut fake, &current, "f8", " F8 "), Err(HotkeyError::Duplicate));
        assert!(fake.calls.is_empty());
        assert_eq!(keys(&fake), vec!["F6", "F7"]);
    }

    #[test]
    fn an_empty_key_is_invalid_and_changes_nothing() {
        let (mut fake, current) = started_with("F6", "F7");
        assert_eq!(apply_hotkeys(&mut fake, &current, "  ", "F9"), Err(HotkeyError::Invalid(String::new())));
        assert!(fake.calls.is_empty());
    }

    #[test]
    fn an_unrecognized_key_name_restores_the_old_keys() {
        let (mut fake, current) = started_with("F6", "F7");
        fake.fail.insert("Banana".to_string(), RegisterError::Invalid);
        let result = apply_hotkeys(&mut fake, &current, "F8", "Banana");
        assert_eq!(result, Err(HotkeyError::Invalid("Banana".to_string())));
        assert_eq!(keys(&fake), vec!["F6", "F7"]);
    }

    #[test]
    fn swapping_the_two_keys_works() {
        let (mut fake, current) = started_with("F6", "F7");
        let result = apply_hotkeys(&mut fake, &current, "F7", "F6");
        assert_eq!(result, Ok(pair("F7", "F6")));
        assert_eq!(keys(&fake), vec!["F6", "F7"]);
    }

    #[test]
    fn a_key_that_failed_at_startup_can_be_replaced() {
        let mut fake = Fake::default();
        fake.registered.insert("F7".to_string());
        let current = Hotkeys { toggle: None, capture: Some("F7".to_string()) };
        let result = apply_hotkeys(&mut fake, &current, "F8", "F7");
        assert_eq!(result, Ok(pair("F8", "F7")));
        assert_eq!(keys(&fake), vec!["F7", "F8"]);
        assert_eq!(fake.calls, vec!["register F8"]);
    }

    #[test]
    fn surrounding_spaces_are_trimmed_from_the_stored_names() {
        let (mut fake, current) = started_with("F6", "F7");
        assert_eq!(apply_hotkeys(&mut fake, &current, " F8 ", "F9	"), Ok(pair("F8", "F9")));
    }

    #[test]
    fn hotkey_errors_serialize_to_json_the_frontend_reads() {
        use serde_json::json;
        assert_eq!(serde_json::to_value(HotkeyError::Duplicate).unwrap(), json!({"type":"duplicate"}));
        assert_eq!(
            serde_json::to_value(HotkeyError::Conflict("F9".to_string())).unwrap(),
            json!({"type":"conflict","data":"F9"})
        );
        assert_eq!(
            serde_json::to_value(HotkeyError::Invalid("x".to_string())).unwrap(),
            json!({"type":"invalid","data":"x"})
        );
    }
}
