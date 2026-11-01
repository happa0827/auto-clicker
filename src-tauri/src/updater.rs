use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt;

/// 起動時に新版を確認し、見つかったら承諾を得てから更新して再起動する。
/// 失敗（オフライン等）しても通常動作を妨げないよう、結果は捨てる。
pub fn check_on_startup(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = check(&app).await;
    });
}

async fn check(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater()?.check().await? else { return Ok(()) };
    let accepted = app
        .dialog()
        .message(format!("新しいバージョン {} があります。更新して再起動しますか？", update.version))
        .title("アップデート")
        .buttons(MessageDialogButtons::OkCancelCustom("更新する".into(), "あとで".into()))
        .blocking_show();
    if accepted {
        update.download_and_install(|_, _| {}, || {}).await?;
        app.restart();
    }
    Ok(())
}
