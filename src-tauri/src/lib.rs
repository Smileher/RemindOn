use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, Monitor, PhysicalPosition, PhysicalSize, RunEvent,
    State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_notification::NotificationExt;

mod i18n;
#[cfg(target_os = "windows")]
mod notification;
mod updater;

const DATA_VERSION: u32 = 4;
const REST_ID: &str = "__rest__";
const TEST_REST_ID: &str = "__test_rest__";
const SHUTDOWN_ID: &str = "__shutdown__";
const TRAY_ID: &str = "main-tray";
const REMINDER_LABEL: &str = "reminder";
const WINDOWED_REMINDER_LABEL: &str = "reminder-windowed";
const REMINDER_MONITOR_PREFIX: &str = "reminder-monitor-";
const REMINDER_WINDOW_WIDTH: f64 = 520.0;
const REMINDER_WINDOW_HEIGHT: f64 = 320.0;
const MAIN_WINDOW_WIDTH: f64 = 780.0;
const MAIN_WINDOW_HEIGHT: f64 = 540.0;
const WINDOW_DESTROY_DELAY: StdDuration = StdDuration::from_secs(30);

fn default_rest_message() -> String {
    i18n::default_rest_message(Language::ZhCn).to_string()
}

fn default_shutdown_time() -> String {
    "23:30".to_string()
}

fn default_shutdown_message() -> String {
    i18n::default_power_message(Language::ZhCn, &PowerAction::Shutdown).to_string()
}

fn default_system_notification_enabled() -> bool {
    true
}

