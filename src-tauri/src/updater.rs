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
    check_interactive(app).await.map(|_| ())
}

/// 手動確認の結果。更新を受け入れた場合は再起動するので、ここには戻らない。
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    UpToDate,
    Declined,
}

/// 新版があれば承諾を得て更新する。最新なら `UpToDate`、見送られたら `Declined`。
pub async fn check_interactive(app: &AppHandle) -> tauri_plugin_updater::Result<CheckOutcome> {
    let Some(update) = app.updater()?.check().await? else { return Ok(CheckOutcome::UpToDate) };
    let accepted = app
        .dialog()
        .message(format!("新しいバージョン {} があります。更新して再起動しますか？", update.version))
        .title("アップデート")
        .buttons(MessageDialogButtons::OkCancelCustom("更新する".into(), "あとで".into()))
        .blocking_show();
    if accepted {
        if let Err(error) = update.download_and_install(|_, _| {}, || {}).await {
            // 無言で失敗すると「押しても何も起きない」ように見えるので、理由を見せる。
            app.dialog()
                .message(format!("更新に失敗しました。
{error}"))
                .title("アップデート")
                .blocking_show();
            return Err(error);
        }
        app.restart();
    }
    Ok(CheckOutcome::Declined)
}
