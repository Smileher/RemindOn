<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { getVersion, setTheme } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { ask, confirm, open, save } from '@tauri-apps/plugin-dialog'
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { LockKeyhole, Power, RotateCw } from '@lucide/vue'
import ReminderPopup from './components/ReminderPopup.vue'
import AppSidebar from './components/AppSidebar.vue'
import EventsView from './components/EventsView.vue'
import RestView from './components/RestView.vue'
import PowerView from './components/PowerView.vue'
import SettingsView from './components/SettingsView.vue'
import AboutView from './components/AboutView.vue'
import { translate } from './i18n'
import type { MessageKey } from './i18n'
import { logError } from './error'
import type { AccentColor, AppData, Language, PowerAction, Reminder, ReminderTriggeredEvent, ReminderType, RestTimerStatus, TestReminderKind, Theme } from './types'
import { defaultData } from './types'
import { useUpdater } from './composables/useUpdater'

type View = 'events' | 'rest' | 'power' | 'settings' | 'about'
type EditableReminderType = Exclude<ReminderType, 'interval'>
type AutomaticPowerAction = Extract<PowerAction, 'shutdown' | 'lock' | 'restart'>

const isPopup = window.location.hash === '#/reminder'
const isStoreBuild = typeof __REMINDON_STORE_BUILD__ !== 'undefined' && __REMINDON_STORE_BUILD__
const data = ref<AppData>(defaultData())
const currentView = ref<View>('rest')
const showForm = ref(false)
const editingId = ref<string | null>(null)
const actionMessage = ref('')
const now = ref(Date.now())
const nextRestTrigger = ref<string | null>(null)
const restIsActive = ref(false)
const nextShutdownTrigger = ref<string | null>(null)
const restMessageDraft = ref(defaultData().settings.restMessage)
const shutdownMessageDraft = ref(defaultData().settings.shutdownReminderMessage)
const notificationError = ref('')
const autostartError = ref('')
const appVersion = ref('1.1')
const popupBackgroundPreview = ref('')
const {
  mode: updateMode, status: updateStatus, newVersion, progress: updateProgress,
  errorMessage: updateError, busy: updateBusy,
  loadStatus: loadUpdateStatus, checkForUpdates, openReleases, openAuthorPage, dispose: disposeUpdater,
} = useUpdater()
let unlisten: (() => void) | undefined
let unlistenNavigation: (() => void) | undefined
let unlistenRestTimer: (() => void) | undefined
let unlistenNotificationFailure: (() => void) | undefined
let unlistenSettingsSync: (() => void) | undefined
let unlistenWindowFocus: (() => void) | undefined
let clockTimer: number | undefined
let timerRefreshToken = 0
let actionMessageTimer: number | undefined

// 状态消息只短暂停留，避免一直占位或把旁边的按钮挤走。
watch(actionMessage, (value) => {
  if (actionMessageTimer) window.clearTimeout(actionMessageTimer)
  actionMessageTimer = undefined
  if (value) {
    actionMessageTimer = window.setTimeout(() => {
      actionMessage.value = ''
    }, 4000)
  }
})

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(data.value.settings.language, key, params)
}

function formatError(error: unknown) {
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error
  try {
    const serialized = JSON.stringify(error)
    return serialized && serialized !== '{}' ? serialized : String(error)
  } catch {
    return String(error)
  }
}

const frequencyOptions = computed<Array<{ value: EditableReminderType; label: string }>>(() => [
  { value: 'once', label: t('frequency.once') },
  { value: 'daily', label: t('frequency.daily') },
  { value: 'weekly', label: t('frequency.weekly') },
  { value: 'monthly', label: t('frequency.monthly') },
])

const powerActionOptions = computed<Array<{ value: AutomaticPowerAction; label: string; icon: typeof Power }>>(() => [
  { value: 'shutdown', label: t('power.shutdown'), icon: Power },
  { value: 'lock', label: t('power.lock'), icon: LockKeyhole },
  { value: 'restart', label: t('power.restart'), icon: RotateCw },
])
const accentColors: AccentColor[] = ['mint', 'blue', 'violet', 'amber']

