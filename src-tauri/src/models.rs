use chrono::Local;
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

fn is_effectively_done_for_date(todo: &TodoItem, date_to_check: &str) -> bool {
    if todo.completed || todo.disabled {
        return true;
    }

    if todo.parent_id.is_some() {
        if let Some(cycle) = TodoStore::cycle_window_for_date(todo, date_to_check) {
            if !cycle.active {
                return true;
            }
        }

        if let Some(expiry) = &todo.expiry_date {
            return date_to_check > expiry.as_str();
        }
    }

    false
}

#[derive(Debug, Clone)]
struct CycleWindowState {
    active: bool,
    window_start: NaiveDate,
    window_end: NaiveDate,
}

fn write_json_atomically(file_path: &PathBuf, json: &str) -> Result<(), String> {
    let parent = file_path
        .parent()
        .ok_or_else(|| format!("Invalid file path: {:?}", file_path))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid file name: {:?}", file_path))?;
    let temp_path = parent.join(format!("{}.tmp", file_name));
    let backup_path = parent.join(format!("{}.bak", file_name));

    {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        file.sync_all()
            .map_err(|e| format!("Failed to flush temp file: {}", e))?;
    }

    if file_path.exists() {
        let _ = fs::remove_file(&backup_path);
        fs::rename(file_path, &backup_path)
            .map_err(|e| format!("Failed to move existing data file to backup: {}", e))?;
    }

    if let Err(rename_err) = fs::rename(&temp_path, file_path) {
        if backup_path.exists() && !file_path.exists() {
            let _ = fs::rename(&backup_path, file_path);
        }
        return Err(format!("Failed to replace data file atomically: {}", rename_err));
    }

    Ok(())
}

/// Global shortcut configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfig {
    #[serde(rename = "showHideWindow", alias = "show_hide_window")]
    pub show_hide_window: String,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            show_hide_window: "CommandOrControl+Shift+Alt+digit0".to_string(),
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
    #[serde(rename = "memoHeight", alias = "memo_height", default = "default_memo_height")]
    pub memo_height: u32,
    #[serde(rename = "autoLaunch", alias = "auto_launch", default = "default_auto_launch")]
    pub auto_launch: bool,
    #[serde(rename = "priorityColors", alias = "priority_colors", default)]
    pub priority_colors: Option<HashMap<u8, String>>,
    #[serde(rename = "backgroundWallpaper", alias = "background_wallpaper", default)]
    pub background_wallpaper: Option<String>,
}

fn default_memo_height() -> u32 {
    200
}

