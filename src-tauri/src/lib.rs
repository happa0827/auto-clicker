mod autostart;
mod clock;
mod controller;
mod enigo_backend;
mod environment;
mod hotkeys;
mod input;
mod model;
mod runner;
mod store;
mod tray;
mod updater;
mod validate;
mod window_policy;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::image::Image;
use tauri::menu::{IsMenuItem, Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use autostart::{AutostartControl, AutostartError};
use clock::SystemClock;
use controller::{BackendFactory, Controller, Event, RunState};
use enigo_backend::EnigoBackend;
use hotkeys::{HotkeyError, HotkeyRegistrar, Hotkeys, RegisterError};
use environment::{EnvInfo, Os};
use input::InputBackend;
use model::{Button, ClickKind, ClickTarget, Interval, Position, Sequence, Settings, StopCondition, TimeUnit};
use store::Store;
use tray::{Rgba, TrayAction};
use validate::Rect;
use window_policy::{CloseAction, Visibility};

struct AppState {
    controller: Controller,
    run: Mutex<(Sequence, StopCondition)>,
    store: Store,
    /// 登録できたホットキー（トグル、キャプチャ）。変更されるので Mutex に入れる。
    keys: Mutex<(Option<Shortcut>, Option<Shortcut>)>,
    /// `keys` に対応するキー名。登録できなかったほうは None。
    key_names: Mutex<Hotkeys>,
    /// 「終了」が選ばれた後は、× ボタンで本当に終了する。
    quitting: AtomicBool,
    /// トレイを作れなかった環境（Linux で appindicator がないなど）では None。
    tray: Option<TrayParts>,
}

struct TrayParts {
    tray: TrayIcon,
    toggle_item: MenuItem<Wry>,
    base_icon: Rgba,
}

#[derive(Serialize, Clone)]
struct Captured {
    x: i32,
    y: i32,
}

/// `HotkeyRegistrar` の実 OS 実装。ロジックは持たない（手動で確認する）。
struct TauriRegistrar<'a>(&'a AppHandle);

impl HotkeyRegistrar for TauriRegistrar<'_> {
    fn register(&mut self, key: &str) -> Result<(), RegisterError> {
        let shortcut: Shortcut = key.parse().map_err(|_| RegisterError::Invalid)?;
        self.0.global_shortcut().register(shortcut).map_err(|_| RegisterError::Conflict)
    }

    fn unregister(&mut self, key: &str) {
        if let Ok(shortcut) = key.parse::<Shortcut>() {
            let _ = self.0.global_shortcut().unregister(shortcut);
        }
    }
}

/// 開発ビルドでだけ、アプリが出すイベントを標準エラーにも書く。
fn debug_log(message: String) {
    if cfg!(debug_assertions) {
        eprintln!("[auto-clicker] {message}");
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// トレイのメニュー表示とアイコンを、実行状態に合わせる。
fn update_tray(app: &AppHandle, run_state: RunState) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let Some(parts) = &state.tray else { return };
    if let Some(entry) = tray::tray_menu(run_state).first() {
        let _ = parts.toggle_item.set_text(entry.label);
    }
    let icon = tray::tray_icon(run_state, &parts.base_icon);
    let _ = parts.tray.set_icon(Some(Image::new_owned(icon.pixels, icon.width, icon.height)));
}

/// 「終了」。実行中なら止めてから終了する。
fn quit_app(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.quitting.store(true, Ordering::SeqCst);
        state.controller.stop_and_wait(Duration::from_secs(2));
    }
    app.exit(0);
}