fn default_popup_fullscreen() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub language: Language,
    pub autostart: bool,
    pub minimize_to_tray: bool,
    pub popup_always_on_top: bool,
    #[serde(default = "default_popup_fullscreen")]
    pub popup_fullscreen: bool,
    pub rest_enabled: bool,
    pub rest_interval_minutes: u32,
    #[serde(default = "default_rest_message")]
    pub rest_message: String,
    #[serde(default = "default_system_notification_enabled")]
    pub system_notification_enabled: bool,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub accent_color: AccentColor,
    #[serde(default)]
    pub popup_background_fit: PopupBackgroundFit,
    #[serde(default)]
    pub popup_background_position: PopupBackgroundPosition,
    #[serde(default)]
    pub popup_text_color: String,
    #[serde(default = "default_popup_title_size")]
    pub popup_title_size: u32,
    #[serde(default = "default_popup_overlay_opacity")]
    pub popup_overlay_opacity: u32,
    #[serde(default)]
    pub shutdown_reminder_enabled: bool,
    #[serde(default)]
    pub power_action: PowerAction,
    #[serde(default = "default_shutdown_time")]
    pub shutdown_reminder_time: String,
    #[serde(default = "default_shutdown_message")]
    pub shutdown_reminder_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PowerAction {
    Shutdown,
    Lock,
    Restart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PopupBackgroundFit {
    Cover,
    Contain,
    Repeat,
    Stretch,
}

impl Default for PopupBackgroundFit {
    fn default() -> Self {
        Self::Cover
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PopupBackgroundPosition {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Default for PopupBackgroundPosition {
    fn default() -> Self {
        Self::Center
    }
}

fn default_popup_title_size() -> u32 {
    32
}

fn default_popup_overlay_opacity() -> u32 {
    55
}

impl Default for PowerAction {
    fn default() -> Self {
        Self::Shutdown
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en")]
    En,
}

impl Default for Language {
    fn default() -> Self {
        Self::ZhCn
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AccentColor {
    Mint,
    Blue,
    Violet,
    Amber,
}

impl Default for AccentColor {
    fn default() -> Self {
        Self::Mint
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::ZhCn,
            autostart: false,
            minimize_to_tray: false,
            popup_always_on_top: true,
            popup_fullscreen: true,
            rest_enabled: false,
            rest_interval_minutes: 45,
            rest_message: default_rest_message(),
            system_notification_enabled: true,
            theme: Theme::Dark,
            accent_color: AccentColor::Mint,
            popup_background_fit: PopupBackgroundFit::Cover,
            popup_background_position: PopupBackgroundPosition::Center,
            popup_text_color: String::new(),
            popup_title_size: default_popup_title_size(),
            popup_overlay_opacity: default_popup_overlay_opacity(),
            shutdown_reminder_enabled: false,
            power_action: PowerAction::Shutdown,
            shutdown_reminder_time: default_shutdown_time(),
            shutdown_reminder_message: default_shutdown_message(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReminderType {
    Once,
    Daily,
    Weekly,
    Monthly,
    Interval,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum TestReminderKind {
    Event,
    Rest,
    Power,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub reminder_type: ReminderType,
    pub trigger_at: Option<String>,
    pub time: Option<String>,
    #[serde(default)]
    pub weekdays: Vec<u32>,
    #[serde(default)]
    pub month_days: Vec<u32>,
    pub enabled: bool,
    #[serde(default)]
    pub next_trigger_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub version: u32,
    pub settings: AppSettings,
    pub reminders: Vec<Reminder>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            version: DATA_VERSION,
            settings: AppSettings::default(),
            reminders: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReminderTriggeredEvent {
    session_id: u64,
    id: String,
    title: String,
    #[serde(rename = "type")]
    reminder_type: ReminderType,
    is_rest: bool,
    is_shutdown: bool,
    power_action: Option<PowerAction>,
    is_test: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestTimerStatus {
    next_trigger_at: Option<String>,
    is_resting: bool,
}

struct InnerState {
    data: Mutex<AppData>,
    data_path: PathBuf,
    paused: AtomicBool,
    scheduler_started: AtomicBool,
    scheduler_stop: AtomicBool,
    next_reminder_session: AtomicU64,
    power_action_session: AtomicU64,
    active_reminder: Mutex<Option<ReminderTriggeredEvent>>,
    // 休息状态的读取和切换统一持有 data 锁，避免设置保存与弹窗状态交错。
    rest_active: AtomicBool,
    rest_round_pending: AtomicBool,
    rest_next: Mutex<Option<DateTime<Local>>>,
    shutdown_next: Mutex<Option<DateTime<Local>>>,
    pending_navigation: Mutex<Option<String>>,
    // All native window work is serialized off the UI thread, including destruction.
    window_operations: Mutex<()>,
    window_cache: Mutex<WindowCache>,
    reminder_targets: Mutex<HashSet<String>>,
    main_ready: AtomicBool,
    update_state: Mutex<updater::UpdateRuntimeState>,
    update_progress: Mutex<Option<updater::UpdateProgress>>,
}

#[derive(Clone)]
struct AppState(Arc<InnerState>);

fn data_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to resolve the app configuration directory: {error}"))?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Failed to create the app configuration directory: {error}"))?;
    Ok(directory.join("remindon.json"))
}

fn write_json(path: &Path, data: &AppData) -> Result<(), String> {
    let content = serde_json::to_string_pretty(data)
        .map_err(|error| format!("Failed to serialize settings: {error}"))?;
    // Windows cannot replace an existing file with std::fs::rename, so keep this
    // small local configuration write straightforward and portable.
    fs::write(path, content).map_err(|error| format!("Failed to save settings: {error}"))
}

fn corrupt_backup_path(path: &Path) -> PathBuf {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default();
    PathBuf::from(format!("{}.corrupt.{seconds}", path.display()))
}

fn load_json(path: &Path) -> Result<AppData, String> {
    if !path.exists() {
        return Ok(AppData::default());
    }
    let content =
        fs::read_to_string(path).map_err(|error| format!("Failed to read settings: {error}"))?;
    match serde_json::from_str::<AppData>(&content) {
        Ok(data) => Ok(data),
        Err(error) => {
            let backup = corrupt_backup_path(path);
            fs::rename(path, &backup).map_err(|rename_error| {
                format!(
                    "Settings are invalid ({error}) and the corrupted file could not be preserved: {rename_error}"
                )
            })?;
            Ok(AppData::default())
        }
    }
}

fn parse_datetime(value: &str) -> Result<DateTime<Local>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&Local))
        .map_err(|error| format!("Invalid date-time format: {error}"))
}

fn parse_time(value: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| "Reminder time must use HH:MM format".to_string())
}

fn local_datetime(date: NaiveDate, time: NaiveTime) -> Result<DateTime<Local>, String> {
    Local
        .from_local_datetime(&date.and_time(time))
        .earliest()
        .or_else(|| Local.from_local_datetime(&date.and_time(time)).latest())
        .ok_or_else(|| "Failed to resolve the local reminder time".to_string())
}

fn next_daily(time: &str, after: DateTime<Local>) -> Result<String, String> {
    let parsed_time = parse_time(time)?;
    let today = local_datetime(after.date_naive(), parsed_time)?;
    let candidate = if today > after {
        today
    } else {
        local_datetime(after.date_naive() + Duration::days(1), parsed_time)?
    };
    Ok(candidate.to_rfc3339())
}

fn next_recurring(reminder: &Reminder, after: DateTime<Local>) -> Result<String, String> {
    let time = reminder
        .time
        .as_deref()
        .ok_or_else(|| "Repeating reminder is missing time".to_string())?;
    let parsed_time = parse_time(time)?;

    for offset in 0..=370 {
        let date = after.date_naive() + Duration::days(offset);
        let weekday = date.weekday().number_from_monday();
        let matches = match reminder.reminder_type {
            ReminderType::Daily => true,
            ReminderType::Weekly => reminder.weekdays.contains(&weekday),
            ReminderType::Monthly => reminder.month_days.contains(&date.day()),
            ReminderType::Once | ReminderType::Interval => false,
        };
        if !matches {
            continue;
        }
        if let Ok(candidate) = local_datetime(date, parsed_time) {
            if candidate > after {
                return Ok(candidate.to_rfc3339());
            }
        }
    }
    Err("Failed to calculate the next reminder time".to_string())
}

fn should_trigger_power_action(due: DateTime<Local>, now: DateTime<Local>) -> bool {
    due <= now && now.signed_duration_since(due) <= Duration::minutes(1)
}

fn complete_rest_round(state: &AppState) -> bool {
    let data = state.0.data.lock().expect("settings lock poisoned");
    // 稍后提醒已关闭本次弹窗；重复关闭或关闭其他通知不能覆盖延后的时间。
    if !state.0.rest_active.swap(false, Ordering::SeqCst) {
        return false;
    }
    state.0.rest_round_pending.store(false, Ordering::SeqCst);
    if data.settings.rest_enabled {
        *state
            .0
            .rest_next
            .lock()
            .expect("break reminder lock poisoned") =
            Some(Local::now() + Duration::minutes(data.settings.rest_interval_minutes as i64));
    } else {
        *state
            .0
            .rest_next
            .lock()
            .expect("break reminder lock poisoned") = None;
    }
    true
}

fn snooze_rest_round(state: &AppState, seconds: u32) -> bool {
    let data = state.0.data.lock().expect("settings lock poisoned");
    if !state.0.rest_active.swap(false, Ordering::SeqCst) {
        return false;
    }
    state
        .0
        .rest_round_pending
        .store(data.settings.rest_enabled, Ordering::SeqCst);
    *state
        .0
        .rest_next
        .lock()
        .expect("break reminder lock poisoned") = data
        .settings
        .rest_enabled
        .then(|| Local::now() + Duration::seconds(seconds.max(1) as i64));
    true
}

fn validate_and_normalize(data: &mut AppData) -> Result<(), String> {
    data.version = DATA_VERSION;
    data.settings.rest_interval_minutes = data.settings.rest_interval_minutes.clamp(1, 1440);
    if data.settings.rest_message.trim().is_empty() {
        data.settings.rest_message = i18n::default_rest_message(data.settings.language).to_string();
    }
    if data.settings.shutdown_reminder_message.trim().is_empty() {
        data.settings.shutdown_reminder_message =
            i18n::default_power_message(data.settings.language, &data.settings.power_action)
                .to_string();
    }
    parse_time(&data.settings.shutdown_reminder_time)?;
    let now = Local::now();
    for reminder in &mut data.reminders {
        if reminder.id.trim().is_empty() {
            return Err("Reminder is missing id".to_string());
        }
        if reminder.title.trim().is_empty() {
            return Err("Reminder text cannot be empty".to_string());
        }
        match reminder.reminder_type {
            ReminderType::Once => {
                let trigger = reminder
                    .trigger_at
                    .as_deref()
                    .ok_or_else(|| "One-time reminder is missing triggerAt".to_string())?;
                parse_datetime(trigger)?;
                reminder.next_trigger_at = Some(trigger.to_string());
            }
            ReminderType::Daily | ReminderType::Weekly | ReminderType::Monthly => {
                let time = reminder
                    .time
                    .as_deref()
                    .ok_or_else(|| "Repeating reminder is missing time".to_string())?;
                parse_time(time)?;
                reminder.weekdays.sort_unstable();
                reminder.weekdays.dedup();
                reminder.month_days.sort_unstable();
                reminder.month_days.dedup();
                if reminder.reminder_type == ReminderType::Weekly
                    && (reminder.weekdays.is_empty()
                        || reminder.weekdays.iter().any(|day| !(1..=7).contains(day)))
                {
                    return Err("Weekly reminder must include at least one weekday".to_string());
                }
                if reminder.reminder_type == ReminderType::Monthly
                    && (reminder.month_days.is_empty()
                        || reminder
                            .month_days
                            .iter()
                            .any(|day| !(1..=31).contains(day)))
                {
                    return Err("Monthly reminder must include at least one date".to_string());
                }
                if reminder.next_trigger_at.is_none() {
                    reminder.next_trigger_at = Some(next_recurring(reminder, now)?);
                } else if let Some(next) = reminder.next_trigger_at.as_deref() {
                    parse_datetime(next)?;
                }
            }
            ReminderType::Interval => {
                reminder.next_trigger_at = None;
            }
        }
    }
    Ok(())
}

fn app_data(state: &AppState) -> AppData {
    state.0.data.lock().expect("settings lock poisoned").clone()
}

fn rest_timer_status(state: &AppState) -> RestTimerStatus {
    let data = state.0.data.lock().expect("settings lock poisoned");
    if state.0.rest_active.load(Ordering::SeqCst) {
        return RestTimerStatus {
            next_trigger_at: None,
            is_resting: true,
        };
    }
    if !data.settings.rest_enabled {
        return RestTimerStatus {
            next_trigger_at: None,
            is_resting: false,
        };
    }
    let mut next = state
        .0
        .rest_next
        .lock()
        .expect("break reminder lock poisoned");
    if next.is_none() {
        *next = Some(Local::now() + Duration::minutes(data.settings.rest_interval_minutes as i64));
    }
    RestTimerStatus {
        next_trigger_at: next.as_ref().map(DateTime::to_rfc3339),
        is_resting: false,
    }
}

fn emit_rest_timer_updated(app: &AppHandle, state: &AppState) {
    let _ = app.emit_to("main", "rest-timer-updated", rest_timer_status(state));
}

fn is_reminder_window_label(label: &str) -> bool {
    label == REMINDER_LABEL
        || label == WINDOWED_REMINDER_LABEL
        || label.starts_with(REMINDER_MONITOR_PREFIX)
}

fn reminder_windows(app: &AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows()
        .into_values()
        .filter(|window| is_reminder_window_label(window.label()))
        .collect()
}

fn emit_to_reminder_windows<S: Clone + Serialize>(app: &AppHandle, event: &str, payload: S) {
    for window in reminder_windows(app) {
        let _ = app.emit_to(window.label(), event, payload.clone());
    }
}

#[derive(Default)]
struct WindowCache {
    idle: HashMap<String, Instant>,
    destroying: HashSet<String>,
}

impl WindowCache {
    fn hide(&mut self, label: &str, now: Instant) {
        // Repeated close events must not extend the same idle period.
        self.idle
            .entry(label.to_string())
            .or_insert(now + WINDOW_DESTROY_DELAY);
    }

    fn reuse(&mut self, label: &str) {
        self.idle.remove(label);
    }

    fn expired(&self, now: Instant) -> Vec<String> {
        self.idle
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(label, _)| label.clone())
            .collect()
    }
}

// Internal helpers require window_operations; native UI callbacks must enqueue work.
fn hide_cached_window(window: &WebviewWindow, state: &AppState) -> Result<(), String> {
    window.hide().map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    if is_reminder_window_label(window.label()) {
        let _ = window.set_simple_fullscreen(false);
        window.hide().map_err(|error| error.to_string())?;
    }
    state
        .0
        .window_cache
        .lock()
        .expect("window cache lock poisoned")
        .hide(window.label(), Instant::now());
    Ok(())
}

fn hide_reminder_windows(app: &AppHandle, state: &AppState) {
    state
        .0
        .reminder_targets
        .lock()
        .expect("reminder targets lock poisoned")
        .clear();
    for window in reminder_windows(app) {
        if let Err(error) = hide_cached_window(&window, state) {
            eprintln!("Failed to hide {}: {error}", window.label());
        }
    }
}

fn wait_for_window_destroyed(app: &AppHandle, state: &AppState, label: &str) -> Result<(), String> {
    if !state
        .0
        .window_cache
        .lock()
        .expect("window cache lock poisoned")
        .destroying
        .contains(label)
    {
        return Ok(());
    }
    // destroy() only posts a message. Reuse the label after the manager handles Destroyed.
    // Waiting is safe only on a background thread while the native event loop keeps running.
    let deadline = Instant::now() + StdDuration::from_secs(2);
    while app.get_webview_window(label).is_some() {
        if Instant::now() >= deadline {
            return Err(format!("Window {label} is still being destroyed"));
        }
        thread::sleep(StdDuration::from_millis(10));
    }
    state
        .0
        .window_cache
        .lock()
        .expect("window cache lock poisoned")
        .destroying
        .remove(label);
    Ok(())
}

fn reclaim_idle_windows(app: &AppHandle, state: &AppState) {
    let labels = state
        .0
        .window_cache
        .lock()
        .expect("window cache lock poisoned")
        .expired(Instant::now());
    for label in labels {
        let Some(window) = app.get_webview_window(&label) else {
            let mut cache = state
                .0
                .window_cache
                .lock()
                .expect("window cache lock poisoned");
            cache.idle.remove(&label);
            cache.destroying.remove(&label);
            continue;
        };
        if !matches!(window.is_visible(), Ok(false)) {
            continue;
        }
        if state
            .0
            .reminder_targets
            .lock()
            .expect("reminder targets lock poisoned")
            .contains(&label)
        {
            continue;
        }
        state
            .0
            .window_cache
            .lock()
            .expect("window cache lock poisoned")
            .destroying
            .insert(label.clone());
        match window.destroy() {
            Ok(()) => {
                state
                    .0
                    .window_cache
                    .lock()
                    .expect("window cache lock poisoned")
                    .idle
                    .remove(&label);
                if label == "main" {
                    state.0.main_ready.store(false, Ordering::SeqCst);
                }
                if let Err(error) = wait_for_window_destroyed(app, state, &label) {
                    eprintln!("{error}");
                }
            }
            Err(error) => {
                state
                    .0
                    .window_cache
                    .lock()
                    .expect("window cache lock poisoned")
                    .destroying
                    .remove(&label);
                eprintln!("Failed to destroy {label}: {error}");
            }
        }
    }
}

fn queue_window_action<F>(app: &AppHandle, action: F)
where
    F: FnOnce(&AppHandle, &AppState) -> Result<(), String> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state
            .0
            .window_operations
            .lock()
            .expect("window operations lock poisoned");
        if let Err(error) = action(&app, state.inner()) {
            eprintln!("Window operation failed: {error}");
        }
    });
}

fn close_reminder_session(app: &AppHandle, state: &AppState, session_id: u64) -> bool {
    let mut active = state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned");
    if active.as_ref().map(|event| event.session_id) != Some(session_id) {
        return false;
    }
    *active = None;
    drop(active);
    let _ = state.0.power_action_session.compare_exchange(
        session_id,
        0,
        Ordering::SeqCst,
        Ordering::SeqCst,
    );
    emit_to_reminder_windows(app, "reminder-closed", session_id);
    hide_reminder_windows(app, state);
    true
}

fn reset_reminder_session(app: &AppHandle, state: &AppState, event: &str) {
    *state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned") = None;
    state.0.power_action_session.store(0, Ordering::SeqCst);
    emit_to_reminder_windows(app, event, ());
    hide_reminder_windows(app, state);
}

fn cancel_active_rest_reminder(app: &AppHandle, state: &AppState) {
    let mut active = state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned");
    if !active.as_ref().is_some_and(|event| event.is_rest) {
        return;
    }
    *active = None;
    drop(active);
    emit_to_reminder_windows(app, "rest-cancelled", ());
    hide_reminder_windows(app, state);
}

fn active_reminder_matches(state: &AppState, id: &str, session_id: u64) -> bool {
    state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned")
        .as_ref()
        .is_some_and(|event| event.session_id == session_id && event.id == id)
}

fn activate_reminder_session(app: &AppHandle, state: &AppState, event: ReminderTriggeredEvent) {
    state.0.power_action_session.store(0, Ordering::SeqCst);
    let previous = state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned")
        .replace(event);
    if let Some(previous) = previous {
        emit_to_reminder_windows(app, "reminder-closed", previous.session_id);
        hide_reminder_windows(app, state);
    }
}

fn same_monitor(left: &Monitor, right: &Monitor) -> bool {
    left.position() == right.position() && left.size() == right.size()
}

fn ordered_monitors(app: &AppHandle) -> Vec<Monitor> {
    let Ok(mut monitors) = app.available_monitors() else {
        return Vec::new();
    };
    if let Ok(Some(primary)) = app.primary_monitor() {
        if let Some(index) = monitors
            .iter()
            .position(|monitor| same_monitor(monitor, &primary))
        {
            monitors.swap(0, index);
        }
    }
    monitors
}

fn configure_windowed_reminder(window: &WebviewWindow, settings: &AppSettings) {
    let _ = window.hide();
    let _ = window.set_title(i18n::notification_window_title(settings.language));
    let _ = window.set_always_on_top(settings.popup_always_on_top);
    let _ = window.set_decorations(true);
    let _ = window.set_resizable(false);
    let _ = window.set_maximizable(false);
    let _ = window.set_skip_taskbar(false);
    let _ = window.set_size(LogicalSize::new(
        REMINDER_WINDOW_WIDTH,
        REMINDER_WINDOW_HEIGHT,
    ));
    let _ = window.center();
    let _ = window.hide();
}

fn configure_fullscreen_reminder(
    window: &WebviewWindow,
    monitor: &Monitor,
    settings: &AppSettings,
    is_controller: bool,
) {
    #[cfg(not(target_os = "macos"))]
    let reuse_native_fullscreen = window.is_fullscreen().unwrap_or(false)
        && window
            .current_monitor()
            .ok()
            .flatten()
            .as_ref()
            .is_some_and(|current| same_monitor(current, monitor));
    #[cfg(not(target_os = "macos"))]
    if !reuse_native_fullscreen {
        let _ = window.hide();
        let _ = window.set_fullscreen(false);
        let _ = window.hide();
    }
    let _ = window.set_title(i18n::notification_window_title(settings.language));
    let _ = window.set_always_on_top(settings.popup_always_on_top);
    let _ = window.set_decorations(false);
    let _ = window.set_resizable(false);
    let _ = window.set_maximizable(false);
    let _ = window.set_skip_taskbar(true);

    #[cfg(target_os = "macos")]
    {
        let _ = window.set_position(PhysicalPosition::new(
            monitor.position().x,
            monitor.position().y,
        ));
        if is_controller {
            let _ = window.set_simple_fullscreen(true);
            let _ = window.hide();
            return;
        }
        let _ = window.set_size(PhysicalSize::new(
            monitor.size().width,
            monitor.size().height,
        ));
        let _ = window.hide();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = is_controller;
        if !reuse_native_fullscreen {
            let _ = window.set_position(PhysicalPosition::new(
                monitor.position().x,
                monitor.position().y,
            ));
            let _ = window.set_size(PhysicalSize::new(
                monitor.size().width,
                monitor.size().height,
            ));
            let _ = window.set_fullscreen(true);
        }
        // 原生全屏切换可能自行显示窗口，内容准备完成前必须保持隐藏。
        let _ = window.hide();
    }
}

fn create_reminder_window(
    app: &AppHandle,
    label: String,
    settings: AppSettings,
) -> Result<WebviewWindow, String> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html#/reminder".into()))
        .title(i18n::notification_window_title(settings.language))
        .visible(false)
        .decorations(false)
        .resizable(false)
        .maximizable(false)
        .skip_taskbar(true)
        .always_on_top(settings.popup_always_on_top)
        .build()
        .map_err(|error| format!("Failed to create reminder window: {error}"))
}

fn prepare_reminder_windows(
    app: &AppHandle,
    state: &AppState,
    settings: &AppSettings,
) -> Result<Vec<String>, String> {
    let monitors = if settings.popup_fullscreen {
        ordered_monitors(app)
    } else {
        Vec::new()
    };
    // Keep native fullscreen and windowed instances separate: exiting fullscreen
    // restores saved window placement and can briefly show the old fullscreen surface.
    let labels: Vec<String> = (0..monitors.len().max(1))
        .map(|index| {
            if monitors.is_empty() {
                WINDOWED_REMINDER_LABEL.to_string()
            } else if index == 0 {
                REMINDER_LABEL.to_string()
            } else {
                format!("{REMINDER_MONITOR_PREFIX}{index}")
            }
        })
        .collect();
    for window in reminder_windows(app) {
        if !labels.iter().any(|label| label == window.label()) {
            hide_cached_window(&window, state)?;
        }
    }
    for (index, label) in labels.iter().enumerate() {
        wait_for_window_destroyed(app, state, label)?;
        let window = match app.get_webview_window(label) {
            Some(window) => window,
            None => create_reminder_window(app, label.clone(), settings.clone())?,
        };
        state
            .0
            .window_cache
            .lock()
            .expect("window cache lock poisoned")
            .reuse(label);
        if let Some(monitor) = monitors.get(index) {
            configure_fullscreen_reminder(&window, monitor, settings, index == 0);
        } else {
            configure_windowed_reminder(&window, settings);
        }
    }
    *state
        .0
        .reminder_targets
        .lock()
        .expect("reminder targets lock poisoned") = labels.iter().cloned().collect();
    Ok(labels)
}

fn sync_reminder_settings(app: &AppHandle, settings: &AppSettings) {
    for window in reminder_windows(app) {
        let _ = window.set_title(i18n::notification_window_title(settings.language));
        let _ = window.set_always_on_top(settings.popup_always_on_top);
        let _ = app.emit_to(window.label(), "settings-updated", settings);
    }
}

fn prepare_notification(state: &AppState, event: &ReminderTriggeredEvent) -> Option<AppSettings> {
    let data = state.0.data.lock().expect("settings lock poisoned");
    let settings = &data.settings;
    // 到期事件生成后若用户关闭了休息提醒，不再重新激活已取消的弹窗。
    if event.is_rest && !event.is_test && !settings.rest_enabled {
        return None;
    }

    // 测试和定时休息弹窗使用相同的暂停及完成逻辑。
    if event.is_rest {
        state.0.rest_active.store(true, Ordering::SeqCst);
        *state
            .0
            .rest_next
            .lock()
            .expect("break reminder lock poisoned") = None;
        state.0.rest_round_pending.store(true, Ordering::SeqCst);
    } else if state.0.rest_active.swap(false, Ordering::SeqCst) {
        // 共用窗口替换了休息通知，相当于关闭该休息；已稍后提醒的轮次不受影响。
        state.0.rest_round_pending.store(false, Ordering::SeqCst);
        *state
            .0
            .rest_next
            .lock()
            .expect("break reminder lock poisoned") = settings
            .rest_enabled
            .then(|| Local::now() + Duration::minutes(settings.rest_interval_minutes as i64));
    }
    Some(settings.clone())
}

fn dispatch_trigger(
    app: &AppHandle,
    state: &AppState,
    mut event: ReminderTriggeredEvent,
) -> Result<(), String> {
    let Some(settings) = prepare_notification(state, &event) else {
        return Ok(());
    };
    emit_rest_timer_updated(app, state);
    event.session_id = state.0.next_reminder_session.fetch_add(1, Ordering::SeqCst) + 1;
    activate_reminder_session(app, state, event.clone());
    let labels = match prepare_reminder_windows(app, state, &settings) {
        Ok(labels) => labels,
        Err(error) => {
            if event.is_rest && complete_rest_round(state) {
                emit_rest_timer_updated(app, state);
            }
            close_reminder_session(app, state, event.session_id);
            return Err(error);
        }
    };
    for label in labels {
        if app.get_webview_window(&label).is_some() {
            let _ = app.emit_to(&label, "reminder-triggered", event.clone());
        }
    }

    // 主窗口必须先收到事件，即使可选的系统通知发送失败，也不能留下过期倒计时。
    let _ = app.emit_to("main", "reminder-triggered", event.clone());

    if settings.system_notification_enabled {
        let title = i18n::notification_title(settings.language, &event);
        #[cfg(target_os = "windows")]
        let result = if option_env!("REMINDON_STORE_BUILD").is_some() {
            notification::show_packaged(title, &event.title)
        } else {
            app.notification()
                .builder()
                .title(title)
                .body(&event.title)
                .show()
                .map_err(|error| error.to_string())
        };
        #[cfg(not(target_os = "windows"))]
        let result = app
            .notification()
            .builder()
            .title(title)
            .body(&event.title)
            .show()
            .map_err(|error| error.to_string());
        result.map_err(|error| i18n::notification_send_failed(settings.language, &error))?;
    }

    Ok(())
}

fn process_due(app: &AppHandle, state: &AppState) {
    let now = Local::now();
    let mut triggered = Vec::new();
    let mut changed = false;
    {
        let mut data = state.0.data.lock().expect("settings lock poisoned");
        for reminder in &mut data.reminders {
            if !reminder.enabled {
                continue;
            }
            let Some(next_value) = reminder.next_trigger_at.as_deref() else {
                continue;
            };
            let Ok(next) = parse_datetime(next_value) else {
                continue;
            };
            if next > now {
                continue;
            }
            triggered.push(ReminderTriggeredEvent {
                session_id: 0,
                id: reminder.id.clone(),
                title: reminder.title.clone(),
                reminder_type: reminder.reminder_type.clone(),
                is_rest: false,
                is_shutdown: false,
                power_action: None,
                is_test: false,
            });
            changed = true;
            match reminder.reminder_type {
                ReminderType::Once => {
                    reminder.enabled = false;
                    reminder.next_trigger_at = None;
                }
                ReminderType::Daily | ReminderType::Weekly | ReminderType::Monthly => {
                    reminder.next_trigger_at = next_recurring(reminder, now).ok();
                }
                ReminderType::Interval => {}
            }
        }
        if data.settings.rest_enabled {
            if !state.0.rest_active.load(Ordering::SeqCst) {
                let due = {
                    let mut next_rest = state
                        .0
                        .rest_next
                        .lock()
                        .expect("break reminder lock poisoned");
                    if next_rest.is_none() {
                        *next_rest = Some(
                            now + Duration::minutes(data.settings.rest_interval_minutes as i64),
                        );
                        None
                    } else if next_rest.is_some_and(|value| value <= now) {
                        next_rest.take()
                    } else {
                        None
                    }
                };
                if due.is_some() {
                    triggered.push(ReminderTriggeredEvent {
                        session_id: 0,
                        id: REST_ID.to_string(),
                        title: data.settings.rest_message.clone(),
                        reminder_type: ReminderType::Interval,
                        is_rest: true,
                        is_shutdown: false,
                        power_action: None,
                        is_test: false,
                    });
                    *state
                        .0
                        .rest_next
                        .lock()
                        .expect("break reminder lock poisoned") = None;
                    state.0.rest_round_pending.store(true, Ordering::SeqCst);
                    state.0.rest_active.store(true, Ordering::SeqCst);
                }
            }
        }
        if data.settings.shutdown_reminder_enabled {
            let mut next_shutdown = state
                .0
                .shutdown_next
                .lock()
                .expect("scheduled action lock poisoned");
            if next_shutdown.is_none() {
                *next_shutdown = next_daily(&data.settings.shutdown_reminder_time, now)
                    .ok()
                    .and_then(|value| parse_datetime(&value).ok());
            } else if let Some(due) = next_shutdown.as_ref().filter(|value| **value <= now) {
                if should_trigger_power_action(due.clone(), now) {
                    triggered.push(ReminderTriggeredEvent {
                        session_id: 0,
                        id: SHUTDOWN_ID.to_string(),
                        title: data.settings.shutdown_reminder_message.clone(),
                        reminder_type: ReminderType::Daily,
                        is_rest: false,
                        is_shutdown: true,
                        power_action: Some(data.settings.power_action.clone()),
                        is_test: false,
                    });
                }
                *next_shutdown = next_daily(&data.settings.shutdown_reminder_time, now)
                    .ok()
                    .and_then(|value| parse_datetime(&value).ok());
            }
        } else {
            *state
                .0
                .shutdown_next
                .lock()
                .expect("scheduled action lock poisoned") = None;
        }
        if changed {
            let _ = write_json(&state.0.data_path, &data);
        }
    }
    for event in triggered {
        if let Err(error) = dispatch_trigger(app, state, event) {
            let _ = app.emit_to("main", "notification-failed", error);
        }
    }
}

fn spawn_scheduler(app: AppHandle, state: AppState) {
    if state.0.scheduler_started.swap(true, Ordering::SeqCst) {
        return;
    }
    state.0.scheduler_stop.store(false, Ordering::SeqCst);
    thread::spawn(move || {
        while !state.0.scheduler_stop.load(Ordering::SeqCst) {
            {
                let _operation = state
                    .0
                    .window_operations
                    .lock()
                    .expect("window operations lock poisoned");
                if !state.0.paused.load(Ordering::SeqCst) {
                    process_due(&app, &state);
                }
                reclaim_idle_windows(&app, &state);
            }
            thread::sleep(StdDuration::from_secs(1));
        }
        state.0.scheduler_started.store(false, Ordering::SeqCst);
    });
}

#[tauri::command]
fn load_data(state: State<'_, AppState>) -> Result<AppData, String> {
    Ok(app_data(&state))
}

#[tauri::command]
async fn save_data(
    mut data: AppData,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppData, String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    validate_and_normalize(&mut data)?;
    let mut current = state.0.data.lock().expect("settings lock poisoned");
    let rest_enabled_changed = current.settings.rest_enabled != data.settings.rest_enabled;
    let rest_interval_changed =
        current.settings.rest_interval_minutes != data.settings.rest_interval_minutes;
    let shutdown_schedule_changed = current.settings.shutdown_reminder_enabled
        != data.settings.shutdown_reminder_enabled
        || current.settings.shutdown_reminder_time != data.settings.shutdown_reminder_time;
    write_json(&state.0.data_path, &data)?;
    *current = data.clone();
    if rest_enabled_changed && !data.settings.rest_enabled {
        state.0.rest_active.store(false, Ordering::SeqCst);
        state.0.rest_round_pending.store(false, Ordering::SeqCst);
        *state
            .0
            .rest_next
            .lock()
            .expect("break reminder lock poisoned") = None;
    } else if (rest_enabled_changed || rest_interval_changed)
        && !state.0.rest_round_pending.load(Ordering::SeqCst)
    {
        *state
            .0
            .rest_next
            .lock()
            .expect("break reminder lock poisoned") = None;
    }
    if shutdown_schedule_changed {
        *state
            .0
            .shutdown_next
            .lock()
            .expect("scheduled action lock poisoned") = None;
    }
    drop(current);
    let _ = update_tray_menu(
        &app,
        data.settings.language,
        state.0.paused.load(Ordering::SeqCst),
    );
    #[cfg(target_os = "macos")]
    apply_application_menu(&app, data.settings.language);
    sync_reminder_settings(&app, &data.settings);
    if rest_enabled_changed && !data.settings.rest_enabled {
        cancel_active_rest_reminder(&app, state.inner());
    }
    if rest_enabled_changed || rest_interval_changed {
        emit_rest_timer_updated(&app, &state);
    }
    Ok(data)
}

#[tauri::command(rename = "start_scheduler")]
fn start_scheduler_command(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    spawn_scheduler(app, state.inner().clone());
    Ok(())
}

#[tauri::command(rename = "stop_scheduler")]
fn stop_scheduler_command(state: State<'_, AppState>) -> Result<(), String> {
    state.0.scheduler_stop.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn set_scheduler_paused(paused: bool, state: State<'_, AppState>) -> Result<(), String> {
    state.0.paused.store(paused, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn snooze_reminder(
    id: String,
    seconds: u32,
    session_id: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    if !active_reminder_matches(state.inner(), &id, session_id) {
        return Ok(());
    }
    if id == REST_ID || id == TEST_REST_ID {
        if snooze_rest_round(state.inner(), seconds) {
            emit_rest_timer_updated(&app, &state);
        }
        close_reminder_session(&app, state.inner(), session_id);
        return Ok(());
    }
    let mut data = state.0.data.lock().expect("settings lock poisoned");
    let delay = Duration::seconds(seconds.max(1) as i64);
    if id == SHUTDOWN_ID {
        *state
            .0
            .shutdown_next
            .lock()
            .expect("scheduled action lock poisoned") = Some(Local::now() + delay);
    } else if let Some(reminder) = data.reminders.iter_mut().find(|item| item.id == id) {
        reminder.enabled = true;
        reminder.next_trigger_at = Some((Local::now() + delay).to_rfc3339());
        write_json(&state.0.data_path, &data)?;
    }
    drop(data);
    close_reminder_session(&app, state.inner(), session_id);
    Ok(())
}

#[tauri::command]
async fn dismiss_reminder(
    id: String,
    session_id: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    if !active_reminder_matches(state.inner(), &id, session_id) {
        return Ok(());
    }
    if id == REST_ID || id == TEST_REST_ID {
        if complete_rest_round(state.inner()) {
            emit_rest_timer_updated(&app, state.inner());
        }
    }
    close_reminder_session(&app, state.inner(), session_id);
    Ok(())
}

#[tauri::command]
async fn get_active_reminder(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<ReminderTriggeredEvent>, String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    if !state
        .0
        .reminder_targets
        .lock()
        .expect("reminder targets lock poisoned")
        .contains(window.label())
    {
        return Ok(None);
    }
    Ok(state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned")
        .clone())
}

#[tauri::command]
async fn show_reminder(
    window: WebviewWindow,
    session_id: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    let active = state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned")
        .clone();
    let current = active
        .as_ref()
        .is_some_and(|event| event.session_id == session_id);
    if !current
        || !state
            .0
            .reminder_targets
            .lock()
            .expect("reminder targets lock poisoned")
            .contains(window.label())
    {
        return Ok(false);
    }
    // Restore only after content and geometry are ready; unminimize itself may show a window.
    if let Err(error) = window.unminimize().and_then(|_| window.show()) {
        if active.as_ref().is_some_and(|event| event.is_rest) && complete_rest_round(state.inner())
        {
            emit_rest_timer_updated(&app, state.inner());
        }
        close_reminder_session(&app, state.inner(), session_id);
        let message = error.to_string();
        let _ = app.emit_to("main", "notification-failed", &message);
        return Err(message);
    }
    if window.label() == REMINDER_LABEL || window.label() == WINDOWED_REMINDER_LABEL {
        window.set_focus().map_err(|error| error.to_string())?;
    }
    Ok(true)
}

#[tauri::command]
async fn hide_idle_window(window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    if window.label() == "main"
        || (is_reminder_window_label(window.label())
            && !state
                .0
                .reminder_targets
                .lock()
                .expect("reminder targets lock poisoned")
                .contains(window.label()))
    {
        hide_cached_window(&window, state.inner())?;
    }
    Ok(())
}

#[tauri::command]
fn get_rest_timer_status(state: State<'_, AppState>) -> RestTimerStatus {
    rest_timer_status(state.inner())
}

#[tauri::command]
fn get_next_shutdown_trigger(state: State<'_, AppState>) -> Option<String> {
    let data = state.0.data.lock().expect("settings lock poisoned");
    if !data.settings.shutdown_reminder_enabled {
        return None;
    }
    let mut next = state
        .0
        .shutdown_next
        .lock()
        .expect("scheduled action lock poisoned");
    if next.is_none() {
        *next = next_daily(&data.settings.shutdown_reminder_time, Local::now())
            .ok()
            .and_then(|value| parse_datetime(&value).ok());
    }
    next.as_ref().map(DateTime::to_rfc3339)
}

#[cfg(target_os = "windows")]
fn windows_power_command(action: &PowerAction, system_root: &Path) -> (PathBuf, Vec<&'static str>) {
    let system32 = system_root.join("System32");
    match action {
        PowerAction::Lock => (
            system32.join("rundll32.exe"),
            vec!["user32.dll,LockWorkStation"],
        ),
        PowerAction::Shutdown => (system32.join("shutdown.exe"), vec!["/s", "/t", "0"]),
        PowerAction::Restart => (system32.join("shutdown.exe"), vec!["/r", "/t", "0"]),
    }
}

fn execute_power_action_impl(action: &PowerAction) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let system_root = std::env::var_os("SystemRoot")
            .ok_or_else(|| "Failed to resolve the Windows system directory".to_string())?;
        let (program, args) = windows_power_command(action, &PathBuf::from(system_root));
        Command::new(program)
            .args(args)
            .spawn()
            .map_err(|error| format!("Failed to execute the system action: {error}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        if *action == PowerAction::Lock {
            Command::new(
                "/System/Library/CoreServices/Menu Extras/User.menu/Contents/Resources/CGSession",
            )
            .arg("-suspend")
            .spawn()
            .map_err(|error| format!("Failed to lock the computer: {error}"))?;
            return Ok(());
        }
        let script = if *action == PowerAction::Shutdown {
            "tell application \"System Events\" to shut down"
        } else {
            "tell application \"System Events\" to restart"
        };
        Command::new("/usr/bin/osascript")
            .args(["-e", script])
            .spawn()
            .map_err(|error| format!("Failed to execute the system action: {error}"))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("The current platform does not support this scheduled action".to_string())
}

#[tauri::command]
async fn execute_power_action(
    action: PowerAction,
    session_id: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    let matches = state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned")
        .as_ref()
        .is_some_and(|event| {
            event.session_id == session_id && event.power_action.as_ref() == Some(&action)
        });
    if !matches {
        return Ok(false);
    }
    if state
        .0
        .power_action_session
        .compare_exchange(0, session_id, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(false);
    }
    if let Err(error) = execute_power_action_impl(&action) {
        let _ = state.0.power_action_session.compare_exchange(
            session_id,
            0,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        return Err(error);
    }
    close_reminder_session(&app, state.inner(), session_id);
    Ok(true)
}

fn test_reminder_event(settings: &AppSettings, kind: TestReminderKind) -> ReminderTriggeredEvent {
    match kind {
        TestReminderKind::Event => ReminderTriggeredEvent {
            session_id: 0,
            id: "__test_event__".to_string(),
            title: i18n::test_notification(settings.language).to_string(),
            reminder_type: ReminderType::Once,
            is_rest: false,
            is_shutdown: false,
            power_action: None,
            is_test: true,
        },
        TestReminderKind::Rest => ReminderTriggeredEvent {
            session_id: 0,
            id: TEST_REST_ID.to_string(),
            title: settings.rest_message.clone(),
            reminder_type: ReminderType::Interval,
            is_rest: true,
            is_shutdown: false,
            power_action: None,
            is_test: true,
        },
        TestReminderKind::Power => ReminderTriggeredEvent {
            session_id: 0,
            id: "__test_power__".to_string(),
            title: settings.shutdown_reminder_message.clone(),
            reminder_type: ReminderType::Daily,
            is_rest: false,
            is_shutdown: true,
            power_action: Some(settings.power_action.clone()),
            is_test: true,
        },
    }
}

#[tauri::command]
async fn test_reminder(
    kind: TestReminderKind,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    let event = test_reminder_event(&app_data(&state).settings, kind);
    // 弹窗窗口会复用，触发前必须把最新设置同步过去，否则外观类设置不会生效。
    let settings = app_data(&state).settings;
    sync_reminder_settings(&app, &settings);
    dispatch_trigger(&app, &state, event)
}

#[tauri::command]
async fn import_data(
    path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppData, String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read the import file: {error}"))?;
    let mut data: AppData =
        serde_json::from_str(&content).map_err(|error| format!("Invalid import file: {error}"))?;
    validate_and_normalize(&mut data)?;
    let mut current = state.0.data.lock().expect("settings lock poisoned");
    write_json(&state.0.data_path, &data)?;
    *current = data.clone();
    state.0.rest_active.store(false, Ordering::SeqCst);
    state.0.rest_round_pending.store(false, Ordering::SeqCst);
    *state
        .0
        .rest_next
        .lock()
        .expect("break reminder lock poisoned") = None;
    *state
        .0
        .shutdown_next
        .lock()
        .expect("scheduled action lock poisoned") = None;
    state.0.paused.store(false, Ordering::SeqCst);
    drop(current);
    reset_reminder_session(&app, state.inner(), "reminders-reset");
    let _ = update_tray_menu(&app, data.settings.language, false);
    #[cfg(target_os = "macos")]
    apply_application_menu(&app, data.settings.language);
    sync_reminder_settings(&app, &data.settings);
    emit_rest_timer_updated(&app, &state);
    Ok(data)
}

#[tauri::command]
fn export_data(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let data = app_data(&state);
    let content = serde_json::to_string_pretty(&data)
        .map_err(|error| format!("Failed to serialize the export: {error}"))?;
    fs::write(path, content).map_err(|error| format!("Failed to write the export file: {error}"))
}

/// Copy a user-picked image into the configuration folder and return it as a data URL.
/// The file lives next to `remindon.json`, so development and installed builds share one copy.
#[tauri::command]
fn import_popup_image(source: String, app: AppHandle) -> Result<String, String> {
    let origin = PathBuf::from(&source);
    if !origin.is_file() {
        return Err("The selected image does not exist".to_string());
    }
    let mime = extension_mime(&image_extension(&origin)?);
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to resolve the app configuration directory: {error}"))?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Failed to create the app configuration directory: {error}"))?;
    // 固定文件名，用户选什么图都存成同一个名字，配置里不需要记录来源。
    let target = directory.join("popup-background.img");
    fs::copy(&origin, &target).map_err(|error| format!("Failed to copy the image: {error}"))?;
    let bytes = fs::read(&target).map_err(|error| format!("Failed to read the image: {error}"))?;
    Ok(format!("data:{mime};base64,{}", base64_encode(&bytes)))
}

fn image_extension(path: &Path) -> Result<String, String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| {
            matches!(
                value.as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "avif"
            )
        })
        .ok_or_else(|| "Unsupported image format".to_string())
}

fn extension_mime(extension: &str) -> &'static str {
    match extension {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "avif" => "image/avif",
        _ => "image/png",
    }
}

