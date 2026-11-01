use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::model::Button;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct InputError(pub String);

pub trait InputBackend {
    fn move_to(&mut self, x: i32, y: i32) -> Result<(), InputError>;
    fn click(&mut self, button: Button) -> Result<(), InputError>;
    fn double_click(&mut self, button: Button) -> Result<(), InputError>;
    fn press(&mut self, button: Button) -> Result<(), InputError>;
    fn release(&mut self, button: Button) -> Result<(), InputError>;
}

/// 停止要求。複数スレッドから共有する。
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub trait Clock {
    /// `duration` だけ待つ。待ち切れたら true、途中で `cancel` されたら false。
    fn sleep(&self, duration: Duration, cancel: &CancelToken) -> bool;
}
