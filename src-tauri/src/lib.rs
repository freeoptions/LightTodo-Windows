mod models;

use models::{TodoItem, TodoStore, Settings, SettingsStore, WindowState, LongTermTodo, LongTermTodoStore, DeadlineReminder, DeadlineReminderStore};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering as AtomicOrdering};

#[cfg(target_os = "windows")]
static EDGE_DOCK_HIDDEN: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static EDGE_DOCK_OLD_WND_PROC: AtomicIsize = AtomicIsize::new(0);
#[cfg(target_os = "windows")]
static EDGE_DOCK_WND_PROC_INSTALLED: AtomicBool = AtomicBool::new(false);

// Windows-specific code for forcing window to foreground
#[cfg(target_os = "windows")]
mod windows_focus {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId,
        SetForegroundWindow, ShowWindow, SW_SHOW,
        IsIconic, IsWindowVisible,
        SetWindowPos, HWND_TOPMOST, HWND_NOTOPMOST,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        WM_SYSCOMMAND, SC_RESTORE, PostMessageA,
    };
    use windows::Win32::System::Threading::{GetCurrentThreadId, AttachThreadInput};
    use windows::Win32::Foundation::WPARAM;
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use std::thread;
    use std::time::Duration;

    /// Enable high DPI awareness for crisp icons on high-DPI displays
    pub fn enable_dpi_awareness() {
        unsafe {
            // Try to set Per-Monitor V2 DPI awareness (Windows 10 1703+)
            // This provides the best quality for high-DPI displays
            let result = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            if result.is_err() {
                // Fallback to system default DPI awareness
                eprintln!("Failed to set Per-Monitor V2 DPI awareness, using system default");
            } else {
                eprintln!("Enabled Per-Monitor V2 DPI awareness for crisp icons");
            }
        }
    }

    /// Force a window to the foreground on Windows - optimized for speed
    pub fn force_set_foreground(hwnd: HWND) -> Result<(), String> {
        unsafe {
            // Check if window is minimized and restore it
            if IsIconic(hwnd).as_bool() {
                let _ = PostMessageA(Some(hwnd), WM_SYSCOMMAND, WPARAM(SC_RESTORE as usize), LPARAM(0));
                thread::sleep(Duration::from_millis(10));
            }

            // Make sure window is visible
            if !IsWindowVisible(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_SHOW);
            }

            // Get the current foreground window
            let foreground_wnd = GetForegroundWindow();
            let foreground_thread = GetWindowThreadProcessId(foreground_wnd, None);
            let current_thread = GetCurrentThreadId();

            // Try AttachThreadInput trick first (most reliable)
            if AttachThreadInput(current_thread, foreground_thread, true).as_bool() {
                let _ = BringWindowToTop(hwnd);
                let result = SetForegroundWindow(hwnd).as_bool();
                let _ = AttachThreadInput(current_thread, foreground_thread, false);

                if result {
                    return Ok(());
                }
            }

            // Fallback: Use TOPMOST trick (fast alternative)
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0, 0, 0, 0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            );

            let _ = BringWindowToTop(hwnd);
            let result = SetForegroundWindow(hwnd).as_bool();

            // Remove TOPMOST
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_NOTOPMOST),
                0, 0, 0, 0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            );

            if result {
                return Ok(());
            }

            Err("Failed to bring window to foreground".to_string())
        }
    }
}

#[cfg(target_os = "windows")]
use windows_focus::force_set_foreground;

#[cfg(target_os = "windows")]
fn get_auto_launch_registry_value() -> Result<Option<String>, String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| format!("Failed to open Run registry key: {}", e))?;

    match run_key.get_value::<String, _>("LightTodo") {
        Ok(value) => Ok(Some(value)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("Failed to read auto launch registry value: {}", err)),
    }
}

