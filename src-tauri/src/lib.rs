use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use tauri_plugin_notification::NotificationExt;

const DATA_VERSION: u32 = 3;
const REST_ID: &str = "__rest__";
const SHUTDOWN_ID: &str = "__shutdown__";
const TRAY_ID: &str = "main-tray";

fn default_rest_message() -> String {
    "休息时间到了，该休息一下了。".to_string()
}

fn default_shutdown_time() -> String {
    "23:30".to_string()
}

fn default_shutdown_message() -> String {
    "即将自动关闭电脑。".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub language: Language,
    pub autostart: bool,
    pub minimize_to_tray: bool,
    pub popup_always_on_top: bool,
    pub rest_enabled: bool,
    pub rest_interval_minutes: u32,
    #[serde(default = "default_rest_message")]
    pub rest_message: String,
    #[serde(default)]
    pub notification_mode: NotificationMode,
    #[serde(default)]
    pub notification_style: NotificationStyle,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub accent_color: AccentColor,
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
#[serde(rename_all = "lowercase")]
pub enum NotificationMode {
    System,
    Popup,
}

impl Default for NotificationMode {
    fn default() -> Self {
        Self::Popup
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationStyle {
    Compact,
    Standard,
    Prominent,
}

impl Default for NotificationStyle {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PowerAction {
    Shutdown,
    Lock,
    Restart,
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
            rest_enabled: false,
            rest_interval_minutes: 45,
            rest_message: default_rest_message(),
            notification_mode: NotificationMode::Popup,
            notification_style: NotificationStyle::Standard,
            theme: Theme::Dark,
            accent_color: AccentColor::Mint,
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
    rest_active: AtomicBool,
    rest_next: Mutex<Option<DateTime<Local>>>,
    shutdown_next: Mutex<Option<DateTime<Local>>>,
}

#[derive(Clone)]
struct AppState(Arc<InnerState>);

fn data_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("无法取得配置目录：{error}"))?;
    fs::create_dir_all(&directory).map_err(|error| format!("无法创建配置目录：{error}"))?;
    Ok(directory.join("remindon.json"))
}

fn write_json(path: &Path, data: &AppData) -> Result<(), String> {
    let content =
        serde_json::to_string_pretty(data).map_err(|error| format!("序列化配置失败：{error}"))?;
    // Windows cannot replace an existing file with std::fs::rename, so keep this
    // small local configuration write straightforward and portable.
    fs::write(path, content).map_err(|error| format!("保存配置失败：{error}"))
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
    let content = fs::read_to_string(path).map_err(|error| format!("读取配置失败：{error}"))?;
    match serde_json::from_str::<AppData>(&content) {
        Ok(data) => Ok(data),
        Err(error) => {
            let backup = corrupt_backup_path(path);
            fs::rename(path, &backup).map_err(|rename_error| {
                format!("配置格式无效（{error}），且无法保留损坏文件：{rename_error}")
            })?;
            Ok(AppData::default())
        }
    }
}

fn parse_datetime(value: &str) -> Result<DateTime<Local>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&Local))
        .map_err(|error| format!("时间格式无效：{error}"))
}

fn parse_time(value: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| "提醒时间必须是 HH:MM".to_string())
}

fn local_datetime(date: NaiveDate, time: NaiveTime) -> Result<DateTime<Local>, String> {
    Local
        .from_local_datetime(&date.and_time(time))
        .earliest()
        .or_else(|| Local.from_local_datetime(&date.and_time(time)).latest())
        .ok_or_else(|| "无法计算本地提醒时间".to_string())
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
        .ok_or_else(|| "重复提醒缺少 time".to_string())?;
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
    Err("无法计算下一次提醒时间".to_string())
}

fn should_trigger_power_action(due: DateTime<Local>, now: DateTime<Local>) -> bool {
    due <= now && now.signed_duration_since(due) <= Duration::minutes(1)
}

