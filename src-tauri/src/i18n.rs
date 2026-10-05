use crate::{Language, ReminderTriggeredEvent};

pub fn default_rest_message(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "休息时间到了，该休息一下了，不要卷。",
        Language::En => "It is time to take a break. Do not overwork yourself.",
    }
}

pub fn notification_title(language: Language, event: &ReminderTriggeredEvent) -> &'static str {
    match (language, event.is_rest) {
        (Language::ZhCn, true) => "RemindOn · 休息提醒",
        (Language::ZhCn, false) => "RemindOn · 定时提醒",
        (Language::En, true) => "RemindOn · Break reminder",
        (Language::En, false) => "RemindOn · Scheduled reminder",
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
