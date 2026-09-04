export type ReminderType = 'once' | 'daily' | 'weekly' | 'monthly' | 'workday' | 'weekend' | 'interval'
export type NotificationMode = 'system' | 'popup'
export type Theme = 'dark' | 'light' | 'system'
export type Language = 'zh-CN' | 'en'
export type AccentColor = 'mint' | 'blue' | 'violet' | 'amber'
export type NotificationStyle = 'compact' | 'standard' | 'prominent'
export type PowerAction = 'shutdown' | 'lock' | 'restart'

export interface Reminder {
  id: string
  title: string
  type: ReminderType
  triggerAt?: string | null
  time?: string | null
  weekdays?: number[]
  monthDays?: number[]
  enabled: boolean
  nextTriggerAt?: string | null
}

export interface AppSettings {
  language: Language
  autostart: boolean
  minimizeToTray: boolean
  popupAlwaysOnTop: boolean
  restEnabled: boolean
  restIntervalMinutes: number
  restMessage: string
  notificationMode: NotificationMode
  notificationStyle: NotificationStyle
  theme: Theme
  accentColor: AccentColor
  shutdownReminderEnabled: boolean
  powerAction: PowerAction
  shutdownReminderTime: string
  shutdownReminderMessage: string
}

export interface AppData {
  version: number
  settings: AppSettings
  reminders: Reminder[]
}

export interface ReminderTriggeredEvent {
  id: string
  title: string
  type: ReminderType
  isRest: boolean
  isShutdown: boolean
  powerAction?: PowerAction | null
}

export const defaultData = (): AppData => ({
  version: 3,
  settings: {
    language: 'zh-CN',
    autostart: false,
    minimizeToTray: true,
    popupAlwaysOnTop: true,
    restEnabled: false,
    restIntervalMinutes: 45,
    restMessage: '休息时间到了，该休息一下了。',
    notificationMode: 'popup',
    notificationStyle: 'standard',
    theme: 'dark',
    accentColor: 'mint',
    shutdownReminderEnabled: false,
    powerAction: 'shutdown',
    shutdownReminderTime: '23:30',
    shutdownReminderMessage: '时间不早了，记得关闭电脑。',
  },
  reminders: [],
})