fn build_tray(app: &AppHandle) -> Result<TrayParts, tauri::Error> {
    let items = tray::tray_menu(RunState::Idle)
        .iter()
        .map(|entry| MenuItem::with_id(app, entry.id, entry.label, true, None::<&str>))
        .collect::<Result<Vec<_>, _>>()?;
    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|item| item as &dyn IsMenuItem<Wry>).collect();
    let menu = Menu::with_items(app, &refs)?;
    let icon = app.default_window_icon().cloned().ok_or(tauri::Error::WindowNotFound)?;
    let base_icon = Rgba { width: icon.width(), height: icon.height(), pixels: icon.rgba().to_vec() };

    let tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("オートクリッカー")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match tray::tray_action(event.id().as_ref()) {
            Some(TrayAction::Toggle) => {
                let state = app.state::<AppState>();
                if let Err(error) = toggle_run(&state) {
                    let _ = app.emit("run-start-error", error);
                }
            }
            Some(TrayAction::ShowWindow) => show_main_window(app),
            Some(TrayAction::Quit) => quit_app(app),
            None => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(TrayParts { tray, toggle_item: items[0].clone(), base_icon })
}

/// `AutostartControl` の実 OS 実装。ロジックは持たない（手動で確認する）。
struct TauriAutostart<'a>(&'a AppHandle);

impl AutostartControl for TauriAutostart<'_> {
    fn is_enabled(&self) -> Result<bool, String> {
        self.0.autolaunch().is_enabled().map_err(|e| e.to_string())
    }
    fn enable(&mut self) -> Result<(), String> {
        self.0.autolaunch().enable().map_err(|e| e.to_string())
    }
    fn disable(&mut self) -> Result<(), String> {
        self.0.autolaunch().disable().map_err(|e| e.to_string())
    }
}

fn monitors(app: &AppHandle) -> Vec<Rect> {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| Rect { x: m.position().x, y: m.position().y, width: m.size().width, height: m.size().height })
        .collect()
}

fn env_info() -> EnvInfo {
    EnvInfo {
        os: if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        },
        xdg_session_type: std::env::var("XDG_SESSION_TYPE").ok(),
        wayland_display: std::env::var("WAYLAND_DISPLAY").ok(),
        // macOS の権限状態の取得は未実装。手動確認の対象（docs/manual-check.md）。
        mac_accessibility_trusted: true,
    }
}

/// 起動時の登録結果。setup の時点では画面がまだ購読していないので、画面側から問い合わせる。
#[tauri::command]
fn hotkey_status(state: tauri::State<AppState>) -> Hotkeys {
    state.key_names.lock().unwrap().clone()
}

/// ホットキーを付け替える。競合や不正なキーなら、元のキーに戻してエラーを返す。
#[tauri::command]
fn set_hotkeys(
    app: AppHandle,
    state: tauri::State<AppState>,
    toggle: String,
    capture: String,
) -> Result<Hotkeys, HotkeyError> {
    let mut names = state.key_names.lock().unwrap();
    let applied = hotkeys::apply_hotkeys(&mut TauriRegistrar(&app), &names, &toggle, &capture)?;

    let parse = |key: &Option<String>| key.as_ref().and_then(|k| k.parse::<Shortcut>().ok());
    *state.keys.lock().unwrap() = (parse(&applied.toggle), parse(&applied.capture));
    state.controller.set_hotkeys_ready(applied.toggle.is_some() && applied.capture.is_some());
    *names = applied.clone();

    let mut settings = state.store.load_settings();
    settings.toggle_hotkey = applied.toggle.clone().unwrap_or(settings.toggle_hotkey);
    settings.capture_hotkey = applied.capture.clone().unwrap_or(settings.capture_hotkey);
    let _ = state.store.save_settings(&settings);
    Ok(applied)
}

#[tauri::command]
fn warnings() -> Vec<String> {
    environment::check(&env_info()).iter().map(|w| format!("{w:?}")).collect()
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.store.load_settings()
}

/// 停止条件だけを保存する。設定の他の項目（キー、自動起動など）は触らない。
#[tauri::command]
fn set_stop(state: tauri::State<AppState>, stop: StopCondition) -> Result<(), String> {
    let mut settings = state.store.load_settings();
    settings.stop = stop;
    state.store.save_settings(&settings).map_err(|e| format!("{e:?}"))?;
    state.run.lock().unwrap().1 = stop;
    Ok(())
}