#[cfg(target_os = "windows")]
fn set_auto_launch_enabled(enabled: bool) -> Result<(), String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _) = hkcu
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| format!("Failed to open Run registry key: {}", e))?;

    if enabled {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to resolve current executable: {}", e))?;
        let command = format!("\"{}\"", exe_path.to_string_lossy());
        run_key
            .set_value("LightTodo", &command)
            .map_err(|e| format!("Failed to enable auto launch: {}", e))?;
    } else {
        match run_key.delete_value("LightTodo") {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("Failed to disable auto launch: {}", err)),
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_auto_launch_enabled(_enabled: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn sync_auto_launch_with_settings(settings: &Settings, store: &SettingsStore) -> Result<(), String> {
    let current_registry_enabled = get_auto_launch_registry_value()?.is_some();
    if current_registry_enabled != settings.auto_launch {
        set_auto_launch_enabled(settings.auto_launch)?;
    }

    let mut latest = settings.clone();
    if latest.auto_launch != current_registry_enabled && latest.auto_launch != settings.auto_launch {
        latest.auto_launch = settings.auto_launch;
        store.save(&latest)?;
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn sync_auto_launch_with_settings(_settings: &Settings, _store: &SettingsStore) -> Result<(), String> {
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(target_os = "windows")]
        {
            match window.hwnd() {
                Ok(hwnd) => {
                    if force_set_foreground(hwnd).is_err() {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_always_on_top(false);
                        let _ = window.set_focus();
                    }
                }
                Err(_) => {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_always_on_top(false);
                    let _ = window.set_focus();
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_always_on_top(false);
            let _ = window.set_focus();
        }
    }
}

fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        match window.is_visible() {
            Ok(true) => hide_main_window(app),
            Ok(false) => show_main_window(app),
            Err(_) => show_main_window(app),
        }
    }
}

/// Get the data directory for storing todos and settings
/// In dev mode, uses project root/data to avoid triggering Tauri's file watcher
/// In release mode, uses the exe directory (portable mode)
fn get_data_dir(_handle: &AppHandle) -> PathBuf {
    if cfg!(debug_assertions) {
        // In dev mode, use data folder in project root
        let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Navigate up from src-tauri or target directory to project root
        if path.ends_with("src-tauri") {
            path = path.parent().unwrap_or(&path).to_path_buf();
        } else if path.ends_with("target") {
            path = path.parent().unwrap_or(&path).to_path_buf();
            if path.ends_with("src-tauri") {
                path = path.parent().unwrap_or(&path).to_path_buf();
            }
        }
        path.push("data");
        path
    } else {
        // In release mode, use the exe directory (portable mode)
        std::env::current_exe()
            .and_then(|exe_path| exe_path.parent().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No parent dir")).map(|p| p.to_path_buf()))
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

/// Get all todos (with daily reset logic applied)
#[tauri::command]
async fn get_todos(handle: AppHandle, target_date: Option<String>) -> Result<Vec<TodoItem>, String> {
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);
    store.get_todos(target_date.as_deref())
}

/// Add a new todo
#[tauri::command]
async fn add_todo(handle: AppHandle, content: String, repeat_mode: String, weekdays: Option<String>, active_weekdays: Option<String>, specific_dates: Option<String>, parent_id: Option<String>, completed: Option<bool>, disabled: Option<bool>, expiry_date: Option<String>, priority: Option<u8>, cycle_start_date: Option<String>, cycle_active_days: Option<u32>, cycle_interval_weeks: Option<u32>, cycle_completion_mode: Option<String>) -> Result<Vec<TodoItem>, String> {
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;
    let mut new_todo = TodoItem::new(content.clone(), repeat_mode, weekdays, parent_id.clone());
    // Set specific_dates if provided
    if let Some(dates) = specific_dates {
        new_todo.specific_dates = Some(dates);
    }
    if let Some(days) = active_weekdays {
        new_todo.active_weekdays = Some(days);
    }
    // Set completed status if provided
    if let Some(completed_val) = completed {
        new_todo.completed = completed_val;
        if completed_val {
            new_todo.completed_at = Some(chrono::Local::now().timestamp());
        }
    }
    // Set disabled status if provided
    if let Some(disabled_val) = disabled {
        new_todo.disabled = disabled_val;
    }
    // Set expiry_date if provided
    if let Some(expiry) = expiry_date {
        new_todo.expiry_date = Some(expiry);
    }
    if parent_id.is_some() {
        new_todo.cycle_start_date = cycle_start_date;
        new_todo.cycle_active_days = cycle_active_days;
        new_todo.cycle_interval_weeks = cycle_interval_weeks;
        new_todo.cycle_completion_mode = match cycle_completion_mode.as_deref() {
            Some("once") => "once".to_string(),
            _ => "daily".to_string(),
        };
    }
    // Set priority if provided
    if let Some(p) = priority {
        new_todo.priority = Some(p);
    }
    // Set week_start for weekly_this_week mode
    if new_todo.repeat_mode == "weekly_this_week" {
        let week_start = TodoStore::current_week_monday();
        eprintln!("=== [BACKEND] Set week_start={} for weekly_this_week todo", week_start);
        new_todo.week_start = Some(week_start);
    }
    // Set month_start for monthly_this_month mode
    if new_todo.repeat_mode == "monthly_this_month" {
        let month_start = TodoStore::current_month_first();
        eprintln!("=== [BACKEND] Set month_start={} for monthly_this_month todo", month_start);
        new_todo.month_start = Some(month_start);
    }
    eprintln!("=== [BACKEND] Adding todo: id={}, content='{}', parent_id={:?}, specific_dates={:?}, completed={:?}", new_todo.id, new_todo.content, new_todo.parent_id, new_todo.specific_dates, new_todo.completed);
    eprintln!("=== [BACKEND] Current todos count before add: {}", todos.len());
    todos.push(new_todo.clone());
    eprintln!("=== [BACKEND] Todos count after push: {}", todos.len());

    store.save(&todos)?;
    eprintln!("=== [BACKEND] Saved todos to file, total todos: {}", todos.len());

    // Return the raw todo list WITHOUT date filtering, so newly created weekly todos are included
    // This is important because the frontend needs to find the parent ID to add subtodos
    eprintln!("=== [BACKEND] Returning {} todos (unfiltered, including newly created)", todos.len());
    Ok(todos)
}

/// Toggle todo completion status
#[tauri::command]
async fn toggle_todo(handle: AppHandle, id: String, target_date: Option<String>) -> Result<Vec<TodoItem>, String> {
    use chrono::Local;

    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;
    let today = TodoStore::today();
    let date_to_check = target_date.as_deref().unwrap_or(&today);

    eprintln!("=== [TOGGLE_TODO] Starting toggle for todo id={}", id);

    // First, check if this is a subtodo and get its parent_id
    let parent_id: Option<String> = todos.iter()
        .find(|t| t.id == id)
        .and_then(|t| t.parent_id.clone());

    eprintln!("=== [TOGGLE_TODO] parent_id={:?}", parent_id);

    // Then, toggle the todo
    let is_completed = if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        let old_completed = todo.completed;
        todo.completed = !todo.completed;
        eprintln!("=== [TOGGLE_TODO] Toggled todo '{}': {} -> {}", todo.content, old_completed, todo.completed);
        if todo.completed {
            todo.completed_at = Some(Local::now().timestamp());
            TodoStore::mark_completed_for_date(todo, date_to_check);
        } else {
            // 周期子待办保留当前窗口标记，避免刷新时被误判为进入新周期。
            todo.completed_at = None;
            if todo.cycle_start_date.is_none() {
                todo.last_reset_date = None;
            }
        }
        !old_completed // true if marking as completed, false if unmarking
    } else {
        false
    };

    eprintln!("=== [TOGGLE_TODO] is_completed={}", is_completed);

    // If this is a parent todo (has subtodos), cascade the completion state to all subtodos
    // But only when marking as completed, not when uncompleting
    let is_parent = todos.iter().any(|t| t.parent_id.as_deref() == Some(id.as_str()));
    if is_parent && is_completed {
        // Only cascade when marking parent as completed
        eprintln!("Parent todo marked as completed, cascading to {} subtodos",
            todos.iter().filter(|t| t.parent_id.as_deref() == Some(id.as_str())).count());
        for todo in todos.iter_mut() {
            if todo.parent_id.as_deref() == Some(id.as_str()) {
                todo.completed = true;
                todo.completed_at = Some(Local::now().timestamp());
                TodoStore::mark_completed_for_date(todo, date_to_check);
            }
        }
    } else if is_parent && !is_completed {
        // Parent is being uncompleted, also uncomplete all subtodos
        // (except disabled ones, which should stay completed)
        eprintln!("Parent todo is being uncompleted, uncompleting all subtodos");
        for todo in todos.iter_mut() {
            if todo.parent_id.as_deref() == Some(id.as_str()) && !todo.disabled {
                todo.completed = false;
                todo.completed_at = None;
                eprintln!("  Uncompleted subtodo '{}' (disabled={})", todo.content, todo.disabled);
            }
        }
    }

    // If this was a subtodo, check if all siblings are completed
    if let Some(pid) = parent_id {
        // Collect sibling info for debugging
        let siblings: Vec<_> = todos.iter()
            .filter(|t| t.parent_id.as_deref() == Some(&pid))
            .map(|t| (t.id.clone(), t.content.clone(), t.completed, t.disabled))
            .collect();

        eprintln!("=== [TOGGLE SUBTODO] Parent ID: {}", pid);
        for (sid, scontent, scompleted, sdisabled) in &siblings {
            eprintln!("  Subtodo: {} ({}) - completed={}, disabled={}", scontent, sid, scompleted, sdisabled);
        }

        // Check if all subtodos are completed OR disabled
        let all_siblings_done = todos.iter()
            .filter(|t| t.parent_id.as_deref() == Some(&pid))
            .all(|t| TodoStore::is_effectively_done_for_date(t, date_to_check));

        eprintln!("  all_siblings_done = {}", all_siblings_done);

        if let Some(parent_todo) = todos.iter_mut().find(|t| t.id == pid) {
            eprintln!("  Parent before: '{}' completed={}", parent_todo.content, parent_todo.completed);

            // IMPORTANT: Always update parent's last_reset_date when a subtodo is completed
            // This ensures that partially completed subtodos don't get reset when switching views
            if is_completed && (parent_todo.repeat_mode == "daily" || parent_todo.repeat_mode == "weekly") {
                parent_todo.last_reset_date = Some(date_to_check.to_string());
                eprintln!("  -> Updated parent's last_reset_date to today because a subtodo was completed");
            }

            if all_siblings_done {
                // All subtodos completed or disabled, mark parent as completed
                parent_todo.completed = true;
                parent_todo.completed_at = Some(Local::now().timestamp());
                TodoStore::mark_completed_for_date(parent_todo, date_to_check);
                eprintln!("  -> All subtodos completed/disabled, marking parent as completed");
            } else {
                // Not all subtodos are done, mark parent as uncompleted
                parent_todo.completed = false;
                parent_todo.completed_at = None;
                eprintln!("  -> Not all subtodos completed/disabled, marking parent as UNCOMPLETED");
            }
            eprintln!("  Parent after: '{}' completed={}, last_reset_date={:?}",
                parent_todo.content, parent_todo.completed, parent_todo.last_reset_date);
        }
    }

    // Log all todos before saving
    eprintln!("=== [TOGGLE_TODO] Before saving, all todos:");
    for todo in &todos {
        let subtodo_info = if todo.parent_id.is_some() {
            format!("(subtodo, parent_id={:?})", todo.parent_id)
        } else {
            "(parent)".to_string()
        };
        eprintln!("  '{}' (id={}) completed={} {}",
            todo.content, todo.id, todo.completed, subtodo_info);
    }

    store.save(&todos)?;

    // Always return the fully processed current-view todo list so runtime fields like
    // inactive_by_weekday stay in sync after toggling.
    store.get_todos(Some(date_to_check))
}

/// Delete a todo (and all its subtodos if it's a parent)
#[tauri::command]
async fn delete_todo(handle: AppHandle, id: String) -> Result<Vec<TodoItem>, String> {
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;

    // First, find the todo to get its info before deletion
    let todo_to_delete = todos.iter().find(|t| t.id == id).cloned();

    // Check if this is a parent (has children with this parent_id)
    let child_count = todos.iter().filter(|t| t.parent_id.as_deref() == Some(id.as_str())).count();
    eprintln!("Deleting todo with id={}, has {} direct children", id, child_count);

    // Remove the todo itself
    todos.retain(|t| t.id != id);

    // If the deleted todo was a parent, also delete all its subtodos recursively
    if let Some(todo) = todo_to_delete {
        delete_subtree_recursive(&mut todos, &id);
        eprintln!("Deleted todo '{}' and all its subtodos", todo.content);
    }

    // Clean up orphan subtodos (subtodos whose parent doesn't exist)
    let all_parent_ids: std::collections::HashSet<String> = todos.iter()
        .filter(|t| t.parent_id.is_none())
        .map(|t| t.id.clone())
        .collect();
    let before_count = todos.len();
    todos.retain(|t| {
        if let Some(pid) = &t.parent_id {
            all_parent_ids.contains(pid)
        } else {
            true
        }
    });
    let after_count = todos.len();
    if before_count != after_count {
        eprintln!("Cleaned up {} orphan subtodos", before_count - after_count);
    }

    store.save(&todos)?;
    store.get_todos(None)
}

/// Recursively delete all subtodos of a parent todo
fn delete_subtree_recursive(todos: &mut Vec<TodoItem>, parent_id: &str) {
    // Find all direct children of this parent
    let child_ids: Vec<String> = todos.iter()
        .filter(|t| t.parent_id.as_deref() == Some(parent_id))
        .map(|t| t.id.clone())
        .collect();

    // Remove all direct children
    todos.retain(|t| t.parent_id.as_deref() != Some(parent_id) || t.id != parent_id);

    // Recursively delete children of children
    for child_id in child_ids {
        delete_subtree_recursive(todos, &child_id);
    }
}

/// Edit a todo
#[tauri::command]
async fn edit_todo(handle: AppHandle, id: String, content: String, repeat_mode: String, weekdays: Option<String>, active_weekdays: Option<String>, specific_dates: Option<String>, expiry_date: Option<String>, priority: Option<u8>, cycle_start_date: Option<String>, cycle_active_days: Option<u32>, cycle_interval_weeks: Option<u32>, cycle_completion_mode: Option<String>) -> Result<Vec<TodoItem>, String> {
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;

    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        let old_repeat_mode = todo.repeat_mode.clone();
        let old_completed = todo.completed;
        let _old_last_reset = todo.last_reset_date.clone();
        let is_subtodo = todo.parent_id.is_some();

        todo.content = content;
        todo.repeat_mode = repeat_mode.clone();
        todo.weekdays = if repeat_mode == "weekly" { weekdays } else { None };
        if is_subtodo {
            // The current editor does not expose weekday activation yet; preserve it
            // when the payload omits the field instead of silently clearing it.
            if active_weekdays.is_some() {
                todo.active_weekdays = active_weekdays;
            }
        } else {
            todo.active_weekdays = None;
        }
        todo.specific_dates = if repeat_mode == "specific_dates" { specific_dates } else { None };
        // Allow expiry_date for daily/weekly repeat modes OR for subtodos
        todo.expiry_date = if repeat_mode == "daily" || repeat_mode == "weekly" || is_subtodo { expiry_date } else { None };
        todo.cycle_start_date = if is_subtodo { cycle_start_date } else { None };
        todo.cycle_active_days = if is_subtodo { cycle_active_days } else { None };
        todo.cycle_interval_weeks = if is_subtodo { cycle_interval_weeks } else { None };
        todo.cycle_completion_mode = if is_subtodo {
            match cycle_completion_mode.as_deref() {
                Some("once") => "once".to_string(),
                _ => "daily".to_string(),
            }
        } else {
            "daily".to_string()
        };
        // Set priority (only for parent todos)
        todo.priority = if is_subtodo { None } else { priority };

        eprintln!("Editing todo: repeat_mode {} -> {}, completed={}",
            old_repeat_mode, repeat_mode, old_completed);

        // Handle repeat mode changes
        if old_repeat_mode != "daily" && repeat_mode == "daily" {
            // Switching TO daily mode (from none or once)
            eprintln!("Switching TO daily mode, completed={}", old_completed);
            if old_completed {
                // If already completed, set last_reset_date to today so it resets tomorrow
                todo.last_reset_date = Some(TodoStore::today());
                eprintln!("  - Set last_reset_date to today (will reset tomorrow)");
            } else {
                // If not completed, clear last_reset_date (will be set when first completed)
                todo.last_reset_date = None;
                eprintln!("  - Cleared last_reset_date (will be set on first completion)");
            }
        } else if old_repeat_mode == "daily" && repeat_mode != "daily" {
            // Switching FROM daily mode (to none or once)
            eprintln!("Switching FROM daily mode, completed={}", old_completed);
            // Clear last_reset_date since daily tracking is no longer needed
            todo.last_reset_date = None;
            eprintln!("  - Cleared last_reset_date (no longer a daily todo)");
        } else if repeat_mode == "weekly" {
            // Weekly mode
            if old_completed {
                todo.last_reset_date = Some(TodoStore::today());
            } else {
                todo.last_reset_date = None;
            }
        } else {
            eprintln!("No repeat mode change or same mode, keeping last_reset_date as is");
        }

        eprintln!("  Final state: repeat_mode={}, completed={}, last_reset_date={:?}, weekdays={:?}, specific_dates={:?}",
            todo.repeat_mode, todo.completed, todo.last_reset_date, todo.weekdays, todo.specific_dates);
    }

    store.save(&todos)?;
    store.get_todos(None)
}

/// Toggle expanded state of a parent todo
#[tauri::command]
async fn toggle_expand(handle: AppHandle, id: String) -> Result<Vec<TodoItem>, String> {
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;

    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        todo.expanded = !todo.expanded;
        eprintln!("Toggled expand state for todo '{}': expanded={}", todo.content, todo.expanded);
    }

    store.save(&todos)?;
    store.get_todos(None)
}

/// Reorder todos by moving sourceId to the position of targetId
#[tauri::command]
async fn reorder_todos(handle: AppHandle, source_id: String, target_id: String) -> Result<Vec<TodoItem>, String> {
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;
    eprintln!("=== [REORDER] Reordering: {} -> {}", source_id, target_id);

    // Find the indices
    let source_idx = todos.iter().position(|t| t.id == source_id);
    let target_idx = todos.iter().position(|t| t.id == target_id);

    match (source_idx, target_idx) {
        (Some(src), Some(tgt)) => {
            // Remove source item
            let source_item = todos.remove(src);
            let content = source_item.content.clone();
            eprintln!("=== [REORDER] Removed '{}' at index {}", content, src);

            // Insert at target position (adjust index if we removed from before target)
            let new_tgt = if src < tgt { tgt - 1 } else { tgt };
            todos.insert(new_tgt, source_item);
            eprintln!("=== [REORDER] Inserted '{}' at index {}", content, new_tgt);

            // Save the reordered list
            store.save(&todos)?;
            eprintln!("=== [REORDER] Saved reordered todos");

            // Return filtered todos for current view
            store.get_todos(None)
        }
        _ => {
            let msg = format!("Source or target not found: source={:?}, target={:?}", source_idx, target_idx);
            eprintln!("=== [REORDER] ERROR: {}", msg);
            Err(msg)
        }
    }
}

/// Move todo up by swapping with previous sibling (same parent_id)
#[tauri::command]
async fn move_todo_up(handle: AppHandle, id: String) -> Result<Vec<TodoItem>, String> {
    eprintln!("=== [MOVE UP] Starting for todo id={}", id);
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);
    let mut todos = store.load()?;
    eprintln!("=== [MOVE UP] Loaded {} todos", todos.len());

    // Find the todo and get its parent_id
    let (idx, parent_id) = match todos.iter().enumerate().find(|(_, t)| t.id == id) {
        Some((i, t)) => {
            eprintln!("=== [MOVE UP] Found todo '{}' at index {}, parent_id={:?}", t.content, i, t.parent_id);
            (i, t.parent_id.clone())
        },
        None => {
            let err = format!("Todo not found: {}", id);
            eprintln!("=== [MOVE UP] ERROR: {}", err);
            return Err(err);
        }
    };

    let current_todo = todos[idx].clone();

    // Find siblings in the same visible move group. This avoids swapping with completed,
    // disabled, or different-priority items that are currently rendered in another group.
    let sibling_indices: Vec<usize> = todos.iter()
        .enumerate()
        .filter(|(_, t)| {
            if t.parent_id != parent_id {
                return false;
            }
            if parent_id.is_none() {
                t.priority == current_todo.priority
                    && t.completed == current_todo.completed
                    && t.disabled == current_todo.disabled
            } else {
                t.completed == current_todo.completed && t.disabled == current_todo.disabled
            }
        })
        .map(|(i, _)| i)
        .collect();
    eprintln!("=== [MOVE UP] Found {} siblings with parent_id={:?}", sibling_indices.len(), parent_id);

    // Find position among siblings
    let pos = sibling_indices.iter().position(|&i| i == idx);
    let pos = match pos {
        Some(p) => p,
        None => {
            let err = format!("Todo not in siblings list");
            eprintln!("=== [MOVE UP] ERROR: {}", err);
            return Err(err);
        }
    };
    eprintln!("=== [MOVE UP] Todo is at position {} among siblings", pos);

    // If not first among siblings, swap with previous in file
    if pos > 0 {
        let prev_idx = sibling_indices[pos - 1];
        eprintln!("=== [MOVE UP] Swapping positions in file: {} <-> {}", idx, prev_idx);
        todos.swap(idx, prev_idx);
        eprintln!("=== [MOVE UP] Swap complete, saving to file...");
        store.save(&todos)?;
        eprintln!("=== [MOVE UP] File saved successfully");
    } else {
        eprintln!("=== [MOVE UP] Todo is already first, nothing to do");
    }

    // Return all todos without filtering to preserve items that may not show today
    // The frontend will handle displaying based on current view date
    eprintln!("=== [MOVE UP] Returning all todos without date filtering");
    Ok(todos)
}

