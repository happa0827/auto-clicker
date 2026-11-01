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

/// RGBA 8 ビットの画像。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

const RUNNING_DOT: [u8; 4] = [34, 197, 94, 255];

/// 待機中は元の画像のまま。実行中は右下に緑の点を描く。
pub fn tray_icon(state: RunState, base: &Rgba) -> Rgba {
    let mut icon = base.clone();
    if state == RunState::Idle || base.width == 0 || base.height == 0 {
        return icon;
    }
    let radius = (base.width.min(base.height) / 5).max(1) as i64;
    let margin = (base.width.min(base.height) / 16).max(1) as i64;
    let center_x = base.width as i64 - radius - margin;
    let center_y = base.height as i64 - radius - margin;
    for y in 0..base.height as i64 {
        for x in 0..base.width as i64 {
            let (dx, dy) = (x - center_x, y - center_y);
            if dx * dx + dy * dy <= radius * radius {
                let at = ((y * base.width as i64 + x) * 4) as usize;
                icon.pixels[at..at + 4].copy_from_slice(&RUNNING_DOT);
            }
        }
    }
    icon
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

    fn grey_icon(size: u32) -> Rgba {
        Rgba { width: size, height: size, pixels: [128u8, 128, 128, 255].repeat((size * size) as usize) }
    }

    fn pixel(icon: &Rgba, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * icon.width + x) * 4) as usize;
        [icon.pixels[at], icon.pixels[at + 1], icon.pixels[at + 2], icon.pixels[at + 3]]
    }

    #[test]
    fn the_idle_icon_is_the_base_image_unchanged() {
        let base = grey_icon(32);
        assert_eq!(tray_icon(RunState::Idle, &base), base);
    }

    #[test]
    fn the_running_icon_has_a_green_dot_in_the_bottom_right() {
        let base = grey_icon(32);
        let icon = tray_icon(RunState::Running, &base);
        assert_eq!((icon.width, icon.height), (32, 32));
        assert_eq!(icon.pixels.len(), base.pixels.len());
        assert_eq!(pixel(&icon, 24, 24), [34, 197, 94, 255]);
    }

    #[test]
    fn the_dot_leaves_the_rest_of_the_image_alone_and_stays_small() {
        let base = grey_icon(32);
        let icon = tray_icon(RunState::Running, &base);
        for (x, y) in [(0, 0), (31, 0), (0, 31), (16, 16), (8, 24), (24, 8)] {
            assert_eq!(pixel(&icon, x, y), [128, 128, 128, 255], "({x}, {y})");
        }
        let changed = icon.pixels.chunks(4).zip(base.pixels.chunks(4)).filter(|(a, b)| a != b).count();
        assert!(changed > 0 && changed < (32 * 32) / 3, "changed {changed}");
    }

    #[test]
    fn it_does_not_modify_the_base_and_works_on_tiny_images() {
        let base = grey_icon(32);
        let _ = tray_icon(RunState::Running, &base);
        assert_eq!(base, grey_icon(32));
        let tiny = grey_icon(4);
        assert_eq!(tray_icon(RunState::Running, &tiny).pixels.len(), tiny.pixels.len());
        let empty = Rgba { width: 0, height: 0, pixels: vec![] };
        assert_eq!(tray_icon(RunState::Running, &empty), empty);
    }
}
