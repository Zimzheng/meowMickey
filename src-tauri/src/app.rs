use crate::config::{load_rules_from_paths, Action, Rules};
use crate::timer::{Clock, Scheduler, SystemClock};
use crate::hydration::{self, HydrationRecord, HydrationSummary};
use crate::window_state::{self, WindowPosition};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::menu::{ContextMenu, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WindowEvent};

const SPRITE_W: f64 = window_state::SPRITE_WIDTH;
const SPRITE_H: f64 = window_state::SPRITE_HEIGHT;

pub struct AppState {
    pub scheduler: Scheduler,
    pub data_dir: PathBuf,
    pub external_rules_path: PathBuf,
    pub bundled_rules_path: PathBuf,
    pub scale: f64,
    pub hydration_panel_open: bool,
    // The tray menu is owned by the tray icon after construction, but we also
    // need to popup() it from other entry points (right-click handler,
    // show_context_menu command). Wrap in Arc so we can share a clone with the
    // right-click closure while keeping the original in state.
    pub tray_menu: Arc<Menu<tauri::Wry>>,
}

pub type SharedState = Arc<Mutex<AppState>>;

#[derive(Clone, Serialize)]
struct TriggerPayload {
    actions: Vec<String>,
}

#[derive(Clone, Serialize)]
struct PausedPayload {
    paused: bool,
}

