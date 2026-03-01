mod models;

use models::{TodoItem, TodoStore, Settings, SettingsStore, WindowState};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

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
    use std::thread;
    use std::time::Duration;

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
async fn add_todo(handle: AppHandle, content: String, repeat_mode: String, weekdays: Option<String>, specific_dates: Option<String>, parent_id: Option<String>) -> Result<Vec<TodoItem>, String> {
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;
    let mut new_todo = TodoItem::new(content.clone(), repeat_mode, weekdays, parent_id.clone());
    // Set specific_dates if provided
    if let Some(dates) = specific_dates {
        new_todo.specific_dates = Some(dates);
    }
    eprintln!("=== [BACKEND] Adding todo: id={}, content='{}', parent_id={:?}, specific_dates={:?}", new_todo.id, new_todo.content, new_todo.parent_id, new_todo.specific_dates);
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
async fn toggle_todo(handle: AppHandle, id: String) -> Result<Vec<TodoItem>, String> {
    use chrono::Local;

    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;

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
            if todo.repeat_mode == "daily" || todo.repeat_mode == "weekly" {
                todo.last_reset_date = Some(TodoStore::today());
            }
        } else {
            // 取消完成时，清除 last_reset_date，避免触发自动重置
            todo.completed_at = None;
            todo.last_reset_date = None;
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
                if todo.repeat_mode == "daily" || todo.repeat_mode == "weekly" {
                    todo.last_reset_date = Some(TodoStore::today());
                }
            }
        }
    } else if is_parent && !is_completed {
        // Parent is being uncompleted, check if any subtodo is still completed
        // If any subtodo is completed, parent should remain completed
        let any_subtodo_completed = todos.iter()
            .filter(|t| t.parent_id.as_deref() == Some(id.as_str()))
            .any(|t| t.completed);

        if any_subtodo_completed {
            // Some subtodos are still completed, mark parent back as completed
            if let Some(parent_todo) = todos.iter_mut().find(|t| t.id == id) {
                parent_todo.completed = true;
                parent_todo.completed_at = Some(Local::now().timestamp());
                eprintln!("Parent has completed subtodos, keeping parent '{}' as completed", parent_todo.content);
            }
        }
    }

    // If this was a subtodo, check if all siblings are completed
    if let Some(pid) = parent_id {
        // Collect sibling info for debugging
        let siblings: Vec<_> = todos.iter()
            .filter(|t| t.parent_id.as_deref() == Some(&pid))
            .map(|t| (t.id.clone(), t.content.clone(), t.completed))
            .collect();

        eprintln!("=== [TOGGLE SUBTODO] Parent ID: {}", pid);
        for (sid, scontent, scompleted) in &siblings {
            eprintln!("  Subtodo: {} ({}) - completed={}", scontent, sid, scompleted);
        }

        let all_siblings_completed = todos.iter()
            .filter(|t| t.parent_id.as_deref() == Some(&pid))
            .all(|t| t.completed);

        eprintln!("  all_siblings_completed = {}", all_siblings_completed);

        if let Some(parent_todo) = todos.iter_mut().find(|t| t.id == pid) {
            eprintln!("  Parent before: '{}' completed={}", parent_todo.content, parent_todo.completed);
            if all_siblings_completed {
                // All subtodos completed, mark parent as completed
                parent_todo.completed = true;
                parent_todo.completed_at = Some(Local::now().timestamp());
                if parent_todo.repeat_mode == "daily" || parent_todo.repeat_mode == "weekly" {
                    parent_todo.last_reset_date = Some(TodoStore::today());
                }
                eprintln!("  -> All subtodos completed, marking parent as completed");
            } else {
                // Not all subtodos are completed, mark parent as uncompleted
                parent_todo.completed = false;
                parent_todo.completed_at = None;
                parent_todo.last_reset_date = None;
                eprintln!("  -> Not all subtodos completed, marking parent as UNCOMPLETED");
            }
            eprintln!("  Parent after: '{}' completed={}", parent_todo.content, parent_todo.completed);
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
    store.get_todos(None)
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
async fn edit_todo(handle: AppHandle, id: String, content: String, repeat_mode: String, weekdays: Option<String>, specific_dates: Option<String>) -> Result<Vec<TodoItem>, String> {
    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;

    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        let old_repeat_mode = todo.repeat_mode.clone();
        let old_completed = todo.completed;
        let _old_last_reset = todo.last_reset_date.clone();

        todo.content = content;
        todo.repeat_mode = repeat_mode.clone();
        todo.weekdays = if repeat_mode == "weekly" { weekdays } else { None };
        todo.specific_dates = if repeat_mode == "specific_dates" { specific_dates } else { None };

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

    // Find all siblings with same parent_id
    let sibling_indices: Vec<usize> = todos.iter()
        .enumerate()
        .filter(|(_, t)| t.parent_id == parent_id)
        .map(|(i, _)| i)
        .collect();
    eprintln!("=== [MOVE UP] Found {} siblings with parent_id={:?}", sibling_indices.len(), parent_id);

    // Find position among siblings
    let pos = match sibling_indices.iter().position(|&i| i == idx) {
        Some(p) => p,
        None => {
            let err = format!("Todo not in siblings list");
            eprintln!("=== [MOVE UP] ERROR: {}", err);
            return Err(err);
        }
    };
    eprintln!("=== [MOVE UP] Todo is at position {} among siblings", pos);

    // If not first among siblings, swap with previous
    if pos > 0 {
        let prev_idx = sibling_indices[pos - 1];
        eprintln!("=== [MOVE UP] Swapping index {} with index {}", idx, prev_idx);
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

    // Find all siblings with same parent_id
    let sibling_indices: Vec<usize> = todos.iter()
        .enumerate()
        .filter(|(_, t)| t.parent_id == parent_id)
        .map(|(i, _)| i)
        .collect();
    eprintln!("=== [MOVE DOWN] Found {} siblings with parent_id={:?}", sibling_indices.len(), parent_id);

    // Find position among siblings
    let pos = match sibling_indices.iter().position(|&i| i == idx) {
        Some(p) => p,
        None => {
            let err = format!("Todo not in siblings list");
            eprintln!("=== [MOVE DOWN] ERROR: {}", err);
            return Err(err);
        }
    };
    eprintln!("=== [MOVE DOWN] Todo is at position {} among siblings (total: {})", pos, sibling_indices.len());

    // If not last among siblings, swap with next
    if pos < sibling_indices.len() - 1 {
        let next_idx = sibling_indices[pos + 1];
        eprintln!("=== [MOVE DOWN] Swapping index {} with index {}", idx, next_idx);
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
async fn complete_all_subtodos(handle: AppHandle, id: String) -> Result<Vec<TodoItem>, String> {
    use chrono::Local;

    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;

    // First, mark all subtodos as completed
    let parent_id = id.clone();
    for todo in todos.iter_mut() {
        if todo.parent_id.as_ref() == Some(&parent_id) {
            todo.completed = true;
            todo.completed_at = Some(Local::now().timestamp());
            if todo.repeat_mode == "daily" || todo.repeat_mode == "weekly" {
                todo.last_reset_date = Some(TodoStore::today());
            }
            eprintln!("Marked subtodo '{}' as completed", todo.content);
        }
    }

    // Then, mark the parent as completed
    if let Some(parent_todo) = todos.iter_mut().find(|t| t.id == id) {
        parent_todo.completed = true;
        parent_todo.completed_at = Some(Local::now().timestamp());
        if parent_todo.repeat_mode == "daily" || parent_todo.repeat_mode == "weekly" {
            parent_todo.last_reset_date = Some(TodoStore::today());
        }
        eprintln!("Marked parent '{}' as completed", parent_todo.content);
    }

    store.save(&todos)?;
    store.get_todos(None)
}

/// Complete a weekly todo or subtodo early (mark as completed on any day)
/// This allows users to complete a weekly repeating todo (or its subtodos) before its scheduled day
/// Note: For subtodos, this will also check if all siblings are completed and mark parent as completed.
#[tauri::command]
async fn complete_early(handle: AppHandle, id: String) -> Result<Vec<TodoItem>, String> {
    use chrono::Local;

    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;

    // Find the todo and mark as completed
    let parent_id: Option<String> = if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        let parent_id = todo.parent_id.clone();
        todo.completed = true;
        todo.completed_at = Some(Local::now().timestamp());
        // Set last_reset_date to today so it resets on the next scheduled weekday
        // This works for both parent todos and subtodos
        todo.last_reset_date = Some(TodoStore::today());

        let todo_type = if todo.parent_id.is_some() { "subtodo" } else { "parent todo" };
        eprintln!("Completed {} '{}' early on {}", todo_type, todo.content, TodoStore::today());

        parent_id
    } else {
        return Err("Todo not found".to_string());
    };

    // If this was a subtodo, check if all siblings are completed and mark parent as completed
    if let Some(pid) = parent_id {
        let all_siblings_completed = todos.iter()
            .filter(|t| t.parent_id.as_deref() == Some(&pid))
            .all(|t| t.completed);

        if all_siblings_completed {
            if let Some(parent_todo) = todos.iter_mut().find(|t| t.id == pid) {
                parent_todo.completed = true;
                parent_todo.completed_at = Some(Local::now().timestamp());
                if parent_todo.repeat_mode == "daily" || parent_todo.repeat_mode == "weekly" {
                    parent_todo.last_reset_date = Some(TodoStore::today());
                }
                eprintln!("All subtodos completed, marking parent '{}' as completed", parent_todo.content);
            }
        }
    }

    store.save(&todos)?;
    store.get_todos(None)
}

/// Toggle disable status of a todo
/// When disabled, the todo is marked as completed and cannot be operated on
/// When enabled (undisabled), the todo can be operated on normally
#[tauri::command]
async fn toggle_disable(handle: AppHandle, id: String) -> Result<Vec<TodoItem>, String> {
    use chrono::Local;

    let data_dir = get_data_dir(&handle);
    let store = TodoStore::new(data_dir);

    let mut todos = store.load()?;

    // Find the todo and toggle its disabled status
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        todo.disabled = !todo.disabled;
        let todo_type = if todo.parent_id.is_some() { "subtodo" } else { "parent todo" };

        if todo.disabled {
            // When disabling, mark as completed
            todo.completed = true;
            todo.completed_at = Some(Local::now().timestamp());
            if todo.repeat_mode == "daily" || todo.repeat_mode == "weekly" {
                todo.last_reset_date = Some(TodoStore::today());
            }
            eprintln!("Disabled {} '{}', marked as completed", todo_type, todo.content);
        } else {
            // When enabling, mark as uncompleted
            todo.completed = false;
            todo.completed_at = None;
            todo.last_reset_date = None;
            eprintln!("Enabled {} '{}', marked as uncompleted", todo_type, todo.content);
        }
    } else {
        return Err("Todo not found".to_string());
    }

    store.save(&todos)?;
    store.get_todos(None)
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

    match action.as_str() {
        "showHideWindow" => {
            settings.shortcuts.show_hide_window = shortcut.clone();
            eprintln!("Updated show/hide shortcut to: {} (restart required)", shortcut);
        }
        "toggleFloating" => {
            settings.shortcuts.toggle_floating = shortcut.clone();
            eprintln!("Updated toggle floating shortcut to: {} (restart required)", shortcut);
        }
        _ => return Err("Invalid action".to_string()),
    }

    store.save(&settings)?;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Single instance check - try to acquire lock
    let single_instance = single_instance::SingleInstance::new("MyTODO-app-instance").unwrap();
    if !single_instance.is_single() {
        // Another instance is already running - show dialog and exit
        std::thread::spawn(|| {
            use std::process::Command;
            let _ = Command::new("mshta")
                .args(&["vbscript:msgbox(\"MyTODO 已在运行！请检查系统托盘或任务栏。\",16,\"MyTODO\")(window.close)"])
                .output();
        });
        eprintln!("Another instance is already running. Exiting...");
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            eprintln!("=== MyTODO Starting ===");

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

            app.manage(settings_store.clone());

            eprintln!("Settings loaded: shortcuts={:?}", settings.shortcuts);

            // Initialize global shortcuts plugin with Builder pattern
            let show_hide_shortcut_str = settings.shortcuts.show_hide_window.clone();
            let toggle_floating_shortcut_str = settings.shortcuts.toggle_floating.clone();
            let app_handle = app.handle().clone();

            // Use a shared state to track window visibility more reliably
            use std::sync::atomic::{AtomicBool, Ordering};
            let window_visible = std::sync::Arc::new(AtomicBool::new(true));
            let window_visible_for_handler = window_visible.clone();

            let plugin = tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([
                    show_hide_shortcut_str.as_str(),
                    toggle_floating_shortcut_str.as_str(),
                ])?
                .with_handler(move |_app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let show_hide_matches = shortcut.matches(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT, Code::Digit0);
                        let show_hide_matches_mac = shortcut.matches(Modifiers::SUPER | Modifiers::SHIFT | Modifiers::ALT, Code::Digit0);
                        let toggle_floating_matches = shortcut.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyF);
                        let toggle_floating_matches_mac = shortcut.matches(Modifiers::SUPER | Modifiers::SHIFT, Code::KeyF);

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
                                                    Ok(_) => {}
                                                    Err(_) => {
                                                        let _ = window.unminimize();
                                                        let _ = window.show();
                                                        let _ = window.set_focus();
                                                    }
                                                }
                                            }
                                            Err(_) => {
                                                let _ = window.unminimize();
                                                let _ = window.show();
                                                let _ = window.set_focus();
                                            }
                                        }
                                    }
                                    #[cfg(not(target_os = "windows"))]
                                    {
                                        let _ = window.unminimize();
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                        let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
                                    }
                                }
                            }
                        } else if toggle_floating_matches || toggle_floating_matches_mac {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                tauri::async_runtime::block_on(async move {
                                    match window.is_always_on_top() {
                                        Ok(is_on_top) => {
                                            let _ = window.set_always_on_top(!is_on_top);
                                        }
                                        Err(_) => {}
                                    }
                                });
                            }
                        }
                    }
                })
                .build();

            match app.handle().plugin(plugin) {
                Ok(_) => {
                    eprintln!("Registered global shortcuts: {} (show/hide), {} (toggle floating)",
                        show_hide_shortcut_str, toggle_floating_shortcut_str);
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

            // Ensure the main window is shown on startup
            if let Some(window) = app.get_webview_window("main") {
                eprintln!("Showing main window...");
                let _ = window.show();
                let _ = window.set_focus();
            }

            // Handle window close event - hide instead of close
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                let window_clone2 = window.clone();
                let store = settings_store.clone();
                let window_visible_for_close = window_visible.clone();

                // Temporarily disable window state restoration
                eprintln!("Window state restoration disabled for debugging");

                // Spawn a task to save window state when closing
                window_clone.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            // Save window state to settings.json before hiding
                            eprintln!("Saving window state before close...");

                            // Get window info
                            let position = window_clone2.outer_position();
                            let size = window_clone2.outer_size();

                            if let (Ok(pos), Ok(sz)) = (position, size) {
                                let mut settings = store.load().unwrap_or_default();

                                // Get scale factor and convert physical to logical coordinates
                                let scale = window_clone2.scale_factor().unwrap_or(1.0);
                                let logical_pos: tauri::LogicalPosition<f64> = pos.to_logical(scale);
                                let logical_size: tauri::LogicalSize<f64> = sz.to_logical(scale);

                                // Round to nearest integer to avoid precision issues
                                settings.window.x = logical_pos.x.round() as i32;
                                settings.window.y = logical_pos.y.round() as i32;
                                settings.window.width = logical_size.width.round() as u32;
                                settings.window.height = logical_size.height.round() as u32;

                                let _ = store.save(&settings);
                                eprintln!("Window state saved: x={}, y={}, w={}, h={} (scale={})",
                                    settings.window.x, settings.window.y, settings.window.width, settings.window.height, scale);
                            }

                            api.prevent_close();
                            let _ = window_clone2.hide();
                            // Update visibility state
                            window_visible_for_close.store(false, Ordering::Relaxed);
                            eprintln!("[CloseEvent] Window hidden, visibility state updated to false");
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
            complete_early,
            toggle_disable,
            reorder_todos,
            move_todo_up,
            move_todo_down,
            get_settings,
            update_shortcut,
            save_window_state,
            update_memo,
            update_memo_height,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::image::Image;

    // Create tray menu items - only quit item
    let quit_item = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_item])?;

    // Try to load icon.ico first (Windows preferred), then fallback to PNG
    let icon_result = app.path().resource_dir()?
        .join("icons").join("icon.ico")
        .canonicalize();

    let icon = if let Ok(icon_path) = icon_result {
        if let Ok(bytes) = std::fs::read(&icon_path) {
            Image::new_owned(bytes, 32, 32)
        } else {
            // Fallback to create a simple colored icon
            create_simple_icon()
        }
    } else {
        create_simple_icon()
    };

    // Build tray icon with menu
    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("MyTODO - 待办事项管理")
        .on_menu_event(move |app, event| {
            match event.id.0.as_str() {
                "quit" => {
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
    // Create a simple 32x32 RGBA icon with orange color
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let idx = (y * 32 + x) * 4;
            // Orange color with transparency gradient
            let center_x = 16.0;
            let center_y = 16.0;
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = if dist < 14.0 { 255.0 } else { (15.0 - dist).max(0.0) * 17.0 };

            pixels[idx] = 79;   // R
            pixels[idx + 1] = 70;  // G
            pixels[idx + 2] = 229; // B (purple/indigo)
            pixels[idx + 3] = alpha as u8; // A
        }
    }
    Image::new_owned(pixels, 32, 32)
}