const weekdayOptions = computed(() => Array.from({ length: 7 }, (_, index) => ({
  value: index + 1,
  label: t(`weekday.${index + 1}` as MessageKey),
})))

const typeLabels = computed<Record<ReminderType, string>>(() => ({
  once: t('frequency.once'), daily: t('frequency.daily'), weekly: t('frequency.weekly'), monthly: t('frequency.monthly'),
  interval: t('frequency.interval'),
}))

const form = reactive({
  title: '',
  type: 'once' as EditableReminderType,
  triggerAt: '',
  time: '09:00',
  weekdays: [1] as number[],
  monthDays: [1] as number[],
})

const sortedReminders = computed(() =>
  [...data.value.reminders].sort((a, b) => {
    const left = a.nextTriggerAt || a.triggerAt || a.time || ''
    const right = b.nextTriggerAt || b.triggerAt || b.time || ''
    return left.localeCompare(right)
  }),
)

const restStatusText = computed(() =>
  restIsActive.value
    ? t('rest.resting')
    : data.value.settings.restEnabled
      ? (nextRestTrigger.value ? formatCountdown(nextRestTrigger.value) : t('common.calculating'))
      : t('common.paused'),
)

const powerStatusText = computed(() =>
  data.value.settings.shutdownReminderEnabled
    ? (nextShutdownTrigger.value ? formatCountdown(nextShutdownTrigger.value) : t('common.calculating'))
    : t('common.paused'),
)

// 重置只恢复参数默认值，提醒列表必须保留。
// 默认文案随语言变化，所以比较时要按当前语言重新生成一份默认值，否则切到英文后永远判定为「已修改」。
const canResetSettings = computed(() => {
  const defaults = defaultData().settings
  const current = data.value.settings
  const language = current.language
  const localized = {
    ...defaults,
    language,
    restMessage: translate(language, 'rest.defaultMessage'),
    shutdownReminderMessage: translate(language, 'power.defaultShutdownMessage'),
  }
  return Boolean(popupBackgroundPreview.value) || (Object.keys(localized) as Array<keyof typeof localized>).some(
    (key) => JSON.stringify(current[key]) !== JSON.stringify(localized[key]),
  )
})

const restProgress = computed(() => {
  if (restIsActive.value) return 0
  if (!data.value.settings.restEnabled || !nextRestTrigger.value) return 0
  const remaining = new Date(nextRestTrigger.value).getTime() - now.value
  const total = data.value.settings.restIntervalMinutes * 60 * 1000
  if (!Number.isFinite(remaining) || total <= 0) return 0
  return Math.max(0, Math.min(100, (remaining / total) * 100))
})

const updateStatusText = computed(() => {
  if (updateStatus.value === 'checking') return t('update.checking')
  if (updateStatus.value === 'downloading') {
    return updateProgress.value === null
      ? t('update.downloading')
      : t('update.progress', { progress: updateProgress.value })
  }
  if (updateStatus.value === 'installing') return t('update.installing')
  if (updateStatus.value === 'upToDate') return t('update.upToDate')
  if (updateStatus.value === 'error') return t('update.checkFailed')
  if (updateMode.value === 'development') return t('update.development')
  if (updateMode.value === 'unsupported') return t('update.unsupported')
  if (newVersion.value) return t('update.available', { version: newVersion.value })
  return t('update.idle')
})

function patchForm(patch: Record<string, unknown>) {
  Object.assign(form, patch)
}

function resetForm() {
  form.title = ''
  form.type = 'once'
  form.triggerAt = toDateTimeLocal(new Date(Date.now() + 10 * 60 * 1000))
  form.time = '09:00'
  form.weekdays = [new Date().getDay() || 7]
  form.monthDays = [new Date().getDate()]
  editingId.value = null
  actionMessage.value = ''
}

function toDateTimeLocal(date: Date) {
  const offset = date.getTimezoneOffset()
  return new Date(date.getTime() - offset * 60 * 1000).toISOString().slice(0, 16)
}

