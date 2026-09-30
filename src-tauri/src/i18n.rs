use crate::{Language, PowerAction, ReminderTriggeredEvent};

pub fn default_rest_message(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "休息时间到了，该休息一下了，不要卷。",
        Language::En => "It is time to take a break. Do not overwork yourself.",
    }
}

pub fn default_power_message(language: Language, action: &PowerAction) -> &'static str {
    match (language, action) {
        (Language::ZhCn, PowerAction::Shutdown) => "即将自动关闭电脑。",
        (Language::ZhCn, PowerAction::Lock) => "即将自动锁定电脑。",
        (Language::ZhCn, PowerAction::Restart) => "即将自动重启电脑。",
        (Language::En, PowerAction::Shutdown) => "The computer will shut down shortly.",
        (Language::En, PowerAction::Lock) => "The computer will lock shortly.",
        (Language::En, PowerAction::Restart) => "The computer will restart shortly.",
    }
}

pub fn notification_title(language: Language, event: &ReminderTriggeredEvent) -> &'static str {
    match (language, event.is_rest, event.is_shutdown) {
        (Language::ZhCn, true, _) => "RemindOn · 休息提醒",
        (Language::ZhCn, _, true) => "RemindOn · 定时操作",
        (Language::ZhCn, _, _) => "RemindOn · 事件提醒",
        (Language::En, true, _) => "RemindOn · Break reminder",
        (Language::En, _, true) => "RemindOn · Scheduled action",
        (Language::En, _, _) => "RemindOn · Reminder",
    }
}

pub fn notification_window_title(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "RemindOn 通知",
        Language::En => "RemindOn Notification",
    }
}

pub fn test_notification(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "这是一条测试通知",
        Language::En => "This is a test notification",
    }
}

pub fn tray_labels(language: Language) -> (&'static str, &'static str, &'static str) {
    match language {
        Language::ZhCn => ("打开", "关闭", "关于"),
        Language::En => ("Open", "Close", "About"),
    }
}

pub fn notification_send_failed(language: Language, error: &str) -> String {
    match language {
        Language::ZhCn => format!("系统通知发送失败：{error}"),
        Language::En => format!("Failed to send system notification: {error}"),
    }
}
