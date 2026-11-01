use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeUnit {
    Milliseconds,
    Seconds,
    Minutes,
}

impl TimeUnit {
    fn to_millis(self, value: f64) -> f64 {
        match self {
            TimeUnit::Milliseconds => value,
            TimeUnit::Seconds => value * 1000.0,
            TimeUnit::Minutes => value * 60_000.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    IntervalTooShort,
    IntervalNotFinite,
    EmptySequence,
    ZeroRounds,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::IntervalTooShort => write!(f, "interval must be at least 1 ms"),
            ModelError::IntervalNotFinite => write!(f, "interval must be a finite number"),
            ModelError::EmptySequence => write!(f, "sequence must have at least one step"),
            ModelError::ZeroRounds => write!(f, "round count must be at least 1"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawInterval")]
pub struct Interval {
    value: f64,
    unit: TimeUnit,
}

#[derive(Deserialize)]
struct RawInterval {
    value: f64,
    unit: TimeUnit,
}

impl TryFrom<RawInterval> for Interval {
    type Error = ModelError;

    fn try_from(raw: RawInterval) -> Result<Self, ModelError> {
        Interval::new(raw.value, raw.unit)
    }
}

impl Interval {
    pub fn new(value: f64, unit: TimeUnit) -> Result<Self, ModelError> {
        if !value.is_finite() {
            return Err(ModelError::IntervalNotFinite);
        }
        if unit.to_millis(value) < 1.0 {
            return Err(ModelError::IntervalTooShort);
        }
        Ok(Self { value, unit })
    }

    pub fn as_duration(&self) -> Duration {
        Duration::from_secs_f64(self.unit.to_millis(self.value) / 1000.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Position {
    Current,
    Fixed { x: i32, y: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClickKind {
    Single,
    Double,
    Hold { duration: Interval },
}

impl ClickKind {
    pub fn hold_duration(&self) -> Option<Interval> {
        match self {
            ClickKind::Hold { duration } => Some(*duration),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClickTarget {
    pub position: Position,
    pub button: Button,
    pub kind: ClickKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub target: ClickTarget,
    pub interval: Interval,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawSequence")]
pub struct Sequence {
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct RawSequence {
    steps: Vec<Step>,
}

impl TryFrom<RawSequence> for Sequence {
    type Error = ModelError;

    fn try_from(raw: RawSequence) -> Result<Self, ModelError> {
        Sequence::new(raw.steps)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub sequence: Sequence,
}

impl Sequence {
    pub fn new(steps: Vec<Step>) -> Result<Self, ModelError> {
        if steps.is_empty() {
            return Err(ModelError::EmptySequence);
        }
        Ok(Self { steps })
    }

    pub fn single(target: ClickTarget, interval: Interval) -> Self {
        Self { steps: vec![Step { target, interval }] }
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawStopCondition", into = "RawStopCondition")]
pub enum StopCondition {
    Manual,
    Rounds { count: u32 },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RawStopCondition {
    Manual,
    Rounds { count: u32 },
}

impl TryFrom<RawStopCondition> for StopCondition {
    type Error = ModelError;

    fn try_from(raw: RawStopCondition) -> Result<Self, ModelError> {
        match raw {
            RawStopCondition::Manual => Ok(StopCondition::Manual),
            RawStopCondition::Rounds { count: 0 } => Err(ModelError::ZeroRounds),
            RawStopCondition::Rounds { count } => Ok(StopCondition::Rounds { count }),
        }
    }
}

impl From<StopCondition> for RawStopCondition {
    fn from(stop: StopCondition) -> Self {
        match stop {
            StopCondition::Manual => RawStopCondition::Manual,
            StopCondition::Rounds { count } => RawStopCondition::Rounds { count },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub toggle_hotkey: String,
    pub capture_hotkey: String,
    pub stop: StopCondition,
    pub autostart: bool,
    pub start_in_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            toggle_hotkey: "F6".to_string(),
            capture_hotkey: "F7".to_string(),
            stop: StopCondition::Manual,
            autostart: false,
            start_in_tray: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn converts_each_unit_to_duration() {
        assert_eq!(Interval::new(250.0, TimeUnit::Milliseconds).unwrap().as_duration(), Duration::from_millis(250));
        assert_eq!(Interval::new(1.5, TimeUnit::Seconds).unwrap().as_duration(), Duration::from_millis(1500));
        assert_eq!(Interval::new(2.0, TimeUnit::Minutes).unwrap().as_duration(), Duration::from_secs(120));
    }

    #[test]
    fn accepts_exactly_one_millisecond() {
        assert!(Interval::new(1.0, TimeUnit::Milliseconds).is_ok());
        assert!(Interval::new(0.001, TimeUnit::Seconds).is_ok());
    }

    #[test]
    fn rejects_below_one_millisecond() {
        assert_eq!(Interval::new(0.9, TimeUnit::Milliseconds), Err(ModelError::IntervalTooShort));
        assert_eq!(Interval::new(0.0, TimeUnit::Seconds), Err(ModelError::IntervalTooShort));
        assert_eq!(Interval::new(-5.0, TimeUnit::Minutes), Err(ModelError::IntervalTooShort));
    }

    #[test]
    fn rejects_non_finite_values() {
        assert_eq!(Interval::new(f64::NAN, TimeUnit::Milliseconds), Err(ModelError::IntervalNotFinite));
        assert_eq!(Interval::new(f64::INFINITY, TimeUnit::Seconds), Err(ModelError::IntervalNotFinite));
    }

    fn ms(v: f64) -> Interval {
        Interval::new(v, TimeUnit::Milliseconds).unwrap()
    }

    fn fixed(x: i32, y: i32) -> ClickTarget {
        ClickTarget { position: Position::Fixed { x, y }, button: Button::Left, kind: ClickKind::Single }
    }

    #[test]
    fn single_target_becomes_one_step_sequence() {
        let target = ClickTarget { position: Position::Current, button: Button::Right, kind: ClickKind::Double };
        let seq = Sequence::single(target, ms(100.0));
        assert_eq!(seq.steps().len(), 1);
        assert_eq!(seq.steps()[0].target, target);
        assert_eq!(seq.steps()[0].interval, ms(100.0));
    }

    #[test]
    fn sequence_keeps_step_order() {
        let steps = vec![
            Step { target: fixed(1, 1), interval: ms(10.0) },
            Step { target: fixed(2, 2), interval: ms(20.0) },
        ];
        let seq = Sequence::new(steps.clone()).unwrap();
        assert_eq!(seq.steps(), steps.as_slice());
    }

    #[test]
    fn empty_sequence_is_rejected() {
        assert_eq!(Sequence::new(vec![]), Err(ModelError::EmptySequence));
    }

    #[test]
    fn only_hold_clicks_carry_a_hold_duration() {
        let hold = ClickKind::Hold { duration: ms(2000.0) };
        assert_eq!(hold.hold_duration(), Some(ms(2000.0)));
        assert_eq!(ClickKind::Single.hold_duration(), None);
        assert_eq!(ClickKind::Double.hold_duration(), None);
    }

    const PRESET_JSON: &str = r#"{"name":"demo","sequence":{"steps":[{"target":{"position":{"type":"fixed","x":10,"y":20},"button":"left","kind":{"type":"hold","duration":{"value":2.0,"unit":"seconds"}}},"interval":{"value":500.0,"unit":"milliseconds"}},{"target":{"position":{"type":"current"},"button":"middle","kind":{"type":"double"}},"interval":{"value":1.0,"unit":"minutes"}}]}}"#;

    fn demo_preset() -> Preset {
        let first = Step {
            target: ClickTarget {
                position: Position::Fixed { x: 10, y: 20 },
                button: Button::Left,
                kind: ClickKind::Hold { duration: Interval::new(2.0, TimeUnit::Seconds).unwrap() },
            },
            interval: Interval::new(500.0, TimeUnit::Milliseconds).unwrap(),
        };
        let second = Step {
            target: ClickTarget { position: Position::Current, button: Button::Middle, kind: ClickKind::Double },
            interval: Interval::new(1.0, TimeUnit::Minutes).unwrap(),
        };
        Preset { name: "demo".to_string(), sequence: Sequence::new(vec![first, second]).unwrap() }
    }

    #[test]
    fn reads_the_stored_preset_format() {
        let preset: Preset = serde_json::from_str(PRESET_JSON).unwrap();
        assert_eq!(preset, demo_preset());
    }

    #[test]
    fn a_saved_preset_reads_back_the_same() {
        let json = serde_json::to_string(&demo_preset()).unwrap();
        let back: Preset = serde_json::from_str(&json).unwrap();
        assert_eq!(back, demo_preset());
    }

    #[test]
    fn stored_preset_with_no_steps_is_rejected() {
        let json = r#"{"name":"x","sequence":{"steps":[]}}"#;
        assert!(serde_json::from_str::<Preset>(json).is_err());
    }

    #[test]
    fn stored_interval_below_one_millisecond_is_rejected() {
        let json = PRESET_JSON.replace(r#"{"value":500.0,"unit":"milliseconds"}"#, r#"{"value":0.5,"unit":"milliseconds"}"#);
        assert!(serde_json::from_str::<Preset>(&json).is_err());
    }

    #[test]
    fn reads_the_stored_settings_format() {
        let json = r#"{"toggle_hotkey":"F8","capture_hotkey":"F9","stop":{"type":"rounds","count":5},"autostart":true,"start_in_tray":true}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.toggle_hotkey, "F8");
        assert_eq!(settings.capture_hotkey, "F9");
        assert_eq!(settings.stop, StopCondition::Rounds { count: 5 });
        assert!(settings.autostart);
        assert!(settings.start_in_tray);
    }

    #[test]
    fn missing_settings_fall_back_to_defaults() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.toggle_hotkey, "F6");
        assert_eq!(settings.capture_hotkey, "F7");
        assert_eq!(settings.stop, StopCondition::Manual);
        assert!(!settings.autostart);
        assert!(!settings.start_in_tray);
    }

    #[test]
    fn stop_after_zero_rounds_is_rejected() {
        let json = r#"{"stop":{"type":"rounds","count":0}}"#;
        assert!(serde_json::from_str::<Settings>(json).is_err());
    }
}
