use chrono::Local;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

/// Global shortcut configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfig {
    #[serde(rename = "showHideWindow", alias = "show_hide_window")]
    pub show_hide_window: String,
    #[serde(rename = "toggleFloating", alias = "toggle_floating")]
    pub toggle_floating: String,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            show_hide_window: "CommandOrControl+Shift+Alt+digit0".to_string(),
            toggle_floating: "CommandOrControl+Shift+F".to_string(),
        }
    }
}

/// Window state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 600,
            height: 700,
        }
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub shortcuts: ShortcutConfig,
    #[serde(rename = "window", alias = "window_state")]
    pub window: WindowState,
    #[serde(default)]
    pub memo: String,
    #[serde(default = "default_memo_height")]
    pub memo_height: u32,
}

fn default_memo_height() -> u32 {
    80
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcuts: ShortcutConfig::default(),
            window: WindowState::default(),
            memo: String::new(),
            memo_height: 80,
        }
    }
}

/// Settings store for managing application settings in JSON file
#[derive(Clone)]
pub struct SettingsStore {
    file_path: PathBuf,
}

impl SettingsStore {
    /// Create a new SettingsStore with the specified file path
    pub fn new(config_dir: PathBuf) -> Self {
        let file_path = config_dir.join("settings.json");
        // Ensure the directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        Self { file_path }
    }

    /// Load settings from the JSON file
    pub fn load(&self) -> Result<Settings, String> {
        if !self.file_path.exists() {
            let default = Settings::default();
            self.save(&default)?;
            return Ok(default);
        }

        let content = fs::read_to_string(&self.file_path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;

        if content.trim().is_empty() {
            let default = Settings::default();
            self.save(&default)?;
            return Ok(default);
        }

        // Try to parse, if fails, backup old file and use defaults
        match serde_json::from_str::<Settings>(&content) {
            Ok(settings) => Ok(settings),
            Err(e) => {
                eprintln!("Failed to parse settings: {}, using defaults", e);
                // Backup old settings file
                let backup_path = self.file_path.with_extension("json.backup");
                let _ = fs::copy(&self.file_path, &backup_path);
                eprintln!("Old settings backed up to: {:?}", backup_path);
                // Use defaults and save
                let default = Settings::default();
                self.save(&default)?;
                Ok(default)
            }
        }
    }

    /// Save settings to the JSON file
    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        let mut file = File::create(&self.file_path)
            .map_err(|e| format!("Failed to create settings file: {}", e))?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write settings: {}", e))?;

        Ok(())
    }
}

/// Todo item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    #[serde(rename = "repeatMode", alias = "repeat_mode")]
    pub repeat_mode: String, // "daily", "weekly", "none", "specific_dates"
    #[serde(rename = "weekdays", alias = "week_days")]
    pub weekdays: Option<String>, // For weekly mode: "1,3,5" (周一=1, 周日=7)
    #[serde(rename = "specificDates", alias = "specific_dates")]
    #[serde(default)]
    pub specific_dates: Option<String>, // For specific_dates mode: "2025-03-10,2025-03-11"
    #[serde(rename = "parentId", alias = "parent_id")]
    pub parent_id: Option<String>, // 父待办ID，None表示是顶级待办
    #[serde(rename = "expanded", default = "serde_aux::default_true")]
    pub expanded: bool, // 是否展开子待办
    pub completed: bool,
    #[serde(default)]
    pub disabled: bool, // 是否禁用，禁用后任务会变成完成状态且无法操作
    #[serde(rename = "createdAt", alias = "created_at")]
    pub created_at: i64,
    #[serde(rename = "completedAt", alias = "completed_at")]
    pub completed_at: Option<i64>,
    #[serde(rename = "lastResetDate", alias = "last_reset_date")]
    pub last_reset_date: Option<String>, // YYYY-MM-DD for daily/weekly items
}

// Helper module for serde default value
mod serde_aux {
    pub fn default_true() -> bool { true }
}

impl TodoItem {
    pub fn new(content: String, repeat_mode: String, weekdays: Option<String>, parent_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            repeat_mode,
            weekdays,
            specific_dates: None,
            parent_id,
            expanded: true, // 默认展开
            completed: false,
            disabled: false, // 默认不禁用
            created_at: Local::now().timestamp(),
            completed_at: None,
            last_reset_date: None,
        }
    }
}

/// Todo store for managing todos in JSON file
pub struct TodoStore {
    file_path: PathBuf,
}

