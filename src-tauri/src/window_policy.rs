#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    HideToTray,
    Exit,
}

/// ウィンドウの × ボタン。トレイがあるときだけ、アプリを残してウィンドウを隠す。
pub fn on_close_requested(quitting: bool, tray_available: bool) -> CloseAction {
    if tray_available && !quitting {
        CloseAction::HideToTray
    } else {
        CloseAction::Exit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Shown,
    Hidden,
}

/// 起動時のウィンドウ。トレイがなければ、隠すと操作できなくなるので必ず表示する。
/// 自動起動かどうかには関係なく、`start_in_tray` だけで決まる。
pub fn initial_visibility(start_in_tray: bool, tray_available: bool) -> Visibility {
    if start_in_tray && tray_available {
        Visibility::Hidden
    } else {
        Visibility::Shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_close_button_hides_the_window_when_a_tray_icon_exists() {
        assert_eq!(on_close_requested(false, true), CloseAction::HideToTray);
    }

    #[test]
    fn without_a_tray_icon_closing_exits_so_the_app_is_never_left_invisible() {
        assert_eq!(on_close_requested(false, false), CloseAction::Exit);
    }

    #[test]
    fn once_the_user_chose_quit_closing_really_exits() {
        assert_eq!(on_close_requested(true, true), CloseAction::Exit);
        assert_eq!(on_close_requested(true, false), CloseAction::Exit);
    }

    #[test]
    fn the_window_is_shown_at_launch_unless_start_in_tray_is_on() {
        assert_eq!(initial_visibility(false, true), Visibility::Shown);
        assert_eq!(initial_visibility(true, true), Visibility::Hidden);
    }

    #[test]
    fn start_in_tray_is_ignored_without_a_tray_icon() {
        assert_eq!(initial_visibility(true, false), Visibility::Shown);
    }
}
