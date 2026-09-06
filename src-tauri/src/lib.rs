use std::sync::{Mutex, OnceLock};
use tauri::Manager;
use tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder};

// 全局静态变量存储通过 OS 文件关联 / 命令行打开的文件路径
// macOS/iOS/Android 在 RunEvent::Opened 中填充；Windows/Linux 在启动时从命令行参数读取。
// 注意：Tauri v2 的 RunEvent::Opened 仅在 macOS/iOS 存在，Windows/Linux 不会触发，
// 因此 Windows/Linux 必须自行从 std::env::args() 读取文件路径。
static OPENED_PATHS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn opened_paths_lock() -> &'static Mutex<Vec<String>> {
    OPENED_PATHS.get_or_init(|| Mutex::new(vec![]))
}

// 冷启动时前端调用，返回本次启动需要打开的文件路径列表
#[tauri::command]
fn opened_paths(app: tauri::AppHandle) -> Vec<String> {
    use tauri_plugin_fs::FsExt;
    let paths = opened_paths_lock().lock().unwrap().clone();
    // 授权前端读取这些文件（冷启动时 fs_scope 可能还没初始化，这里补授权）
    for path in &paths {
        let _ = app.fs_scope().allow_file(std::path::Path::new(path));
    }
    paths
}

// 动态授权前端读取单个文件（用于最近文件、手动打开的文件等）
#[tauri::command]
fn allow_file(app: tauri::AppHandle, path: String) {
    use tauri_plugin_fs::FsExt;
    let _ = app.fs_scope().allow_file(std::path::Path::new(&path));
}

// 动态授权前端读取目录（用于左侧文件夹浏览器）
#[tauri::command]
fn allow_directory(app: tauri::AppHandle, path: String) {
    use tauri_plugin_fs::FsExt;
    let _ = app.fs_scope().allow_directory(std::path::Path::new(&path), true);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![opened_paths, allow_file, allow_directory])
        .setup(|app| {
            // macOS 自定义菜单：绑定快捷键并通过事件通知前端
            let new_item = MenuItemBuilder::with_id("new", "New")
                .accelerator("CmdOrCtrl+N")
                .build(app)?;
            let open_item = MenuItemBuilder::with_id("open", "Open File")
                .accelerator("CmdOrCtrl+O")
                .build(app)?;
            let open_url_item = MenuItemBuilder::with_id("open_url", "Open from URL")
                .accelerator("CmdOrCtrl+Shift+O")
                .build(app)?;
            let save_item = MenuItemBuilder::with_id("save", "Save")
                .accelerator("CmdOrCtrl+S")
                .build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit")
                .accelerator("CmdOrCtrl+Q")
                .build(app)?;
            let minimize_item = MenuItemBuilder::with_id("minimize", "Minimize")
                .accelerator("CmdOrCtrl+M")
                .build(app)?;
            let hide_item = MenuItemBuilder::with_id("hide", "Hide")
                .accelerator("CmdOrCtrl+H")
                .build(app)?;

            let file_menu = SubmenuBuilder::new(app, "File")
                .item(&new_item)
                .item(&open_item)
                .item(&open_url_item)
                .item(&save_item)
                .separator()
                .item(&minimize_item)
                .item(&hide_item)
                .item(&quit_item)
                .build()?;

            // Edit 菜单：使用预定义菜单项恢复 macOS 默认编辑快捷键（Cmd+C/V/X/A）
            // 预定义菜单项由系统原生处理，直接在 webview 中执行编辑操作
            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let menu = MenuBuilder::new(app)
                .item(&file_menu)
                .item(&edit_menu)
                .build()?;

            app.set_menu(menu)?;

            let app_handle = app.handle().clone();
            app.on_menu_event(move |app, event| {
                use tauri::Emitter;
                match event.id().0.as_str() {
                    "new" => { let _ = app_handle.emit("menu:new", ()); }
                    "open" => { let _ = app_handle.emit("menu:open", ()); }
                    "open_url" => { let _ = app_handle.emit("menu:open_url", ()); }
                    "save" => { let _ = app_handle.emit("menu:save", ()); }
                    "minimize" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.minimize();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "quit" => { app.exit(0); }
                    _ => {}
                }
            });

            // Windows / Linux：文件关联双击打开时，系统把文件路径作为命令行参数传入。
            // 注意 Tauri v2 的 RunEvent::Opened 仅在 macOS/iOS 存在，Windows/Linux
            // 必须自行从 std::env::args() 读取。在冷启动时捕获并保存，
            // 前端通过 opened_paths 命令（启动后 ~100ms）获取，或监听 opened 事件。
            #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
            {
                use tauri::Emitter;
                use tauri_plugin_fs::FsExt;
                let mut new_paths: Vec<String> = Vec::new();
                for arg in std::env::args().skip(1) {
                    let path = std::path::Path::new(&arg);
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ext.eq_ignore_ascii_case("json") {
                                let p = path.to_string_lossy().to_string();
                                opened_paths_lock().lock().unwrap().push(p.clone());
                                let _ = app.fs_scope().allow_file(path);
                                new_paths.push(p.clone());
                            }
                        }
                    }
                }
                if !new_paths.is_empty() {
                    let _ = app.emit("opened", new_paths);
                }
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // macOS / iOS / Android：文件通过 RunEvent::Opened 交付
            #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
            if let tauri::RunEvent::Opened { urls } = event {
                use tauri::Emitter;
                use tauri_plugin_fs::FsExt;
                let mut new_paths: Vec<String> = Vec::new();
                for url in &urls {
                    if let Ok(path) = url.to_file_path() {
                        let p = path.to_string_lossy().to_string();
                        opened_paths_lock().lock().unwrap().push(p.clone());
                        let _ = app.fs_scope().allow_file(&path);
                        new_paths.push(p);
                    }
                }
                if !new_paths.is_empty() {
                    let _ = app.emit("opened", new_paths);
                }
            }
            #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
            {
                let _ = (app, event);
            }
        });
}