impl TodoStore {
    /// Create a new TodoStore with the specified file path
    pub fn new(config_dir: PathBuf) -> Self {
        let file_path = config_dir.join("todos.json");
        // Ensure the directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        Self { file_path }
    }

    /// Load all todos from the JSON file
    pub fn load(&self) -> Result<Vec<TodoItem>, String> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.file_path)
            .map_err(|e| format!("Failed to read todos file: {}", e))?;

        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse todos: {}", e))
    }

    /// Save todos to the JSON file
    pub fn save(&self, todos: &[TodoItem]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(todos)
            .map_err(|e| format!("Failed to serialize todos: {}", e))?;

        let mut file = File::create(&self.file_path)
            .map_err(|e| format!("Failed to create todos file: {}", e))?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write todos: {}", e))?;

        Ok(())
    }

    /// Get current date as YYYY-MM-DD string
    pub fn today() -> String {
        Local::now().format("%Y-%m-%d").to_string()
    }

    /// Check if a daily todo needs to be reset
    pub fn should_reset_daily(todo: &TodoItem) -> bool {
        if todo.repeat_mode != "daily" {
            return false;
        }

        let today = Self::today();
        let should_reset = match &todo.last_reset_date {
            Some(last_date) => last_date != &today,
            None => true,
        };

        if should_reset {
            eprintln!("Daily todo needs reset: '{}', last_reset={:?}, today={}",
                todo.content, todo.last_reset_date, today);
        }

        should_reset
    }

    /// Get current day of week (1-7, Monday=1, Sunday=7)
    pub fn current_weekday() -> u32 {
        Local::now().weekday().number_from_monday()
    }

    /// Get the Monday of the current week as YYYY-MM-DD string
    pub fn current_week_monday() -> String {
        let now = Local::now();
        let weekday = now.weekday().number_from_monday(); // 1-7, Monday=1
        let days_since_monday = (weekday - 1) as i64;
        let monday = now - chrono::Duration::days(days_since_monday);
        monday.format("%Y-%m-%d").to_string()
    }

    /// Check if a weekly todo needs to be reset
    pub fn should_reset_weekly(todo: &TodoItem) -> bool {
        if todo.repeat_mode != "weekly" {
            return false;
        }

        // Get the weekdays from the todo (e.g., "1,3,5")
        let weekdays_str = match &todo.weekdays {
            Some(w) => w,
            None => return false, // No weekdays selected, don't reset
        };

        // Parse the weekdays
        let selected_weekdays: Vec<u32> = weekdays_str
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if selected_weekdays.is_empty() {
            return false;
        }

        // Get current day of week (1-7, Monday=1, Sunday=7)
        let today = Self::current_weekday();
        let is_today_selected = selected_weekdays.contains(&today);

        if !is_today_selected {
            return false;
        }

        // Check if already reset in the current week (natural week)
        // This prevents re-resetting when someone completed early on a different weekday
        let today_str = Self::today();
        let current_week_monday = Self::current_week_monday();

        let should_reset = match &todo.last_reset_date {
            Some(last_date) => {
                // Parse the last reset date
                let last_reset_date = chrono::NaiveDate::parse_from_str(last_date, "%Y-%m-%d").ok();
                let week_monday = chrono::NaiveDate::parse_from_str(&current_week_monday, "%Y-%m-%d").ok();

                if let (Some(last_reset), Some(week_monday)) = (last_reset_date, week_monday) {
                    // Only reset if the last reset was before this week's Monday
                    // This means if someone completed early this week, it won't reset again
                    last_reset < week_monday
                } else {
                    // If parsing failed, use the simple check (already reset today?)
                    last_date != &today_str
                }
            }
            None => true,
        };

        if should_reset {
            eprintln!("Weekly todo needs reset: '{}', last_reset={:?}, today={}, selected_weekdays={}",
                todo.content, todo.last_reset_date, today, weekdays_str);
        }

        should_reset
    }

    /// Check if a weekly todo should be shown for a specific date
    pub fn should_show_for_date(todo: &TodoItem, date_str: &str, all_todos: &[TodoItem]) -> bool {
        // If this is a subtodo, check if parent should show
        if let Some(parent_id) = &todo.parent_id {
            let parent_should_show = all_todos.iter()
                .any(|t| t.id == *parent_id && Self::should_show_for_date(t, date_str, all_todos));
            eprintln!("Subtodo '{}': parent_id={}, parent_should_show={}",
                todo.content, parent_id, parent_should_show);
            if !parent_should_show {
                eprintln!("Subtodo '{}' hidden because parent is not visible", todo.content);
                return false;
            }
            eprintln!("Subtodo '{}' visible because parent is visible", todo.content);
            return true; // Subtodo always shows if parent shows
        }

        // For "none" repeat mode todos, only show on the creation date
        if todo.repeat_mode == "none" {
            // Parse creation date
            let created_date = chrono::NaiveDateTime::from_timestamp_opt(todo.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string());
            let created_date_str = created_date.as_deref();
            // Only show if target date matches creation date
            let is_creation_date = created_date_str.map_or(false, |d| d == date_str);
            eprintln!("Non-repeating todo '{}': created_at={}, created_date_str={:?}, target_date={}, show={}",
                todo.content, todo.created_at, created_date_str, date_str, is_creation_date);
            return is_creation_date;
        }

        // For "specific_dates" mode, only show on the specified dates
        if todo.repeat_mode == "specific_dates" {
            if let Some(dates_str) = &todo.specific_dates {
                let selected_dates: std::collections::HashSet<String> = dates_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                let should_show = selected_dates.contains(date_str);
                eprintln!("Specific dates todo '{}': target_date={}, dates={:?}, should_show={}",
                    todo.content, date_str, selected_dates, should_show);
                return should_show;
            }
            return false;
        }

        if todo.repeat_mode != "weekly" {
            return true; // Daily todos always show
        }

        // Parse the target date
        let target_date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => return true, // Invalid date, always show
        };

        // Get the weekdays from the todo (e.g., "1,3,5")
        let weekdays_str = match &todo.weekdays {
            Some(w) => w,
            None => return true, // No weekdays selected, always show
        };

        // Parse the weekdays
        let selected_weekdays: Vec<u32> = weekdays_str
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if selected_weekdays.is_empty() {
            return true;
        }

        // Get day of week for target date (1-7, Monday=1, Sunday=7)
        let target_weekday = target_date.weekday().number_from_monday();
        let is_day_selected = selected_weekdays.contains(&target_weekday);

        eprintln!("Weekly todo '{}': target_date={}, target_weekday={}, selected={:?}, should_show={}",
            todo.content, date_str, target_weekday, selected_weekdays, is_day_selected);

        is_day_selected
    }

    /// Get all todos with daily reset logic applied
    pub fn get_todos(&self, target_date: Option<&str>) -> Result<Vec<TodoItem>, String> {
        let mut todos = self.load()?;

        eprintln!("Loading todos, checking for daily/weekly resets...");

        // Apply daily and weekly reset logic (only when getting today's todos)
        let today_str = Self::today();
        let date_to_check = target_date.unwrap_or(&today_str);
        let is_today = date_to_check == today_str;

        if is_today {
            let mut needs_save = false;
            let mut parent_ids_to_reset: Vec<String> = Vec::new();

            eprintln!("=== [GET_TODOS] Checking for daily/weekly resets on today view");

            // Check if it's the start of a new week (Monday)
            let current_weekday = Self::current_weekday();
            let is_new_week = current_weekday == 1; // Monday
            let current_week_monday = Self::current_week_monday();

            if is_new_week {
                eprintln!("Today is Monday ({}), checking for new week start...", current_week_monday);
            }

            // First pass: identify parents that need resetting
            for todo in &todos {
                // Only consider parent todos (not child todos) for reset
                // Skip disabled todos - they should not be reset
                let needs_reset = if todo.parent_id.is_some() {
                    // Skip child todos
                    false
                } else if todo.disabled {
                    // Skip disabled todos - they stay completed
                    eprintln!("=== [RESET] Skipping disabled parent todo '{}'", todo.content);
                    false
                } else if todo.repeat_mode == "daily" {
                    Self::should_reset_daily(todo)
                } else if todo.repeat_mode == "weekly" {
                    // For weekly todos, check if it's a new week
                    if is_new_week {
                        // Check if last reset was in a previous week
                        match &todo.last_reset_date {
                            Some(last_date) => {
                                // Parse the last reset date and check if it's before this week's Monday
                                let last_reset_date = chrono::NaiveDate::parse_from_str(last_date, "%Y-%m-%d").ok();
                                let week_monday = chrono::NaiveDate::parse_from_str(&current_week_monday, "%Y-%m-%d").ok();

                                if let (Some(last_reset), Some(week_monday)) = (last_reset_date, week_monday) {
                                    let should_reset = last_reset < week_monday;
                                    if should_reset {
                                        eprintln!("Weekly todo '{}' last reset on {} (before this week's {}), resetting for new week",
                                            todo.content, last_date, current_week_monday);
                                    }
                                    should_reset
                                } else {
                                    // If parsing failed, reset it to be safe
                                    true
                                }
                            }
                            None => true,
                        }
                    } else {
                        // Not Monday, use the original logic (only reset if today is selected)
                        Self::should_reset_weekly(todo)
                    }
                } else {
                    false
                };

                if needs_reset {
                    parent_ids_to_reset.push(todo.id.clone());
                }
            }

            // Second pass: apply resets to parent todos AND their subtodos
            // For subtodos, only reset those that are NOT already completed
            for todo in &mut todos {
                let needs_reset = if todo.disabled {
                    // Skip disabled todos - they stay completed
                    false
                } else if todo.parent_id.is_none() && parent_ids_to_reset.contains(&todo.id) {
                    // This is a parent todo that needs resetting
                    eprintln!("=== [RESET] Resetting parent todo '{}' (id={}) for new cycle", todo.content, todo.id);
                    todo.completed = false;
                    todo.completed_at = None;
                    todo.last_reset_date = Some(Self::today());
                    true
                } else if todo.parent_id.is_some() && parent_ids_to_reset.contains(&todo.parent_id.clone().unwrap_or_default()) {
                    // This is a subtodo whose parent needs resetting
                    // Only reset if NOT already completed - preserve completed subtodos!
                    if !todo.completed {
                        eprintln!("=== [RESET] Resetting subtodo '{}' (parent_id={:?}) for new cycle", todo.content, todo.parent_id);
                        todo.completed = false;
                        todo.completed_at = None;
                        true
                    } else {
                        eprintln!("=== [RESET] Skipping already-completed subtodo '{}' (parent_id={:?})", todo.content, todo.parent_id);
                        false
                    }
                } else {
                    false
                };

                if needs_reset {
                    needs_save = true;
                }
            }

            // Save if any todos were reset
            if needs_save {
                eprintln!("Saving todos after daily/weekly resets...");
                self.save(&todos)?;
            }
        }

        // Filter out weekly todos that shouldn't show for the target date
        // Need to check against the original list before filtering for parent-child relationships
        let todos_before_filter = todos.clone();
        eprintln!("=== FILTER START ===");
        eprintln!("Date: {}, Total todos before filter: {}", date_to_check, todos_before_filter.len());
        for todo in &todos_before_filter {
            eprintln!("  [ID: {}] content={}, repeat_mode={}, parent_id={:?}",
                todo.id, todo.content, todo.repeat_mode, todo.parent_id);
        }

        // First, let's count how many subtodos we have
        let subtodo_count = todos_before_filter.iter().filter(|t| t.parent_id.is_some()).count();
        eprintln!("Total subtodos: {}", subtodo_count);

        todos.retain(|todo| {
            let result = Self::should_show_for_date(todo, date_to_check, &todos_before_filter);
            if let Some(pid) = &todo.parent_id {
                eprintln!("  [RETAIN] Subtodo '{}' (parent={}): {}", todo.content, pid, result);
            } else {
                eprintln!("  [RETAIN] Parent todo '{}': {}", todo.content, result);
            }
            result
        });

        eprintln!("Total todos after filter: {}", todos.len());
        eprintln!("=== FILTER END ===");

        // For non-today dates, reset completion status based on repeat mode
        // - Daily parent todos and their subtodos: reset to incomplete
        // - Weekly todos and non-repeating todos: keep their state
        if !is_today {
            eprintln!("Non-today view, adjusting completed status");

            // First, collect IDs of daily repeating parent todos
            let daily_parent_ids: std::collections::HashSet<String> = todos.iter()
                .filter(|t| t.parent_id.is_none() && t.repeat_mode == "daily")
                .map(|t| t.id.clone())
                .collect();

            eprintln!("Daily parent IDs: {:?}", daily_parent_ids);

            // Reset daily parent todos and their subtodos
            for todo in &mut todos {
                let should_reset = if todo.parent_id.is_none() {
                    // Parent todo: reset if daily
                    todo.repeat_mode == "daily"
                } else {
                    // Subtodo: reset if its parent is daily
                    todo.parent_id.as_ref().map_or(false, |pid| daily_parent_ids.contains(pid))
                };

                if should_reset {
                    eprintln!("Resetting todo '{}' (parent_id={:?}, repeat_mode={})",
                        todo.content, todo.parent_id, todo.repeat_mode);
                    todo.completed = false;
                }
            }
        }

        // Note: Sorting removed to preserve user's manual order via move up/down buttons
        // The order is maintained as stored in the JSON file

        Ok(todos)
    }
}