/// Read the stored background image back as a data URL. An absent file simply means no background.
#[tauri::command]
fn read_popup_image(app: AppHandle) -> Result<Option<String>, String> {
    let directory = match app.path().app_config_dir() {
        Ok(directory) => directory,
        Err(_) => return Ok(None),
    };
    // 固定文件名，找不到就代表用户还没设置背景。
    let target = directory.join("popup-background.img");
    if !target.is_file() {
        return Ok(None);
    }
    let extension = image_extension(&target).unwrap_or_else(|_| "png".to_string());
    let bytes = fs::read(&target).map_err(|error| format!("Failed to read the image: {error}"))?;
    Ok(Some(format!(
        "data:{};base64,{}",
        extension_mime(&extension),
        base64_encode(&bytes)
    )))
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        output.push(ALPHABET[(triple >> 18) as usize & 0x3f] as char);
        output.push(ALPHABET[(triple >> 12) as usize & 0x3f] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[(triple >> 6) as usize & 0x3f] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[triple as usize & 0x3f] as char
        } else {
            '='
        });
    }
    output
}

#[tauri::command]
fn clear_popup_image(app: AppHandle) -> Result<(), String> {
    let directory = match app.path().app_config_dir() {
        Ok(directory) => directory,
        Err(_) => return Ok(()),
    };
    let target = directory.join("popup-background.img");
    if target.is_file() {
        fs::remove_file(&target).map_err(|error| format!("Failed to remove the image: {error}"))?;
    }
    // 清理旧版本按扩展名保存的残留文件。
    if let Ok(entries) = fs::read_dir(&directory) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("popup-background.") && name != "popup-background.img" {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

fn tray_menu(
    app: &AppHandle,
    language: Language,
    _paused: bool,
) -> tauri::Result<Menu<tauri::Wry>> {
    let (show_text, quit_text, about_text) = i18n::tray_labels(language);
    let show = MenuItemBuilder::with_id("show", show_text).build(app)?;
    let about = MenuItemBuilder::with_id("about", about_text).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", quit_text).build(app)?;
    MenuBuilder::new(app)
        .item(&show)
        .item(&quit)
        .separator()
        .item(&about)
        .build()
}

fn update_tray_menu(app: &AppHandle, language: Language, paused: bool) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(tray_menu(app, language, paused)?))?;
    }
    Ok(())
}