function openAddForm() {
  resetForm()
  showForm.value = true
}

function editReminder(reminder: Reminder) {
  editingId.value = reminder.id
  form.title = reminder.title
  form.type = reminder.type === 'interval' ? 'once' : reminder.type
  form.triggerAt = reminder.triggerAt ? toDateTimeLocal(new Date(reminder.triggerAt)) : ''
  form.time = reminder.time || '09:00'
  form.weekdays = [...(reminder.weekdays || [])]
  form.monthDays = [...(reminder.monthDays || [])]
  showForm.value = true
  actionMessage.value = ''
}

function toggleNumber(values: number[], value: number) {
  const index = values.indexOf(value)
  if (index >= 0) values.splice(index, 1)
  else values.push(value)
  values.sort((a, b) => a - b)
}

function formatCountdown(value?: string | null) {
  if (!value) return ''
  const diff = new Date(value).getTime() - now.value
  if (Number.isNaN(diff)) return ''
  if (diff <= 0) return t('countdown.due')
  const totalSeconds = Math.ceil(diff / 1000)
  const days = Math.floor(totalSeconds / 86400)
  const hours = Math.floor((totalSeconds % 86400) / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  if (days > 0) return t('countdown.days', { days, hours, minutes, seconds })
  if (hours > 0) return t('countdown.hours', { hours, minutes, seconds })
  if (minutes > 0) return t('countdown.minutes', { minutes, seconds })
  return t('countdown.seconds', { seconds })
}

function formatRule(reminder: Reminder) {
  if (reminder.type === 'once') {
    return reminder.triggerAt
      ? new Intl.DateTimeFormat(data.value.settings.language, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(reminder.triggerAt))
      : t('rule.unset')
  }
  if (reminder.type === 'weekly') {
    const prefix = t('rule.weekPrefix')
    const separator = t('common.listSeparator')
    const days = (reminder.weekdays || []).map((day) => `${prefix}${weekdayOptions.value[day - 1]?.label || day}`).join(separator)
    return `${days} ${reminder.time}`
  }
  if (reminder.type === 'monthly') {
    return t('rule.monthly', { days: (reminder.monthDays || []).join(t('common.listSeparator')), time: reminder.time || '' })
  }
  return `${typeLabels.value[reminder.type]} ${reminder.time}`
}

function formatNext(reminder: Reminder) {
  const value = reminder.nextTriggerAt || reminder.triggerAt
  const countdown = formatCountdown(value)
  return countdown ? `${formatRule(reminder)} · ${countdown}` : formatRule(reminder)
}

// 串行写入并在实际发送时读取最新数据，防止旧快照最后落盘。
let persistToken = 0
let persistQueue: Promise<unknown> = Promise.resolve()
async function persist(popupFullscreen?: boolean) {
  const token = ++persistToken
  const operation = persistQueue.then(() => invoke<AppData>('save_data', {
    data: data.value,
    popupFullscreen: popupFullscreen ?? null,
  }))
  persistQueue = operation.catch(() => {})
  const saved = await operation
  if (token === persistToken) data.value = saved
}

async function saveReminder() {
  const title = form.title.trim()
  if (!title || (form.type === 'once' && !form.triggerAt) || (form.type !== 'once' && !form.time)) {
    actionMessage.value = t('validation.complete')
    return
  }
  if (form.type === 'weekly' && form.weekdays.length === 0) {
    actionMessage.value = t('validation.weekday')
    return
  }
  if (form.type === 'monthly' && form.monthDays.length === 0) {
    actionMessage.value = t('validation.monthDay')
    return
  }

  const existing = editingId.value
    ? data.value.reminders.find((item) => item.id === editingId.value)
    : undefined
  const reminder: Reminder = {
    id: editingId.value || crypto.randomUUID(),
    title,
    type: form.type,
    triggerAt: form.type === 'once' ? new Date(form.triggerAt).toISOString() : null,
    time: form.type === 'once' ? null : form.time,
    weekdays: form.type === 'weekly' ? [...form.weekdays] : [],
    monthDays: form.type === 'monthly' ? [...form.monthDays] : [],
    enabled: existing?.enabled ?? true,
    nextTriggerAt: null,
  }

  const previous = [...data.value.reminders]
  const index = data.value.reminders.findIndex((item) => item.id === reminder.id)
  if (index >= 0) data.value.reminders.splice(index, 1, reminder)
  else data.value.reminders.push(reminder)
  try {
    await persist()
    showForm.value = false
    actionMessage.value = t('status.reminderSaved')
  } catch (error) {
    data.value.reminders = previous
    logError('save reminder', error)
    actionMessage.value = t('status.saveFailed')
  }
}

async function toggleReminder(reminder: Reminder) {
  const previous = reminder.enabled
  reminder.enabled = !reminder.enabled
  try {
    await persist()
  } catch (error) {
    reminder.enabled = previous
    logError('toggle reminder', error)
    actionMessage.value = t('status.saveFailed')
  }
}

async function removeReminder(id: string) {
  const reminder = data.value.reminders.find((item) => item.id === id)
  if (!reminder) return
  const confirmed = await ask(t('events.deleteConfirm', { title: reminder.title }), {
    title: t('events.deleteConfirmTitle'),
    kind: 'warning',
    okLabel: t('events.delete'),
    cancelLabel: t('common.cancel'),
  })
  if (!confirmed) return

  const previous = data.value.reminders
  data.value.reminders = data.value.reminders.filter((item) => item.id !== id)
  try {
    await persist()
    actionMessage.value = t('status.reminderDeleted')
  } catch (error) {
    data.value.reminders = previous
    logError('delete reminder', error)
    actionMessage.value = t('status.saveFailed')
  }
}

async function applyNativeTheme(theme: Theme) {
  try {
    await setTheme(theme === 'system' ? null : theme)
  } catch (error) {
    // The standalone Vite preview has no native title bar to update.
    logError('apply native theme', error)
  }
}

async function updateSetting<K extends keyof AppData['settings']>(key: K, value: AppData['settings'][K]) {
  const previous = data.value.settings
  data.value.settings = { ...previous, [key]: value }
  if (key === 'theme') await applyNativeTheme(value as Theme)
  try {
    await persist(key === 'popupFullscreen' ? value as boolean : undefined)
    await refreshTimers()
    if (key === 'systemNotificationEnabled') notificationError.value = ''
    return true
  } catch (error) {
    data.value.settings = previous
    if (key === 'theme') await applyNativeTheme(previous.theme)
    logError(`save setting: ${String(key)}`, error)
    actionMessage.value = t('status.saveFailed')
    return false
  }
}

async function applySetting(key: keyof AppData['settings'], value: AppData['settings'][keyof AppData['settings']]) {
  if (key === 'autostart') {
    await updateAutostart(Boolean(value))
    return
  }
  if (key === 'systemNotificationEnabled' && value === true) {
    // macOS 上被用户拒绝过的通知会静默失败，开启前先确认权限。
    if (!await ensureNotificationPermission()) return
  }
  if (key === 'language') {
    await updateLanguage(value as Language)
    return
  }
  if (key === 'powerAction') {
    await updatePowerAction(value as AutomaticPowerAction)
    return
  }
  if (key === 'shutdownReminderTime') {
    if (/^\d{2}:\d{2}$/.test(String(value))) {
      await updateSetting('shutdownReminderTime', String(value))
    }
    return
  }
  await updateSetting(key, value as never)
}

async function ensureNotificationPermission() {
  try {
    if (await isPermissionGranted()) return true
    if (await requestPermission() === 'granted') return true
  } catch {
    // The standalone Vite preview has no permission bridge; do not block the toggle there.
    return true
  }
  notificationError.value = t('status.notificationDenied')
  return false
}

async function updateAutostart(value: boolean) {
  autostartError.value = ''
  try {
    if (value) await enable()
    else await disable()
    if (!await updateSetting('autostart', value)) {
      if (value) await disable()
      else await enable()
    }
  } catch (error) {
    logError('update autostart', error)
    autostartError.value = t('status.autostartFailed', { error: formatError(error) })
  }
}

function localizedDefaultMessages(language: Language) {
  return {
    rest: translate(language, 'rest.defaultMessage'),
    shutdown: translate(language, 'power.defaultShutdownMessage'),
    lock: translate(language, 'power.defaultLockMessage'),
    restart: translate(language, 'power.defaultRestartMessage'),
  }
}

async function updateLanguage(language: Language) {
  const previous = data.value.settings
  const currentDefaults = localizedDefaultMessages(previous.language)
  const nextDefaults = localizedDefaultMessages(language)
  const currentPowerDefault = currentDefaults[previous.powerAction]
  const nextPowerDefault = nextDefaults[previous.powerAction]
  data.value.settings = {
    ...previous,
    language,
    restMessage: previous.restMessage === currentDefaults.rest ? nextDefaults.rest : previous.restMessage,
    shutdownReminderMessage: previous.shutdownReminderMessage === currentPowerDefault
      ? nextPowerDefault
      : previous.shutdownReminderMessage,
  }
  restMessageDraft.value = data.value.settings.restMessage
  shutdownMessageDraft.value = data.value.settings.shutdownReminderMessage
  try {
    await persist()
    await refreshTimers()
  } catch (error) {
    data.value.settings = previous
    restMessageDraft.value = previous.restMessage
    shutdownMessageDraft.value = previous.shutdownReminderMessage
    logError('save language', error)
    actionMessage.value = t('status.saveFailed')
  }
}

async function updatePowerAction(value: AutomaticPowerAction) {
  const previous = data.value.settings
  const localizedDefaults = localizedDefaultMessages(previous.language)
  const defaults = (['zh-CN', 'en'] as const).flatMap((language) => {
    const messages = localizedDefaultMessages(language)
    return [messages.shutdown, messages.lock, messages.restart]
  })
  data.value.settings = {
    ...previous,
    powerAction: value,
    shutdownReminderMessage: defaults.includes(previous.shutdownReminderMessage)
      ? localizedDefaults[value]
      : previous.shutdownReminderMessage,
  }
  shutdownMessageDraft.value = data.value.settings.shutdownReminderMessage
  try {
    await persist()
    await refreshTimers()
  } catch (error) {
    data.value.settings = previous
    logError('update power action', error)
    actionMessage.value = t('status.saveFailed')
  }
}

async function updateRestInterval(raw: string) {
  const value = Number(raw)
  if (Number.isInteger(value) && value >= 1 && value <= 1440) {
    await updateSetting('restIntervalMinutes', value)
  }
}

async function updateShutdownTimeValue(raw: string) {
  if (/^\d{2}:\d{2}$/.test(raw)) {
    await updateSetting('shutdownReminderTime', raw)
  }
}

async function saveRestMessage(value: string) {
  const previous = data.value.settings
  restMessageDraft.value = value
  data.value.settings = { ...previous, restMessage: value }
  try {
    await persist()
  } catch (error) {
    data.value.settings = previous
    restMessageDraft.value = previous.restMessage
    logError('save rest message', error)
    actionMessage.value = t('status.saveFailed')
  }
}

async function saveShutdownMessage(value: string) {
  const previous = data.value.settings
  shutdownMessageDraft.value = value
  data.value.settings = { ...previous, shutdownReminderMessage: value }
  try {
    await persist()
  } catch (error) {
    data.value.settings = previous
    shutdownMessageDraft.value = previous.shutdownReminderMessage
    logError('save power message', error)
    actionMessage.value = t('status.saveFailed')
  }
}

// 选图后由 Rust 复制成配置目录里的固定文件名，清空则是删掉该文件。
// 配置里不记录图片信息，弹窗按文件是否存在决定要不要显示背景。
async function pickPopupImage() {
  actionMessage.value = ''
  try {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: t('settings.imageFilter'), extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp', 'avif'] }],
    })
    if (typeof path !== 'string') return
    const dataUrl = await invoke<string>('import_popup_image', { source: path })
    popupBackgroundPreview.value = dataUrl
    actionMessage.value = t('status.imageSaved')
  } catch (error) {
    logError('pick popup image', error)
    actionMessage.value = t('status.imageFailed')
  }
}

