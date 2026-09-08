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

// ---------------------------------------------------------------------------
// Folder browser backend
// ---------------------------------------------------------------------------
// The fs plugin's `readDir` does not expose timestamps or sizes, so we list the
// directory here and return rich metadata sorted by modification time.
#[derive(serde::Serialize)]
struct DirJsonFile {
    name: String,
    path: String,
    size: u64,
    mtime: i64, // seconds since UNIX epoch, 0 when unavailable
}

fn json_files_in_dir_impl(dir: &str) -> Vec<DirJsonFile> {
    let mut out: Vec<DirJsonFile> = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_json = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false);
        if !is_json {
            continue;
        }
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        out.push(DirJsonFile {
            name: entry.file_name().to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            size: meta.len(),
            mtime,
        });
    }
    // Newest first
    out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    out
}

#[tauri::command]
fn list_json_files(app: tauri::AppHandle, dir: String) -> Vec<DirJsonFile> {
    use tauri_plugin_fs::FsExt;
    let _ = app.fs_scope().allow_directory(std::path::Path::new(&dir), true);
    json_files_in_dir_impl(&dir)
}

#[tauri::command]
fn rename_file(from: String, to: String) -> Result<(), String> {
    if std::path::Path::new(&to).exists() {
        return Err("target already exists".into());
    }
    std::fs::rename(&from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_file(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // Single-instance: a second launch (e.g. double-clicking a .json while
        // the app is already open) must NOT open a new window. Forward its file
        // paths to the running instance, which opens them in new tabs.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            use tauri::Emitter;
            use tauri_plugin_fs::FsExt;
            let mut new_paths: Vec<String> = Vec::new();
            for arg in args.iter().skip(1) {
                let path = std::path::Path::new(arg);
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if ext.eq_ignore_ascii_case("json") {
                            let p = path.to_string_lossy().to_string();
                            let _ = app.fs_scope().allow_file(path);
                            new_paths.push(p);
                        }
                    }
                }
            }
            // Bring the existing window to the foreground.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            if !new_paths.is_empty() {
                let _ = app.emit("opened", new_paths);
            }
        }))
        .invoke_handler(tauri::generate_handler![opened_paths, allow_file, allow_directory, list_json_files, rename_file, delete_file])
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
                // Prefer emitting on the window: that is the most reliable channel,
                // and the global `app.emit` is used as a fallback.
                let emit = |name: &str| {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit(name, ());
                    } else {
                        let _ = app_handle.emit(name, ());
                    }
                };
                match event.id().0.as_str() {
                    "new" => emit("menu:new"),
                    "open" => emit("menu:open"),
                    "open_url" => emit("menu:open_url"),
                    "save" => emit("menu:save"),
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