/// Move todo down by swapping with next sibling (same parent_id)
#[tauri::command]
async fn move_todo_down(handle: AppHandle, id: String) -> Result<Vec<TodoItem>, String> {
    eprintln!("=== [MOVE DOWN] Starting for todo id={}", id);
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);
    let mut todos = store.load()?;
    eprintln!("=== [MOVE DOWN] Loaded {} todos", todos.len());

    // Find the todo and get its parent_id
    let (idx, parent_id) = match todos.iter().enumerate().find(|(_, t)| t.id == id) {
        Some((i, t)) => {
            eprintln!("=== [MOVE DOWN] Found todo '{}' at index {}, parent_id={:?}", t.content, i, t.parent_id);
            (i, t.parent_id.clone())
        },
        None => {
            let err = format!("Todo not found: {}", id);
            eprintln!("=== [MOVE DOWN] ERROR: {}", err);
            return Err(err);
        }
    };

    let current_todo = todos[idx].clone();

    // Find siblings in the same visible move group. This avoids swapping with completed,
    // disabled, or different-priority items that are currently rendered in another group.
    let sibling_indices: Vec<usize> = todos.iter()
        .enumerate()
        .filter(|(_, t)| {
            if t.parent_id != parent_id {
                return false;
            }
            if parent_id.is_none() {
                t.priority == current_todo.priority
                    && t.completed == current_todo.completed
                    && t.disabled == current_todo.disabled
            } else {
                t.completed == current_todo.completed && t.disabled == current_todo.disabled
            }
        })
        .map(|(i, _)| i)
        .collect();
    eprintln!("=== [MOVE DOWN] Found {} siblings with parent_id={:?}", sibling_indices.len(), parent_id);

    // Find position among siblings
    let pos = sibling_indices.iter().position(|&i| i == idx);
    let pos = match pos {
        Some(p) => p,
        None => {
            let err = format!("Todo not in siblings list");
            eprintln!("=== [MOVE DOWN] ERROR: {}", err);
            return Err(err);
        }
    };
    eprintln!("=== [MOVE DOWN] Todo is at position {} among siblings (total: {})", pos, sibling_indices.len());

    // If not last among siblings, swap with next in file
    if pos < sibling_indices.len() - 1 {
        let next_idx = sibling_indices[pos + 1];
        eprintln!("=== [MOVE DOWN] Swapping positions in file: {} <-> {}", idx, next_idx);
        todos.swap(idx, next_idx);
        eprintln!("=== [MOVE DOWN] Swap complete, saving to file...");
        store.save(&todos)?;
        eprintln!("=== [MOVE DOWN] File saved successfully");
    } else {
        eprintln!("=== [MOVE DOWN] Todo is already last, nothing to do");
    }

    // Return all todos without filtering to preserve items that may not show today
    // The frontend will handle displaying based on current view date
    eprintln!("=== [MOVE DOWN] Returning all todos without date filtering");
    Ok(todos)
}