async function clearPopupImage() {
  actionMessage.value = ''
  try {
    await invoke('clear_popup_image')
    popupBackgroundPreview.value = ''
    actionMessage.value = t('status.imageCleared')
  } catch (error) {
    logError('clear popup image', error)
    actionMessage.value = t('status.imageFailed')
  }
}

async function loadPopupImagePreview() {
  try {
    popupBackgroundPreview.value = (await invoke<string | null>('read_popup_image')) ?? ''
  } catch {
    popupBackgroundPreview.value = ''
  }
}

async function importData() {
  actionMessage.value = ''
  try {
    const path = await open({ multiple: false, directory: false, filters: [{ name: t('dialog.backupName'), extensions: ['json'] }] })
    if (typeof path === 'string') {
      data.value = await invoke<AppData>('import_data', { path })
      restMessageDraft.value = data.value.settings.restMessage
      shutdownMessageDraft.value = data.value.settings.shutdownReminderMessage
      await applyNativeTheme(data.value.settings.theme)
      actionMessage.value = t('status.imported')
      await refreshTimers()
    }
  } catch (error) {
    logError('import data', error)
    actionMessage.value = t('status.importFailed')
  }
}

async function exportData() {
  actionMessage.value = ''
  try {
    const path = await save({ defaultPath: 'RemindOn-backup.json', filters: [{ name: t('dialog.backupName'), extensions: ['json'] }] })
    if (typeof path === 'string') {
      await invoke('export_data', { path })
      actionMessage.value = t('status.exported')
    }
  } catch (error) {
    logError('export data', error)
    actionMessage.value = t('status.exportFailed')
  }
}