pub fn run() -> tauri::Result<()> {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Resolve resource paths. Bundled resources live under app_data_dir on most platforms,
            // but Tauri exposes a dedicated resource_dir. Try both.
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("resolve app_data_dir");
            std::fs::create_dir_all(&data_dir).ok();

            let bundled_rules = app_handle
                .path()
                .resolve("rules.json", tauri::path::BaseDirectory::Resource)
                .unwrap_or_else(|_| data_dir.join("rules.json"));
            let external_rules = locate_external_rules(&data_dir, &bundled_rules);

            let rules = load_rules_from_paths(&external_rules, &bundled_rules);

            let scheduler = Scheduler::new(rules.clone(), clock.as_ref());

            // Restore window state (position + scale) before building the tray menu,
            // so the menu can mark the current size.
            let saved_state = window_state::load(&data_dir).and_then(|mut saved| {
                // Recover legacy/off-screen states that can leave a 50% pet nearly
                // invisible. Version 1.2.0 could persist this value while the
                // hydration panel was transitioning, so migrate it once to the
                // normal size. Users can still choose another size afterward.
                if saved.scale > 0.75 {
                    saved.scale = (saved.scale * 0.5)
                        .clamp(window_state::MIN_SCALE, window_state::MAX_SCALE);
                }
                Some(saved)
            });
            let initial_scale = saved_state
                .as_ref()
                .map(|s| s.scale.clamp(window_state::MIN_SCALE, window_state::MAX_SCALE))
                .unwrap_or(window_state::DEFAULT_SCALE);

            // Build the tray menu before constructing the tray icon so we can
            // share it via AppState.
            let menu = build_tray_menu(&app_handle, initial_scale)?;
            let menu_arc = Arc::new(menu);

            let state = Arc::new(Mutex::new(AppState {
                scheduler,
                data_dir: data_dir.clone(),
                external_rules_path: external_rules,
                bundled_rules_path: bundled_rules,
                scale: initial_scale,
                hydration_panel_open: false,
                tray_menu: menu_arc.clone(),
            }));

            app.manage(state.clone());

            // Restore window position + size.
            if let Some(window) = app_handle.get_webview_window("main") {
                let pos = if let Some(monitor) = window.primary_monitor().ok().flatten() {
                    let monitor_pos = monitor.position();
                    let monitor_size = monitor.size();
                    let scale_factor = monitor.scale_factor();
                    let candidate = saved_state.unwrap_or_else(|| {
                        window_state::default_physical_position(
                            monitor_pos.x,
                            monitor_pos.y,
                            monitor_size.width,
                            monitor_size.height,
                            scale_factor,
                            initial_scale,
                        )
                    });
                    window_state::clamp_to_monitor(
                        candidate,
                        monitor_pos.x,
                        monitor_pos.y,
                        monitor_size.width,
                        monitor_size.height,
                        scale_factor,
                    )
                } else {
                    saved_state.unwrap_or(WindowPosition {
                        x: 100.0,
                        y: 100.0,
                        scale: initial_scale,
                    })
                };
                let _ = window.set_position(PhysicalPosition::new(pos.x, pos.y));
                let _ = window.set_size(LogicalSize::new(
                    SPRITE_W * initial_scale,
                    SPRITE_H * initial_scale,
                ));
                // Persist the clamped position and any legacy scale migration
                // immediately so the next launch cannot revive the stale state.
                let _ = window_state::save(&data_dir, WindowPosition {
                    x: pos.x,
                    y: pos.y,
                    scale: initial_scale,
                });
                let _ = window.show();
            }

            // Build tray icon (the menu was already built above and cached on
            // AppState; we share the same Arc with the tray builder).
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app_handle.default_window_icon().cloned().unwrap())
                .icon_as_template(false)
                .menu(menu_arc.as_ref())
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    handle_menu_event(app, event);
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        match button {
                            MouseButton::Right => popup_tray_menu(tray.app_handle()),
                            MouseButton::Left => show_pet(tray.app_handle()),
                            _ => {}
                        }
                    }
                })
                .build(app)?;

            // Spawn the rules.json file watcher. When the external file changes,
            // reload rules and emit `rules_changed` so the frontend can refresh.
            let app_handle_for_watcher = app_handle.clone();
            let state_for_watcher = state.clone();
            let clock_for_watcher = clock.clone();
            let watch_path = {
                let st = state_for_watcher.lock().unwrap();
                st.external_rules_path.clone()
            };
            let watch_dir = watch_path.parent().map(|p| p.to_path_buf());
            if let Some(dir) = watch_dir {
                tauri::async_runtime::spawn(async move {
                    use notify::{RecursiveMode, Watcher};
                    let app = app_handle_for_watcher.clone();
                    let state = state_for_watcher.clone();
                    let clock = clock_for_watcher.clone();
                    let watch_path_str = watch_path.to_string_lossy().to_string();
                    let res = tauri::async_runtime::spawn_blocking(move || -> notify::Result<()> {
                        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
                        let mut watcher = notify::recommended_watcher(tx)?;
                        watcher.watch(&dir, RecursiveMode::NonRecursive)?;
                        for event in rx.into_iter().flatten() {
                            let touched = event.paths.iter().any(|p| {
                                p.to_string_lossy() == watch_path_str
                            });
                            if touched {
                                let rules = {
                                    let mut st = state.lock().unwrap();
                                    let rules = load_rules_from_paths(
                                        &st.external_rules_path,
                                        &st.bundled_rules_path,
                                    );
                                    st.scheduler.reload_rules(rules.clone(), clock.as_ref());
                                    rules
                                };
                                let _ = app.emit("rules_changed", rules);
                            }
                        }
                        Ok(())
                    }).await;
                    let _ = res;
                });
            }

            // Spawn the tick loop.
            let app_handle_for_tick = app_handle.clone();
            let state_for_tick = state.clone();
            let clock_for_tick = clock.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(33));
                loop {
                    interval.tick().await;
                    let mut st = state_for_tick.lock().unwrap();
                    if let Some(actions) = st.scheduler.tick(clock_for_tick.as_ref()) {
                        drop(st);
                        emit_sequence(&app_handle_for_tick, actions);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide instead of quit so tray stays useful.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_rules,
            start_drag,
            update_drag,
            end_drag,
            trigger_action,
            show_context_menu,
            quit,
            native_drag,
            set_scale,
            get_scale,
            set_local_hour,
            record_hydration,
            get_hydration_summary,
            snooze_hydration,
            show_hydration_panel,
            undo_hydration,
            set_hydration_panel_open,
            copy_image_to_clipboard,
        ])
        .run(tauri::generate_context!())
}