/// Complete all subtodos of a parent todo (with user confirmation)
#[tauri::command]
async fn complete_all_subtodos(handle: AppHandle, id: String, target_date: Option<String>) -> Result<Vec<TodoItem>, String> {
    use chrono::Local;

    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;
    let today = TodoStore::today();
    let date_to_check = target_date.as_deref().unwrap_or(&today);

    // First, mark all subtodos as completed
    let parent_id = id.clone();
    for todo in todos.iter_mut() {
        if todo.parent_id.as_ref() == Some(&parent_id) {
            todo.completed = true;
            todo.completed_at = Some(Local::now().timestamp());
            TodoStore::mark_completed_for_date(todo, date_to_check);
            eprintln!("Marked subtodo '{}' as completed", todo.content);
        }
    }

    // Then, mark the parent as completed
    if let Some(parent_todo) = todos.iter_mut().find(|t| t.id == id) {
        parent_todo.completed = true;
        parent_todo.completed_at = Some(Local::now().timestamp());
        TodoStore::mark_completed_for_date(parent_todo, date_to_check);
        eprintln!("Marked parent '{}' as completed", parent_todo.content);
    }

    store.save(&todos)?;
    store.get_todos(Some(date_to_check))
}

/// Toggle disable status of a todo
/// When disabled, the todo is marked as completed and cannot be operated on
/// When enabled (undisabled), the todo can be operated on normally
#[tauri::command]
async fn toggle_disable(handle: AppHandle, id: String, target_date: Option<String>) -> Result<Vec<TodoItem>, String> {
    use chrono::Local;

    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;
    let today = TodoStore::today();
    let date_to_check = target_date.as_deref().unwrap_or(&today);

    // Find the todo and toggle its disabled status
    let is_disabling = if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        if !todo.disabled && TodoStore::is_cycle_inactive_for_date(todo, date_to_check) {
            // An inactive cycle window is a runtime projection, not a manual disable.
            // Do not turn a temporary inactive state into a persisted disabled flag.
            return store.get_todos(Some(date_to_check));
        }
        let auto_disabled_by_expiry = todo.parent_id.is_some()
            && !todo.disabled
            && todo.expiry_date.as_ref().is_some_and(|expiry| date_to_check > expiry.as_str());
        let was_effectively_disabled = todo.disabled || auto_disabled_by_expiry;
        let is_disabling = !was_effectively_disabled;
        let todo_type = if todo.parent_id.is_some() { "subtodo" } else { "parent todo" };

        if is_disabling {
            // When disabling, mark as completed
            todo.disabled = true;
            todo.completed = true;
            todo.completed_at = Some(Local::now().timestamp());
            TodoStore::mark_completed_for_date(todo, date_to_check);
            eprintln!("Disabled {} '{}', marked as completed", todo_type, todo.content);
        } else {
            // When enabling, mark as uncompleted
            todo.disabled = false;
            todo.completed = false;
            todo.completed_at = None;
            todo.last_reset_date = None;
            // Clear expiry date when manually enabling an expired subtodo
            // This prevents it from being auto-disabled again on next startup
            if todo.parent_id.is_some() && todo.expiry_date.is_some() {
                todo.expiry_date = None;
                eprintln!("Cleared expired expiry_date for {}", todo_type);
            }
            eprintln!(
                "Enabled {} '{}', marked as uncompleted (auto_disabled_by_expiry={})",
                todo_type,
                todo.content,
                auto_disabled_by_expiry
            );
        }
        is_disabling
    } else {
        return Err("Todo not found".to_string());
    };

    // If this was a parent todo, also disable/enable all its subtodos
    if let Some(parent_id) = todos.iter().find(|t| t.id == id && t.parent_id.is_none()).map(|t| t.id.clone()) {
        eprintln!("Parent todo was {}, propagating to all subtodos", if is_disabling { "disabled" } else { "enabled" });
        for subtodo in todos.iter_mut().filter(|t| t.parent_id.as_deref() == Some(&parent_id)) {
            if is_disabling {
                // Disable subtodo and mark as completed
                if !subtodo.disabled {
                    subtodo.disabled = true;
                    subtodo.completed = true;
                    subtodo.completed_at = Some(Local::now().timestamp());
                    if subtodo.repeat_mode == "daily" || subtodo.repeat_mode == "weekly" {
                        subtodo.last_reset_date = Some(TodoStore::today());
                    }
                    eprintln!("  -> Disabled subtodo '{}', marked as completed", subtodo.content);
                }
            } else {
                // Enable subtodo and mark as uncompleted
                let auto_disabled_by_expiry = subtodo.expiry_date.as_ref().is_some_and(|expiry| date_to_check > expiry.as_str());
                if subtodo.disabled || auto_disabled_by_expiry {
                    subtodo.disabled = false;
                    subtodo.completed = false;
                    subtodo.completed_at = None;
                    subtodo.last_reset_date = None;
                    if subtodo.expiry_date.is_some() {
                        subtodo.expiry_date = None;
                    }
                    eprintln!("  -> Enabled subtodo '{}', marked as uncompleted", subtodo.content);
                }
            }
        }
    }

    // If this was a subtodo, check if all siblings are completed or disabled
    // If so, mark the parent as completed
    if let Some(parent_id) = todos.iter()
        .find(|t| t.id == id)
        .and_then(|t| t.parent_id.clone()) {

        // Check if all subtodos are completed OR disabled
        let all_siblings_done = todos.iter()
            .filter(|t| t.parent_id.as_deref() == Some(&parent_id))
            .all(|t| TodoStore::is_effectively_done_for_date(t, date_to_check));

        if let Some(parent_todo) = todos.iter_mut().find(|t| t.id == parent_id) {
            if all_siblings_done {
                // All subtodos are completed or disabled, mark parent as completed
                parent_todo.completed = true;
                parent_todo.completed_at = Some(Local::now().timestamp());
                TodoStore::mark_completed_for_date(parent_todo, date_to_check);
                eprintln!("All subtodos completed/disabled, marking parent '{}' as completed", parent_todo.content);
            } else {
                // Not all subtodos are done, mark parent as uncompleted
                // BUT DO NOT reset last_reset_date - this prevents unwanted subtodo resets
                parent_todo.completed = false;
                parent_todo.completed_at = None;
                // last_reset_date should remain unchanged to avoid resetting subtodos
                eprintln!("Not all subtodos completed/disabled, marking parent '{}' as uncompleted (keeping last_reset_date={:?})", parent_todo.content, parent_todo.last_reset_date);
            }
        }
    }

    store.save(&todos)?;
    store.get_todos(Some(date_to_check))
}