// 只恢复参数默认值，提醒列表原样保留。自启与主题要同步到系统层，所以单独处理。
async function resetSettings() {
  actionMessage.value = ''
  const confirmed = await confirm(t('settings.resetConfirmBody'), {
    title: t('settings.resetConfirmTitle'),
    kind: 'warning',
  })
  if (!confirmed) return

  await persistQueue
  const previous = data.value.settings
  const defaults = defaultData().settings
  data.value.settings = {
    ...defaults,
    language: previous.language,
    restMessage: translate(previous.language, 'rest.defaultMessage'),
    shutdownReminderMessage: translate(previous.language, 'power.defaultShutdownMessage'),
  }
  restMessageDraft.value = data.value.settings.restMessage
  shutdownMessageDraft.value = data.value.settings.shutdownReminderMessage
  try {
    if (previous.autostart) await disable()
    await setTheme(data.value.settings.theme === 'system' ? null : data.value.settings.theme)
    await persist(defaults.popupFullscreen)
    await refreshTimers()
  } catch (error) {
    data.value.settings = previous
    restMessageDraft.value = previous.restMessage
    shutdownMessageDraft.value = previous.shutdownReminderMessage
    const rollback = await Promise.allSettled([
      previous.autostart ? enable() : disable(),
      setTheme(previous.theme === 'system' ? null : previous.theme),
    ])
    for (const result of rollback) {
      if (result.status === 'rejected') logError('restore system settings after reset', result.reason)
    }
    if (rollback[0].status === 'rejected') {
      autostartError.value = t('status.autostartFailed', { error: formatError(rollback[0].reason) })
    }
    logError('reset settings', error)
    actionMessage.value = t('status.saveFailed')
    return
  }
  autostartError.value = ''
  notificationError.value = ''
  // 参数已落盘，删图失败时保留真实图片状态并提示，不回滚成旧参数。
  try {
    await invoke('clear_popup_image')
    popupBackgroundPreview.value = ''
    actionMessage.value = t('status.resetDone')
  } catch (error) {
    logError('clear popup image on reset', error)
    await loadPopupImagePreview()
    actionMessage.value = t('status.imageFailed')
  }
}

