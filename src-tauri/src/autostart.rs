use serde::Serialize;

pub trait AutostartControl {
    fn is_enabled(&self) -> Result<bool, String>;
    fn enable(&mut self) -> Result<(), String>;
    fn disable(&mut self) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AutostartError(pub String);

pub fn sync_autostart(control: &mut dyn AutostartControl, desired: bool) -> Result<(), AutostartError> {
    let current = control.is_enabled().map_err(AutostartError)?;
    if current == desired {
        return Ok(());
    }
    let result = if desired { control.enable() } else { control.disable() };
    result.map_err(AutostartError)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        enabled: bool,
        calls: Vec<&'static str>,
        fail_enable: bool,
        fail_status: bool,
    }

    impl Fake {
        fn new(enabled: bool) -> Self {
            Self { enabled, calls: vec![], fail_enable: false, fail_status: false }
        }
    }

    impl AutostartControl for Fake {
        fn is_enabled(&self) -> Result<bool, String> {
            if self.fail_status {
                return Err("cannot read".to_string());
            }
            Ok(self.enabled)
        }
        fn enable(&mut self) -> Result<(), String> {
            self.calls.push("enable");
            if self.fail_enable {
                return Err("denied".to_string());
            }
            self.enabled = true;
            Ok(())
        }
        fn disable(&mut self) -> Result<(), String> {
            self.calls.push("disable");
            self.enabled = false;
            Ok(())
        }
    }

    #[test]
    fn turns_it_on_when_it_is_off() {
        let mut fake = Fake::new(false);
        assert_eq!(sync_autostart(&mut fake, true), Ok(()));
        assert!(fake.enabled);
        assert_eq!(fake.calls, vec!["enable"]);
    }

    #[test]
    fn turns_it_off_when_it_is_on() {
        let mut fake = Fake::new(true);
        assert_eq!(sync_autostart(&mut fake, false), Ok(()));
        assert!(!fake.enabled);
        assert_eq!(fake.calls, vec!["disable"]);
    }

    #[test]
    fn does_nothing_when_the_state_already_matches() {
        let mut on = Fake::new(true);
        assert_eq!(sync_autostart(&mut on, true), Ok(()));
        let mut off = Fake::new(false);
        assert_eq!(sync_autostart(&mut off, false), Ok(()));
        assert!(on.calls.is_empty() && off.calls.is_empty());
    }

    #[test]
    fn an_os_failure_is_reported_and_the_state_stays_off() {
        let mut fake = Fake::new(false);
        fake.fail_enable = true;
        assert_eq!(sync_autostart(&mut fake, true), Err(AutostartError("denied".to_string())));
        assert!(!fake.enabled);
    }

    #[test]
    fn not_being_able_to_read_the_state_is_an_error_and_changes_nothing() {
        let mut fake = Fake::new(false);
        fake.fail_status = true;
        assert_eq!(sync_autostart(&mut fake, true), Err(AutostartError("cannot read".to_string())));
        assert!(fake.calls.is_empty());
    }

    #[test]
    fn the_error_serializes_as_its_message() {
        assert_eq!(serde_json::to_value(AutostartError("denied".to_string())).unwrap(), serde_json::json!("denied"));
    }
}