/// Get current settings
#[tauri::command]
fn get_settings(store: State<SettingsStore>) -> Result<Settings, String> {
    store.load()
}

/// Update a shortcut
/// Note: In the current API, dynamic shortcut updates require app restart.
/// The shortcut will be updated in settings and applied on next launch.
#[tauri::command]
async fn update_shortcut(
    store: State<'_, SettingsStore>,
    action: String,
    shortcut: String,
    _app: AppHandle,
) -> Result<Settings, String> {
    let mut settings = store.load()?;
    let shortcut = shortcut.trim().to_string();
    let old_shortcut = match action.as_str() {
        "showHideWindow" => settings.shortcuts.show_hide_window.clone(),
        _ => return Err("Invalid action".to_string()),
    };

    match action.as_str() {
        "showHideWindow" => {
            settings.shortcuts.show_hide_window = shortcut.clone();
            eprintln!("Updated show/hide shortcut to: {} (restart required)", shortcut);
        }
        _ => unreachable!(),
    }

    store.save(&settings)?;

    if shortcut.is_empty() && !old_shortcut.trim().is_empty() {
        if let Err(e) = _app.global_shortcut().unregister(old_shortcut.as_str()) {
            eprintln!("Failed to unregister shortcut {}: {}", old_shortcut, e);
        }
    }

    Ok(settings)
}

/// Update memo
#[tauri::command]
fn update_memo(store: State<'_, SettingsStore>, content: String) -> Result<Settings, String> {
    let mut settings = store.load()?;
    settings.memo = content;
    store.save(&settings)?;
    Ok(settings)
}

/// Update memo height
#[tauri::command]
fn update_memo_height(store: State<'_, SettingsStore>, height: u32) -> Result<Settings, String> {
    let mut settings = store.load()?;
    settings.memo_height = height.min(300).max(60); // Limit between 60-300px
    store.save(&settings)?;
    Ok(settings)
}

/// Update priority color
#[tauri::command]
fn update_priority_color(store: State<'_, SettingsStore>, priority: u8, color: String) -> Result<Settings, String> {
    let mut settings = store.load()?;
    if settings.priority_colors.is_none() {
        settings.priority_colors = Some(std::collections::HashMap::new());
    }
    settings.priority_colors.as_mut().unwrap().insert(priority, color);
    store.save(&settings)?;
    Ok(settings)
}

/// Update auto launch
#[tauri::command]
fn update_auto_launch(store: State<'_, SettingsStore>, enabled: bool) -> Result<Settings, String> {
    let mut settings = store.load()?;
    settings.auto_launch = enabled;
    set_auto_launch_enabled(enabled)?;
    store.save(&settings)?;
    Ok(settings)
}

/// Save window state
#[tauri::command]
fn save_window_state(
    store: State<'_, SettingsStore>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(), String> {
    let mut settings = store.load()?;
    // Only update values that are provided
    if let Some(v) = x { settings.window.x = v; }
    if let Some(v) = y { settings.window.y = v; }
    if let Some(v) = width { settings.window.width = v; }
    if let Some(v) = height { settings.window.height = v; }
    store.save(&settings)?;
    Ok(())
}

/// Persist the user-selected window geometry using the same size semantics as
/// `set_size`: the content area, not the native frame around it.
fn save_window_geometry(window: &tauri::WebviewWindow, store: &SettingsStore) {
    let position = window.outer_position();
    let size = window.inner_size();

    if let (Ok(pos), Ok(sz)) = (position, size) {
        let mut settings = store.load().unwrap_or_default();
        let scale = window.scale_factor().unwrap_or(1.0);
        let logical_pos: tauri::LogicalPosition<f64> = pos.to_logical(scale);
        let logical_size: tauri::LogicalSize<f64> = sz.to_logical(scale);

        settings.window.x = logical_pos.x.round() as i32;
        settings.window.y = logical_pos.y.round() as i32;
        settings.window.width = logical_size.width.round() as u32;
        settings.window.height = logical_size.height.round() as u32;

        if let Err(error) = store.save(&settings) {
            eprintln!("Failed to save window geometry: {}", error);
        } else {
            eprintln!(
                "Window geometry saved: x={}, y={}, w={}, h={} (scale={})",
                settings.window.x,
                settings.window.y,
                settings.window.width,
                settings.window.height,
                scale
            );
        }
    }
}

// ============ Long Term Todo Commands ============

/// Get all long term todos
#[tauri::command]
fn get_long_term_todos(store: State<'_, LongTermTodoStore>) -> Result<Vec<LongTermTodo>, String> {
    store.load()
}

/// Add a new long term todo
#[tauri::command]
fn add_long_term_todo(store: State<'_, LongTermTodoStore>, content: String) -> Result<Vec<LongTermTodo>, String> {
    store.add(content)?;
    store.load()
}

/// Toggle long term todo completion
#[tauri::command]
fn toggle_long_term_todo(store: State<'_, LongTermTodoStore>, id: String) -> Result<Vec<LongTermTodo>, String> {
    store.toggle(&id)?;
    store.load()
}

/// Delete a long term todo
#[tauri::command]
fn delete_long_term_todo(store: State<'_, LongTermTodoStore>, id: String) -> Result<Vec<LongTermTodo>, String> {
    store.delete(&id)?;
    store.load()
}