async function testNotification(kind: TestReminderKind) {
  actionMessage.value = ''
  notificationError.value = ''
  try {
    await persist()
    await invoke('test_reminder', { kind })
  } catch (error) {
    logError('test notification', error)
    const message = t('status.notificationFailed', { error: formatError(error) })
    notificationError.value = message
    actionMessage.value = message
  }
}

async function refreshTimers() {
  if (isPopup) return
  const refreshToken = ++timerRefreshToken
  const [rest, shutdown] = await Promise.allSettled([
    invoke<RestTimerStatus>('get_rest_timer_status'),
    invoke<string | null>('get_next_shutdown_trigger'),
  ])
  if (refreshToken !== timerRefreshToken) return
  if (rest.status === 'fulfilled') {
    nextRestTrigger.value = rest.value.nextTriggerAt
    restIsActive.value = rest.value.isResting
  }
  nextShutdownTrigger.value = shutdown.status === 'fulfilled' ? shutdown.value : null
  now.value = Date.now()
}

function applyRestTimerStatus(status: RestTimerStatus) {
  timerRefreshToken += 1
  restIsActive.value = status.isResting
  nextRestTrigger.value = status.nextTriggerAt
  now.value = Date.now()
}

function handleMainWindowEscape(event: KeyboardEvent) {
  if (event.key !== 'Escape' || event.repeat) return
  void invoke('hide_idle_window').catch((error) => logError('hide main window', error))
}

