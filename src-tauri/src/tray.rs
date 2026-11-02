use crate::controller::RunState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Toggle,
    ShowWindow,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuEntry {
    pub id: &'static str,
    pub label: &'static str,
}

pub const TOGGLE_ID: &str = "toggle";
pub const SHOW_ID: &str = "show";
pub const QUIT_ID: &str = "quit";

pub fn tray_menu(state: RunState) -> Vec<MenuEntry> {
    let toggle = match state {
        RunState::Idle => "開始",
        RunState::Running => "停止",
    };
    vec![
        MenuEntry { id: TOGGLE_ID, label: toggle },
        MenuEntry { id: SHOW_ID, label: "ウィンドウを表示" },
        MenuEntry { id: QUIT_ID, label: "終了" },
    ]
}

pub fn tray_action(id: &str) -> Option<TrayAction> {
    match id {
        TOGGLE_ID => Some(TrayAction::Toggle),
        SHOW_ID => Some(TrayAction::ShowWindow),
        QUIT_ID => Some(TrayAction::Quit),
        _ => None,
    }
}

/// 待機中は波紋がグレー、実行中は緑のトレイ画像(PNG)。
pub fn tray_icon_png(state: RunState) -> &'static [u8] {
    match state {
        RunState::Idle => include_bytes!("../icons/tray-idle.png"),
        RunState::Running => include_bytes!("../icons/tray-running.png"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_has_three_entries_and_the_first_follows_the_run_state() {
        let idle = tray_menu(RunState::Idle);
        assert_eq!(idle.iter().map(|e| e.label).collect::<Vec<_>>(), vec!["開始", "ウィンドウを表示", "終了"]);
        let running = tray_menu(RunState::Running);
        assert_eq!(running.iter().map(|e| e.label).collect::<Vec<_>>(), vec!["停止", "ウィンドウを表示", "終了"]);
    }

    #[test]
    fn entry_ids_stay_the_same_across_states() {
        let ids = |state| tray_menu(state).iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(RunState::Idle), ids(RunState::Running));
    }

    #[test]
    fn menu_ids_map_to_actions_and_unknown_ids_are_ignored() {
        let menu = tray_menu(RunState::Idle);
        let actions: Vec<_> = menu.iter().map(|e| tray_action(e.id)).collect();
        assert_eq!(actions, vec![Some(TrayAction::Toggle), Some(TrayAction::ShowWindow), Some(TrayAction::Quit)]);
        assert_eq!(tray_action("nonsense"), None);
        assert_eq!(tray_action(""), None);
    }

    #[test]
    fn the_idle_and_running_icons_are_different_png_images() {
        let (idle, running) = (tray_icon_png(RunState::Idle), tray_icon_png(RunState::Running));
        assert_ne!(idle, running);
        for png in [idle, running] {
            assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        }
    }
}