/// Update long term todo content
#[tauri::command]
fn update_long_term_todo(store: State<'_, LongTermTodoStore>, id: String, content: String) -> Result<Vec<LongTermTodo>, String> {
    store.update(&id, &content)?;
    store.load()
}

/// Move long term todo up
#[tauri::command]
fn move_long_term_todo_up(store: State<'_, LongTermTodoStore>, id: String) -> Result<Vec<LongTermTodo>, String> {
    store.move_up(&id)?;
    store.load()
}

/// Move long term todo down
#[tauri::command]
fn move_long_term_todo_down(store: State<'_, LongTermTodoStore>, id: String) -> Result<Vec<LongTermTodo>, String> {
    store.move_down(&id)?;
    store.load()
}

// ============ Deadline Reminder Commands ============

#[tauri::command]
fn get_deadline_reminders(store: State<'_, DeadlineReminderStore>) -> Result<Vec<DeadlineReminder>, String> {
    store.load()
}

#[tauri::command]
fn add_deadline_reminder(
    store: State<'_, DeadlineReminderStore>,
    title: String,
    due_date: String,
) -> Result<Vec<DeadlineReminder>, String> {
    store.add(title, due_date)?;
    store.load()
}

#[tauri::command]
fn update_deadline_reminder(
    store: State<'_, DeadlineReminderStore>,
    id: String,
    title: String,
    due_date: String,
) -> Result<Vec<DeadlineReminder>, String> {
    store.update(&id, title, due_date)?;
    store.load()
}

#[tauri::command]
fn delete_deadline_reminder(
    store: State<'_, DeadlineReminderStore>,
    id: String,
) -> Result<Vec<DeadlineReminder>, String> {
    store.delete(&id)?;
    store.load()
}

#[cfg(target_os = "windows")]
fn cursor_position() -> Option<windows::Win32::Foundation::POINT> {
    let mut point = windows::Win32::Foundation::POINT { x: 0, y: 0 };
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point).ok()?;
    }
    Some(point)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn edge_dock_wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, SC_MINIMIZE, SIZE_MINIMIZED, WM_SIZE, WM_SYSCOMMAND,
        WNDPROC,
    };

    if EDGE_DOCK_HIDDEN.load(AtomicOrdering::Relaxed) {
        let is_minimize_command =
            msg == WM_SYSCOMMAND && ((wparam.0 as u32) & 0xfff0) == SC_MINIMIZE;
        let is_minimized_size = msg == WM_SIZE && (wparam.0 as u32) == SIZE_MINIMIZED;

        if is_minimize_command || is_minimized_size {
            return LRESULT(0);
        }
    }

    let old_proc = EDGE_DOCK_OLD_WND_PROC.load(AtomicOrdering::Relaxed);
    if old_proc != 0 {
        let old_proc: WNDPROC = std::mem::transmute(old_proc);
        CallWindowProcW(old_proc, hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

#[cfg(target_os = "windows")]
fn install_edge_dock_wnd_proc(window: &tauri::WebviewWindow) {
    if EDGE_DOCK_WND_PROC_INSTALLED.swap(true, AtomicOrdering::Relaxed) {
        return;
    }

    let Ok(hwnd) = window.hwnd() else {
        EDGE_DOCK_WND_PROC_INSTALLED.store(false, AtomicOrdering::Relaxed);
        return;
    };

    unsafe {
        let old_proc = windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWLP_WNDPROC,
            edge_dock_wnd_proc as *const () as usize as isize,
        );

        if old_proc == 0 {
            EDGE_DOCK_WND_PROC_INSTALLED.store(false, AtomicOrdering::Relaxed);
        } else {
            EDGE_DOCK_OLD_WND_PROC.store(old_proc, AtomicOrdering::Relaxed);
        }
    }
}

#[cfg(target_os = "windows")]
fn set_edge_dock_window_style(window: &tauri::WebviewWindow, _hidden: bool) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED,
            SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        };

        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let next = (current | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;

        if next != current {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next as isize);
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
    }
}

#[cfg(target_os = "windows")]
fn set_edge_window_topmost(window: &tauri::WebviewWindow, topmost: bool) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };

        let insert_after = if topmost {
            Some(HWND_TOPMOST)
        } else {
            Some(HWND_NOTOPMOST)
        };

        let _ = SetWindowPos(
            hwnd,
            insert_after,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

#[cfg(target_os = "windows")]
fn hide_window_close_button(window: &tauri::WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
            SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_SYSMENU,
        };

        // Windows draws the native close button from WS_SYSMENU. Removing only
        // SC_CLOSE from the system menu does not remove the caption button.
        let current = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let next = current & !(WS_SYSMENU.0 as isize);

        if next != current {
            SetWindowLongPtrW(hwnd, GWL_STYLE, next);
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
    }
}

#[cfg(target_os = "windows")]
fn dock_window_to_right_edge(window: &tauri::WebviewWindow, settings: &Settings) {
    const HANDLE_WIDTH: i32 = 18;

    let monitor = window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        let _ = window.show();
        return;
    };

    let scale = window.scale_factor().unwrap_or(1.0);
    let width = settings.window.width.clamp(400, 1000) as f64;
    let height = settings.window.height.clamp(500, 1200) as f64;
    let _ = window.unmaximize();
    let _ = window.set_size(tauri::LogicalSize::new(width, height));

    let size = window.outer_size().ok();
    let physical_height = size
        .map(|value| value.height as i32)
        .unwrap_or_else(|| (height * scale).round() as i32);

    let monitor_top = monitor.position().y;
    let monitor_right = monitor.position().x + monitor.size().width as i32;
    let monitor_bottom = monitor_top + monitor.size().height as i32;
    let hidden_x = monitor_right - HANDLE_WIDTH;
    let requested_y = (settings.window.y as f64 * scale).round() as i32;
    let hidden_y = requested_y.clamp(monitor_top, (monitor_bottom - physical_height).max(monitor_top));

    EDGE_DOCK_HIDDEN.store(true, AtomicOrdering::Relaxed);
    set_edge_dock_window_style(window, true);
    set_edge_window_topmost(window, true);
    let _ = window.set_position(tauri::PhysicalPosition::new(hidden_x, hidden_y));
    let _ = window.show();
}

#[cfg(not(target_os = "windows"))]
fn dock_window_to_right_edge(window: &tauri::WebviewWindow, _settings: &Settings) {
    let _ = window.show();
}