onMounted(async () => {
  if (isPopup) return
  window.addEventListener('keydown', handleMainWindowEscape)
  void loadUpdateStatus?.()
  void loadPopupImagePreview()
  try {
    unlistenNavigation = await getCurrentWindow().listen<View>('navigate-to', (event) => {
      currentView.value = event.payload
    })
    data.value = await invoke<AppData>('load_data')
    restMessageDraft.value = data.value.settings.restMessage
    shutdownMessageDraft.value = data.value.settings.shutdownReminderMessage
    try {
      const pendingNavigation = await invoke<View | null>('take_pending_navigation')
      if (pendingNavigation) currentView.value = pendingNavigation
    } catch {
      // The standalone preview and older bridge mocks do not expose navigation state.
    }
    await applyNativeTheme(data.value.settings.theme)
    try {
      appVersion.value = (await getVersion()).replace(/\.0$/, '')
    } catch {
      // Keep the package-version fallback in standalone preview mode.
    }
    try {
      data.value.settings.autostart = await isEnabled()
    } catch {
      // Keep the saved value when the platform autostart API is unavailable.
    }
    unlistenWindowFocus = await getCurrentWindow().onFocusChanged(() => {
      void refreshTimers()
    })
    unlisten = await getCurrentWindow().listen<ReminderTriggeredEvent>('reminder-triggered', async () => {
      try {
        data.value = await invoke<AppData>('load_data')
      } catch {
        // Keep the current data while the backend is temporarily unavailable.
      }
      await refreshTimers()
    })
    unlistenRestTimer = await getCurrentWindow().listen<RestTimerStatus>('rest-timer-updated', (event) => {
      applyRestTimerStatus(event.payload)
    })
    unlistenNotificationFailure = await getCurrentWindow().listen<string>('notification-failed', (event) => {
      const message = t('status.notificationFailed', { error: event.payload })
      notificationError.value = message
      actionMessage.value = message
    })
    // 外部切换只更新相关字段，不能覆盖正在编辑的其他参数。
    unlistenSettingsSync = await getCurrentWindow().listen<boolean>('popup-fullscreen-updated', (event) => {
      persistToken += 1
      data.value.settings.popupFullscreen = event.payload
    })
    await refreshTimers()
    clockTimer = window.setInterval(() => {
      now.value = Date.now()
    }, 1000)
  } catch (error) {
    logError('load application data', error)
    actionMessage.value = t('status.loadFailed')
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleMainWindowEscape)
  disposeUpdater()
  unlisten?.()
  unlistenNavigation?.()
  unlistenRestTimer?.()
  unlistenNotificationFailure?.()
  unlistenSettingsSync?.()
  unlistenWindowFocus?.()
  if (clockTimer) window.clearInterval(clockTimer)
  if (actionMessageTimer) window.clearTimeout(actionMessageTimer)
})
</script>

