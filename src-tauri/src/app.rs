use crate::config::{load_rules_from_paths, Action, Rules};
use crate::timer::{Clock, Scheduler, SystemClock};
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
    // The tray menu is owned by the tray icon after construction, but we also
    // need to popup() it from other entry points (right-click handler,
    // show_context_menu command). Wrap in Arc so we can share a clone with the
    // right-click closure while keeping the original in state.
    pub tray_menu: Arc<Menu<tauri::Wry>>,
}

pub type SharedState = Arc<Mutex<AppState>>;

#[derive(Clone, Serialize)]
struct TriggerPayload {
    action: String,
}

#[derive(Clone, Serialize)]
struct PausedPayload {
    paused: bool,
}

pub fn run() -> tauri::Result<()> {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);

    tauri::Builder::default()
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Resolve resource paths. Bundled resources live under app_data_dir on most platforms,
            // but Tauri exposes a dedicated resource_dir. Try both.
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("resolve app_data_dir");
            std::fs::create_dir_all(&data_dir).ok();

            let external_rules = locate_external_rules(&app_handle);
            let bundled_rules = app_handle
                .path()
                .resolve("rules.json", tauri::path::BaseDirectory::Resource)
                .unwrap_or_else(|_| data_dir.join("rules.json"));

            let rules = load_rules_from_paths(&external_rules, &bundled_rules);

            let scheduler = Scheduler::new(rules.clone(), clock.as_ref());

            // Restore window state (position + scale) before building the tray menu,
            // so the menu can mark the current size.
            let saved_state = window_state::load(&data_dir);
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
                tray_menu: menu_arc.clone(),
            }));

            app.manage(state.clone());

            // Restore window position + size.
            if let Some(window) = app_handle.get_webview_window("main") {
                let pos = saved_state.unwrap_or_else(|| {
                        // Default: bottom-right of primary monitor.
                        if let Some(monitor) = window.primary_monitor().ok().flatten() {
                            let sf = monitor.size();
                            WindowPosition {
                                x: sf.width as f64 - SPRITE_W - 35.0,
                                y: 55.0,
                                scale: window_state::DEFAULT_SCALE,
                            }
                        } else {
                            WindowPosition {
                                x: 100.0,
                                y: 100.0,
                                scale: window_state::DEFAULT_SCALE,
                            }
                        }
                    });
                let _ = window.set_position(PhysicalPosition::new(pos.x, pos.y));
                let _ = window.set_size(LogicalSize::new(
                    SPRITE_W * initial_scale,
                    SPRITE_H * initial_scale,
                ));
            }

            // Build tray icon (the menu was already built above and cached on
            // AppState; we share the same Arc with the tray builder).
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app_handle.default_window_icon().cloned().unwrap())
                .icon_as_template(true)
                .menu(menu_arc.as_ref())
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    handle_menu_event(app, event);
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Right,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        // Tauri 2.x removed `TrayIcon::menu()`, so we keep the
                        // menu cached on AppState and pop it up from there.
                        popup_tray_menu(tray.app_handle());
                    }
                })
                .build(app)?;

            // Spawn the rules.json file watcher. When the external file changes,
            // reload rules and emit `rules_changed` so the frontend can refresh.
            let app_handle_for_watcher = app_handle.clone();
            let state_for_watcher = state.clone();
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
                                    let st = state.lock().unwrap();
                                    load_rules_from_paths(&st.external_rules_path, &st.bundled_rules_path)
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
                    if let Some(action) = st.scheduler.tick(clock_for_tick.as_ref()) {
                        drop(st);
                        let _ = app_handle_for_tick.emit(
                            "trigger",
                            TriggerPayload { action: action.as_str().to_string() },
                        );
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
        ])
        .run(tauri::generate_context!())
}

fn locate_external_rules(_app: &AppHandle) -> PathBuf {
    // The external rules.json lives next to the executable (same convention as the Swift version).
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    #[cfg(target_os = "macos")]
    {
        // For packaged apps: exe is .../Foo.app/Contents/MacOS/Foo; we want .../ rules.json
        // (i.e., parent of the .app bundle). Climb 4 levels: MacOS → Contents → .app → parent-of-app.
        if let Some(parent) = exe.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).and_then(|p| p.parent()) {
            return parent.join("rules.json");
        }
    }
    // Dev mode, Windows, Linux: just next to the binary.
    exe.parent().map(|p| p.join("rules.json")).unwrap_or(PathBuf::from("rules.json"))
}

fn build_tray_menu(
    app: &AppHandle,
    current_scale: f64,
) -> tauri::Result<Menu<tauri::Wry>> {
    use tauri::menu::Submenu;
    let sneeze = MenuItem::with_id(app, "sneeze", "打喷嚏", true, None::<&str>)?;
    let knead = MenuItem::with_id(app, "knead", "踩奶", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let reload = MenuItem::with_id(app, "reload", "重新加载规则", true, None::<&str>)?;
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
        &[&sneeze, &knead, &sep1, &reload, &open, &pause, &size_submenu, &sep2, &quit],
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
            emit_trigger(app, "sneezing");
            let mut st = state.lock().unwrap();
            st.scheduler
                .trigger(crate::config::Action::Sneezing, &SystemClock);
        }
        "knead" => {
            emit_trigger(app, "kneading");
            let mut st = state.lock().unwrap();
            st.scheduler
                .trigger(crate::config::Action::Kneading, &SystemClock);
        }
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
        Some(w + f)
    } else {
        digits.parse().ok()
    }
}

fn emit_trigger(app: &AppHandle, action: &str) {
    let _ = app.emit("trigger", TriggerPayload { action: action.to_string() });
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
    emit_trigger(&app, &action);
    let mut st = state.lock().unwrap();
    st.scheduler.trigger(parsed, &SystemClock);
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