fn default_auto_launch() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcuts: ShortcutConfig::default(),
            window: WindowState::default(),
            memo: String::new(),
            memo_height: 200,
            auto_launch: true,
            priority_colors: None,
            background_wallpaper: None,
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

    fn apply_json_settings(settings: &mut Settings, value: &serde_json::Value) {
        if let Some(shortcuts) = value.get("shortcuts") {
            if let Some(show_hide) = shortcuts
                .get("showHideWindow")
                .or_else(|| shortcuts.get("show_hide_window"))
                .and_then(|v| v.as_str())
            {
                settings.shortcuts.show_hide_window = show_hide.to_string();
            }
        }

        if let Some(window) = value.get("window").or_else(|| value.get("window_state")) {
            if let Some(x) = window.get("x").and_then(|v| v.as_i64()) {
                settings.window.x = x as i32;
            }
            if let Some(y) = window.get("y").and_then(|v| v.as_i64()) {
                settings.window.y = y as i32;
            }
            if let Some(width) = window.get("width").and_then(|v| v.as_u64()) {
                settings.window.width = width as u32;
            }
            if let Some(height) = window.get("height").and_then(|v| v.as_u64()) {
                settings.window.height = height as u32;
            }
        }

        if let Some(memo) = value.get("memo").and_then(|v| v.as_str()) {
            settings.memo = memo.to_string();
        }

        if let Some(height) = value
            .get("memoHeight")
            .or_else(|| value.get("memo_height"))
            .and_then(|v| v.as_u64())
        {
            settings.memo_height = (height as u32).clamp(60, 300);
        }

        if let Some(auto_launch) = value
            .get("autoLaunch")
            .or_else(|| value.get("auto_launch"))
            .and_then(|v| v.as_bool())
        {
            settings.auto_launch = auto_launch;
        }

        if let Some(priority_colors) = value
            .get("priorityColors")
            .or_else(|| value.get("priority_colors"))
            .cloned()
        {
            if let Ok(colors) = serde_json::from_value::<Option<HashMap<u8, String>>>(priority_colors) {
                settings.priority_colors = colors;
            }
        }

        if let Some(background_wallpaper) = value
            .get("backgroundWallpaper")
            .or_else(|| value.get("background_wallpaper"))
        {
            settings.background_wallpaper = background_wallpaper.as_str().map(|v| v.to_string());
        }
    }

    fn salvage_settings_from_json(&self, content: &str) -> Option<Settings> {
        let value = serde_json::from_str::<serde_json::Value>(content).ok()?;
        let mut settings = Settings::default();
        Self::apply_json_settings(&mut settings, &value);
        Some(settings)
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

        // Try to parse, if fails, backup old file and salvage readable fields.
        match serde_json::from_str::<Settings>(&content) {
            Ok(settings) => {
                if content.contains("\"toggleFloating\"") || content.contains("\"toggle_floating\"") {
                    let _ = self.save(&settings);
                }
                Ok(settings)
            }
            Err(e) => {
                eprintln!("Failed to parse settings: {}, attempting to salvage readable fields", e);
                // Backup old settings file
                let backup_path = self.file_path.with_extension("json.backup");
                let _ = fs::copy(&self.file_path, &backup_path);
                eprintln!("Old settings backed up to: {:?}", backup_path);

                if let Some(settings) = self.salvage_settings_from_json(&content) {
                    self.save(&settings)?;
                    return Ok(settings);
                }

                // Use defaults only when even the JSON shape cannot be read.
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

        write_json_atomically(&self.file_path, &json)
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
    #[serde(rename = "activeWeekdays", alias = "active_weekdays", default)]
    pub active_weekdays: Option<String>,
    #[serde(rename = "specificDates", alias = "specific_dates")]
    #[serde(default)]
    pub specific_dates: Option<String>, // For specific_dates mode: "2025-03-10,2025-03-11"
    #[serde(rename = "parentId", alias = "parent_id")]
    pub parent_id: Option<String>, // 鐖跺緟鍔濱D锛孨one琛ㄧず鏄《绾у緟鍔?
    #[serde(rename = "expanded", default = "serde_aux::default_true")]
    pub expanded: bool, // 是否展开子待办
    pub completed: bool,
    #[serde(default)]
    pub disabled: bool, // 是否禁用，禁用后任务会变成完成状态且无法操作
    #[serde(rename = "inactiveByWeekday", alias = "inactive_by_weekday", default)]
    pub inactive_by_weekday: bool,
    #[serde(rename = "createdAt", alias = "created_at")]
    pub created_at: i64,
    #[serde(rename = "completedAt", alias = "completed_at")]
    pub completed_at: Option<i64>,
    #[serde(rename = "lastResetDate", alias = "last_reset_date")]
    pub last_reset_date: Option<String>, // YYYY-MM-DD for daily/weekly items
    #[serde(rename = "expiryDate", alias = "expiry_date")]
    #[serde(default)]
    pub expiry_date: Option<String>, // YYYY-MM-DD 格式，仅用于子待办
    #[serde(rename = "cycleStartDate", alias = "cycle_start_date", default)]
    pub cycle_start_date: Option<String>,
    #[serde(rename = "cycleActiveDays", alias = "cycle_active_days", default)]
    pub cycle_active_days: Option<u32>,
    #[serde(rename = "cycleIntervalWeeks", alias = "cycle_interval_weeks", default)]
    pub cycle_interval_weeks: Option<u32>,
    #[serde(rename = "priority", default)]
    pub priority: Option<u8>, // 1, 2, 3 或 None，只对父待办有效
    #[serde(rename = "order", default)]
    pub order: Option<i32>, // 排序字段，用于手动调整待办顺序
    #[serde(rename = "weekStart", alias = "week_start", default)]
    pub week_start: Option<String>, // YYYY-MM-DD 格式，记录周待办所属周的周一，仅用于 weekly_this_week 模式
    #[serde(rename = "monthStart", alias = "month_start", default)]
    pub month_start: Option<String>, // YYYY-MM-DD 格式，记录月待办所属月的第一天，仅用于 monthly_this_month 模式
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
            active_weekdays: None,
            specific_dates: None,
            parent_id,
            expanded: true, // 默认展开
            completed: false,
            disabled: false, // 默认不禁用
            inactive_by_weekday: false,
            created_at: Local::now().timestamp(),
            completed_at: None,
            last_reset_date: None,
            expiry_date: None,
            cycle_start_date: None,
            cycle_active_days: None,
            cycle_interval_weeks: None,
            priority: None,
            order: None,
            week_start: None,
            month_start: None,
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

        write_json_atomically(&self.file_path, &json)
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

    /// Get the first day of the current month as YYYY-MM-DD string
    pub fn current_month_first() -> String {
        let now = Local::now();
        let first_day = now.with_day(1).unwrap_or(now);
        first_day.format("%Y-%m-%d").to_string()
    }

    fn parse_weekday_list(raw: Option<&String>) -> Vec<u32> {
        raw.map(|value| {
            value
                .split(',')
                .filter_map(|s| s.trim().parse::<u32>().ok())
                .filter(|d| (1..=7).contains(d))
                .collect()
        }).unwrap_or_default()
    }

    fn is_subtodo_active_on_weekday(todo: &TodoItem, weekday: u32) -> bool {
        let selected_weekdays = Self::parse_weekday_list(todo.active_weekdays.as_ref());
        if selected_weekdays.is_empty() {
            return true;
        }
        selected_weekdays.contains(&weekday)
    }

    fn is_subtodo_active_on_date(todo: &TodoItem, date_str: &str) -> bool {
        let target_date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => return true,
        };
        Self::is_subtodo_active_on_weekday(todo, target_date.weekday().number_from_monday())
    }

    fn cycle_window_for_date(todo: &TodoItem, date_str: &str) -> Option<CycleWindowState> {
        if todo.parent_id.is_none() {
            return None;
        }

        let start_date = NaiveDate::parse_from_str(todo.cycle_start_date.as_ref()?, "%Y-%m-%d").ok()?;
        let target_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
        let interval_weeks = todo.cycle_interval_weeks.unwrap_or(4).clamp(1, 52);
        let interval_days = i64::from(interval_weeks) * 7;
        let active_days = i64::from(todo.cycle_active_days.unwrap_or(7).clamp(1, interval_days as u32));

        if target_date < start_date {
            return Some(CycleWindowState {
                active: false,
                window_start: start_date,
                window_end: start_date + Duration::days(active_days - 1),
            });
        }

        let days_since_start = (target_date - start_date).num_days();
        let cycle_index = days_since_start / interval_days;
        let window_start = start_date + Duration::days(cycle_index * interval_days);
        let window_end = window_start + Duration::days(active_days - 1);

        Some(CycleWindowState {
            active: target_date >= window_start && target_date <= window_end,
            window_start,
            window_end,
        })
    }

    pub fn apply_cycle_activation_for_date(todos: &mut Vec<TodoItem>, date_str: &str) -> bool {
        let mut changed = false;
        let mut parent_ids_to_uncomplete: HashSet<String> = HashSet::new();

        for todo in todos.iter_mut() {
            if todo.parent_id.is_none() || todo.disabled {
                continue;
            }

            let Some(cycle) = Self::cycle_window_for_date(todo, date_str) else {
                continue;
            };

            if !cycle.active {
                continue;
            }

            let window_start = cycle.window_start.format("%Y-%m-%d").to_string();
            if todo.last_reset_date.as_deref() == Some(window_start.as_str()) {
                continue;
            }

            todo.completed = false;
            todo.completed_at = None;
            todo.last_reset_date = Some(window_start);
            if let Some(parent_id) = &todo.parent_id {
                parent_ids_to_uncomplete.insert(parent_id.clone());
            }
            changed = true;
        }

        if !parent_ids_to_uncomplete.is_empty() {
            for todo in todos.iter_mut().filter(|todo| parent_ids_to_uncomplete.contains(&todo.id)) {
                if todo.completed || todo.completed_at.is_some() {
                    todo.completed = false;
                    todo.completed_at = None;
                    changed = true;
                }
            }
        }

        changed
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

        eprintln!("=== [SHOULD_RESET_WEEKLY] todo='{}', today={}, selected_weekdays={}, is_today_selected={}",
            todo.content, today, weekdays_str, is_today_selected);

        if !is_today_selected {
            eprintln!("=== [SHOULD_RESET_WEEKLY] Today not selected, returning false");
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

                if let (Some(_last_reset), Some(_week_monday)) = (last_reset_date, week_monday) {
                    // BUG FIX: For weekly todos with multiple weekdays (e.g., 5,6,7),
                    // we need to reset EACH day that is selected, not just once per week.
                    // The original logic only allowed reset if last_reset was BEFORE week_monday,
                    // but this prevents resetting on Saturday after completing on Friday.
                    //
                    // NEW LOGIC: Reset if:
                    // 1. last_reset is from a previous week (last_reset < week_monday) - classic weekly reset
                    // 2. OR last_reset is from today but a DIFFERENT selected weekday in the same week
                    //    (e.g., completed Friday, now it's Saturday, both are selected)
                    //
                    // Actually simpler: Just check if last_reset is NOT today
                    // If the todo was reset/completed on a different day, reset again
                    let not_reset_today = last_date != &today_str;
                    eprintln!("=== [SHOULD_RESET_WEEKLY] last_reset={}, week_monday={}, today={}, not_reset_today={}",
                        last_date, current_week_monday, today_str, not_reset_today);
                    not_reset_today
                } else {
                    // If parsing failed, use the simple check (already reset today?)
                    let not_reset_today = last_date != &today_str;
                    eprintln!("=== [SHOULD_RESET_WEEKLY] Parsing failed, using simple check: not_reset_today={}", not_reset_today);
                    not_reset_today
                }
            }
            None => {
                eprintln!("=== [SHOULD_RESET_WEEKLY] No last_reset_date, returning true");
                true
            }
        };

        if should_reset {
            eprintln!("=== [SHOULD_RESET_WEEKLY] todo '{}' NEEDS reset", todo.content);
        } else {
            eprintln!("=== [SHOULD_RESET_WEEKLY] todo '{}' does NOT need reset (completed today)", todo.content);
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
            if !Self::is_subtodo_active_on_date(todo, date_str) {
                eprintln!("Subtodo '{}' hidden because weekday is inactive for {}", todo.content, date_str);
                return true;
            }
            eprintln!("Subtodo '{}' visible because parent is visible", todo.content);
            return true;
        }

        // For "long_term" mode, always show (persistent todos without time limit)
        if todo.repeat_mode == "long_term" {
            eprintln!("Long term todo '{}': always showing", todo.content);
            return true;
        }

        // For "none" repeat mode todos, only show on the creation date
        if todo.repeat_mode == "none" {
            // Parse creation date
            let created_date = chrono::DateTime::from_timestamp(todo.created_at, 0)
                .map(|dt| dt.date_naive().format("%Y-%m-%d").to_string());
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

        // For "weekly_this_week" mode (周待办), show if target date is in the same week as the todo
        if todo.repeat_mode == "weekly_this_week" {
            if let Some(week_start_str) = &todo.week_start {
                // Parse the week_start (Monday of the week)
                let week_start = match chrono::NaiveDate::parse_from_str(week_start_str, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => return true, // Invalid date, always show
                };
                // Parse the target date
                let target_date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => return true, // Invalid date, always show
                };
                // Calculate the difference in days
                let days_diff = (target_date - week_start).num_days();
                // Show if target date is within 0-6 days of the week_start (Monday to Sunday)
                let in_same_week = days_diff >= 0 && days_diff <= 6;
                eprintln!("Weekly_this_week todo '{}': week_start={}, target_date={}, days_diff={}, in_same_week={}",
                    todo.content, week_start_str, date_str, days_diff, in_same_week);
                return in_same_week;
            }
            return true; // No week_start, always show
        }

        // For "monthly_this_month" mode (本月待办), show if target date is in the same month as the todo
        if todo.repeat_mode == "monthly_this_month" {
            if let Some(month_start_str) = &todo.month_start {
                // Parse the month_start (first day of the month)
                let month_start = match chrono::NaiveDate::parse_from_str(month_start_str, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => return true, // Invalid date, always show
                };
                // Parse the target date
                let target_date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => return true, // Invalid date, always show
                };
                // Check if both dates are in the same year and month
                let in_same_month = month_start.year() == target_date.year() && month_start.month() == target_date.month();
                eprintln!("Monthly_this_month todo '{}': month_start={}, target_date={}, in_same_month={}",
                    todo.content, month_start_str, date_str, in_same_month);
                return in_same_month;
            }
            return true; // No month_start, always show
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
            None => {
                eprintln!("Weekly todo '{}' has no weekdays selected, hiding", todo.content);
                return false; // No weekdays selected, don't show
            }
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

        // Update expired weekly_this_week and monthly_this_month todos to current week/month
        // This ensures that "this week/month" todos continue to show after week/month boundaries
        let current_week_monday = Self::current_week_monday();
        let current_month_first = Self::current_month_first();
        let mut parent_ids_to_reset_for_period: Vec<String> = Vec::new();

        // First pass: identify parent todos that need period reset (week/month)
        for todo in &todos {
            // Only check parent todos
            if todo.parent_id.is_some() {
                continue;
            }

            // Check weekly_this_week todos
            if todo.repeat_mode == "weekly_this_week" {
                if let Some(week_start_str) = &todo.week_start {
                    let week_start = match chrono::NaiveDate::parse_from_str(week_start_str, "%Y-%m-%d") {
                        Ok(d) => d,
                        Err(_) => continue,
                    };
                    let current_week = match chrono::NaiveDate::parse_from_str(&current_week_monday, "%Y-%m-%d") {
                        Ok(d) => d,
                        Err(_) => continue,
                    };
                    // If the stored week_start is before current week, mark for reset
                    if week_start < current_week {
                        eprintln!("=== [PERIOD RESET] Marking weekly_this_week todo '{}' for reset (week_start: {} -> {})",
                            todo.content, week_start_str, current_week_monday);
                        parent_ids_to_reset_for_period.push(todo.id.clone());
                    }
                }
            }

            // Check monthly_this_month todos
            if todo.repeat_mode == "monthly_this_month" {
                if let Some(month_start_str) = &todo.month_start {
                    let month_start = match chrono::NaiveDate::parse_from_str(month_start_str, "%Y-%m-%d") {
                        Ok(d) => d,
                        Err(_) => continue,
                    };
                    let current_month = match chrono::NaiveDate::parse_from_str(&current_month_first, "%Y-%m-%d") {
                        Ok(d) => d,
                        Err(_) => continue,
                    };
                    // If the stored month_start is before current month, mark for reset
                    if month_start < current_month {
                        eprintln!("=== [PERIOD RESET] Marking monthly_this_month todo '{}' for reset (month_start: {} -> {})",
                            todo.content, month_start_str, current_month_first);
                        parent_ids_to_reset_for_period.push(todo.id.clone());
                    }
                }
            }
        }

        // Second pass: apply period reset to parent todos AND their subtodos
        if !parent_ids_to_reset_for_period.is_empty() {
            eprintln!("=== [PERIOD RESET] Resetting {} parent todos and their subtodos for new period", parent_ids_to_reset_for_period.len());
            let mut needs_save_for_period = false;

            for todo in &mut todos {
                let should_reset = if todo.parent_id.is_none() && parent_ids_to_reset_for_period.contains(&todo.id) {
                    // This is a parent todo that needs period reset
                    eprintln!("=== [PERIOD RESET] Resetting parent todo '{}' (id={}) for new period", todo.content, todo.id);

                    // Update week_start or month_start
                    if todo.repeat_mode == "weekly_this_week" {
                        todo.week_start = Some(current_week_monday.clone());
                    } else if todo.repeat_mode == "monthly_this_month" {
                        todo.month_start = Some(current_month_first.clone());
                    }

                    todo.completed = false;
                    todo.completed_at = None;
                    todo.last_reset_date = Some(Self::today());
                    true
                } else if todo.parent_id.is_some() && parent_ids_to_reset_for_period.contains(&todo.parent_id.clone().unwrap_or_default()) {
                    // This is a subtodo whose parent needs period reset
                    eprintln!("=== [PERIOD RESET] Resetting subtodo '{}' for parent's new period", todo.content);
                    todo.completed = false;
                    todo.completed_at = None;
                    true
                } else {
                    false
                };

                if should_reset {
                    needs_save_for_period = true;
                }
            }

            if needs_save_for_period {
                eprintln!("Saving todos after period reset...");
                self.save(&todos)?;
            }
        }

        // Apply daily and weekly reset logic (only when getting today's todos)
        let today_str = Self::today();
        let date_to_check = target_date.unwrap_or(&today_str);
        let is_today = date_to_check == today_str;

        // Note: Expiry-based disabling is now handled dynamically below, not saved to file
        // This allows todos to be re-enabled when viewing dates before the expiry date

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
            // All subtodos should be reset regardless of their completion status
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
                    // Always reset subtodos regardless of their completion status
                    eprintln!("=== [RESET] Resetting subtodo '{}' (parent_id={:?}, was_completed={}) for new cycle",
                        todo.content, todo.parent_id, todo.completed);
                    todo.completed = false;
                    todo.completed_at = None;
                    true
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

            let parent_repeat_modes: HashMap<String, String> = todos.iter()
                .filter(|t| t.parent_id.is_none())
                .map(|t| (t.id.clone(), t.repeat_mode.clone()))
                .collect();

            let mut subtodo_needs_save = false;
            for todo in &mut todos {
                if todo.parent_id.is_none() || todo.disabled {
                    continue;
                }

                let Some(parent_id) = todo.parent_id.as_ref() else {
                    continue;
                };

                let parent_repeat_mode = parent_repeat_modes.get(parent_id)
                    .map(|mode| mode.as_str())
                    .unwrap_or("");

                // 子待办“按周几启用”的自动重置，只用于普通按日/按周循环。
                // 对“本周内 / 本月内”父待办，完成状态应跟随周/月周期，不能每天刷回未完成。
                if parent_repeat_mode == "weekly_this_week" || parent_repeat_mode == "monthly_this_month" {
                    continue;
                }

                if !Self::is_subtodo_active_on_weekday(todo, current_weekday) {
                    continue;
                }

                if todo.last_reset_date.as_deref() == Some(today_str.as_str()) {
                    continue;
                }

                todo.completed = false;
                todo.completed_at = None;
                todo.last_reset_date = Some(today_str.clone());
                subtodo_needs_save = true;
            }

            if subtodo_needs_save {
                eprintln!("Saving todos after active subtodo weekday resets...");
                self.save(&todos)?;
            }

            if Self::apply_cycle_activation_for_date(&mut todos, today_str.as_str()) {
                eprintln!("Saving todos after cycle subtodo activation resets...");
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
            eprintln!("Non-today view ({}), adjusting completed status", date_to_check);

            // Build a map of parent IDs to (repeat_mode, last_reset_date)
            let parent_info: std::collections::HashMap<String, (String, Option<String>)> = todos.iter()
                .filter(|t| t.parent_id.is_none())
                .map(|t| (t.id.clone(), (t.repeat_mode.clone(), t.last_reset_date.clone())))
                .collect();

            // Reset daily parent todos and their subtodos
            // Reset if last_reset_date is not equal to the VIEW date (not today's date)
            for todo in &mut todos {
                let should_reset = if todo.parent_id.is_none() {
                    // Parent todo: reset if daily and last_reset_date != view date
                    todo.repeat_mode == "daily" && todo.last_reset_date.as_ref().map_or(true, |d| d != date_to_check)
                } else {
                    // Subtodo: check if parent is daily and parent's last_reset_date != view date
                    if let Some(parent_id) = &todo.parent_id {
                        if let Some((parent_repeat_mode, parent_last_reset)) = parent_info.get(parent_id) {
                            // Parent is daily and last_reset_date != view date (or never reset)
                            parent_repeat_mode == "daily" && parent_last_reset.as_ref().map_or(true, |d| d != date_to_check)
                        } else {
                            // Parent not found (shouldn't happen)
                            false
                        }
                    } else {
                        false
                    }
                };

                if should_reset {
                    eprintln!("Resetting todo '{}' (view_date={}, last_reset_date={:?})",
                        todo.content, date_to_check, todo.last_reset_date);
                    todo.completed = false;
                }
            }
        }

        for todo in &mut todos {
            todo.inactive_by_weekday = false;
        }

        // Sort parent todos by priority (higher priority first, i.e., 1 > 2 > 3)
        // When priorities are equal, maintain file order (which is what user controls via move up/down)
        // Subtodos stay with their parents and maintain their order
        let parent_todos_with_index: Vec<(usize, TodoItem)> = todos.iter()
            .enumerate()
            .filter(|(_, t)| t.parent_id.is_none())
            .map(|(i, t)| (i, t.clone()))
            .collect();

        let mut parent_todos_sorted: Vec<(usize, TodoItem)> = parent_todos_with_index.clone();
        parent_todos_sorted.sort_by(|a, b| {
            // First compare by priority
            match (b.1.priority, a.1.priority) {
                (Some(p1), Some(p2)) => {
                    // Same priority: use file index to maintain user's manual order
                    match p1.cmp(&p2) {
                        std::cmp::Ordering::Equal => a.0.cmp(&b.0),
                        other => other,
                    }
                },
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => {
                    // Both have no priority: use file index (user's manual order)
                    a.0.cmp(&b.0)
                },
            }
        });

        let parent_todos: Vec<TodoItem> = parent_todos_sorted.into_iter()
            .map(|(_, t)| t)
            .collect();

        // Rebuild the todos list in sorted order
        let mut sorted_todos = Vec::new();
        for parent in &parent_todos {
            // Add the parent todo
            sorted_todos.push(parent.clone());
            // Add all its subtodos
            for todo in &todos {
                if todo.parent_id.as_ref() == Some(&parent.id) {
                    sorted_todos.push(todo.clone());
                }
            }
        }

        // Note: Sorting logic now includes priority-based sorting for parent todos
        // while preserving user's manual order for todos with same priority

        // Apply dynamic expiry-based disabling (not saved to file)
        // This allows todos to be re-enabled when viewing earlier dates
        // Note: expiry_date only applies to subtodos, not parent todos
        for todo in &mut sorted_todos {
            // Only check subtodos for expiry (parent todos don't have expiry_date)
            if todo.parent_id.is_some() {
                if let Some(cycle) = Self::cycle_window_for_date(todo, date_to_check) {
                    if cycle.active {
                        todo.expiry_date = Some(cycle.window_end.format("%Y-%m-%d").to_string());
                    } else if !todo.disabled {
                        todo.expiry_date = None;
                        todo.inactive_by_weekday = true;
                        todo.completed = true;
                        continue;
                    }
                }

                if let Some(expiry) = &todo.expiry_date {
                    // Check if view date is past expiry date
                    if date_to_check > expiry.as_str() {
                        // Dynamically disable this expired subtodo
                        todo.disabled = true;
                        todo.completed = true;
                    }
                }

                if !todo.disabled && !Self::is_subtodo_active_on_date(todo, date_to_check) {
                    todo.inactive_by_weekday = true;
                    todo.completed = true;
                }
            }
        }

        // After dynamic disabling, check if parent todos should be marked as completed
        // when all their subtodos are completed or disabled
        // First, collect parent IDs that need to be marked as completed
        let parent_ids_to_complete: Vec<String> = sorted_todos.iter()
            .filter(|t| t.parent_id.is_none())
            .filter(|parent| {
                // Check if all subtodos are completed or disabled
                sorted_todos.iter()
                    .filter(|t| t.parent_id.as_deref() == Some(&parent.id))
                    .all(|t| is_effectively_done_for_date(t, date_to_check))
            })
            .map(|t| t.id.clone())
            .collect();

        // Then, mark those parents as completed
        for todo in &mut sorted_todos {
            if parent_ids_to_complete.contains(&todo.id) {
                todo.completed = true;
            }
        }

        Ok(sorted_todos)
    }
}

