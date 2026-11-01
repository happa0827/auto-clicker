use serde::Serialize;

use crate::input::{CancelToken, Clock, InputBackend, InputError};
use crate::model::{ClickKind, Position, Sequence, Step, StopCondition};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunOutcome {
    Completed { rounds: u32 },
    Cancelled { rounds: u32 },
    Failed { rounds: u32, error: InputError },
}

/// Step のクリックを実行する。長押しの待ちが中断されたら false を返す（ボタンは必ず離す）。
fn perform(
    step: &Step,
    backend: &mut dyn InputBackend,
    clock: &dyn Clock,
    cancel: &CancelToken,
) -> Result<bool, InputError> {
    if let Position::Fixed { x, y } = step.target.position {
        backend.move_to(x, y)?;
    }
    let button = step.target.button;
    match step.target.kind {
        ClickKind::Single => backend.click(button).map(|_| true),
        ClickKind::Double => backend.double_click(button).map(|_| true),
        ClickKind::Hold { duration } => {
            backend.press(button)?;
            let held = clock.sleep(duration.as_duration(), cancel);
            backend.release(button)?;
            Ok(held)
        }
    }
}

pub fn run(
    sequence: &Sequence,
    stop: StopCondition,
    backend: &mut dyn InputBackend,
    clock: &dyn Clock,
    cancel: &CancelToken,
    on_round: &mut dyn FnMut(u32),
) -> RunOutcome {
    let steps = sequence.steps();
    let mut rounds = 0;
    loop {
        for (index, step) in steps.iter().enumerate() {
            if cancel.is_cancelled() {
                return RunOutcome::Cancelled { rounds };
            }
            match perform(step, backend, clock, cancel) {
                Err(error) => return RunOutcome::Failed { rounds, error },
                Ok(false) => return RunOutcome::Cancelled { rounds },
                Ok(true) => {}
            }
            if index == steps.len() - 1 {
                rounds += 1;
                on_round(rounds);
                if stop == (StopCondition::Rounds { count: rounds }) {
                    return RunOutcome::Completed { rounds };
                }
            }
            if !clock.sleep(step.interval.as_duration(), cancel) {
                return RunOutcome::Cancelled { rounds };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Event {
        MoveTo(i32, i32),
        Click(Button),
        DoubleClick(Button),
        Press(Button),
        Release(Button),
        Sleep(Duration),
    }

    type Log = Rc<RefCell<Vec<Event>>>;

    struct FakeInput {
        log: Log,
        fail_on_click: bool,
    }

    impl InputBackend for FakeInput {
        fn move_to(&mut self, x: i32, y: i32) -> Result<(), InputError> {
            self.log.borrow_mut().push(Event::MoveTo(x, y));
            Ok(())
        }
        fn click(&mut self, button: Button) -> Result<(), InputError> {
            if self.fail_on_click {
                return Err(InputError("click failed".to_string()));
            }
            self.log.borrow_mut().push(Event::Click(button));
            Ok(())
        }
        fn double_click(&mut self, button: Button) -> Result<(), InputError> {
            self.log.borrow_mut().push(Event::DoubleClick(button));
            Ok(())
        }
        fn press(&mut self, button: Button) -> Result<(), InputError> {
            self.log.borrow_mut().push(Event::Press(button));
            Ok(())
        }
        fn release(&mut self, button: Button) -> Result<(), InputError> {
            self.log.borrow_mut().push(Event::Release(button));
            Ok(())
        }
    }

    struct FakeClock {
        log: Log,
        cancel_on_sleep: Option<usize>,
        sleeps: RefCell<usize>,
    }

    impl Clock for FakeClock {
        fn sleep(&self, duration: Duration, cancel: &CancelToken) -> bool {
            self.log.borrow_mut().push(Event::Sleep(duration));
            let n = {
                let mut sleeps = self.sleeps.borrow_mut();
                *sleeps += 1;
                *sleeps
            };
            if self.cancel_on_sleep == Some(n) {
                cancel.cancel();
            }
            !cancel.is_cancelled()
        }
    }

    struct Harness {
        log: Log,
        input: FakeInput,
        clock: FakeClock,
        cancel: CancelToken,
    }

    impl Harness {
        fn new() -> Self {
            let log: Log = Rc::new(RefCell::new(vec![]));
            Harness {
                input: FakeInput { log: log.clone(), fail_on_click: false },
                clock: FakeClock { log: log.clone(), cancel_on_sleep: None, sleeps: RefCell::new(0) },
                cancel: CancelToken::new(),
                log,
            }
        }

        fn run(&mut self, sequence: &Sequence, stop: StopCondition) -> (RunOutcome, Vec<u32>) {
            let mut rounds = vec![];
            let outcome = run(sequence, stop, &mut self.input, &self.clock, &self.cancel, &mut |n| rounds.push(n));
            (outcome, rounds)
        }

        fn events(&self) -> Vec<Event> {
            self.log.borrow().clone()
        }
    }

    fn ms(v: f64) -> Interval {
        Interval::new(v, TimeUnit::Milliseconds).unwrap()
    }

    fn d(v: u64) -> Duration {
        Duration::from_millis(v)
    }

    fn target(position: Position, kind: ClickKind) -> ClickTarget {
        ClickTarget { position, button: Button::Left, kind }
    }

    fn fixed_click(x: i32, y: i32) -> ClickTarget {
        target(Position::Fixed { x, y }, ClickKind::Single)
    }

    #[test]
    fn one_round_moves_then_clicks_and_does_not_wait_after_the_final_round() {
        let mut h = Harness::new();
        let seq = Sequence::single(fixed_click(10, 20), ms(500.0));
        let (outcome, _) = h.run(&seq, StopCondition::Rounds { count: 1 });
        assert_eq!(outcome, RunOutcome::Completed { rounds: 1 });
        assert_eq!(h.events(), vec![Event::MoveTo(10, 20), Event::Click(Button::Left)]);
    }

    #[test]
    fn waits_the_interval_between_rounds_but_not_after_the_last() {
        let mut h = Harness::new();
        let seq = Sequence::single(fixed_click(1, 2), ms(500.0));
        let (outcome, _) = h.run(&seq, StopCondition::Rounds { count: 3 });
        assert_eq!(outcome, RunOutcome::Completed { rounds: 3 });
        let round = [Event::MoveTo(1, 2), Event::Click(Button::Left)];
        let wait = [Event::Sleep(d(500))];
        let expected: Vec<Event> = [&round[..], &wait, &round, &wait, &round].concat();
        assert_eq!(h.events(), expected);
    }

    #[test]
    fn steps_run_in_order_each_with_its_own_interval() {
        let mut h = Harness::new();
        let seq = Sequence::new(vec![
            Step { target: fixed_click(1, 1), interval: ms(100.0) },
            Step { target: fixed_click(2, 2), interval: ms(200.0) },
        ])
        .unwrap();
        let (outcome, _) = h.run(&seq, StopCondition::Rounds { count: 2 });
        assert_eq!(outcome, RunOutcome::Completed { rounds: 2 });
        let l = Event::Click(Button::Left);
        assert_eq!(
            h.events(),
            vec![
                Event::MoveTo(1, 1), l.clone(), Event::Sleep(d(100)),
                Event::MoveTo(2, 2), l.clone(), Event::Sleep(d(200)),
                Event::MoveTo(1, 1), l.clone(), Event::Sleep(d(100)),
                Event::MoveTo(2, 2), l,
            ]
        );
    }

    #[test]
    fn current_position_does_not_move_and_double_click_uses_the_chosen_button() {
        let mut h = Harness::new();
        let t = ClickTarget { position: Position::Current, button: Button::Right, kind: ClickKind::Double };
        let (outcome, _) = h.run(&Sequence::single(t, ms(10.0)), StopCondition::Rounds { count: 1 });
        assert_eq!(outcome, RunOutcome::Completed { rounds: 1 });
        assert_eq!(h.events(), vec![Event::DoubleClick(Button::Right)]);
    }

    #[test]
    fn hold_presses_waits_the_hold_duration_releases_then_waits_the_interval() {
        let mut h = Harness::new();
        let t = target(Position::Fixed { x: 5, y: 6 }, ClickKind::Hold { duration: ms(2000.0) });
        let (outcome, _) = h.run(&Sequence::single(t, ms(300.0)), StopCondition::Rounds { count: 2 });
        assert_eq!(outcome, RunOutcome::Completed { rounds: 2 });
        let l = Button::Left;
        assert_eq!(
            h.events(),
            vec![
                Event::MoveTo(5, 6), Event::Press(l), Event::Sleep(d(2000)), Event::Release(l), Event::Sleep(d(300)),
                Event::MoveTo(5, 6), Event::Press(l), Event::Sleep(d(2000)), Event::Release(l),
            ]
        );
    }

    #[test]
    fn manual_stop_cancelled_during_an_interval_ends_the_run_with_completed_rounds() {
        let mut h = Harness::new();
        h.clock.cancel_on_sleep = Some(2);
        let seq = Sequence::single(fixed_click(1, 1), ms(50.0));
        let (outcome, _) = h.run(&seq, StopCondition::Manual);
        assert_eq!(outcome, RunOutcome::Cancelled { rounds: 2 });
        let round = [Event::MoveTo(1, 1), Event::Click(Button::Left), Event::Sleep(d(50))];
        assert_eq!(h.events(), [&round[..], &round].concat());
    }

    #[test]
    fn cancelling_during_a_hold_still_releases_the_button_and_skips_the_interval() {
        let mut h = Harness::new();
        h.clock.cancel_on_sleep = Some(1);
        let t = target(Position::Current, ClickKind::Hold { duration: ms(5000.0) });
        let (outcome, _) = h.run(&Sequence::single(t, ms(100.0)), StopCondition::Manual);
        assert_eq!(outcome, RunOutcome::Cancelled { rounds: 0 });
        assert_eq!(h.events(), vec![Event::Press(Button::Left), Event::Sleep(d(5000)), Event::Release(Button::Left)]);
    }

    #[test]
    fn a_run_cancelled_before_it_starts_does_nothing() {
        let mut h = Harness::new();
        h.cancel.cancel();
        let seq = Sequence::single(fixed_click(1, 1), ms(50.0));
        let (outcome, _) = h.run(&seq, StopCondition::Manual);
        assert_eq!(outcome, RunOutcome::Cancelled { rounds: 0 });
        assert!(h.events().is_empty());
    }

    #[test]
    fn an_input_failure_stops_the_run_and_reports_why() {
        let mut h = Harness::new();
        h.input.fail_on_click = true;
        let seq = Sequence::single(fixed_click(1, 1), ms(50.0));
        let (outcome, rounds) = h.run(&seq, StopCondition::Rounds { count: 3 });
        assert_eq!(outcome, RunOutcome::Failed { rounds: 0, error: InputError("click failed".to_string()) });
        assert!(rounds.is_empty());
        assert_eq!(h.events(), vec![Event::MoveTo(1, 1)]);
    }

    #[test]
    fn reports_each_completed_round_number_as_it_finishes() {
        let mut h = Harness::new();
        let seq = Sequence::single(fixed_click(1, 1), ms(10.0));
        let (_, rounds) = h.run(&seq, StopCondition::Rounds { count: 3 });
        assert_eq!(rounds, vec![1, 2, 3]);
    }

    #[test]
    fn a_partly_finished_round_is_not_reported() {
        let mut h = Harness::new();
        h.clock.cancel_on_sleep = Some(1);
        let seq = Sequence::new(vec![
            Step { target: fixed_click(1, 1), interval: ms(10.0) },
            Step { target: fixed_click(2, 2), interval: ms(10.0) },
        ])
        .unwrap();
        let (outcome, rounds) = h.run(&seq, StopCondition::Manual);
        assert_eq!(outcome, RunOutcome::Cancelled { rounds: 0 });
        assert!(rounds.is_empty());
    }
}
