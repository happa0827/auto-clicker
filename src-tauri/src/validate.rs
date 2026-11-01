use serde::Serialize;

use crate::model::{Position, Sequence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    fn contains(&self, x: i32, y: i32) -> bool {
        let (x, y) = (i64::from(x), i64::from(y));
        let (left, top) = (i64::from(self.x), i64::from(self.y));
        x >= left && x < left + i64::from(self.width) && y >= top && y < top + i64::from(self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Problem {
    OffScreen { step_index: usize, x: i32, y: i32 },
}

/// モニター構成が変わったか。並び順は無視する。
pub fn layout_changed(before: &[Rect], after: &[Rect]) -> bool {
    let key = |r: &Rect| (r.x, r.y, r.width, r.height);
    let mut before: Vec<_> = before.iter().map(key).collect();
    let mut after: Vec<_> = after.iter().map(key).collect();
    before.sort_unstable();
    after.sort_unstable();
    before != after
}

pub fn validate(sequence: &Sequence, screens: &[Rect]) -> Vec<Problem> {
    sequence
        .steps()
        .iter()
        .enumerate()
        .filter_map(|(step_index, step)| match step.target.position {
            Position::Fixed { x, y } if !screens.iter().any(|s| s.contains(x, y)) => {
                Some(Problem::OffScreen { step_index, x, y })
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn ms(v: f64) -> Interval {
        Interval::new(v, TimeUnit::Milliseconds).unwrap()
    }

    fn step(position: Position) -> Step {
        Step {
            target: ClickTarget { position, button: Button::Left, kind: ClickKind::Single },
            interval: ms(10.0),
        }
    }

    fn fixed(x: i32, y: i32) -> Step {
        step(Position::Fixed { x, y })
    }

    fn monitor(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect { x, y, width, height }
    }

    fn seq(steps: Vec<Step>) -> Sequence {
        Sequence::new(steps).unwrap()
    }

    #[test]
    fn on_screen_steps_have_no_problems() {
        let screens = [monitor(0, 0, 1920, 1080)];
        assert!(validate(&seq(vec![fixed(0, 0), fixed(960, 540)]), &screens).is_empty());
    }

    #[test]
    fn edge_pixels_are_inside_and_one_past_is_outside() {
        let screens = [monitor(0, 0, 1920, 1080)];
        assert!(validate(&seq(vec![fixed(1919, 1079)]), &screens).is_empty());
        let problems = validate(&seq(vec![fixed(1920, 1079), fixed(1919, 1080), fixed(-1, 0)]), &screens);
        let indexes: Vec<usize> = problems.iter().map(|Problem::OffScreen { step_index, .. }| *step_index).collect();
        assert_eq!(indexes, vec![0, 1, 2]);
    }

    #[test]
    fn reports_the_offending_step_and_position() {
        let screens = [monitor(0, 0, 100, 100)];
        let problems = validate(&seq(vec![fixed(10, 10), fixed(500, -3)]), &screens);
        assert_eq!(problems, vec![Problem::OffScreen { step_index: 1, x: 500, y: -3 }]);
    }

    #[test]
    fn current_position_steps_are_never_checked() {
        let screens = [monitor(0, 0, 100, 100)];
        assert!(validate(&seq(vec![step(Position::Current)]), &screens).is_empty());
    }

    #[test]
    fn negative_coordinates_on_a_left_monitor_are_valid() {
        let screens = [monitor(-1920, 0, 1920, 1080), monitor(0, 0, 1920, 1080)];
        assert!(validate(&seq(vec![fixed(-1920, 0), fixed(-1, 500)]), &screens).is_empty());
    }

    #[test]
    fn a_gap_between_monitors_is_off_screen() {
        let screens = [monitor(0, 0, 100, 100), monitor(200, 0, 100, 100)];
        let problems = validate(&seq(vec![fixed(150, 50)]), &screens);
        assert_eq!(problems.len(), 1);
    }

    #[test]
    fn with_no_screens_every_fixed_step_is_off_screen() {
        assert_eq!(validate(&seq(vec![fixed(0, 0)]), &[]).len(), 1);
    }

    #[test]
    fn identical_layouts_are_not_a_change() {
        let a = [monitor(0, 0, 1920, 1080), monitor(1920, 0, 1280, 720)];
        assert!(!layout_changed(&a, &a));
    }

    #[test]
    fn a_different_order_alone_is_not_a_change() {
        let before = [monitor(0, 0, 1920, 1080), monitor(1920, 0, 1280, 720)];
        let after = [monitor(1920, 0, 1280, 720), monitor(0, 0, 1920, 1080)];
        assert!(!layout_changed(&before, &after));
    }

    #[test]
    fn resolution_position_or_count_changes_are_detected() {
        let before = [monitor(0, 0, 1920, 1080)];
        assert!(layout_changed(&before, &[monitor(0, 0, 1280, 720)]));
        assert!(layout_changed(&before, &[monitor(10, 0, 1920, 1080)]));
        assert!(layout_changed(&before, &[monitor(0, 0, 1920, 1080), monitor(1920, 0, 800, 600)]));
        assert!(layout_changed(&before, &[]));
    }
}