/// Long term todo item - persistent todos without time limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermTodo {
    pub id: String,
    pub content: String,
    pub completed: bool,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "completedAt")]
    pub completed_at: Option<i64>,
}

impl LongTermTodo {
    pub fn new(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            completed: false,
            created_at: Local::now().timestamp(),
            completed_at: None,
        }
    }
}

/// Store for long term todos
pub struct LongTermTodoStore {
    file_path: PathBuf,
}

impl LongTermTodoStore {
    /// Create a new LongTermTodoStore with the specified file path
    pub fn new(config_dir: PathBuf) -> Self {
        let file_path = config_dir.join("long_term_todos.json");
        // Ensure the directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        Self { file_path }
    }

    /// Load all long term todos from the JSON file
    pub fn load(&self) -> Result<Vec<LongTermTodo>, String> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.file_path)
            .map_err(|e| format!("Failed to read long term todos file: {}", e))?;

        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse long term todos: {}", e))
    }

    /// Save long term todos to the JSON file
    pub fn save(&self, todos: &[LongTermTodo]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(todos)
            .map_err(|e| format!("Failed to serialize long term todos: {}", e))?;

        write_json_atomically(&self.file_path, &json)
    }

    /// Add a new long term todo
    pub fn add(&self, content: String) -> Result<LongTermTodo, String> {
        let mut todos = self.load()?;
        let new_todo = LongTermTodo::new(content);
        todos.push(new_todo.clone());
        self.save(&todos)?;
        Ok(new_todo)
    }

    /// Toggle completion status of a long term todo
    pub fn toggle(&self, id: &str) -> Result<LongTermTodo, String> {
        let mut todos = self.load()?;
        let todo = todos.iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("Long term todo not found: {}", id))?;

        todo.completed = !todo.completed;
        if todo.completed {
            todo.completed_at = Some(Local::now().timestamp());
        } else {
            todo.completed_at = None;
        }

        let updated_todo = todo.clone();
        self.save(&todos)?;
        Ok(updated_todo)
    }

    /// Delete a long term todo
    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut todos = self.load()?;
        let original_len = todos.len();
        todos.retain(|t| t.id != id);

        if todos.len() == original_len {
            return Err(format!("Long term todo not found: {}", id));
        }

        self.save(&todos)?;
        Ok(())
    }

    /// Move a long term todo up in the list
    pub fn move_up(&self, id: &str) -> Result<(), String> {
        let mut todos = self.load()?;
        let len = todos.len();

        for i in 1..len {
            if todos[i].id == id {
                todos.swap(i, i - 1);
                self.save(&todos)?;
                return Ok(());
            }
        }

        Err(format!("Cannot move up: {}", id))
    }

    /// Move a long term todo down in the list
    pub fn move_down(&self, id: &str) -> Result<(), String> {
        let mut todos = self.load()?;
        let len = todos.len();

        for i in 0..len.saturating_sub(1) {
            if todos[i].id == id {
                todos.swap(i, i + 1);
                self.save(&todos)?;
                return Ok(());
            }
        }

        Err(format!("Cannot move down: {}", id))
    }

    /// Update a long term todo's content
    pub fn update(&self, id: &str, content: &str) -> Result<(), String> {
        let mut todos = self.load()?;
        let todo = todos.iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("Long term todo not found: {}", id))?;

        todo.content = content.to_string();
        self.save(&todos)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn todo_store_for_test(name: &str) -> TodoStore {
        let dir = std::env::temp_dir().join(format!("lighttodo_cycle_test_{}_{}", name, Uuid::new_v4()));
        TodoStore::new(dir)
    }

    fn parent_todo(id: &str) -> TodoItem {
        let mut todo = TodoItem::new("父待办".to_string(), "daily".to_string(), None, None);
        todo.id = id.to_string();
        todo
    }

    fn cyclic_subtodo(id: &str, parent_id: &str) -> TodoItem {
        let mut todo = TodoItem::new(
            "周期子待办".to_string(),
            "none".to_string(),
            None,
            Some(parent_id.to_string()),
        );
        todo.id = id.to_string();
        todo.cycle_start_date = Some("2026-07-14".to_string());
        todo.cycle_active_days = Some(7);
        todo.cycle_interval_weeks = Some(4);
        todo
    }

    #[test]
    fn cycle_subtodo_is_active_inside_window_and_gets_runtime_expiry_date() {
        let store = todo_store_for_test("active_window");
        let parent = parent_todo("parent-1");
        let child = cyclic_subtodo("child-1", "parent-1");
        store.save(&[parent, child]).unwrap();

        let todos = store.get_todos(Some("2026-07-15")).unwrap();
        let child = todos.iter().find(|todo| todo.id == "child-1").unwrap();

        assert!(!child.disabled);
        assert!(!child.inactive_by_weekday);
        assert!(!child.completed);
        assert_eq!(child.expiry_date.as_deref(), Some("2026-07-20"));
    }

    #[test]
    fn cycle_subtodo_is_inactive_between_windows_without_persisting_disabled() {
        let store = todo_store_for_test("inactive_window");
        let parent = parent_todo("parent-1");
        let child = cyclic_subtodo("child-1", "parent-1");
        store.save(&[parent, child]).unwrap();

        let todos = store.get_todos(Some("2026-07-21")).unwrap();
        let child = todos.iter().find(|todo| todo.id == "child-1").unwrap();
        let saved = store.load().unwrap();
        let saved_child = saved.iter().find(|todo| todo.id == "child-1").unwrap();

        assert!(!child.disabled);
        assert!(child.inactive_by_weekday);
        assert!(child.completed);
        assert_eq!(child.expiry_date, None);
        assert!(!saved_child.disabled);
        assert!(!saved_child.completed);
    }

    #[test]
    fn cycle_subtodo_resets_when_a_new_active_cycle_starts() {
        let mut todos = vec![parent_todo("parent-1"), cyclic_subtodo("child-1", "parent-1")];
        todos[0].completed = true;
        todos[0].completed_at = Some(1_784_035_200);
        todos[1].completed = true;
        todos[1].completed_at = Some(1_784_035_200);
        todos[1].last_reset_date = Some("2026-07-14".to_string());

        let changed = TodoStore::apply_cycle_activation_for_date(&mut todos, "2026-08-11");
        let parent = todos.iter().find(|todo| todo.id == "parent-1").unwrap();
        let child = todos.iter().find(|todo| todo.id == "child-1").unwrap();

        assert!(changed);
        assert!(!parent.completed);
        assert_eq!(parent.completed_at, None);
        assert!(!child.completed);
        assert_eq!(child.completed_at, None);
        assert_eq!(child.last_reset_date.as_deref(), Some("2026-08-11"));
    }

    #[test]
    fn atomic_json_write_replaces_file_and_keeps_previous_backup() {
        let dir = std::env::temp_dir().join(format!("lighttodo_atomic_write_{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("todos.json");
        fs::write(&file_path, "[{\"old\":true}]").unwrap();

        write_json_atomically(&file_path, "[{\"new\":true}]").unwrap();

        assert_eq!(fs::read_to_string(&file_path).unwrap(), "[{\"new\":true}]");
        assert_eq!(fs::read_to_string(dir.join("todos.json.bak")).unwrap(), "[{\"old\":true}]");
        assert!(!dir.join("todos.json.tmp").exists());
    }
}