fn create_main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("RemindOn")
        .inner_size(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT)
        .min_inner_size(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT)
        .resizable(true)
        .center()
        .visible(false)
        .theme(Some(tauri::Theme::Dark))
        .build()
        .map_err(|error| format!("Failed to create main window: {error}"))
}

fn ensure_main_window(
    app: &AppHandle,
    state: &AppState,
    navigation: Option<&str>,
) -> Result<WebviewWindow, String> {
    wait_for_window_destroyed(app, state, "main")?;
    state
        .0
        .window_cache
        .lock()
        .expect("window cache lock poisoned")
        .reuse("main");
    let window = match app.get_webview_window("main") {
        Some(window) => window,
        None => {
            state.0.main_ready.store(false, Ordering::SeqCst);
            create_main_window(app)?
        }
    };
    if let Some(navigation) = navigation {
        if state.0.main_ready.load(Ordering::SeqCst) {
            app.emit_to("main", "navigate-to", navigation)
                .map_err(|error| error.to_string())?;
        } else {
            *state
                .0
                .pending_navigation
                .lock()
                .expect("pending navigation lock poisoned") = Some(navigation.to_string());
        }
    }
    window.unminimize().map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(window)
}

fn show_main_window(app: &AppHandle) {
    queue_window_action(app, |app, state| {
        ensure_main_window(app, state, None).map(|_| ())
    });
}