/// ログイン時の自動起動を切り替える。OS への登録に失敗したら、設定は変えずにエラーを返す。
#[tauri::command]
fn set_autostart(app: AppHandle, state: tauri::State<AppState>, enabled: bool) -> Result<(), AutostartError> {
    autostart::sync_autostart(&mut TauriAutostart(&app), enabled)?;
    let mut settings = state.store.load_settings();
    settings.autostart = enabled;
    state.store.save_settings(&settings).map_err(|e| AutostartError(format!("{e:?}")))
}

/// 手動のアップデート確認。失敗（オフライン等）は文言にして返す。
#[tauri::command]
async fn check_for_update(app: AppHandle) -> Result<updater::CheckOutcome, String> {
    updater::check_interactive(&app).await.map_err(|e| e.to_string())
}

const REPOSITORY_URL: &str = "https://github.com/happa0827/auto-clicker";

/// GitHub のリポジトリを既定のブラウザで開く。URL は固定で、画面からは渡せない。
#[tauri::command]
fn open_repository() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(REPOSITORY_URL).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_start_in_tray(state: tauri::State<AppState>, enabled: bool) -> Result<(), String> {
    let mut settings = state.store.load_settings();
    settings.start_in_tray = enabled;
    state.store.save_settings(&settings).map_err(|e| format!("{e:?}"))
}

#[tauri::command]
fn get_sequence(state: tauri::State<AppState>) -> Sequence {
    state.run.lock().unwrap().0.clone()
}

#[tauri::command]
fn save_sequence(state: tauri::State<AppState>, sequence: Sequence) -> Result<(), String> {
    state.store.save_sequence(&sequence).map_err(|e| format!("{e:?}"))?;
    state.run.lock().unwrap().0 = sequence;
    Ok(())
}

#[tauri::command]
fn list_presets(state: tauri::State<AppState>) -> Vec<model::Preset> {
    state.store.list_presets()
}

#[tauri::command]
fn save_preset(state: tauri::State<AppState>, preset: model::Preset) -> Result<(), String> {
    state.store.save_preset(preset).map_err(|e| format!("{e:?}"))
}

#[tauri::command]
fn load_preset(state: tauri::State<AppState>, name: String) -> Result<model::Preset, String> {
    state.store.load_preset(&name).map_err(|e| format!("{e:?}"))
}

#[tauri::command]
fn delete_preset(state: tauri::State<AppState>, name: String) -> Result<(), String> {
    state.store.delete_preset(&name).map_err(|e| format!("{e:?}"))
}

#[tauri::command]
fn toggle(state: tauri::State<AppState>) -> Result<(), controller::StartError> {
    toggle_run(&state)
}

fn toggle_run(state: &AppState) -> Result<(), controller::StartError> {
    if state.controller.state() == controller::RunState::Running {
        state.controller.stop();
        return Ok(());
    }
    let (sequence, stop) = state.run.lock().unwrap().clone();
    state.controller.start(sequence, stop)
}

