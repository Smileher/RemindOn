import { translate } from './i18n.ts'

export type ReminderType = 'once' | 'daily' | 'weekly' | 'monthly' | 'interval'
export type Theme = 'dark' | 'light' | 'system'
export type Language = 'zh-CN' | 'en'
export type AccentColor = 'mint' | 'blue' | 'violet' | 'amber'
export type PowerAction = 'shutdown' | 'lock' | 'restart'
export type TestReminderKind = 'event' | 'rest'
export type PopupBackgroundFit = 'stretch' | 'contain'

export interface Reminder {
  id: string
  title: string
  type: ReminderType
  triggerAt?: string | null
  time?: string | null
  weekdays?: number[]
  monthDays?: number[]
  enabled: boolean
  powerAction?: PowerAction | null
  nextTriggerAt?: string | null
}

export interface AppSettings {
  language: Language
  autostart: boolean
  minimizeToTray: boolean
  popupAlwaysOnTop: boolean
  popupFullscreen: boolean
  restEnabled: boolean
  restIntervalMinutes: number
  restMessage: string
  systemNotificationEnabled: boolean
  theme: Theme
  accentColor: AccentColor
  popupBackgroundFit: PopupBackgroundFit
  popupBackgroundScale: number
  popupBackgroundOffsetX: number
  popupBackgroundOffsetY: number
  popupFadeEnabled: boolean
  popupTextColor: string
  popupTitleSize: number
  popupOverlayOpacity: number
}

export interface AppData {
  version: number
  settings: AppSettings
  reminders: Reminder[]
}

export interface ReminderTriggeredEvent {
  sessionId: number
  id: string
  title: string
  type: ReminderType
  isRest: boolean
  powerAction?: PowerAction | null
  isTest: boolean
  restStartedAtMs?: number | null
  powerDeadlineMs?: number | null
}

export interface RestTimerStatus {
  nextTriggerAt: string | null
  isResting: boolean
}

export const defaultData = (): AppData => ({
  version: 6,
  settings: {
    language: 'zh-CN',
    autostart: false,
    minimizeToTray: false,
    popupAlwaysOnTop: true,
    popupFullscreen: true,
    restEnabled: false,
    restIntervalMinutes: 45,
    restMessage: translate('zh-CN', 'rest.defaultMessage'),
    systemNotificationEnabled: true,
    theme: 'dark',
    accentColor: 'mint',
    popupBackgroundFit: 'stretch',
    popupBackgroundScale: 100,
    popupBackgroundOffsetX: 0,
    popupBackgroundOffsetY: 0,
    popupFadeEnabled: true,
    popupTextColor: '',
    popupTitleSize: 32,
    popupOverlayOpacity: 55,
  },
  reminders: [],
})