<template>
  <ReminderPopup v-if="isPopup" />
  <div v-else :class="['app-shell', `theme-${data.settings.theme}`, `accent-${data.settings.accentColor}`]">
    <AppSidebar
      :current-view="currentView"
      :language="data.settings.language"
      :app-version="appVersion"
      :has-update="Boolean(newVersion)"
      @navigate="currentView = $event"
    />

    <main :class="['content', { 'content-about': currentView === 'about' }]">
      <div v-if="newVersion && currentView !== 'about'" class="update-banner" role="status">
        <span>{{ t('update.available', { version: newVersion }) }}</span>
        <button class="button" type="button" @click="currentView = 'about'">{{ t('update.view') }}</button>
      </div>

      <EventsView
        v-if="currentView === 'events'"
        :language="data.settings.language"
        :subtitle="t('events.subtitle')"
        :show-form="showForm"
        :editing-id="editingId"
        :form="form"
        :frequency-options="frequencyOptions"
        :weekday-options="weekdayOptions"
        :type-labels="typeLabels"
        :reminders="sortedReminders"
        :action-message="actionMessage"
        :format-next="formatNext"
        @test-notification="testNotification('event')"
        @add="openAddForm"
        @close-form="showForm = false"
        @save="saveReminder"
        @update:form="patchForm"
        @toggle-weekday="toggleNumber(form.weekdays, $event)"
        @toggle-month-day="toggleNumber(form.monthDays, $event)"
        @toggle-reminder="toggleReminder"
        @edit-reminder="editReminder"
        @remove-reminder="removeReminder"
      />

      <RestView
        v-else-if="currentView === 'rest'"
        :language="data.settings.language"
        :subtitle="t('rest.subtitle')"
        :enabled="data.settings.restEnabled"
        :interval-minutes="data.settings.restIntervalMinutes"
        :message="restMessageDraft"
        :progress="restProgress"
        :status="restStatusText"
        :action-message="actionMessage"
        @test-notification="testNotification('rest')"
        @update:enabled="updateSetting('restEnabled', $event)"
        @update:interval="updateRestInterval"
        @update:message="restMessageDraft = $event"
        @message-committed="saveRestMessage($event)"
      />

      <PowerView
        v-else-if="currentView === 'power'"
        :language="data.settings.language"
        :subtitle="t('power.subtitle')"
        :settings="data.settings"
        :status="powerStatusText"
        :action-message="actionMessage"
        :power-action-options="powerActionOptions"
        @test-notification="testNotification('power')"
        @update:enabled="updateSetting('shutdownReminderEnabled', $event)"
        @update:power-action="updatePowerAction"
        @update:time="updateShutdownTimeValue"
        @update:message="shutdownMessageDraft = $event"
        @message-committed="saveShutdownMessage($event)"
      />

      <SettingsView
        v-else-if="currentView === 'settings'"
        :language="data.settings.language"
        :settings="data.settings"
        :accent-colors="accentColors"
        :autostart-error="autostartError"
        :notification-error="notificationError"
        :action-message="actionMessage"
        :can-reset="canResetSettings"
        :background-preview="popupBackgroundPreview"
        @update:setting="applySetting"
        @import-data="importData"
        @export-data="exportData"
        @reset-settings="resetSettings"
        @pick-popup-image="pickPopupImage"
        @clear-popup-image="clearPopupImage"
      />

      <AboutView
        v-else
        :language="data.settings.language"
        :app-version="appVersion"
        :is-store-build="isStoreBuild"
        :update-mode="updateMode"
        :update-status="updateStatus"
        :update-status-text="updateStatusText"
        :new-version="newVersion"
        :update-progress="updateProgress"
        :update-error="updateError"
        :update-busy="updateBusy"
        @check-for-updates="checkForUpdates()"
        @open-releases="openReleases"
        @open-author-page="openAuthorPage"
      />
    </main>
  </div>
</template>