fn register_hotkeys(app: &AppHandle, settings: &Settings) -> (Option<Shortcut>, Option<Shortcut>) {
    let try_register = |key: &str| -> Option<Shortcut> {
        let shortcut: Shortcut = key.parse().ok()?;
        app.global_shortcut().register(shortcut).ok().map(|_| shortcut)
    };
    (try_register(&settings.toggle_hotkey), try_register(&settings.capture_hotkey))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 多重起動の禁止。2 つ目を起動すると、既存のウィンドウを前面に出す。最初に登録する。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main_window(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let state = app.state::<AppState>();
                    let (toggle_key, capture_key) = *state.keys.lock().unwrap();
                    if Some(*shortcut) == toggle_key {
                        if let Err(error) = toggle_run(&state) {
                            debug_log(format!("start refused: {error:?}"));
                            let _ = app.emit("run-start-error", error);
                        }
                    } else if Some(*shortcut) == capture_key {
                        if let Ok(backend) = EnigoBackend::new() {
                            if let Ok((x, y)) = backend.cursor_position() {
                                debug_log(format!("captured: ({x}, {y})"));
                                let _ = app.emit("captured", Captured { x, y });
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let handle = app.handle().clone();
            let store = Store::new(app.path().app_data_dir()?);
            let settings = store.load_settings();
            let factory: BackendFactory =
                Arc::new(|| EnigoBackend::new().map(|b| Box::new(b) as Box<dyn InputBackend>));
            let screens_handle = handle.clone();
            let notify_handle = handle.clone();
            let controller = Controller::new(
                factory,
                Arc::new(SystemClock),
                Arc::new(move || monitors(&screens_handle)),
                Arc::new(move |event: Event| {
                    debug_log(format!("{event:?}"));
                    if let Event::State(run_state) = &event {
                        update_tray(&notify_handle, *run_state);
                    }
                    let _ = notify_handle.emit("run-event", event);
                }),
                std::time::Duration::from_millis(500),
            );

            let (toggle_ok, capture_ok) = register_hotkeys(&handle, &settings);
            controller.set_hotkeys_ready(toggle_ok.is_some() && capture_ok.is_some());
            debug_log(format!(
                "hotkeys: toggle={} capture={}",
                toggle_ok.is_some(),
                capture_ok.is_some()
            ));
            let key_names = Hotkeys {
                toggle: toggle_ok.map(|_| settings.toggle_hotkey.clone()),
                capture: capture_ok.map(|_| settings.capture_hotkey.clone()),
            };

            let default_target =
                ClickTarget { position: Position::Current, button: Button::Left, kind: ClickKind::Single };
            let interval = Interval::new(200.0, TimeUnit::Milliseconds).expect("valid default interval");
            let sequence = store.load_sequence().unwrap_or_else(|| Sequence::single(default_target, interval));

            let tray = match build_tray(&handle) {
                Ok(parts) => Some(parts),
                Err(error) => {
                    debug_log(format!("tray unavailable: {error}"));
                    None
                }
            };
            let tray_available = tray.is_some();

            app.manage(AppState {
                controller,
                run: Mutex::new((sequence, settings.stop)),
                store,
                keys: Mutex::new((toggle_ok, capture_ok)),
                key_names: Mutex::new(key_names),
                quitting: AtomicBool::new(false),
                tray,
            });

            // 設定どおりの自動起動になっているか確かめる。失敗しても起動は続ける。
            if let Err(error) = autostart::sync_autostart(&mut TauriAutostart(&handle), settings.autostart) {
                debug_log(format!("autostart sync failed: {}", error.0));
            }

            if window_policy::initial_visibility(settings.start_in_tray, tray_available) == Visibility::Shown {
                show_main_window(&handle);
            }
            updater::check_on_startup(handle.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            toggle,
            warnings,
            hotkey_status,
            set_hotkeys,
            get_settings,
            set_stop,
            set_autostart,
            set_start_in_tray,
            check_for_update,
            open_repository,
            get_sequence,
            save_sequence,
            list_presets,
            save_preset,
            load_preset,
            delete_preset
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let Some(state) = window.app_handle().try_state::<AppState>() else { return };
                match window_policy::on_close_requested(state.quitting.load(Ordering::SeqCst), state.tray.is_some()) {
                    CloseAction::HideToTray => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    CloseAction::Exit => {}
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // どの経路で終了しても、実行中のクリックを止めてから終わる。
            if let RunEvent::Exit = event {
                if let Some(state) = app.try_state::<AppState>() {
                    state.controller.stop_and_wait(Duration::from_secs(2));
                }
            }
        });
}
