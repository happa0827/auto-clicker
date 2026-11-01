use std::time::{Duration, Instant};

use crate::input::{CancelToken, Clock};

/// キャンセルを確認する間隔。
const POLL_SLICE: Duration = Duration::from_millis(10);

pub struct SystemClock;

impl Clock for SystemClock {
    fn sleep(&self, duration: Duration, cancel: &CancelToken) -> bool {
        let deadline = Instant::now() + duration;
        loop {
            if cancel.is_cancelled() {
                return false;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return true;
            }
            std::thread::sleep(remaining.min(POLL_SLICE));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_at_least_the_requested_time_and_reports_completion() {
        let start = Instant::now();
        let completed = SystemClock.sleep(Duration::from_millis(60), &CancelToken::new());
        assert!(completed);
        assert!(start.elapsed() >= Duration::from_millis(60));
    }

    #[test]
    fn cancelling_interrupts_a_long_wait_quickly() {
        let cancel = CancelToken::new();
        let remote = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            remote.cancel();
        });
        let start = Instant::now();
        let completed = SystemClock.sleep(Duration::from_secs(60), &cancel);
        assert!(!completed);
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn an_already_cancelled_token_returns_immediately() {
        let cancel = CancelToken::new();
        cancel.cancel();
        let start = Instant::now();
        assert!(!SystemClock.sleep(Duration::from_secs(60), &cancel));
        assert!(start.elapsed() < Duration::from_millis(200));
    }
}