#[cfg(target_os = "windows")]
fn start_edge_dock_monitor(app: AppHandle) {
    std::thread::spawn(move || {
        const HANDLE_WIDTH: i32 = 18;
        const EDGE_THRESHOLD: i32 = 24;
        const HOVER_WIDTH: i32 = 7;
        const LEAVE_PADDING: i32 = 10;
        const EXPANDED_MARGIN: i32 = 18;
        const TICK_MS: u64 = 120;

        let mut is_hidden_to_edge = false;
        let mut last_hidden_y: Option<i32> = None;
        let mut startup_dock_checked = false;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(TICK_MS));

            let Some(window) = app.get_webview_window("main") else {
                continue;
            };

            let is_minimized = window.is_minimized().unwrap_or(false);
            if is_minimized {
                if is_hidden_to_edge {
                    let monitor = window
                        .current_monitor()
                        .ok()
                        .flatten()
                        .or_else(|| window.primary_monitor().ok().flatten());

                    if let Some(monitor) = monitor {
                        let monitor_top = monitor.position().y;
                        let monitor_right = monitor.position().x + monitor.size().width as i32;
                        let hidden_x = monitor_right - HANDLE_WIDTH;
                        let y = last_hidden_y.unwrap_or(monitor_top + 120).max(monitor_top);

                        let _ = window.unminimize();
                        let _ = window.show();
                        set_edge_dock_window_style(&window, true);
                        set_edge_window_topmost(&window, true);
                        let _ = window.set_position(tauri::PhysicalPosition::new(hidden_x, y));
                    }
                }
                continue;
            }

            if !window.is_visible().unwrap_or(false) {
                is_hidden_to_edge = false;
                EDGE_DOCK_HIDDEN.store(false, AtomicOrdering::Relaxed);
                set_edge_dock_window_style(&window, false);
                last_hidden_y = None;
                continue;
            }

            let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) else {
                continue;
            };

            let monitor = window
                .current_monitor()
                .ok()
                .flatten()
                .or_else(|| window.primary_monitor().ok().flatten());
            let Some(monitor) = monitor else {
                continue;
            };

            let Some(cursor) = cursor_position() else {
                continue;
            };

            let monitor_left = monitor.position().x;
            let monitor_top = monitor.position().y;
            let monitor_right = monitor_left + monitor.size().width as i32;
            let monitor_bottom = monitor_top + monitor.size().height as i32;
            let x = position.x;
            let y = position.y;
            let width = size.width as i32;
            let height = size.height as i32;
            let window_right = x + width;
            let window_bottom = y + height;

            let hidden_x = monitor_right - HANDLE_WIDTH;
            let expanded_x = (monitor_right - width - EXPANDED_MARGIN).max(monitor_left);
            let clamped_y = y.clamp(monitor_top, (monitor_bottom - height).max(monitor_top));

            let cursor_in_window = cursor.x >= x
                && cursor.x <= window_right
                && cursor.y >= y
                && cursor.y <= window_bottom;
            let cursor_near_handle = cursor.x >= monitor_right - HOVER_WIDTH
                && cursor.x <= monitor_right + HOVER_WIDTH
                && cursor.y >= y - HOVER_WIDTH
                && cursor.y <= window_bottom + HOVER_WIDTH;

            if is_hidden_to_edge || x >= hidden_x - 1 {
                is_hidden_to_edge = true;
                EDGE_DOCK_HIDDEN.store(true, AtomicOrdering::Relaxed);
                last_hidden_y = Some(clamped_y);
                if cursor_near_handle {
                    let _ = window.unminimize();
                    let _ = window.show();
                    set_edge_dock_window_style(&window, false);
                    let _ = window.set_position(tauri::PhysicalPosition::new(expanded_x, clamped_y));
                    set_edge_window_topmost(&window, false);
                    is_hidden_to_edge = false;
                    EDGE_DOCK_HIDDEN.store(false, AtomicOrdering::Relaxed);
                    last_hidden_y = None;
                }
                continue;
            }

            let is_right_docked = window_right >= monitor_right - EDGE_THRESHOLD
                && x < hidden_x
                && width > HANDLE_WIDTH * 4;
            let cursor_left_window = cursor.x < x - LEAVE_PADDING
                || cursor.x > monitor_right + HOVER_WIDTH
                || cursor.y < y - LEAVE_PADDING
                || cursor.y > window_bottom + LEAVE_PADDING;

            if !startup_dock_checked {
                startup_dock_checked = true;

                if is_right_docked {
                    EDGE_DOCK_HIDDEN.store(true, AtomicOrdering::Relaxed);
                    set_edge_dock_window_style(&window, true);
                    set_edge_window_topmost(&window, true);
                    let _ = window.set_position(tauri::PhysicalPosition::new(hidden_x, clamped_y));
                    is_hidden_to_edge = true;
                    last_hidden_y = Some(clamped_y);
                    continue;
                }
            }

            if is_right_docked && !cursor_in_window && cursor_left_window {
                EDGE_DOCK_HIDDEN.store(true, AtomicOrdering::Relaxed);
                set_edge_dock_window_style(&window, true);
                set_edge_window_topmost(&window, true);
                let _ = window.set_position(tauri::PhysicalPosition::new(hidden_x, clamped_y));
                is_hidden_to_edge = true;
                last_hidden_y = Some(clamped_y);
            }
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn start_edge_dock_monitor(_app: AppHandle) {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Enable high DPI awareness for crisp icons on high-DPI displays
    #[cfg(target_os = "windows")]
    windows_focus::enable_dpi_awareness();

    let _single_instance = if cfg!(debug_assertions) {
        None
    } else {
        // Single instance check - try to acquire lock
        let single_instance = single_instance::SingleInstance::new("LightTodo-app-instance").unwrap();
        if !single_instance.is_single() {
            // Another instance is already running - show dialog and exit
            std::thread::spawn(|| {
                use std::process::Command;
                let _ = Command::new("mshta")
                    .args(&["vbscript:msgbox(\"LightTodo is already running. Please check the system tray or taskbar.\",16,\"LightTodo\")(window.close)"])
                    .output();
            });
            eprintln!("Another instance is already running. Exiting...");
            return;
        }
        Some(single_instance)
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Window geometry is persisted in the portable settings.json by our own
        // save/restore flow. Do not let the plugin restore a second, stale size
        // over the user's latest manually resized dimensions.
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .skip_initial_state("main")
                .build(),
        )
        .setup(|app| {
            eprintln!("=== LightTodo Starting ===");

            // Use exe directory for storing settings and todos (portable mode)
            // In dev mode, use project root/data to avoid triggering rebuilds
            let data_dir = if cfg!(debug_assertions) {
                // In dev mode, use data folder in project root (to avoid triggering Tauri's file watcher)
                let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                // Navigate up from src-tauri or target directory to project root
                if path.ends_with("src-tauri") {
                    path = path.parent().unwrap_or(&path).to_path_buf();
                } else if path.ends_with("target") {
                    path = path.parent().unwrap_or(&path).to_path_buf();
                    if path.ends_with("src-tauri") {
                        path = path.parent().unwrap_or(&path).to_path_buf();
                    }
                }
                path.push("data");
                path
            } else {
                // In release mode, use the exe directory (portable mode)
                std::env::current_exe()
                    .and_then(|exe_path| exe_path.parent().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No parent dir")).map(|p| p.to_path_buf()))
                    .unwrap_or_else(|_| PathBuf::from("."))
            };

            eprintln!("Data directory: {:?}", data_dir);

            // Ensure directory exists
            let _ = std::fs::create_dir_all(&data_dir);
            let settings_store = SettingsStore::new(data_dir.clone());
            let mut settings = settings_store.load()?;

            // Validate and correct window position to ensure it's visible on screen
            // This prevents the window from appearing off-screen after monitor changes
            const MAX_SCREEN_COORD: i32 = 30000; // Windows virtual screen max
            const MIN_SCREEN_COORD: i32 = -10000;

            if settings.window.x < MIN_SCREEN_COORD || settings.window.x > MAX_SCREEN_COORD ||
               settings.window.y < MIN_SCREEN_COORD || settings.window.y > MAX_SCREEN_COORD ||
               settings.window.width < 200 || settings.window.width > 10000 ||
               settings.window.height < 200 || settings.window.height > 10000 {
                eprintln!("Window position/size appears invalid, resetting to defaults");
                eprintln!("  x={}, y={}, width={}, height={}",
                    settings.window.x, settings.window.y, settings.window.width, settings.window.height);
                let default_window = WindowState::default();
                settings.window.x = default_window.x;
                settings.window.y = default_window.y;
                settings.window.width = default_window.width;
                settings.window.height = default_window.height;
                let _ = settings_store.save(&settings);
            }

            sync_auto_launch_with_settings(&settings, &settings_store)?;

            app.manage(settings_store.clone());

            // Create and manage long term todo store
            let long_term_todo_store = LongTermTodoStore::new(data_dir.clone());
            app.manage(long_term_todo_store);

            // Create and manage one-off deadline reminder store
            let deadline_reminder_store = DeadlineReminderStore::new(data_dir.clone());
            app.manage(deadline_reminder_store);

            eprintln!("Settings loaded: shortcuts={:?}", settings.shortcuts);

            // Initialize global shortcuts plugin with Builder pattern
            let show_hide_shortcut_str = settings.shortcuts.show_hide_window.clone();
            let mut shortcut_registrations = Vec::new();
            if !show_hide_shortcut_str.trim().is_empty() {
                shortcut_registrations.push(show_hide_shortcut_str.as_str());
            }
            let app_handle = app.handle().clone();

            // Use a shared state to track window visibility more reliably
            use std::sync::atomic::{AtomicBool, Ordering};
            let window_visible = std::sync::Arc::new(AtomicBool::new(true));
            let window_visible_for_handler = window_visible.clone();

            let plugin = tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts(shortcut_registrations)?
                .with_handler(move |_app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let show_hide_matches = shortcut.matches(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT, Code::Digit0);
                        let show_hide_matches_mac = shortcut.matches(Modifiers::SUPER | Modifiers::SHIFT | Modifiers::ALT, Code::Digit0);

                        if show_hide_matches || show_hide_matches_mac {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                // Check tracked state
                                let is_visible = window_visible_for_handler.load(Ordering::Relaxed);

                                if is_visible {
                                    // Hide the window using minimize (more reliable than hide)
                                    #[cfg(target_os = "windows")]
                                    {
                                        use windows::Win32::UI::WindowsAndMessaging::ShowWindow;
                                        use windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE;
                                        match window.hwnd() {
                                            Ok(hwnd) => {
                                                unsafe {
                                                    let _ = ShowWindow(hwnd, SW_MINIMIZE);
                                                }
                                            }
                                            Err(_) => {
                                                let _ = window.minimize();
                                            }
                                        }
                                    }
                                    #[cfg(not(target_os = "windows"))]
                                    {
                                        let _ = window.minimize();
                                    }
                                    window_visible_for_handler.store(false, Ordering::Relaxed);
                                } else {
                                    // Show the window
                                    window_visible_for_handler.store(true, Ordering::Relaxed);

                                    #[cfg(target_os = "windows")]
                                    {
                                        match window.hwnd() {
                                            Ok(hwnd) => {
                                                match force_set_foreground(hwnd) {
                                                    Ok(_) => {
                                                        let _ = window.set_always_on_top(false);
                                                    }
                                                    Err(_) => {
                                                        let _ = window.unminimize();
                                                        let _ = window.show();
                                                        let _ = window.set_always_on_top(false);
                                                        let _ = window.set_focus();
                                                    }
                                                }
                                            }
                                            Err(_) => {
                                                let _ = window.unminimize();
                                                let _ = window.show();
                                                let _ = window.set_always_on_top(false);
                                                let _ = window.set_focus();
                                            }
                                        }
                                    }
                                    #[cfg(not(target_os = "windows"))]
                                    {
                                        let _ = window.unminimize();
                                        let _ = window.show();
                                        let _ = window.set_always_on_top(false);
                                        let _ = window.set_focus();
                                        let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
                                    }
                                }
                            }
                        }
                    }
                })
                .build();

            match app.handle().plugin(plugin) {
                Ok(_) => {
                    eprintln!("Registered global shortcut: {} (show/hide)",
                        show_hide_shortcut_str);
                }
                Err(e) => {
                    eprintln!("Failed to register global shortcut plugin: {}", e);
                }
            }

            // Try to create tray, but don't fail if it doesn't work
            if let Err(e) = setup_tray(app) {
                eprintln!("Failed to setup tray: {}", e);
                eprintln!("Application will continue without tray icon");
            }

            // Put the main window directly on the right edge before showing it.
            if let Some(window) = app.get_webview_window("main") {
                eprintln!("Docking main window to right edge...");
                install_edge_dock_wnd_proc(&window);
                dock_window_to_right_edge(&window, &settings);
                // Tauri applies the final native frame while showing and sizing
                // the window, so remove the close button only after that work
                // has completed.
                #[cfg(target_os = "windows")]
                hide_window_close_button(&window);
            }

            start_edge_dock_monitor(app.handle().clone());

            // Handle window close event - hide instead of close
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                let window_clone2 = window.clone();
                let store = settings_store.clone();
                let window_visible_for_close = window_visible.clone();

                // Save the latest user-selected geometry before hiding the window.
                window_clone.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            eprintln!("Saving window state before close...");
                            save_window_geometry(&window_clone2, &store);

                            api.prevent_close();
                            let _ = window_clone2.hide();
                            // Update visibility state
                            window_visible_for_close.store(false, Ordering::Relaxed);
                            eprintln!("[CloseEvent] Window hidden, visibility state updated to false");
                        }
                        #[cfg(target_os = "windows")]
                        tauri::WindowEvent::Focused(true) |
                        tauri::WindowEvent::Resized { .. } => {
                            // Tauri/Windows may rebuild the non-client frame
                            // when the window is shown or resized. Reapply the
                            // title-bar style at those points as well.
                            hide_window_close_button(&window_clone2);
                        }
                        _ => {}
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_todos,
            add_todo,
            toggle_todo,
            delete_todo,
            edit_todo,
            toggle_expand,
            complete_all_subtodos,
            toggle_disable,
            reorder_todos,
            move_todo_up,
            move_todo_down,
            get_settings,
            update_shortcut,
            save_window_state,
            update_memo,
            update_memo_height,
            update_priority_color,
            update_auto_launch,
            get_long_term_todos,
            add_long_term_todo,
            toggle_long_term_todo,
            delete_long_term_todo,
            move_long_term_todo_up,
            move_long_term_todo_down,
            update_long_term_todo,
            get_deadline_reminders,
            add_deadline_reminder,
            update_deadline_reminder,
            delete_deadline_reminder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "显示软件", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    // Reuse Tauri's decoded default window icon so the tray, window and taskbar
    // always display the exact same asset. The fallback only applies if the
    // application context does not provide an icon.
    let icon = app
        .default_window_icon()
        .map(|icon| Image::new_owned(icon.rgba().to_vec(), icon.width(), icon.height()))
        .unwrap_or_else(create_simple_icon);

    // Build tray icon with menu
    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("LightTodo - 轻量待办管理")
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(&tray.app_handle());
            }
        })
        .on_menu_event(move |app, event| {
            match event.id.0.as_str() {
                "show" => {
                    show_main_window(app);
                }
                "quit" => {
                    // `app.exit()` can bypass the main window's close event,
                    // so flush the latest geometry explicitly before exiting.
                    if let Some(window) = app.get_webview_window("main") {
                        save_window_geometry(&window, app.state::<SettingsStore>().inner());
                    }
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

// Create a simple colored icon as fallback
fn create_simple_icon() -> Image<'static> {
    // Create a simple 32x32 RGBA icon with green checkmark (✅)
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let idx = (y * 32 + x) * 4;
            let center_x = 16.0;
            let center_y = 16.0;
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Draw green circle background
            if dist < 14.0 {
                // Green color
                pixels[idx] = 34;     // R
                pixels[idx + 1] = 197; // G
                pixels[idx + 2] = 94;  // B
                pixels[idx + 3] = 255; // A
            } else {
                pixels[idx + 3] = 0; // Transparent
            }

            // Draw white checkmark
            // Checkmark points: (8,16) -> (14,22) -> (24,10)
            let p1 = (8.0, 16.0);
            let p2 = (14.0, 22.0);
            let p3 = (24.0, 10.0);

            // First segment: p1 to p2
            let on_segment1 = point_to_line_distance(x as f32, y as f32, p1.0, p1.1, p2.0, p2.1) < 2.0
                && x as f32 >= p1.0.min(p2.0) - 1.0
                && x as f32 <= p1.0.max(p2.0) + 1.0
                && y as f32 >= p1.1.min(p2.1) - 1.0
                && y as f32 <= p1.1.max(p2.1) + 1.0;

            // Second segment: p2 to p3
            let on_segment2 = point_to_line_distance(x as f32, y as f32, p2.0, p2.1, p3.0, p3.1) < 2.0
                && x as f32 >= p2.0.min(p3.0) - 1.0
                && x as f32 <= p2.0.max(p3.0) + 1.0
                && y as f32 >= p2.1.min(p3.1) - 1.0
                && y as f32 <= p2.1.max(p3.1) + 1.0;

            if on_segment1 || on_segment2 {
                pixels[idx] = 255;     // R
                pixels[idx + 1] = 255; // G
                pixels[idx + 2] = 255; // B
                pixels[idx + 3] = 255; // A
            }
        }
    }
    Image::new_owned(pixels, 32, 32)
}

// Helper function to calculate distance from point to line segment
fn point_to_line_distance(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let a = px - x1;
    let b = py - y1;
    let c = x2 - x1;
    let d = y2 - y1;

    let dot = a * c + b * d;
    let len_sq = c * c + d * d;
    let mut param = -1.0;
    if len_sq != 0.0 {
        param = dot / len_sq;
    }

    let (xx, yy) = if param < 0.0 {
        (x1, y1)
    } else if param > 1.0 {
        (x2, y2)
    } else {
        (x1 + param * c, y1 + param * d)
    };

    let dx = px - xx;
    let dy = py - yy;
    (dx * dx + dy * dy).sqrt()
}
