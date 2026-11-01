use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::input::{CancelToken, Clock, InputBackend, InputError};
use crate::model::{Sequence, StopCondition};
use crate::runner::{self, RunOutcome};
use crate::validate::{layout_changed, validate, Problem, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Idle,
    Running,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Event {
    State(RunState),
    Round(u32),
    /// 実行中にモニター構成が変わったため、Click Run を止める。
    ScreenLayoutChanged,
    Finished(RunOutcome),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum StartError {
    AlreadyRunning,
    HotkeysUnavailable,
    OffScreen(Vec<Problem>),
}

pub type BackendFactory = Arc<dyn Fn() -> Result<Box<dyn InputBackend>, InputError> + Send + Sync>;

struct Shared {
    state: RunState,
    cancel: Option<CancelToken>,
    hotkeys_ready: bool,
}

pub struct Controller {
    factory: BackendFactory,
    clock: Arc<dyn Clock + Send + Sync>,
    screens: Arc<dyn Fn() -> Vec<Rect> + Send + Sync>,
    notify: Arc<dyn Fn(Event) + Send + Sync>,
    watch_interval: Duration,
    shared: Arc<Mutex<Shared>>,
}

impl Controller {
    pub fn new(
        factory: BackendFactory,
        clock: Arc<dyn Clock + Send + Sync>,
        screens: Arc<dyn Fn() -> Vec<Rect> + Send + Sync>,
        notify: Arc<dyn Fn(Event) + Send + Sync>,
        watch_interval: Duration,
    ) -> Self {
        let shared = Shared { state: RunState::Idle, cancel: None, hotkeys_ready: false };
        Self { factory, clock, screens, notify, watch_interval, shared: Arc::new(Mutex::new(shared)) }
    }

    pub fn set_hotkeys_ready(&self, ready: bool) {
        self.shared.lock().unwrap().hotkeys_ready = ready;
    }

    pub fn state(&self) -> RunState {
        self.shared.lock().unwrap().state
    }

    pub fn start(&self, sequence: Sequence, stop: StopCondition) -> Result<(), StartError> {
        let cancel = {
            let mut shared = self.shared.lock().unwrap();
            if shared.state == RunState::Running {
                return Err(StartError::AlreadyRunning);
            }
            if !shared.hotkeys_ready {
                return Err(StartError::HotkeysUnavailable);
            }
            let layout = (self.screens)();
            let problems = validate(&sequence, &layout);
            if !problems.is_empty() {
                return Err(StartError::OffScreen(problems));
            }
            let cancel = CancelToken::new();
            shared.state = RunState::Running;
            shared.cancel = Some(cancel.clone());
            (cancel, layout)
        };
        let (cancel, layout) = cancel;
        (self.notify)(Event::State(RunState::Running));

        let finished = Arc::new(AtomicBool::new(false));
        {
            let (screens, notify, cancel, finished) =
                (self.screens.clone(), self.notify.clone(), cancel.clone(), finished.clone());
            let interval = self.watch_interval;
            std::thread::spawn(move || loop {
                std::thread::sleep(interval);
                if finished.load(Ordering::SeqCst) {
                    return;
                }
                if layout_changed(&layout, &screens()) {
                    notify(Event::ScreenLayoutChanged);
                    cancel.cancel();
                    return;
                }
            });
        }

        let factory = self.factory.clone();
        let clock = self.clock.clone();
        let notify = self.notify.clone();
        let shared = self.shared.clone();
        std::thread::spawn(move || {
            let outcome = match factory() {
                Ok(mut backend) => {
                    let on_round = notify.clone();
                    runner::run(&sequence, stop, backend.as_mut(), clock.as_ref(), &cancel, &mut |n| {
                        on_round(Event::Round(n))
                    })
                }
                Err(error) => RunOutcome::Failed { rounds: 0, error },
            };
            finished.store(true, Ordering::SeqCst);
            {
                let mut shared = shared.lock().unwrap();
                shared.state = RunState::Idle;
                shared.cancel = None;
            }
            notify(Event::State(RunState::Idle));
            notify(Event::Finished(outcome));
        });
        Ok(())
    }

    /// 停止を要求して、待機中に戻るまで待つ。時間内に戻れば true。アプリ終了時に使う。
    pub fn stop_and_wait(&self, timeout: Duration) -> bool {
        self.stop();
        let deadline = std::time::Instant::now() + timeout;
        while self.state() == RunState::Running {
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        true
    }

    pub fn stop(&self) {
        if let Some(cancel) = &self.shared.lock().unwrap().cancel {
            cancel.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::SystemClock;
    use crate::model::*;
    use std::sync::mpsc::{channel, Receiver};

    #[derive(Clone, Default)]
    struct RecordingInput {
        clicks: Arc<Mutex<Vec<(i32, i32)>>>,
        fail: bool,
    }

    struct Backend {
        shared: RecordingInput,
        at: (i32, i32),
    }

    impl InputBackend for Backend {
        fn move_to(&mut self, x: i32, y: i32) -> Result<(), InputError> {
            self.at = (x, y);
            Ok(())
        }
        fn click(&mut self, _b: Button) -> Result<(), InputError> {
            if self.shared.fail {
                return Err(InputError("boom".to_string()));
            }
            self.shared.clicks.lock().unwrap().push(self.at);
            Ok(())
        }
        fn double_click(&mut self, b: Button) -> Result<(), InputError> {
            self.click(b)
        }
        fn press(&mut self, _b: Button) -> Result<(), InputError> {
            Ok(())
        }
        fn release(&mut self, _b: Button) -> Result<(), InputError> {
            Ok(())
        }
    }

    struct Rig {
        controller: Controller,
        events: Receiver<Event>,
        input: RecordingInput,
        screens: Arc<Mutex<Vec<Rect>>>,
    }

    fn rig(fail: bool) -> Rig {
        rig_with_clock(fail, Arc::new(SystemClock))
    }

    fn rig_with_clock(fail: bool, clock: Arc<dyn Clock + Send + Sync>) -> Rig {
        let input = RecordingInput { fail, ..Default::default() };
        let for_factory = input.clone();
        let factory: BackendFactory =
            Arc::new(move || Ok(Box::new(Backend { shared: for_factory.clone(), at: (0, 0) }) as Box<dyn InputBackend>));
        let (tx, events) = channel();
        let tx = Mutex::new(tx);
        let notify: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(move |e| {
            let _ = tx.lock().unwrap().send(e);
        });
        let screens = Arc::new(Mutex::new(vec![Rect { x: 0, y: 0, width: 1920, height: 1080 }]));
        let screens_for_controller = screens.clone();
        let controller = Controller::new(
            factory,
            clock,
            Arc::new(move || screens_for_controller.lock().unwrap().clone()),
            notify,
            Duration::from_millis(5),
        );
        controller.set_hotkeys_ready(true);
        Rig { controller, events, input, screens }
    }

    fn ms(v: f64) -> Interval {
        Interval::new(v, TimeUnit::Milliseconds).unwrap()
    }

    fn click_at(x: i32, y: i32) -> ClickTarget {
        ClickTarget { position: Position::Fixed { x, y }, button: Button::Left, kind: ClickKind::Single }
    }

    fn next(rig: &Rig) -> Event {
        rig.events.recv_timeout(Duration::from_secs(2)).expect("expected an event")
    }

    #[test]
    fn starts_running_reports_rounds_and_stops_on_request() {
        let rig = rig(false);
        let seq = Sequence::single(click_at(5, 6), Interval::new(60.0, TimeUnit::Seconds).unwrap());
        rig.controller.start(seq, StopCondition::Manual).unwrap();
        assert_eq!(rig.controller.state(), RunState::Running);
        assert_eq!(next(&rig), Event::State(RunState::Running));
        assert_eq!(next(&rig), Event::Round(1));

        rig.controller.stop();
        assert_eq!(next(&rig), Event::State(RunState::Idle));
        assert_eq!(next(&rig), Event::Finished(RunOutcome::Cancelled { rounds: 1 }));
        assert_eq!(rig.controller.state(), RunState::Idle);
        assert_eq!(*rig.input.clicks.lock().unwrap(), vec![(5, 6)]);
    }

    #[test]
    fn returns_to_idle_by_itself_after_the_requested_rounds() {
        let rig = rig(false);
        let seq = Sequence::single(click_at(1, 1), ms(5.0));
        rig.controller.start(seq, StopCondition::Rounds { count: 2 }).unwrap();
        let mut seen = vec![];
        loop {
            let event = next(&rig);
            let done = matches!(event, Event::Finished(_));
            seen.push(event);
            if done {
                break;
            }
        }
        assert_eq!(
            seen,
            vec![
                Event::State(RunState::Running),
                Event::Round(1),
                Event::Round(2),
                Event::State(RunState::Idle),
                Event::Finished(RunOutcome::Completed { rounds: 2 }),
            ]
        );
        assert_eq!(rig.controller.state(), RunState::Idle);
    }

    #[test]
    fn a_second_start_while_running_is_refused() {
        let rig = rig(false);
        let seq = Sequence::single(click_at(1, 1), Interval::new(60.0, TimeUnit::Seconds).unwrap());
        rig.controller.start(seq.clone(), StopCondition::Manual).unwrap();
        assert_eq!(rig.controller.start(seq, StopCondition::Manual), Err(StartError::AlreadyRunning));
        rig.controller.stop();
    }

    #[test]
    fn off_screen_steps_refuse_to_start_and_change_nothing() {
        let rig = rig(false);
        let seq = Sequence::single(click_at(5000, 5), ms(10.0));
        let result = rig.controller.start(seq, StopCondition::Manual);
        assert_eq!(result, Err(StartError::OffScreen(vec![Problem::OffScreen { step_index: 0, x: 5000, y: 5 }])));
        assert_eq!(rig.controller.state(), RunState::Idle);
        assert!(rig.events.try_recv().is_err());
    }

    #[test]
    fn refuses_to_start_while_hotkeys_are_unavailable() {
        let rig = rig(false);
        rig.controller.set_hotkeys_ready(false);
        let seq = Sequence::single(click_at(1, 1), ms(10.0));
        assert_eq!(rig.controller.start(seq, StopCondition::Manual), Err(StartError::HotkeysUnavailable));
        assert_eq!(rig.controller.state(), RunState::Idle);
    }

    #[test]
    fn an_input_failure_returns_to_idle_and_reports_the_reason() {
        let rig = rig(true);
        let seq = Sequence::single(click_at(1, 1), ms(10.0));
        rig.controller.start(seq, StopCondition::Manual).unwrap();
        assert_eq!(next(&rig), Event::State(RunState::Running));
        assert_eq!(next(&rig), Event::State(RunState::Idle));
        assert_eq!(
            next(&rig),
            Event::Finished(RunOutcome::Failed { rounds: 0, error: InputError("boom".to_string()) })
        );
        assert_eq!(rig.controller.state(), RunState::Idle);
    }

    #[test]
    fn can_start_again_after_a_run_ended() {
        let rig = rig(false);
        let seq = Sequence::single(click_at(1, 1), ms(5.0));
        rig.controller.start(seq.clone(), StopCondition::Rounds { count: 1 }).unwrap();
        while !matches!(next(&rig), Event::Finished(_)) {}
        assert!(rig.controller.start(seq, StopCondition::Rounds { count: 1 }).is_ok());
    }

    #[test]
    fn a_screen_layout_change_while_running_stops_the_run_and_says_why() {
        let rig = rig(false);
        let seq = Sequence::single(click_at(1, 1), Interval::new(60.0, TimeUnit::Seconds).unwrap());
        rig.controller.start(seq, StopCondition::Manual).unwrap();
        assert_eq!(next(&rig), Event::State(RunState::Running));
        assert_eq!(next(&rig), Event::Round(1));

        *rig.screens.lock().unwrap() = vec![Rect { x: 0, y: 0, width: 1280, height: 720 }];

        assert_eq!(next(&rig), Event::ScreenLayoutChanged);
        assert_eq!(next(&rig), Event::State(RunState::Idle));
        assert_eq!(next(&rig), Event::Finished(RunOutcome::Cancelled { rounds: 1 }));
        assert_eq!(rig.controller.state(), RunState::Idle);
    }

    fn long_run() -> Sequence {
        Sequence::single(click_at(1, 1), Interval::new(60.0, TimeUnit::Seconds).unwrap())
    }

    #[test]
    fn an_unchanged_layout_never_stops_the_run() {
        let rig = rig(false);
        rig.controller.start(long_run(), StopCondition::Manual).unwrap();
        assert_eq!(next(&rig), Event::State(RunState::Running));
        assert_eq!(next(&rig), Event::Round(1));
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(rig.controller.state(), RunState::Running);
        assert!(rig.events.try_recv().is_err());
        rig.controller.stop();
    }

    #[test]
    fn layout_changes_while_idle_do_nothing_and_the_next_run_uses_the_new_layout() {
        let rig = rig(false);
        *rig.screens.lock().unwrap() = vec![Rect { x: 0, y: 0, width: 1280, height: 720 }];
        std::thread::sleep(Duration::from_millis(30));
        assert!(rig.events.try_recv().is_err());

        rig.controller.start(long_run(), StopCondition::Manual).unwrap();
        assert_eq!(next(&rig), Event::State(RunState::Running));
        assert_eq!(next(&rig), Event::Round(1));
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(rig.controller.state(), RunState::Running);
        rig.controller.stop();
    }

    #[test]
    fn the_watcher_does_not_report_after_the_run_ended() {
        let rig = rig(false);
        let seq = Sequence::single(click_at(1, 1), ms(5.0));
        rig.controller.start(seq, StopCondition::Rounds { count: 1 }).unwrap();
        while !matches!(next(&rig), Event::Finished(_)) {}
        *rig.screens.lock().unwrap() = vec![];
        std::thread::sleep(Duration::from_millis(60));
        assert!(rig.events.try_recv().is_err());
    }

    #[test]
    fn events_serialize_to_the_json_the_frontend_reads() {
        use serde_json::json;
        let cases = [
            (Event::State(RunState::Running), json!({"type":"state","data":"running"})),
            (Event::State(RunState::Idle), json!({"type":"state","data":"idle"})),
            (Event::Round(3), json!({"type":"round","data":3})),
            (Event::ScreenLayoutChanged, json!({"type":"screen_layout_changed"})),
            (
                Event::Finished(RunOutcome::Completed { rounds: 2 }),
                json!({"type":"finished","data":{"type":"completed","rounds":2}}),
            ),
            (
                Event::Finished(RunOutcome::Cancelled { rounds: 1 }),
                json!({"type":"finished","data":{"type":"cancelled","rounds":1}}),
            ),
            (
                Event::Finished(RunOutcome::Failed { rounds: 0, error: InputError("boom".to_string()) }),
                json!({"type":"finished","data":{"type":"failed","rounds":0,"error":"boom"}}),
            ),
        ];
        for (event, expected) in cases {
            assert_eq!(serde_json::to_value(&event).unwrap(), expected);
        }
    }

    #[test]
    fn start_errors_serialize_to_the_json_the_frontend_reads() {
        use serde_json::json;
        assert_eq!(serde_json::to_value(StartError::AlreadyRunning).unwrap(), json!({"type":"already_running"}));
        assert_eq!(
            serde_json::to_value(StartError::HotkeysUnavailable).unwrap(),
            json!({"type":"hotkeys_unavailable"})
        );
        let off = StartError::OffScreen(vec![Problem::OffScreen { step_index: 1, x: 5000, y: -3 }]);
        assert_eq!(
            serde_json::to_value(off).unwrap(),
            json!({"type":"off_screen","data":[{"type":"off_screen","step_index":1,"x":5000,"y":-3}]})
        );
    }

    #[test]
    fn stop_and_wait_returns_once_a_running_run_has_ended() {
        let rig = rig(false);
        rig.controller.start(long_run(), StopCondition::Manual).unwrap();
        assert_eq!(next(&rig), Event::State(RunState::Running));
        assert_eq!(next(&rig), Event::Round(1));
        assert!(rig.controller.stop_and_wait(Duration::from_secs(2)));
        assert_eq!(rig.controller.state(), RunState::Idle);
    }

    #[test]
    fn stop_and_wait_returns_immediately_when_idle() {
        let rig = rig(false);
        let start = std::time::Instant::now();
        assert!(rig.controller.stop_and_wait(Duration::from_secs(5)));
        assert!(start.elapsed() < Duration::from_millis(200));
    }

    /// キャンセルに反応しない時計。止まるのが遅い実行を作るために使う。
    struct StubbornClock;

    impl Clock for StubbornClock {
        fn sleep(&self, duration: Duration, _cancel: &crate::input::CancelToken) -> bool {
            std::thread::sleep(duration);
            true
        }
    }

    #[test]
    fn stop_and_wait_gives_up_when_the_run_does_not_stop_in_time() {
        let rig = rig_with_clock(false, Arc::new(StubbornClock));
        let seq = Sequence::single(click_at(1, 1), ms(400.0));
        rig.controller.start(seq, StopCondition::Manual).unwrap();
        assert_eq!(next(&rig), Event::State(RunState::Running));
        assert_eq!(next(&rig), Event::Round(1));
        let start = std::time::Instant::now();
        assert!(!rig.controller.stop_and_wait(Duration::from_millis(60)));
        assert!(start.elapsed() < Duration::from_millis(300));
        assert_eq!(rig.controller.state(), RunState::Running);
        assert!(rig.controller.stop_and_wait(Duration::from_secs(3)));
    }
}