fn toggle_main_window(app: &AppHandle) {
    queue_window_action(app, |app, state| {
        if let Some(window) = app.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false) {
                return hide_cached_window(&window, state);
            }
        }
        ensure_main_window(app, state, None).map(|_| ())
    });
}

fn show_about(app: &AppHandle) {
    queue_window_action(app, |app, state| {
        ensure_main_window(app, state, Some("about")).map(|_| ())
    });
}

#[tauri::command]
async fn open_power_settings(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    ensure_main_window(&app, state.inner(), Some("power")).map(|_| ())
}

#[tauri::command]
async fn take_pending_navigation(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let _operation = state
        .0
        .window_operations
        .lock()
        .expect("window operations lock poisoned");
    state.0.main_ready.store(true, Ordering::SeqCst);
    Ok(state
        .0
        .pending_navigation
        .lock()
        .expect("pending navigation lock poisoned")
        .take())
}

fn setup_tray(app: &tauri::App, language: Language) -> tauri::Result<()> {
    let menu = tray_menu(app.handle(), language, false)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .icon(if cfg!(target_os = "windows") {
            tauri::include_image!("icons/32x32.png")
        } else {
            tauri::include_image!("icons/icon.png")
        })
        .tooltip("RemindOn")
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: tauri::tray::MouseButton::Left,
                    ..
                }
            ) {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// macOS 的应用菜单默认是英文，这里按当前语言替换成中文。
/// Windows 不受影响，Linux 保持发行版默认行为。
#[cfg(target_os = "macos")]
fn apply_application_menu(app: &tauri::AppHandle, language: Language) {
    use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
    let chinese = language == Language::ZhCn;
    let label = |zh: &str, en: &str| if chinese { zh.to_string() } else { en.to_string() };
    let result = (|| -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
        let app_name = label("RemindOn", "RemindOn");
        let about = PredefinedMenuItem::about(app, Some(&app_name), None)?;
        let hide = MenuItemBuilder::with_id("hide", label("隐藏", "Hide")).build(app)?;
        let hide_others = MenuItemBuilder::with_id("hide-others", label("隐藏其他", "Hide Others")).build(app)?;
        let show_all = MenuItemBuilder::with_id("show-all", label("全部显示", "Show All")).build(app)?;
        let minimize = PredefinedMenuItem::minimize(app, Some(&label("最小化", "Minimize")))?;
        let close = PredefinedMenuItem::close_window(app, Some(&label("关闭窗口", "Close Window")))?;
        let services = PredefinedMenuItem::services(app, Some(&label("服务", "Services")))?;
        let quit = MenuItemBuilder::with_id("quit", label("退出", "Quit")).build(app)?;
        MenuBuilder::new(app)
            .items(&[&about])
            .separator()
            .items(&[&hide, &hide_others, &show_all, &minimize, &close])
            .separator()
            .items(&[&services])
            .separator()
            .items(&[&quit])
            .build()
    })();
    if let Ok(menu) = result {
        let _ = app.set_menu(menu);
    }
}

