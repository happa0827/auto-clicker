use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::model::{Preset, Sequence, Settings};

const SETTINGS_FILE: &str = "settings.json";
const SEQUENCE_FILE: &str = "sequence.json";
const PRESETS_FILE: &str = "presets.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Io(String),
    NotFound,
    InvalidName,
}

pub struct Store {
    dir: PathBuf,
}

fn io<E: std::fmt::Display>(error: E) -> StoreError {
    StoreError::Io(error.to_string())
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, file: &str) -> PathBuf {
        self.dir.join(file)
    }

    fn read<T: DeserializeOwned>(&self, file: &str) -> Option<T> {
        let text = std::fs::read_to_string(self.path(file)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// 一時ファイルに書いてから置き換える。途中で落ちても元のファイルは壊れない。
    fn write<T: Serialize>(&self, file: &str, value: &T) -> Result<(), StoreError> {
        std::fs::create_dir_all(&self.dir).map_err(io)?;
        let text = serde_json::to_string_pretty(value).map_err(io)?;
        let tmp = self.path(&format!("{file}.tmp"));
        std::fs::write(&tmp, text).map_err(io)?;
        std::fs::rename(&tmp, self.path(file)).map_err(io)
    }

    pub fn load_settings(&self) -> Settings {
        self.read(SETTINGS_FILE).unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), StoreError> {
        self.write(SETTINGS_FILE, settings)
    }

    pub fn load_sequence(&self) -> Option<Sequence> {
        self.read(SEQUENCE_FILE)
    }

    pub fn save_sequence(&self, sequence: &Sequence) -> Result<(), StoreError> {
        self.write(SEQUENCE_FILE, sequence)
    }

    /// 壊れていたら削除せず `.corrupt` に退避して、空の一覧で続行する。
    fn read_presets(&self) -> Vec<Preset> {
        let path = self.path(PRESETS_FILE);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return vec![];
        };
        serde_json::from_str(&text).unwrap_or_else(|_| {
            let _ = std::fs::rename(&path, self.path(&format!("{PRESETS_FILE}.corrupt")));
            vec![]
        })
    }

    pub fn save_preset(&self, mut preset: Preset) -> Result<(), StoreError> {
        preset.name = preset.name.trim().to_string();
        if preset.name.is_empty() {
            return Err(StoreError::InvalidName);
        }
        let mut presets = self.read_presets();
        presets.retain(|p| p.name != preset.name);
        presets.push(preset);
        presets.sort_by(|a, b| a.name.cmp(&b.name));
        self.write(PRESETS_FILE, &presets)
    }

    pub fn list_presets(&self) -> Vec<Preset> {
        self.read_presets()
    }

    pub fn load_preset(&self, name: &str) -> Result<Preset, StoreError> {
        self.read_presets().into_iter().find(|p| p.name == name).ok_or(StoreError::NotFound)
    }

    pub fn delete_preset(&self, name: &str) -> Result<(), StoreError> {
        let mut presets = self.read_presets();
        let before = presets.len();
        presets.retain(|p| p.name != name);
        if presets.len() == before {
            return Err(StoreError::NotFound);
        }
        self.write(PRESETS_FILE, &presets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use tempfile::TempDir;

    fn store() -> (TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        (dir, store)
    }

    #[test]
    fn missing_files_give_defaults() {
        let (_dir, store) = store();
        assert_eq!(store.load_settings(), Settings::default());
        assert_eq!(store.load_sequence(), None);
    }

    #[test]
    fn saved_settings_are_restored_by_a_new_store() {
        let (dir, store) = store();
        let settings = Settings {
            toggle_hotkey: "F9".to_string(),
            stop: StopCondition::Rounds { count: 7 },
            autostart: true,
            ..Settings::default()
        };
        store.save_settings(&settings).unwrap();
        assert_eq!(Store::new(dir.path()).load_settings(), settings);
    }

    fn sample_sequence() -> Sequence {
        let target = ClickTarget {
            position: Position::Fixed { x: 3, y: 4 },
            button: Button::Middle,
            kind: ClickKind::Hold { duration: Interval::new(1.5, TimeUnit::Seconds).unwrap() },
        };
        Sequence::single(target, Interval::new(250.0, TimeUnit::Milliseconds).unwrap())
    }

    #[test]
    fn the_last_sequence_is_restored_by_a_new_store() {
        let (dir, store) = store();
        store.save_sequence(&sample_sequence()).unwrap();
        assert_eq!(Store::new(dir.path()).load_sequence(), Some(sample_sequence()));
    }

    #[test]
    fn saving_creates_a_missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        let store = Store::new(&nested);
        store.save_settings(&Settings::default()).unwrap();
        assert!(nested.join("settings.json").exists());
    }

    #[test]
    fn corrupt_settings_and_sequence_fall_back_without_failing() {
        let (dir, store) = store();
        std::fs::write(dir.path().join("settings.json"), "{ not json").unwrap();
        std::fs::write(dir.path().join("sequence.json"), r#"{"steps":[]}"#).unwrap();
        assert_eq!(store.load_settings(), Settings::default());
        assert_eq!(store.load_sequence(), None);
    }

    #[test]
    fn a_save_leaves_no_temporary_file_and_replaces_the_old_content() {
        let (dir, store) = store();
        store.save_settings(&Settings::default()).unwrap();
        let second = Settings { autostart: true, ..Settings::default() };
        store.save_settings(&second).unwrap();
        assert_eq!(store.load_settings(), second);
        let names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["settings.json".to_string()]);
    }

    fn preset(name: &str, x: i32) -> Preset {
        let target = ClickTarget {
            position: Position::Fixed { x, y: 0 },
            button: Button::Left,
            kind: ClickKind::Single,
        };
        Preset {
            name: name.to_string(),
            sequence: Sequence::single(target, Interval::new(100.0, TimeUnit::Milliseconds).unwrap()),
        }
    }

    #[test]
    fn a_saved_preset_can_be_loaded_by_name_after_a_restart() {
        let (dir, store) = store();
        store.save_preset(preset("login", 1)).unwrap();
        assert_eq!(Store::new(dir.path()).load_preset("login"), Ok(preset("login", 1)));
    }

    #[test]
    fn saving_under_an_existing_name_overwrites_it() {
        let (_dir, store) = store();
        store.save_preset(preset("a", 1)).unwrap();
        store.save_preset(preset("a", 2)).unwrap();
        assert_eq!(store.list_presets(), vec![preset("a", 2)]);
    }

    #[test]
    fn presets_are_listed_by_name() {
        let (_dir, store) = store();
        for name in ["charlie", "alpha", "bravo"] {
            store.save_preset(preset(name, 0)).unwrap();
        }
        let names: Vec<String> = store.list_presets().into_iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn deleting_removes_only_that_preset() {
        let (dir, store) = store();
        store.save_preset(preset("a", 1)).unwrap();
        store.save_preset(preset("b", 2)).unwrap();
        store.delete_preset("a").unwrap();
        assert_eq!(Store::new(dir.path()).list_presets(), vec![preset("b", 2)]);
    }

    #[test]
    fn unknown_names_are_reported_as_not_found() {
        let (_dir, store) = store();
        store.save_preset(preset("a", 1)).unwrap();
        assert_eq!(store.load_preset("zzz"), Err(StoreError::NotFound));
        assert_eq!(store.delete_preset("zzz"), Err(StoreError::NotFound));
        assert_eq!(store.list_presets(), vec![preset("a", 1)]);
    }

    #[test]
    fn empty_or_blank_names_are_rejected() {
        let (_dir, store) = store();
        assert_eq!(store.save_preset(preset("", 1)), Err(StoreError::InvalidName));
        assert_eq!(store.save_preset(preset("   	", 1)), Err(StoreError::InvalidName));
        assert!(store.list_presets().is_empty());
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_from_names() {
        let (_dir, store) = store();
        store.save_preset(preset("  daily  ", 1)).unwrap();
        assert_eq!(store.load_preset("daily"), Ok(preset("daily", 1)));
        store.save_preset(preset("daily", 2)).unwrap();
        assert_eq!(store.list_presets(), vec![preset("daily", 2)]);
    }

    #[test]
    fn a_corrupt_preset_file_is_set_aside_not_deleted() {
        let (dir, store) = store();
        let broken = "[{ definitely not json";
        std::fs::write(dir.path().join("presets.json"), broken).unwrap();

        assert!(store.list_presets().is_empty());
        assert_eq!(std::fs::read_to_string(dir.path().join("presets.json.corrupt")).unwrap(), broken);
        assert!(!dir.path().join("presets.json").exists());

        store.save_preset(preset("fresh", 1)).unwrap();
        assert_eq!(Store::new(dir.path()).list_presets(), vec![preset("fresh", 1)]);
        assert_eq!(std::fs::read_to_string(dir.path().join("presets.json.corrupt")).unwrap(), broken);
    }

    #[test]
    fn saving_over_a_corrupt_preset_file_keeps_the_old_bytes_aside() {
        let (dir, store) = store();
        let broken = "garbage";
        std::fs::write(dir.path().join("presets.json"), broken).unwrap();
        store.save_preset(preset("new", 1)).unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("presets.json.corrupt")).unwrap(), broken);
        assert_eq!(store.list_presets(), vec![preset("new", 1)]);
    }
}