#[tauri::command]
fn copy_image_to_clipboard(data_url: String) -> Result<(), String> {
    let encoded = data_url.split_once(',').map(|(_, value)| value).unwrap_or(&data_url);
    let temp = std::env::temp_dir().join(format!("mickey-share-{}.png", std::process::id()));
    #[cfg(target_os = "macos")]
    {
        let mut decode = std::process::Command::new("base64").args(["-D", "-o"]).arg(&temp).stdin(std::process::Stdio::piped()).spawn().map_err(|e| e.to_string())?;
        use std::io::Write;
        decode.stdin.as_mut().unwrap().write_all(encoded.as_bytes()).map_err(|e| e.to_string())?;
        if !decode.wait().map_err(|e| e.to_string())?.success() { return Err("图片解码失败".into()); }
        let script = format!("set the clipboard to (read POSIX file \"{}\" as «class PNGf»)" , temp.display());
        let ok = std::process::Command::new("osascript").args(["-e", &script]).status().map_err(|e| e.to_string())?.success();
        let _ = std::fs::remove_file(&temp); if ok { Ok(()) } else { Err("系统剪贴板不可用".into()) }
    }
    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        let mut decode = std::process::Command::new("certutil").args(["-decode", "-f", "CON", temp.to_str().unwrap()]).stdin(std::process::Stdio::piped()).spawn().map_err(|e| e.to_string())?;
        decode.stdin.as_mut().unwrap().write_all(encoded.as_bytes()).map_err(|e| e.to_string())?; let _ = decode.wait();
        let script = format!("Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $i=[Drawing.Image]::FromFile('{}'); [Windows.Forms.Clipboard]::SetImage($i)", temp.display());
        let ok = std::process::Command::new("powershell").args(["-NoProfile", "-Command", &script]).status().map_err(|e| e.to_string())?.success(); let _ = std::fs::remove_file(&temp); if ok { Ok(()) } else { Err("系统剪贴板不可用".into()) }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { let _ = encoded; Err("当前系统暂不支持图片剪贴板".into()) }
}