#[cfg(target_os = "macos")]
fn hide_all_windows(app: &tauri::AppHandle) {
    for (_, window) in app.webview_windows() {
        let _ = window.hide();
    }
}

#[cfg(target_os = "macos")]
fn hide_other_windows(app: &tauri::AppHandle) {
    for (label, window) in app.webview_windows() {
        if label != "main" {
            let _ = window.hide();
        }
    }
}

#[cfg(target_os = "macos")]
fn show_all_windows(app: &tauri::AppHandle) {
    for (label, window) in app.webview_windows() {
        if label == "main" {
            let _ = window.show();
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let path = data_path(app.handle()).map_err(std::io::Error::other)?;
            let mut data = load_json(&path).map_err(std::io::Error::other)?;
            validate_and_normalize(&mut data).map_err(std::io::Error::other)?;
            write_json(&path, &data).map_err(std::io::Error::other)?;
            let hide_on_start = data.settings.minimize_to_tray;
            let language = data.settings.language;
            let state = AppState(Arc::new(InnerState {
                data: Mutex::new(data),
                data_path: path,
                paused: AtomicBool::new(false),
                scheduler_started: AtomicBool::new(false),
                scheduler_stop: AtomicBool::new(false),
                next_reminder_session: AtomicU64::new(0),
                power_action_session: AtomicU64::new(0),
                active_reminder: Mutex::new(None),
                rest_active: AtomicBool::new(false),
                rest_round_pending: AtomicBool::new(false),
                rest_next: Mutex::new(None),
                shutdown_next: Mutex::new(None),
                pending_navigation: Mutex::new(None),
                window_operations: Mutex::new(()),
                window_cache: Mutex::new(WindowCache::default()),
                reminder_targets: Mutex::new(HashSet::new()),
                main_ready: AtomicBool::new(false),
                update_state: Mutex::new(updater::UpdateRuntimeState::default()),
                update_progress: Mutex::new(None),
            }));
            app.manage(state.clone());
            setup_tray(app, language)?;
            if !hide_on_start {
                show_main_window(app.handle());
            }
            spawn_scheduler(app.handle().clone(), state);
            updater::start_background_update_checks(
                app.handle().clone(),
                app.state::<AppState>().inner().clone(),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_data,
            save_data,
            start_scheduler_command,
            stop_scheduler_command,
            set_scheduler_paused,
            snooze_reminder,
            dismiss_reminder,
            get_active_reminder,
            show_reminder,
            hide_idle_window,
            get_rest_timer_status,
            get_next_shutdown_trigger,
            execute_power_action,
            test_reminder,
            import_data,
            export_data,
            import_popup_image,
            read_popup_image,
            clear_popup_image,
            take_pending_navigation,
            open_power_settings,
            updater::get_update_mode,
            updater::get_update_status,
            updater::get_update_progress,
            updater::check_for_updates,
            updater::download_portable_update,
            updater::download_portable_update_from_gitee,
            updater::install_update_from_gitee
        ])
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                show_main_window(app);
            }
            "pause" => {
                let state = app.state::<AppState>();
                let paused = !state.0.paused.load(Ordering::SeqCst);
                state.0.paused.store(paused, Ordering::SeqCst);
                let language = state
                    .0
                    .data
                    .lock()
                    .expect("settings lock poisoned")
                    .settings
                    .language;
                let _ = update_tray_menu(app, language, paused);
            }
            "about" => show_about(app),
            "quit" => app.exit(0),
            #[cfg(target_os = "macos")]
            "hide" => hide_all_windows(app),
            #[cfg(target_os = "macos")]
            "hide-others" => hide_other_windows(app),
            #[cfg(target_os = "macos")]
            "show-all" => show_all_windows(app),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("RemindOn initialization failed")
        .run(|app, event| {
            if let RunEvent::ExitRequested { code, ref api, .. } = event {
                if code.is_none() {
                    // Keep the tray-only process alive after the last WebView is destroyed.
                    api.prevent_exit();
                } else {
                    app.state::<AppState>()
                        .0
                        .scheduler_stop
                        .store(true, Ordering::SeqCst);
                }
            }
            if let RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } = event
            {
                if label == "main" || is_reminder_window_label(&label) {
                    api.prevent_close();
                    // Capture the closing session before queuing: a newer reminder may arrive
                    // while an earlier native window operation is still completing.
                    let closing_reminder = if is_reminder_window_label(&label) {
                        app.state::<AppState>()
                            .0
                            .active_reminder
                            .lock()
                            .expect("reminder session lock poisoned")
                            .clone()
                    } else {
                        None
                    };
                    queue_window_action(app, move |app, state| {
                        if label == "main" {
                            if let Some(window) = app.get_webview_window(&label) {
                                hide_cached_window(&window, state)?;
                            }
                        } else if state
                            .0
                            .reminder_targets
                            .lock()
                            .expect("reminder targets lock poisoned")
                            .contains(&label)
                        {
                            if let Some(active) = closing_reminder {
                                if !active_reminder_matches(state, &active.id, active.session_id) {
                                    return Ok(());
                                }
                                if active.is_rest && complete_rest_round(state) {
                                    emit_rest_timer_updated(app, state);
                                }
                                close_reminder_session(app, state, active.session_id);
                            }
                        } else if let Some(window) = app.get_webview_window(&label) {
                            hide_cached_window(&window, state)?;
                        }
                        Ok(())
                    });
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reopening_cancels_reclamation_and_reclosing_starts_a_new_delay() {
        let mut cache = WindowCache::default();
        let now = Instant::now();
        cache.hide("main", now);
        cache.reuse("main");
        assert!(cache.expired(now + WINDOW_DESTROY_DELAY).is_empty());
        cache.hide("main", now + StdDuration::from_secs(20));
        assert!(cache.expired(now + StdDuration::from_secs(49)).is_empty());
        assert_eq!(
            cache.expired(now + StdDuration::from_secs(50)),
            vec!["main"]
        );
    }

    #[test]
    fn duplicate_close_does_not_extend_idle_time_or_affect_other_windows() {
        let mut cache = WindowCache::default();
        let now = Instant::now();
        cache.hide("main", now);
        cache.hide("reminder", now + StdDuration::from_secs(10));
        cache.hide("main", now + StdDuration::from_secs(20));
        assert!(cache.expired(now + StdDuration::from_secs(29)).is_empty());
        assert_eq!(cache.expired(now + WINDOW_DESTROY_DELAY), vec!["main"]);
        cache.reuse("main");
        assert_eq!(
            cache.expired(now + StdDuration::from_secs(40)),
            vec!["reminder"]
        );
    }

    fn rest_state(enabled: bool) -> AppState {
        let mut data = AppData::default();
        data.settings.rest_enabled = enabled;
        data.settings.rest_interval_minutes = 1;
        AppState(Arc::new(InnerState {
            data: Mutex::new(data),
            data_path: PathBuf::new(),
            paused: AtomicBool::new(false),
            scheduler_started: AtomicBool::new(false),
            scheduler_stop: AtomicBool::new(false),
            next_reminder_session: AtomicU64::new(0),
            power_action_session: AtomicU64::new(0),
            active_reminder: Mutex::new(None),
            rest_active: AtomicBool::new(false),
            rest_round_pending: AtomicBool::new(false),
            rest_next: Mutex::new(None),
            shutdown_next: Mutex::new(None),
            pending_navigation: Mutex::new(None),
            window_operations: Mutex::new(()),
            window_cache: Mutex::new(WindowCache::default()),
            reminder_targets: Mutex::new(HashSet::new()),
            main_ready: AtomicBool::new(false),
            update_state: Mutex::new(updater::UpdateRuntimeState::default()),
            update_progress: Mutex::new(None),
        }))
    }

    fn rest_event(state: &AppState, is_test: bool) -> ReminderTriggeredEvent {
        let mut event = test_reminder_event(&app_data(state).settings, TestReminderKind::Rest);
        if !is_test {
            event.id = REST_ID.to_string();
            event.is_test = false;
        }
        event
    }

    fn sample_once(trigger_at: String) -> Reminder {
        Reminder {
            id: "sample".to_string(),
            title: "测试提醒".to_string(),
            reminder_type: ReminderType::Once,
            trigger_at: Some(trigger_at),
            time: None,
            weekdays: Vec::new(),
            month_days: Vec::new(),
            enabled: true,
            next_trigger_at: None,
        }
    }

    fn sample_recurring(reminder_type: ReminderType, time: &str) -> Reminder {
        Reminder {
            id: "recurring".to_string(),
            title: "重复提醒".to_string(),
            reminder_type,
            trigger_at: None,
            time: Some(time.to_string()),
            weekdays: Vec::new(),
            month_days: Vec::new(),
            enabled: true,
            next_trigger_at: None,
        }
    }

    #[test]
    fn daily_time_after_current_time_moves_to_next_day() {
        let after = Local.with_ymd_and_hms(2026, 9, 3, 10, 0, 0).unwrap();
        let next = next_daily("09:00", after).unwrap();
        let parsed = parse_datetime(&next).unwrap();
        assert_eq!(parsed.date_naive(), after.date_naive() + Duration::days(1));
        assert_eq!(parsed.time(), NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    }

    #[test]
    fn validation_rejects_empty_title() {
        let mut data = AppData {
            reminders: vec![sample_once(Local::now().to_rfc3339())],
            ..AppData::default()
        };
        data.reminders[0].title.clear();
        assert!(validate_and_normalize(&mut data).is_err());
    }

    #[test]
    fn once_reminder_gets_next_trigger_at() {
        let trigger = Local::now().to_rfc3339();
        let mut data = AppData {
            reminders: vec![sample_once(trigger.clone())],
            ..AppData::default()
        };
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(
            data.reminders[0].next_trigger_at.as_deref(),
            Some(trigger.as_str())
        );
    }

    #[test]
    fn weekly_reminder_uses_selected_weekdays() {
        let after = Local.with_ymd_and_hms(2026, 9, 4, 10, 0, 0).unwrap();
        let mut reminder = sample_recurring(ReminderType::Weekly, "09:00");
        reminder.weekdays = vec![1, 3];
        let next = parse_datetime(&next_recurring(&reminder, after).unwrap()).unwrap();
        assert_eq!(
            next.date_naive(),
            NaiveDate::from_ymd_opt(2026, 9, 7).unwrap()
        );
    }

    #[test]
    fn monthly_reminder_skips_unselected_dates() {
        let after = Local.with_ymd_and_hms(2026, 9, 4, 10, 0, 0).unwrap();
        let mut reminder = sample_recurring(ReminderType::Monthly, "09:00");
        reminder.month_days = vec![4, 15];
        let next = parse_datetime(&next_recurring(&reminder, after).unwrap()).unwrap();
        assert_eq!(
            next.date_naive(),
            NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()
        );
    }

    #[test]
    fn current_settings_are_normalized() {
        let mut data = AppData::default();
        data.settings.rest_interval_minutes = 0;
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(data.version, DATA_VERSION);
        assert_eq!(data.settings.rest_interval_minutes, 1);
        assert_eq!(data.settings.language, Language::ZhCn);
        assert!(!data.settings.minimize_to_tray);
        assert_eq!(data.settings.power_action, PowerAction::Shutdown);
        assert!(data.settings.popup_fullscreen);
    }

    #[test]
    fn legacy_settings_without_fullscreen_option_use_fullscreen_default() {
        let mut value = serde_json::to_value(AppData::default()).unwrap();
        value["version"] = serde_json::json!(3);
        value["settings"]
            .as_object_mut()
            .unwrap()
            .remove("popupFullscreen");
        let mut data: AppData = serde_json::from_value(value).unwrap();

        assert!(data.settings.popup_fullscreen);
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(data.version, 4);
    }

    #[test]
    fn reminder_sessions_reject_stale_ids() {
        let state = rest_state(true);
        let mut event = rest_event(&state, false);
        event.session_id = 7;
        *state.0.active_reminder.lock().unwrap() = Some(event);

        assert!(active_reminder_matches(&state, REST_ID, 7));
        assert!(!active_reminder_matches(&state, REST_ID, 6));
        assert!(!active_reminder_matches(&state, "other", 7));
    }

    #[test]
    fn reminder_window_labels_do_not_match_unrelated_windows() {
        assert!(is_reminder_window_label(REMINDER_LABEL));
        assert!(is_reminder_window_label(WINDOWED_REMINDER_LABEL));
        assert!(is_reminder_window_label("reminder-monitor-1"));
        assert!(!is_reminder_window_label("main"));
        assert!(!is_reminder_window_label("reminder-preview"));
    }

    #[test]
    fn scheduled_and_test_rest_popups_stop_the_existing_countdown() {
        for is_test in [false, true] {
            let state = rest_state(true);
            assert!(rest_timer_status(&state).next_trigger_at.is_some());
            assert!(prepare_notification(&state, &rest_event(&state, is_test)).is_some());

            // 焦点切换和最小化会再次读取状态，不能因此启动下一轮。
            for _ in 0..3 {
                let status = rest_timer_status(&state);
                assert!(status.is_resting);
                assert!(status.next_trigger_at.is_none());
                assert!(state.0.rest_next.lock().unwrap().is_none());
            }
        }
    }

    #[test]
    fn rest_completion_starts_a_full_interval_from_completion() {
        let state = rest_state(true);
        prepare_notification(&state, &rest_event(&state, false)).unwrap();
        let before = Local::now();
        assert!(complete_rest_round(&state));
        let after = Local::now();
        let status = rest_timer_status(&state);
        let next = parse_datetime(status.next_trigger_at.as_deref().unwrap()).unwrap();

        assert!(!status.is_resting);
        assert!(next >= before + Duration::minutes(1));
        assert!(next <= after + Duration::minutes(1));
        assert!(!complete_rest_round(&state));
        assert_eq!(
            rest_timer_status(&state).next_trigger_at,
            status.next_trigger_at
        );
    }

    #[test]
    fn four_hour_snooze_only_delays_the_current_round() {
        for is_test in [false, true] {
            let state = rest_state(true);
            prepare_notification(&state, &rest_event(&state, is_test)).unwrap();
            let before_snooze = Local::now();
            assert!(snooze_rest_round(&state, 4 * 60 * 60));
            let after_snooze = Local::now();
            let status = rest_timer_status(&state);
            let next = parse_datetime(status.next_trigger_at.as_deref().unwrap()).unwrap();

            assert!(!status.is_resting);
            assert!(next >= before_snooze + Duration::hours(4));
            assert!(next <= after_snooze + Duration::hours(4));
            assert_eq!(app_data(&state).settings.rest_interval_minutes, 1);

            // 延后到期再次弹出时进入休息；完成后恢复原来的 1 分钟间隔。
            prepare_notification(&state, &rest_event(&state, false)).unwrap();
            assert!(rest_timer_status(&state).is_resting);
            let before_completion = Local::now();
            assert!(complete_rest_round(&state));
            let after_completion = Local::now();
            let next = parse_datetime(
                rest_timer_status(&state)
                    .next_trigger_at
                    .as_deref()
                    .unwrap(),
            )
            .unwrap();
            assert!(next >= before_completion + Duration::minutes(1));
            assert!(next <= after_completion + Duration::minutes(1));
        }
    }

    #[test]
    fn repeated_close_and_other_notifications_preserve_snooze() {
        let state = rest_state(true);
        prepare_notification(&state, &rest_event(&state, false)).unwrap();
        assert!(snooze_rest_round(&state, 4 * 60 * 60));
        let delayed = rest_timer_status(&state).next_trigger_at;
        assert!(!complete_rest_round(&state));
        assert!(!snooze_rest_round(&state, 30));
        let event = test_reminder_event(&app_data(&state).settings, TestReminderKind::Event);
        prepare_notification(&state, &event).unwrap();
        assert!(!complete_rest_round(&state));
        assert_eq!(rest_timer_status(&state).next_trigger_at, delayed);
        assert!(state.0.rest_round_pending.load(Ordering::SeqCst));
    }

    #[test]
    fn replacing_an_active_rest_finishes_it_with_a_full_interval() {
        let state = rest_state(true);
        prepare_notification(&state, &rest_event(&state, false)).unwrap();
        let event = test_reminder_event(&app_data(&state).settings, TestReminderKind::Event);
        let before = Local::now();
        prepare_notification(&state, &event).unwrap();
        let after = Local::now();
        let status = rest_timer_status(&state);
        let next = parse_datetime(status.next_trigger_at.as_deref().unwrap()).unwrap();

        assert!(!status.is_resting);
        assert!(next >= before + Duration::minutes(1));
        assert!(next <= after + Duration::minutes(1));
        assert!(!complete_rest_round(&state));
    }

    #[test]
    fn disabled_rest_test_pauses_without_enabling_scheduled_reminders() {
        for snooze in [false, true] {
            let state = rest_state(false);
            prepare_notification(&state, &rest_event(&state, true)).unwrap();
            let status = rest_timer_status(&state);
            assert!(status.is_resting);
            assert!(status.next_trigger_at.is_none());
            if snooze {
                assert!(snooze_rest_round(&state, 4 * 60 * 60));
            } else {
                assert!(complete_rest_round(&state));
            }
            let status = rest_timer_status(&state);
            assert!(!status.is_resting);
            assert!(status.next_trigger_at.is_none());
            assert!(!app_data(&state).settings.rest_enabled);
        }
    }

    #[test]
    fn cancelled_scheduled_rest_cannot_reactivate_a_popup() {
        let state = rest_state(false);
        assert!(prepare_notification(&state, &rest_event(&state, false)).is_none());
        assert!(!rest_timer_status(&state).is_resting);
    }

    #[test]
    fn appended_system_rest_notifications_keep_the_interval_running() {
        let state = rest_state(true);
        state
            .0
            .data
            .lock()
            .unwrap()
            .settings
            .system_notification_enabled = true;
        let before = rest_timer_status(&state).next_trigger_at;
        prepare_notification(&state, &rest_event(&state, true)).unwrap();

        assert!(rest_timer_status(&state).is_resting);
        assert!(rest_timer_status(&state).next_trigger_at.is_none());
        assert!(before.is_some());
    }

    #[test]
    fn overdue_automatic_power_action_is_skipped() {
        let now = Local.with_ymd_and_hms(2026, 9, 4, 23, 32, 0).unwrap();
        let due = Local.with_ymd_and_hms(2026, 9, 4, 23, 30, 0).unwrap();
        assert!(!should_trigger_power_action(due, now));
        let recent_due = Local.with_ymd_and_hms(2026, 9, 4, 23, 31, 30).unwrap();
        assert!(should_trigger_power_action(recent_due, now));
    }

    #[test]
    fn lock_action_uses_stable_json_value() {
        assert_eq!(
            serde_json::to_string(&PowerAction::Lock).unwrap(),
            "\"lock\""
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_power_actions_use_expected_commands() {
        let system_root = Path::new(r"C:\Windows");
        let (lock_program, lock_args) = windows_power_command(&PowerAction::Lock, system_root);
        let (shutdown_program, shutdown_args) =
            windows_power_command(&PowerAction::Shutdown, system_root);
        let (restart_program, restart_args) =
            windows_power_command(&PowerAction::Restart, system_root);

        assert_eq!(
            lock_program,
            system_root.join("System32").join("rundll32.exe")
        );
        assert_eq!(lock_args, vec!["user32.dll,LockWorkStation"]);
        assert_eq!(
            shutdown_program,
            system_root.join("System32").join("shutdown.exe")
        );
        assert_eq!(shutdown_args, vec!["/s", "/t", "0"]);
        assert_eq!(
            restart_program,
            system_root.join("System32").join("shutdown.exe")
        );
        assert_eq!(restart_args, vec!["/r", "/t", "0"]);
    }

    #[test]
    fn event_test_reminder_is_generic_and_non_power() {
        let event = test_reminder_event(&AppSettings::default(), TestReminderKind::Event);

        assert!(event.is_test);
        assert!(!event.is_rest);
        assert!(!event.is_shutdown);
        assert_eq!(event.reminder_type, ReminderType::Once);
        assert_eq!(event.title, "这是一条测试通知");
        assert!(event.power_action.is_none());
    }

    #[test]
    fn rest_test_reminder_uses_configured_content_without_real_rest_id() {
        let mut settings = AppSettings::default();
        settings.rest_message = "起来活动一下".to_string();
        let event = test_reminder_event(&settings, TestReminderKind::Rest);

        assert!(event.is_test);
        assert!(event.is_rest);
        assert_eq!(event.title, settings.rest_message);
        assert_ne!(event.id, REST_ID);
        assert!(event.power_action.is_none());
    }

    #[test]
    fn power_test_reminder_uses_configured_action_without_real_power_id() {
        let mut settings = AppSettings::default();
        settings.power_action = PowerAction::Restart;
        settings.shutdown_reminder_message = "准备重新启动".to_string();
        let event = test_reminder_event(&settings, TestReminderKind::Power);

        assert!(event.is_test);
        assert!(event.is_shutdown);
        assert_eq!(event.title, settings.shutdown_reminder_message);
        assert_eq!(event.power_action, Some(PowerAction::Restart));
        assert_ne!(event.id, SHUTDOWN_ID);
    }
}
