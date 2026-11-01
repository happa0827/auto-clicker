//! `InputBackend` の実 OS 実装。ロジックは持たず、enigo への薄い橋渡しだけを行う。
//! 自動テストの対象外で、手動で確認する（docs/manual-check.md）。

use enigo::{Button as EButton, Coordinate, Direction, Enigo, Mouse, Settings};

use crate::input::{InputBackend, InputError};
use crate::model::Button;

pub struct EnigoBackend {
    enigo: Enigo,
}

fn convert(button: Button) -> EButton {
    match button {
        Button::Left => EButton::Left,
        Button::Right => EButton::Right,
        Button::Middle => EButton::Middle,
    }
}

fn fail<E: std::fmt::Display>(error: E) -> InputError {
    InputError(error.to_string())
}

impl EnigoBackend {
    pub fn new() -> Result<Self, InputError> {
        Enigo::new(&Settings::default()).map(|enigo| Self { enigo }).map_err(fail)
    }

    pub fn cursor_position(&self) -> Result<(i32, i32), InputError> {
        self.enigo.location().map_err(fail)
    }
}

impl InputBackend for EnigoBackend {
    fn move_to(&mut self, x: i32, y: i32) -> Result<(), InputError> {
        self.enigo.move_mouse(x, y, Coordinate::Abs).map_err(fail)
    }

    fn click(&mut self, button: Button) -> Result<(), InputError> {
        self.enigo.button(convert(button), Direction::Click).map_err(fail)
    }

    fn double_click(&mut self, button: Button) -> Result<(), InputError> {
        self.click(button)?;
        self.click(button)
    }

    fn press(&mut self, button: Button) -> Result<(), InputError> {
        self.enigo.button(convert(button), Direction::Press).map_err(fail)
    }

    fn release(&mut self, button: Button) -> Result<(), InputError> {
        self.enigo.button(convert(button), Direction::Release).map_err(fail)
    }
}