fn locate_external_rules(data_dir: &std::path::Path, bundled_rules: &std::path::Path) -> PathBuf {
    // Portable builds keep rules.json beside the .app/executable. Installed builds
    // use the writable per-user data directory.
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let beside_executable = {
        #[cfg(target_os = "macos")]
        {
            // For packaged apps: exe is .../Foo.app/Contents/MacOS/Foo; we want .../ rules.json
            // (i.e., parent of the .app bundle). Climb 4 levels: MacOS → Contents → .app → parent-of-app.
            if let Some(parent) = exe
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
            {
                parent.join("rules.json")
            } else {
                PathBuf::from("rules.json")
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            exe.parent()
                .map(|p| p.join("rules.json"))
                .unwrap_or(PathBuf::from("rules.json"))
        }
    };
    if beside_executable.is_file() {
        return beside_executable;
    }

    let user_rules = data_dir.join("rules.json");
    if !user_rules.exists() {
        let initial = std::fs::read(bundled_rules).unwrap_or_else(|_| {
            serde_json::to_vec_pretty(&Rules::default_rules()).unwrap_or_default()
        });
        let _ = std::fs::write(&user_rules, initial);
    }
    user_rules
}

fn build_tray_menu(
    app: &AppHandle,
    current_scale: f64,
) -> tauri::Result<Menu<tauri::Wry>> {
    use tauri::menu::Submenu;
    let sneeze = MenuItem::with_id(app, "sneeze", "打喷嚏", true, None::<&str>)?;
    let knead = MenuItem::with_id(app, "knead", "踩奶", true, None::<&str>)?;
    let head_tilt = MenuItem::with_id(app, "headtilt", "好奇歪头", true, None::<&str>)?;
    let stretch = MenuItem::with_id(app, "stretch", "伸懒腰", true, None::<&str>)?;
    let hydration = MenuItem::with_id(app, "hydration", "喝水与记录", true, None::<&str>)?;
    let hydration_undo = MenuItem::with_id(app, "hydration_undo", "撤销最近一次喝水", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let reload = MenuItem::with_id(app, "reload", "重新加载规则", true, None::<&str>)?;
    let update_version = MenuItem::with_id(app, "update_version", "更新米奇版本", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "打开规则文件", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "暂停／继续定时动作", true, None::<&str>)?;

    // Size submenu
    let sizes: [(&str, &str); 6] = [
        ("size_0_5", "50%"),
        ("size_0_75", "75%"),
        ("size_1_0", "100%"),
        ("size_1_25", "125%"),
        ("size_1_5", "150%"),
        ("size_2_0", "200%"),
    ];
    let size_items: Vec<MenuItem<tauri::Wry>> = sizes
        .iter()
        .map(|(id, label)| MenuItem::with_id(app, *id, *label, true, None::<&str>).unwrap())
        .collect();
    let size_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = size_items
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect();
    // Note: tauri 2.11 Submenu::with_items requires `&[&dyn IsMenuItem]`
    let size_submenu = Submenu::with_items(app, "大小", true, &size_refs)?;

    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出米奇", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[&sneeze, &knead, &head_tilt, &stretch, &hydration, &hydration_undo, &sep1, &reload, &update_version, &open, &pause, &size_submenu, &sep2, &quit],
    )
    // keep `current_scale` referenced so the linter doesn't complain;
    // it's used in handle_menu_event for the ✓ indicator (future work).
    .map(|m| {
        let _ = current_scale;
        m
    })
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    let state: tauri::State<SharedState> = app.state();
    match event.id().as_ref() {
        "sneeze" => {
            let mut st = state.lock().unwrap();
            let action = crate::config::Action::Sneezing;
            st.scheduler.trigger(action, &SystemClock);
            drop(st);
            emit_sequence(app, Scheduler::sequence_for(action));
        }
        "knead" => {
            let mut st = state.lock().unwrap();
            let action = crate::config::Action::Kneading;
            st.scheduler.trigger(action, &SystemClock);
            drop(st);
            emit_sequence(app, Scheduler::sequence_for(action));
        }
        "headtilt" => emit_sequence(app, vec![Action::HeadTilt]),
        "stretch" => emit_sequence(app, vec![Action::Stretching]),
        "hydration" => {
            let _ = app.emit("hydration_prompt", ());
        }
        "hydration_undo" => { let _ = app.emit("hydration_undo_request", ()); }
        "update_version" => crate::updater::start(app),
        "reload" => {
            reload_rules_into(app);
        }
        "open" => {
            let st = state.lock().unwrap();
            let path = &st.external_rules_path;
            // Best-effort: open with the OS default handler.
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(path).spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd")
                .args(&["/c", "start", "", path.to_str().unwrap_or("")])
                .spawn();
            #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
        "pause" => {
            let mut st = state.lock().unwrap();
            let new_paused = !st.scheduler.paused();
            st.scheduler.set_paused(new_paused);
            drop(st);
            let _ = app.emit("paused", PausedPayload { paused: new_paused });
        }
        id if id.starts_with("size_") => {
            if let Some(scale) = parse_size_id(id) {
                apply_scale(app, scale);
            }
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

fn apply_scale(app: &AppHandle, scale: f64) {
    let scale = scale.clamp(window_state::MIN_SCALE, window_state::MAX_SCALE);
    let state: tauri::State<SharedState> = app.state();
    let mut st = state.lock().unwrap();
    st.scale = scale;
    let pos = app
        .get_webview_window("main")
        .and_then(|w| w.outer_position().ok())
        .map(|p| (p.x as f64, p.y as f64))
        .unwrap_or((0.0, 0.0));
    let _ = window_state::save(
        &st.data_dir,
        window_state::WindowPosition {
            x: pos.0,
            y: pos.1,
            scale,
        },
    );
    drop(st);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_size(LogicalSize::new(
            window_state::SPRITE_WIDTH * scale,
            window_state::SPRITE_HEIGHT * scale,
        ));
    }
    let _ = app.emit("scale_changed", scale);
}

fn parse_size_id(id: &str) -> Option<f64> {
    let digits = id.strip_prefix("size_")?;
    if let Some((whole, frac)) = digits.split_once('_') {
        let w: f64 = whole.parse().ok()?;
        let f: f64 = format!("0.{}", frac).parse().ok()?;
        Some((w + f) * 0.5)
    } else {
        digits.parse::<f64>().ok().map(|v| v * 0.5)
    }
}

fn emit_sequence(app: &AppHandle, actions: Vec<Action>) {
    let actions = actions.into_iter().map(|action| action.as_str().to_string()).collect();
    let _ = app.emit("trigger", TriggerPayload { actions });
}

fn reload_rules_into(app: &AppHandle) {
    let state: tauri::State<SharedState> = app.state();
    let mut st = state.lock().unwrap();
    let rules = load_rules_from_paths(&st.external_rules_path, &st.bundled_rules_path);
    st.scheduler.reload_rules(rules.clone(), &SystemClock);
    drop(st);
    let _ = app.emit("rules_changed", rules);
}

/// Pop up the tray menu against the main webview window. Looks up the cached
/// menu from AppState rather than calling `tray.menu()` (which doesn't exist on
/// `TrayIcon` in Tauri 2.x — the menu is only available via `set_menu`).
fn popup_tray_menu(app: &AppHandle) {
    let state: tauri::State<SharedState> = app.state();
    let menu = {
        let st = state.lock().unwrap();
        st.tray_menu.clone()
    };
    // ContextMenu::popup requires a `Window`, not a `WebviewWindow`. We look up
    // the raw window by label instead of the webview wrapper.
    if let Some(window) = app.get_window("main") {
        let _ = menu.popup(window);
    }
}

fn show_pet(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
    }
}

// ---------- Commands ----------

#[tauri::command]
fn get_rules(state: tauri::State<SharedState>) -> Rules {
    let st = state.lock().unwrap();
    // We don't store the parsed Rules on AppState (only inside the Scheduler),
    // so reload from disk to return the current truth.
    load_rules_from_paths(&st.external_rules_path, &st.bundled_rules_path)
}

#[derive(Serialize, Clone, Copy)]
struct DragOrigin {
    x: f64,
    y: f64,
}

// Per-drag state stored in a OnceLock-equivalent (we use a Mutex<Option<...>> on a global).
static DRAG_ORIGIN: OnceLock<Mutex<Option<DragOrigin>>> = OnceLock::new();

fn drag_cell() -> &'static Mutex<Option<DragOrigin>> {
    DRAG_ORIGIN.get_or_init(|| Mutex::new(None))
}

#[tauri::command]
fn start_drag(window: tauri::Window) -> DragOrigin {
    let pos = window.outer_position().unwrap_or_default();
    let origin = DragOrigin {
        x: pos.x as f64,
        y: pos.y as f64,
    };
    *drag_cell().lock().unwrap() = Some(origin);
    origin
}

#[tauri::command]
fn update_drag(window: tauri::Window, x: f64, y: f64) {
    let Some(origin) = *drag_cell().lock().unwrap() else { return };
    let _ = window.set_position(PhysicalPosition::new(
        origin.x + x,
        origin.y + y,
    ));
}

#[tauri::command]
fn end_drag(state: tauri::State<SharedState>, window: tauri::Window) {
    *drag_cell().lock().unwrap() = None;
    if let Ok(pos) = window.outer_position() {
        let st = state.lock().unwrap();
        let _ = window_state::save(&st.data_dir, WindowPosition {
            x: pos.x as f64,
            y: pos.y as f64,
            scale: st.scale,
        });
    }
}

#[tauri::command]
fn trigger_action(app: AppHandle, state: tauri::State<SharedState>, action: String) {
    let parsed = match Action::from_name(&action) {
        Some(a) => a,
        None => return,
    };
    if parsed == Action::Idle {
        return;
    }
    let mut st = state.lock().unwrap();
    st.scheduler.trigger(parsed, &SystemClock);
    drop(st);
    emit_sequence(&app, Scheduler::sequence_for(parsed));
}

#[tauri::command]
fn show_context_menu(app: AppHandle) {
    popup_tray_menu(&app);
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn native_drag(window: tauri::Window) {
    let _ = window.start_dragging();
}

#[tauri::command]
fn set_scale(app: AppHandle, scale: f64) {
    apply_scale(&app, scale);
}

#[tauri::command]
fn get_scale(state: tauri::State<SharedState>) -> f64 {
    state.lock().unwrap().scale
}

#[tauri::command]
fn set_local_hour(state: tauri::State<SharedState>, hour: u8) {
    state.lock().unwrap().scheduler.set_local_hour(hour);
}

#[tauri::command]
fn record_hydration(
    app: AppHandle,
    state: tauri::State<SharedState>,
    amount_ml: u32,
    recorded_at_ms: u64,
    local_date: String,
    local_time: String,
    source: String,
) -> Result<HydrationSummary, String> {
    let (data_dir, id) = {
        let mut st = state.lock().unwrap();
        st.scheduler.reset_hydration(&SystemClock, None);
        (st.data_dir.clone(), format!("{}-{}", recorded_at_ms, amount_ml))
    };
    let result = hydration::append(&data_dir, HydrationRecord {
        id, amount_ml, recorded_at_ms, local_date, local_time,
        source: if source == "manual" { "manual".into() } else { "reminder".into() },
    })?;
    emit_sequence(&app, vec![Action::Drinking, Action::Contented]);
    Ok(result)
}

#[tauri::command]
fn get_hydration_summary(state: tauri::State<SharedState>, local_date: String) -> HydrationSummary {
    let st = state.lock().unwrap();
    hydration::summary(&hydration::load(&st.data_dir), &local_date)
}

#[tauri::command]
fn snooze_hydration(state: tauri::State<SharedState>, minutes: Option<f64>) {
    state.lock().unwrap().scheduler.reset_hydration(&SystemClock, minutes.or(Some(10.0)));
}

#[tauri::command]
fn show_hydration_panel(app: AppHandle) {
    let _ = app.emit("hydration_prompt", ());
}

#[tauri::command]
fn undo_hydration(state: tauri::State<SharedState>, local_date: String) -> Result<HydrationSummary, String> {
    let st = state.lock().unwrap();
    hydration::undo_latest(&st.data_dir, &local_date)
}

#[tauri::command]
async fn set_hydration_panel_open(window: tauri::Window, state: tauri::State<'_, SharedState>, open: bool) -> Result<(), String> {
    let app = window.app_handle();
    state.lock().unwrap().hydration_panel_open = open;
    if !open {
        if let Some(panel) = app.get_webview_window("hydration-side") {
            panel.hide().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    // Keep the pet's webview geometry unchanged. Only the side window moves.
    let pet = app.get_webview_window("main").ok_or("米奇窗口不存在")?;
    let panel = if let Some(panel) = app.get_webview_window("hydration-side") {
        panel
    } else {
        tauri::WebviewWindowBuilder::new(app, "hydration-side", tauri::WebviewUrl::App("index.html?side=hydration".into()))
            .title("米奇 · 喝水与记录").inner_size(270.0, 410.0)
            .decorations(false).transparent(true).shadow(false)
            .resizable(false).always_on_top(true).skip_taskbar(true)
            .visible(false).build().map_err(|e| e.to_string())?
    };
    let pos = pet.outer_position().map_err(|e| e.to_string())?;
    let size = pet.outer_size().map_err(|e| e.to_string())?;
    let factor = pet.scale_factor().unwrap_or(1.0);
    let mut x = pos.x as f64 - 278.0 * factor;
    let mut y = pos.y as f64 + size.height as f64 - 410.0 * factor;
    if let Ok(Some(monitor)) = pet.current_monitor() {
        let origin = monitor.position();
        let screen = monitor.size();
        if x < origin.x as f64 {
            x = pos.x as f64 + size.width as f64 + 8.0 * factor;
        }
        y = y.clamp(origin.y as f64, (origin.y as f64 + screen.height as f64 - 410.0 * factor).max(origin.y as f64));
    }
    panel.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    let _ = app.emit_to("hydration-side", "hydration_side_open", ());
    panel.show().map_err(|e| e.to_string())
}