fn validate_and_normalize(data: &mut AppData) -> Result<(), String> {
    data.version = DATA_VERSION;
    data.settings.rest_interval_minutes = data.settings.rest_interval_minutes.clamp(1, 1440);
    if data.settings.rest_message.trim().is_empty() {
        data.settings.rest_message = default_rest_message();
    }
    if data.settings.shutdown_reminder_message.trim().is_empty() {
        data.settings.shutdown_reminder_message = default_shutdown_message();
    }
    parse_time(&data.settings.shutdown_reminder_time)?;
    let now = Local::now();
    for reminder in &mut data.reminders {
        if reminder.id.trim().is_empty() {
            return Err("提醒缺少 id".to_string());
        }
        if reminder.title.trim().is_empty() {
            return Err("提醒内容不能为空".to_string());
        }
        match reminder.reminder_type {
            ReminderType::Once => {
                let trigger = reminder
                    .trigger_at
                    .as_deref()
                    .ok_or_else(|| "单次提醒缺少 triggerAt".to_string())?;
                parse_datetime(trigger)?;
                reminder.next_trigger_at = Some(trigger.to_string());
            }
            ReminderType::Daily | ReminderType::Weekly | ReminderType::Monthly => {
                let time = reminder
                    .time
                    .as_deref()
                    .ok_or_else(|| "重复提醒缺少 time".to_string())?;
                parse_time(time)?;
                reminder.weekdays.sort_unstable();
                reminder.weekdays.dedup();
                reminder.month_days.sort_unstable();
                reminder.month_days.dedup();
                if reminder.reminder_type == ReminderType::Weekly
                    && (reminder.weekdays.is_empty()
                        || reminder.weekdays.iter().any(|day| !(1..=7).contains(day)))
                {
                    return Err("每周提醒至少需要选择一天".to_string());
                }
                if reminder.reminder_type == ReminderType::Monthly
                    && (reminder.month_days.is_empty()
                        || reminder
                            .month_days
                            .iter()
                            .any(|day| !(1..=31).contains(day)))
                {
                    return Err("每月提醒至少需要选择一个日期".to_string());
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
    state.0.data.lock().expect("配置锁被中毒").clone()
}

fn notification_title(language: Language, event: &ReminderTriggeredEvent) -> &'static str {
    match (language, event.is_rest, event.is_shutdown) {
        (Language::ZhCn, true, _) => "RemindOn · 休息提醒",
        (Language::ZhCn, _, true) => "RemindOn · 定时操作",
        (Language::ZhCn, _, _) => "RemindOn · 事件提醒",
        (Language::En, true, _) => "RemindOn · Break reminder",
        (Language::En, _, true) => "RemindOn · Scheduled action",
        (Language::En, _, _) => "RemindOn · Reminder",
    }
}

fn notification_window_title(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "RemindOn 通知",
        Language::En => "RemindOn Notification",
    }
}

fn dispatch_trigger(app: &AppHandle, state: &AppState, event: ReminderTriggeredEvent) {
    let settings = app_data(state).settings;
    let requires_popup = event.power_action.is_some();

    // 真实休息通知打开后保持暂停状态，直到用户完成或稍后提醒。
    if event.is_rest && !event.is_test && settings.notification_mode == NotificationMode::Popup {
        state.0.rest_active.store(true, Ordering::SeqCst);
        *state.0.rest_next.lock().expect("休息提醒锁被中毒") = None;
    }

    if settings.notification_mode == NotificationMode::System && !requires_popup {
        if let Some(window) = app.get_webview_window("reminder") {
            let _ = window.hide();
        }
        let _ = app
            .notification()
            .builder()
            .title(notification_title(settings.language, &event))
            .body(&event.title)
            .show();
    } else if let Some(window) = app.get_webview_window("reminder") {
        let _ = window.set_title(notification_window_title(settings.language));
        let _ = window.set_always_on_top(settings.popup_always_on_top);
        let _ = window.center();
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit_to("reminder", "reminder-triggered", event.clone());
    }

    let _ = app.emit_to("main", "reminder-triggered", event);
}

fn process_due(app: &AppHandle, state: &AppState) {
    let now = Local::now();
    let mut triggered = Vec::new();
    let mut changed = false;
    {
        let mut data = state.0.data.lock().expect("配置锁被中毒");
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
                let mut next_rest = state.0.rest_next.lock().expect("休息提醒锁被中毒");
                if next_rest.is_none() {
                    *next_rest =
                        Some(now + Duration::minutes(data.settings.rest_interval_minutes as i64));
                } else if next_rest.is_some_and(|value| value <= now) {
                    triggered.push(ReminderTriggeredEvent {
                        id: REST_ID.to_string(),
                        title: data.settings.rest_message.clone(),
                        reminder_type: ReminderType::Interval,
                        is_rest: true,
                        is_shutdown: false,
                        power_action: None,
                        is_test: false,
                    });
                    if data.settings.notification_mode == NotificationMode::Popup {
                        *next_rest = None;
                        state.0.rest_active.store(true, Ordering::SeqCst);
                    } else {
                        *next_rest = Some(
                            now + Duration::minutes(data.settings.rest_interval_minutes as i64),
                        );
                    }
                }
            }
        } else {
            state.0.rest_active.store(false, Ordering::SeqCst);
            *state.0.rest_next.lock().expect("休息提醒锁被中毒") = None;
        }
        if data.settings.shutdown_reminder_enabled {
            let mut next_shutdown = state.0.shutdown_next.lock().expect("关机提醒锁被中毒");
            if next_shutdown.is_none() {
                *next_shutdown = next_daily(&data.settings.shutdown_reminder_time, now)
                    .ok()
                    .and_then(|value| parse_datetime(&value).ok());
            } else if let Some(due) = next_shutdown.as_ref().filter(|value| **value <= now) {
                if should_trigger_power_action(due.clone(), now) {
                    triggered.push(ReminderTriggeredEvent {
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
            *state.0.shutdown_next.lock().expect("关机提醒锁被中毒") = None;
        }
        if changed {
            let _ = write_json(&state.0.data_path, &data);
        }
    }
    for event in triggered {
        dispatch_trigger(app, state, event);
    }
}

fn spawn_scheduler(app: AppHandle, state: AppState) {
    if state.0.scheduler_started.swap(true, Ordering::SeqCst) {
        return;
    }
    state.0.scheduler_stop.store(false, Ordering::SeqCst);
    thread::spawn(move || {
        while !state.0.scheduler_stop.load(Ordering::SeqCst) {
            if !state.0.paused.load(Ordering::SeqCst) {
                process_due(&app, &state);
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
fn save_data(
    mut data: AppData,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppData, String> {
    validate_and_normalize(&mut data)?;
    let (rest_schedule_changed, shutdown_schedule_changed) = {
        let current = state.0.data.lock().expect("配置锁被中毒");
        (
            current.settings.rest_enabled != data.settings.rest_enabled
                || current.settings.rest_interval_minutes != data.settings.rest_interval_minutes,
            current.settings.shutdown_reminder_enabled != data.settings.shutdown_reminder_enabled
                || current.settings.shutdown_reminder_time != data.settings.shutdown_reminder_time,
        )
    };
    write_json(&state.0.data_path, &data)?;
    *state.0.data.lock().expect("配置锁被中毒") = data.clone();
    if rest_schedule_changed {
        state.0.rest_active.store(false, Ordering::SeqCst);
        *state.0.rest_next.lock().expect("休息提醒锁被中毒") = None;
    }
    if shutdown_schedule_changed {
        *state.0.shutdown_next.lock().expect("关机提醒锁被中毒") = None;
    }
    let _ = update_tray_menu(
        &app,
        data.settings.language,
        state.0.paused.load(Ordering::SeqCst),
    );
    if rest_schedule_changed {
        let _ = app.emit_to("main", "rest-timer-updated", ());
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
fn snooze_reminder(
    id: String,
    seconds: u32,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut data = state.0.data.lock().expect("配置锁被中毒");
    let delay = Duration::seconds(seconds.max(1) as i64);
    if id == REST_ID {
        state.0.rest_active.store(false, Ordering::SeqCst);
        *state.0.rest_next.lock().expect("休息提醒锁被中毒") = Some(Local::now() + delay);
    } else if id == SHUTDOWN_ID {
        *state.0.shutdown_next.lock().expect("关机提醒锁被中毒") = Some(Local::now() + delay);
    } else if let Some(reminder) = data.reminders.iter_mut().find(|item| item.id == id) {
        reminder.enabled = true;
        reminder.next_trigger_at = Some((Local::now() + delay).to_rfc3339());
        write_json(&state.0.data_path, &data)?;
    }
    drop(data);
    if id == REST_ID {
        let _ = app.emit_to("main", "rest-timer-updated", ());
    }
    Ok(())
}

#[tauri::command]
fn dismiss_reminder(id: String, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if id == REST_ID {
        let data = state.0.data.lock().expect("配置锁被中毒");
        state.0.rest_active.store(false, Ordering::SeqCst);
        *state.0.rest_next.lock().expect("休息提醒锁被中毒") =
            Some(Local::now() + Duration::minutes(data.settings.rest_interval_minutes as i64));
        drop(data);
        let _ = app.emit_to("main", "rest-timer-updated", ());
    }
    Ok(())
}

#[tauri::command]
fn get_rest_timer_status(state: State<'_, AppState>) -> RestTimerStatus {
    let data = state.0.data.lock().expect("配置锁被中毒");
    if !data.settings.rest_enabled {
        return RestTimerStatus {
            next_trigger_at: None,
            is_resting: false,
        };
    }
    if state.0.rest_active.load(Ordering::SeqCst) {
        return RestTimerStatus {
            next_trigger_at: None,
            is_resting: true,
        };
    }
    let mut next = state.0.rest_next.lock().expect("休息提醒锁被中毒");
    if next.is_none() {
        *next = Some(Local::now() + Duration::minutes(data.settings.rest_interval_minutes as i64));
    }
    RestTimerStatus {
        next_trigger_at: next.as_ref().map(DateTime::to_rfc3339),
        is_resting: false,
    }
}

#[tauri::command]
fn get_next_shutdown_trigger(state: State<'_, AppState>) -> Option<String> {
    let data = state.0.data.lock().expect("配置锁被中毒");
    if !data.settings.shutdown_reminder_enabled {
        return None;
    }
    let mut next = state.0.shutdown_next.lock().expect("关机提醒锁被中毒");
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

#[tauri::command]
fn execute_power_action(action: PowerAction) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let system_root = std::env::var_os("SystemRoot")
            .ok_or_else(|| "无法确定 Windows 系统目录".to_string())?;
        let (program, args) = windows_power_command(&action, &PathBuf::from(system_root));
        Command::new(program)
            .args(args)
            .spawn()
            .map_err(|error| format!("无法执行系统操作：{error}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        if action == PowerAction::Lock {
            Command::new(
                "/System/Library/CoreServices/Menu Extras/User.menu/Contents/Resources/CGSession",
            )
            .arg("-suspend")
            .spawn()
            .map_err(|error| format!("无法锁定电脑：{error}"))?;
            return Ok(());
        }
        let script = if action == PowerAction::Shutdown {
            "tell application \"System Events\" to shut down"
        } else {
            "tell application \"System Events\" to restart"
        };
        Command::new("/usr/bin/osascript")
            .args(["-e", script])
            .spawn()
            .map_err(|error| format!("无法执行系统操作：{error}"))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("当前系统不支持此自动操作".to_string())
}

fn test_reminder_event(settings: &AppSettings, kind: TestReminderKind) -> ReminderTriggeredEvent {
    match kind {
        TestReminderKind::Event => ReminderTriggeredEvent {
            id: "__test_event__".to_string(),
            title: match settings.language {
                Language::ZhCn => "这是一条测试通知".to_string(),
                Language::En => "This is a test notification".to_string(),
            },
            reminder_type: ReminderType::Once,
            is_rest: false,
            is_shutdown: false,
            power_action: None,
            is_test: true,
        },
        TestReminderKind::Rest => ReminderTriggeredEvent {
            id: "__test_rest__".to_string(),
            title: settings.rest_message.clone(),
            reminder_type: ReminderType::Interval,
            is_rest: true,
            is_shutdown: false,
            power_action: None,
            is_test: true,
        },
        TestReminderKind::Power => ReminderTriggeredEvent {
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
fn test_reminder(
    kind: TestReminderKind,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let event = test_reminder_event(&app_data(&state).settings, kind);
    dispatch_trigger(&app, &state, event);
    Ok(())
}

#[tauri::command]
fn import_data(
    path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppData, String> {
    let content =
        fs::read_to_string(&path).map_err(|error| format!("读取导入文件失败：{error}"))?;
    let mut data: AppData =
        serde_json::from_str(&content).map_err(|error| format!("导入文件格式无效：{error}"))?;
    validate_and_normalize(&mut data)?;
    write_json(&state.0.data_path, &data)?;
    *state.0.data.lock().expect("配置锁被中毒") = data.clone();
    state.0.rest_active.store(false, Ordering::SeqCst);
    *state.0.rest_next.lock().expect("休息提醒锁被中毒") = None;
    *state.0.shutdown_next.lock().expect("关机提醒锁被中毒") = None;
    let _ = update_tray_menu(
        &app,
        data.settings.language,
        state.0.paused.load(Ordering::SeqCst),
    );
    Ok(data)
}

#[tauri::command]
fn export_data(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let data = app_data(&state);
    let content =
        serde_json::to_string_pretty(&data).map_err(|error| format!("序列化导出失败：{error}"))?;
    fs::write(path, content).map_err(|error| format!("写出导出文件失败：{error}"))
}

fn tray_menu(app: &AppHandle, language: Language, paused: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let (show_text, pause_text, resume_text, about_text, quit_text) = match language {
        Language::ZhCn => ("打开 RemindOn", "暂停提醒", "恢复提醒", "关于", "退出"),
        Language::En => (
            "Open RemindOn",
            "Pause reminders",
            "Resume reminders",
            "About",
            "Quit",
        ),
    };
    let show = MenuItemBuilder::with_id("show", show_text).build(app)?;
    let pause = MenuItemBuilder::with_id("pause", if paused { resume_text } else { pause_text })
        .build(app)?;
    let about = MenuItemBuilder::with_id("about", about_text).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", quit_text).build(app)?;
    MenuBuilder::new(app)
        .items(&[&show, &pause, &about])
        .separator()
        .item(&quit)
        .build()
}

fn update_tray_menu(app: &AppHandle, language: Language, paused: bool) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(tray_menu(app, language, paused)?))?;
    }
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_about(app: &AppHandle) {
    show_main_window(app);
    let _ = app.emit_to("main", "navigate-to", "about");
}

fn setup_tray(app: &tauri::App, language: Language) -> tauri::Result<()> {
    let menu = tray_menu(app.handle(), language, false)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .icon(tauri::include_image!("icons/icon.png"))
        .tooltip("RemindOn")
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
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
                rest_active: AtomicBool::new(false),
                rest_next: Mutex::new(None),
                shutdown_next: Mutex::new(None),
            }));
            app.manage(state.clone());
            setup_tray(app, language)?;
            if !hide_on_start {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.center();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            spawn_scheduler(app.handle().clone(), state);
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
            get_rest_timer_status,
            get_next_shutdown_trigger,
            execute_power_action,
            test_reminder,
            import_data,
            export_data
        ])
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                show_main_window(app);
            }
            "pause" => {
                let state = app.state::<AppState>();
                let paused = !state.0.paused.load(Ordering::SeqCst);
                state.0.paused.store(paused, Ordering::SeqCst);
                let language = state.0.data.lock().expect("配置锁被中毒").settings.language;
                let _ = update_tray_menu(app, language, paused);
            }
            "about" => show_about(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("RemindOn 初始化失败")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } = event {
                app.state::<AppState>()
                    .0
                    .scheduler_stop
                    .store(true, Ordering::SeqCst);
            }
            if let RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } = event
            {
                if label == "main" {
                    api.prevent_close();
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                } else if label == "reminder" {
                    api.prevent_close();
                    if let Some(window) = app.get_webview_window("reminder") {
                        let _ = window.hide();
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

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
