use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

static BUSY: AtomicBool = AtomicBool::new(false);
const MENU_LABEL: &str = "更新米奇版本";

fn menu_status(app: &AppHandle, label: &str, enabled: bool) {
    let state: tauri::State<crate::app::SharedState> = app.state();
    let menu = state.lock().unwrap().tray_menu.clone();
    if let Some(item) = menu.get("update_version").and_then(|item| item.as_menuitem().cloned()) {
        let _ = item.set_text(label);
        let _ = item.set_enabled(enabled);
    }
}

fn notice(app: &AppHandle, message: impl Into<String>, error: bool) {
    app.dialog().message(message).title("米奇 · 版本更新")
        .kind(if error { MessageDialogKind::Error } else { MessageDialogKind::Info })
        .buttons(MessageDialogButtons::OkCustom("知道了".into())).show(|_| {});
}

async fn confirm(app: &AppHandle, message: String, yes: &str, no: &str) -> bool {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog().message(message).title("米奇 · 版本更新")
        .buttons(MessageDialogButtons::OkCancelCustom(yes.into(), no.into()))
        .show(move |answer| { let _ = sender.send(answer); });
    receiver.await.unwrap_or(false)
}

// One update operation at a time, including confirmation and download.
pub fn start(app: &AppHandle) {
    if BUSY.swap(true, Ordering::AcqRel) { return; }
    let app = app.clone();
    menu_status(&app, "正在检查更新…", false);
    tauri::async_runtime::spawn(async move {
        if let Err(message) = check_and_install(&app).await { notice(&app, message, true); }
        menu_status(&app, MENU_LABEL, true);
        BUSY.store(false, Ordering::Release);
    });
}

async fn check_and_install(app: &AppHandle) -> Result<(), String> {
    let updater = app.updater_builder().timeout(Duration::from_secs(60)).build()
        .map_err(|_| "无法初始化版本更新，请稍后重试。".to_string())?;
    let update = match updater.check().await {
        Ok(update) => update,
        Err(tauri_plugin_updater::Error::ReleaseNotFound) => {
            notice(app, format!("目前暂无可安装的更新包。\n当前版本：{}", app.package_info().version), false);
            return Ok(());
        }
        Err(error) => {
            eprintln!("Mickey update check: {error}");
            return Err("暂时无法检查更新，请检查网络后重试。当前安装不会改变。".into());
        }
    };
    let Some(mut update) = update else {
        notice(app, format!("当前版本 {}，已是最新版本。", app.package_info().version), false);
        return Ok(());
    };
    let notes = update.body.as_deref().unwrap_or("优化体验与修复问题。");
    if !confirm(app, format!("发现米奇 {}\n当前版本：{}\n\n{}\n\n安装后需要重启米奇，喝水和互动记录会保留。", update.version, app.package_info().version, notes), "更新并重启", "稍后再说").await {
        return Ok(());
    }
    update.timeout = Some(Duration::from_secs(10 * 60));
    menu_status(app, "正在下载更新…", false);
    let mut received = 0u64;
    let mut last_percent = None;
    update.download_and_install(|chunk, total| {
        received += chunk as u64;
        if let Some(total) = total.filter(|total| *total > 0) {
            let percent = (received.saturating_mul(100) / total).min(100);
            if last_percent != Some(percent) {
                last_percent = Some(percent);
                menu_status(app, &format!("正在下载更新… {percent}%"), false);
            }
        }
    }, || menu_status(app, "正在验证并安装…", false)).await.map_err(|error| {
        eprintln!("Mickey update install: {error}");
        "更新未完成，请稍后重试。下载包必须通过签名验证，喝水和互动记录不会被删除。".to_string()
    })?;
    // Windows installers handle application exit/restart; macOS needs an explicit restart.
    app.restart();
}
