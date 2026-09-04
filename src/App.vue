<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { getVersion, setTheme } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import {
  BellRing, CalendarClock, Check, Clock3, Coffee, Download, Info, LockKeyhole,
  Pencil, Play, Plus, Power, RotateCw, Settings2, Trash2, Upload, X,
} from '@lucide/vue'
import ReminderPopup from './components/ReminderPopup.vue'
import brandIcon from './assets/remindon.svg'
import donationCode from './assets/donate.png'
import { translate } from './i18n'
import type { MessageKey } from './i18n'
import type { AccentColor, AppData, PowerAction, Reminder, ReminderTriggeredEvent, ReminderType, RestTimerStatus, Theme } from './types'
import { defaultData } from './types'

type View = 'events' | 'rest' | 'power' | 'settings' | 'about'
type EditableReminderType = Exclude<ReminderType, 'interval'>
type AutomaticPowerAction = Extract<PowerAction, 'shutdown' | 'lock' | 'restart'>

const isPopup = window.location.hash === '#/reminder'
const data = ref<AppData>(defaultData())
const currentView = ref<View>('events')
const showForm = ref(false)
const editingId = ref<string | null>(null)
const actionMessage = ref('')
const now = ref(Date.now())
const nextRestTrigger = ref<string | null>(null)
const restIsActive = ref(false)
const nextShutdownTrigger = ref<string | null>(null)
const appVersion = ref('0.3.2')
let unlisten: (() => void) | undefined
let unlistenNavigation: (() => void) | undefined
let unlistenRestTimer: (() => void) | undefined
let clockTimer: number | undefined

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(data.value.settings.language, key, params)
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

const restProgress = computed(() => {
  if (restIsActive.value) return 100
  if (!data.value.settings.restEnabled || !nextRestTrigger.value) return 0
  const remaining = new Date(nextRestTrigger.value).getTime() - now.value
  const total = data.value.settings.restIntervalMinutes * 60 * 1000
  if (!Number.isFinite(remaining) || total <= 0) return 0
  return Math.max(0, Math.min(100, (1 - remaining / total) * 100))
})

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
    const separator = data.value.settings.language === 'zh-CN' ? '、' : ', '
    const days = (reminder.weekdays || []).map((day) => `${prefix}${weekdayOptions.value[day - 1]?.label || day}`).join(separator)
    return `${days} ${reminder.time}`
  }
  if (reminder.type === 'monthly') {
    return t('rule.monthly', { days: (reminder.monthDays || []).join(data.value.settings.language === 'zh-CN' ? '、' : ', '), time: reminder.time || '' })
  }
  return `${typeLabels.value[reminder.type]} ${reminder.time}`
}

function formatNext(reminder: Reminder) {
  const value = reminder.nextTriggerAt || reminder.triggerAt
  const countdown = formatCountdown(value)
  return countdown ? `${formatRule(reminder)} · ${countdown}` : formatRule(reminder)
}

async function persist() {
  data.value = await invoke<AppData>('save_data', { data: data.value })
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
    actionMessage.value = t('status.saveFailed', { error: String(error) })
  }
}

async function toggleReminder(reminder: Reminder) {
  const previous = reminder.enabled
  reminder.enabled = !reminder.enabled
  try {
    await persist()
  } catch (error) {
    reminder.enabled = previous
    actionMessage.value = t('status.saveFailed', { error: String(error) })
  }
}

async function removeReminder(id: string) {
  const previous = data.value.reminders
  data.value.reminders = data.value.reminders.filter((item) => item.id !== id)
  try {
    await persist()
    actionMessage.value = t('status.reminderDeleted')
  } catch (error) {
    data.value.reminders = previous
    actionMessage.value = t('status.saveFailed', { error: String(error) })
  }
}

async function applyNativeTheme(theme: Theme) {
  try {
    await setTheme(theme === 'system' ? null : theme)
  } catch {
    // The standalone Vite preview has no native title bar to update.
  }
}

async function updateSetting<K extends keyof AppData['settings']>(key: K, value: AppData['settings'][K]) {
  const previous = data.value.settings
  data.value.settings = { ...previous, [key]: value }
  if (key === 'theme') await applyNativeTheme(value as Theme)
  try {
    await persist()
    await refreshTimers()
    return true
  } catch (error) {
    data.value.settings = previous
    if (key === 'theme') await applyNativeTheme(previous.theme)
    actionMessage.value = t('status.saveFailed', { error: String(error) })
    return false
  }
}

async function updateAutostart(value: boolean) {
  try {
    if (value) await enable()
    else await disable()
    if (!await updateSetting('autostart', value)) {
      if (value) await disable()
      else await enable()
    }
  } catch (error) {
    actionMessage.value = t('status.autostartFailed', { error: String(error) })
  }
}

async function updatePowerAction(value: AutomaticPowerAction) {
  const previous = data.value.settings
  const defaults = [
    translate('zh-CN', 'power.defaultShutdownMessage'),
    translate('zh-CN', 'power.defaultLockMessage'),
    translate('zh-CN', 'power.defaultRestartMessage'),
    translate('en', 'power.defaultShutdownMessage'),
    translate('en', 'power.defaultLockMessage'),
    translate('en', 'power.defaultRestartMessage'),
  ]
  const messageKey = `power.default${value[0].toUpperCase()}${value.slice(1)}Message` as MessageKey
  data.value.settings = {
    ...previous,
    powerAction: value,
    shutdownReminderMessage: defaults.includes(previous.shutdownReminderMessage)
      ? t(messageKey)
      : previous.shutdownReminderMessage,
  }
  try {
    await persist()
    await refreshTimers()
  } catch (error) {
    data.value.settings = previous
    actionMessage.value = t('status.saveFailed', { error: String(error) })
  }
}

async function updateRestInterval(event: Event) {
  const value = (event.target as HTMLInputElement).valueAsNumber
  if (Number.isInteger(value) && value >= 1 && value <= 1440) {
    await updateSetting('restIntervalMinutes', value)
  }
}

async function updateShutdownTime(event: Event) {
  const value = (event.target as HTMLInputElement).value
  if (/^\d{2}:\d{2}$/.test(value)) {
    await updateSetting('shutdownReminderTime', value)
  }
}

async function importData() {
  actionMessage.value = ''
  try {
    const path = await open({ multiple: false, directory: false, filters: [{ name: t('dialog.backupName'), extensions: ['json'] }] })
    if (typeof path === 'string') {
      data.value = await invoke<AppData>('import_data', { path })
      await applyNativeTheme(data.value.settings.theme)
      actionMessage.value = t('status.imported')
      await refreshTimers()
    }
  } catch (error) {
    actionMessage.value = t('status.importFailed', { error: String(error) })
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
    actionMessage.value = t('status.exportFailed', { error: String(error) })
  }
}

async function testNotification() {
  actionMessage.value = ''
  try {
    await persist()
    await invoke('test_reminder')
  } catch (error) {
    actionMessage.value = t('status.testFailed', { error: String(error) })
  }
}

async function refreshTimers() {
  if (isPopup) return
  const [rest, shutdown] = await Promise.allSettled([
    invoke<RestTimerStatus>('get_rest_timer_status'),
    invoke<string | null>('get_next_shutdown_trigger'),
  ])
  nextRestTrigger.value = rest.status === 'fulfilled' ? rest.value.nextTriggerAt : null
  restIsActive.value = rest.status === 'fulfilled' && rest.value.isResting
  nextShutdownTrigger.value = shutdown.status === 'fulfilled' ? shutdown.value : null
  now.value = Date.now()
}

onMounted(async () => {
  if (isPopup) return
  try {
    unlistenNavigation = await listen<View>('navigate-to', (event) => {
      currentView.value = event.payload
    })
    data.value = await invoke<AppData>('load_data')
    await applyNativeTheme(data.value.settings.theme)
    try {
      appVersion.value = await getVersion()
    } catch {
      // Keep the package-version fallback in standalone preview mode.
    }
    try {
      data.value.settings.autostart = await isEnabled()
    } catch {
      // Keep the saved value when the platform autostart API is unavailable.
    }
    await refreshTimers()
    unlisten = await listen<ReminderTriggeredEvent>('reminder-triggered', async () => {
      data.value = await invoke<AppData>('load_data')
      await refreshTimers()
    })
    unlistenRestTimer = await listen('rest-timer-updated', () => void refreshTimers())
    clockTimer = window.setInterval(() => {
      now.value = Date.now()
    }, 1000)
  } catch (error) {
    actionMessage.value = t('status.loadFailed', { error: String(error) })
  }
})

onUnmounted(() => {
  unlisten?.()
  unlistenNavigation?.()
  unlistenRestTimer?.()
  if (clockTimer) window.clearInterval(clockTimer)
})
</script>

<template>
  <ReminderPopup v-if="isPopup" />
  <div v-else :class="['app-shell', `theme-${data.settings.theme}`, `accent-${data.settings.accentColor}`]">
    <aside class="sidebar">
      <div class="brand">
        <img class="brand-mark" :src="brandIcon" alt="" />
        <div class="brand-copy"><strong>RemindOn</strong><span>{{ t('app.tagline') }}</span></div>
      </div>
      <nav class="nav-list" aria-label="Navigation">
        <button :class="['nav-item', { active: currentView === 'events' }]" @click="currentView = 'events'"><CalendarClock :size="17" /><span>{{ t('nav.events') }}</span></button>
        <button :class="['nav-item', { active: currentView === 'rest' }]" @click="currentView = 'rest'"><Coffee :size="17" /><span>{{ t('nav.rest') }}</span></button>
        <button :class="['nav-item', { active: currentView === 'power' }]" @click="currentView = 'power'"><Power :size="17" /><span>{{ t('nav.power') }}</span></button>
        <button :class="['nav-item', { active: currentView === 'settings' }]" @click="currentView = 'settings'"><Settings2 :size="17" /><span>{{ t('nav.settings') }}</span></button>
        <button :class="['nav-item', { active: currentView === 'about' }]" @click="currentView = 'about'"><Info :size="17" /><span>{{ t('nav.about') }}</span></button>
      </nav>
      <div class="sidebar-footer">RemindOn v{{ appVersion }}</div>
    </aside>

    <main class="content">
      <section v-if="currentView === 'events'" class="page-section">
        <header class="page-header"><div><p class="eyebrow">REMINDERS</p><h1>{{ t('events.title') }}</h1><p class="page-subtitle">{{ t('events.subtitle') }}</p></div><button class="button button-primary" type="button" @click="openAddForm"><Plus :size="15" />{{ t('events.add') }}</button></header>

        <div v-if="showForm" class="form-panel">
          <div class="form-heading"><div><p class="eyebrow">REMINDER</p><h2>{{ editingId ? t('events.editTitle') : t('events.addTitle') }}</h2></div><button class="icon-button" type="button" :aria-label="t('common.close')" :title="t('common.close')" @click="showForm = false"><X :size="18" /></button></div>
          <label class="field"><span>{{ t('events.content') }}</span><input v-model="form.title" type="text" maxlength="120" :placeholder="t('events.contentPlaceholder')" /></label>
          <div class="field"><span>{{ t('events.frequency') }}</span><div class="segmented frequency-segments"><button v-for="option in frequencyOptions" :key="option.value" :class="{ selected: form.type === option.value }" type="button" @click="form.type = option.value">{{ option.label }}</button></div></div>
          <div class="field-row"><label v-if="form.type === 'once'" class="field"><span>{{ t('events.reminderTime') }}</span><input v-model="form.triggerAt" type="datetime-local" /></label><label v-else class="field"><span>{{ t('events.exactTime') }}</span><input v-model="form.time" type="time" /></label></div>
          <div v-if="form.type === 'weekly'" class="field"><span>{{ t('events.selectWeekday') }}</span><div class="choice-grid weekday-grid"><button v-for="day in weekdayOptions" :key="day.value" :class="{ selected: form.weekdays.includes(day.value) }" type="button" @click="toggleNumber(form.weekdays, day.value)">{{ t('rule.weekPrefix') }}{{ day.label }}</button></div></div>
          <div v-if="form.type === 'monthly'" class="field"><span>{{ t('events.selectDate') }}</span><div class="choice-grid month-grid"><button v-for="day in 31" :key="day" :class="{ selected: form.monthDays.includes(day) }" type="button" @click="toggleNumber(form.monthDays, day)">{{ day }}</button></div><small>{{ t('events.missingDateHint') }}</small></div>
          <small v-if="actionMessage" class="status-message form-message">{{ actionMessage }}</small>
          <div class="form-actions"><button class="button" type="button" @click="showForm = false">{{ t('common.cancel') }}</button><button class="button button-primary" type="button" @click="saveReminder"><Check :size="15" />{{ t('events.save') }}</button></div>
        </div>

        <div v-if="sortedReminders.length" class="reminder-list">
          <article v-for="reminder in sortedReminders" :key="reminder.id" :class="['reminder-row', { disabled: !reminder.enabled }]">
            <div :class="['reminder-status', { enabled: reminder.enabled }]"></div>
            <div class="reminder-main"><div class="reminder-title"><strong>{{ reminder.title }}</strong><span class="type-chip">{{ typeLabels[reminder.type] }}</span></div><span>{{ formatNext(reminder) }}</span></div>
            <button class="switch" :class="{ on: reminder.enabled }" type="button" :aria-label="reminder.enabled ? t('common.disabled') : t('common.enabled')" @click="toggleReminder(reminder)"><span></span></button>
            <button class="icon-button row-action" type="button" :aria-label="t('events.edit')" :title="t('events.edit')" @click="editReminder(reminder)"><Pencil :size="15" /></button>
            <button class="icon-button row-action danger" type="button" :aria-label="t('events.delete')" :title="t('events.delete')" @click="removeReminder(reminder.id)"><Trash2 :size="15" /></button>
          </article>
        </div>
        <div v-else-if="!showForm" class="empty-state"><div class="empty-icon"><CalendarClock :size="22" /></div><h2>{{ t('events.emptyTitle') }}</h2><p>{{ t('events.emptyBody') }}</p><button class="button" type="button" @click="openAddForm"><Plus :size="15" />{{ t('events.addFirst') }}</button></div>
      </section>

      <section v-else-if="currentView === 'rest'" class="page-section narrow-section">
        <header class="page-header compact-header"><div><p class="eyebrow">BREAK</p><h1>{{ t('rest.title') }}</h1><p class="page-subtitle">{{ t('rest.subtitle') }}</p></div></header>
        <div class="status-panel">
          <div class="status-panel-top"><span class="status-icon"><Coffee :size="18" /></span><div><span class="card-label">{{ t('rest.next') }}</span><strong>{{ data.settings.restEnabled ? (restIsActive ? t('rest.resting') : (nextRestTrigger ? formatCountdown(nextRestTrigger) : t('common.calculating'))) : t('common.paused') }}</strong></div><label class="setting-toggle compact-toggle"><input :checked="data.settings.restEnabled" type="checkbox" @change="updateSetting('restEnabled', ($event.target as HTMLInputElement).checked)" /></label></div>
          <div class="progress-track"><span :style="{ width: `${restProgress}%` }"></span></div><p>{{ t('rest.scheduleHint') }}</p>
        </div>
        <div class="settings-group">
          <div class="setting-card"><div><strong>{{ t('rest.interval') }}</strong><span>{{ t('rest.intervalHint') }}</span></div><label class="number-field"><input :value="data.settings.restIntervalMinutes" type="number" min="1" max="1440" @input="updateRestInterval" /><span>{{ t('common.minutes') }}</span></label></div>
          <label class="setting-card stacked-setting"><div><strong>{{ t('rest.message') }}</strong><span>{{ t('rest.messageHint') }}</span></div><input :value="data.settings.restMessage" type="text" maxlength="120" @change="updateSetting('restMessage', ($event.target as HTMLInputElement).value)" /></label>
        </div>
        <small v-if="actionMessage" class="status-message page-message">{{ actionMessage }}</small>
      </section>

      <section v-else-if="currentView === 'power'" class="page-section narrow-section">
        <header class="page-header compact-header"><div><p class="eyebrow">SYSTEM</p><h1>{{ t('power.title') }}</h1><p class="page-subtitle">{{ t('power.subtitle') }}</p></div></header>
        <div class="status-panel power-status">
          <div class="status-panel-top"><span class="status-icon"><Power :size="18" /></span><div><span class="card-label">{{ t('power.next') }}</span><strong>{{ data.settings.shutdownReminderEnabled ? (nextShutdownTrigger ? formatCountdown(nextShutdownTrigger) : t('common.calculating')) : t('common.paused') }}</strong></div></div>
        </div>
        <div class="settings-group">
          <label class="setting-card setting-toggle"><div><strong>{{ t('power.enable') }}</strong><span>{{ t('power.enableHint') }}</span></div><input :checked="data.settings.shutdownReminderEnabled" type="checkbox" @change="updateSetting('shutdownReminderEnabled', ($event.target as HTMLInputElement).checked)" /></label>
          <div class="setting-card setting-choice power-choice"><div><strong>{{ t('power.action') }}</strong><span>{{ t('power.actionHint') }}</span></div><div class="segmented power-segments"><button v-for="option in powerActionOptions" :key="option.value" :class="{ selected: data.settings.powerAction === option.value }" type="button" @click="updatePowerAction(option.value)"><component :is="option.icon" :size="14" />{{ option.label }}</button></div></div>
          <label class="setting-card"><div><strong>{{ t('power.dailyTime') }}</strong><span>{{ t('power.dailyTimeHint') }}</span></div><span class="time-control"><Clock3 :size="15" /><input class="time-input" :value="data.settings.shutdownReminderTime" type="time" @input="updateShutdownTime" /></span></label>
          <label class="setting-card stacked-setting"><div><strong>{{ t('power.message') }}</strong><span>{{ t('power.messageHint') }}</span></div><input :value="data.settings.shutdownReminderMessage" type="text" maxlength="120" @change="updateSetting('shutdownReminderMessage', ($event.target as HTMLInputElement).value)" /></label>
        </div>
        <small v-if="actionMessage" class="status-message page-message">{{ actionMessage }}</small>
      </section>

      <section v-else-if="currentView === 'settings'" class="page-section narrow-section">
        <header class="page-header compact-header"><div><p class="eyebrow">PREFERENCES</p><h1>{{ t('settings.title') }}</h1><p class="page-subtitle">{{ t('settings.subtitle') }}</p></div></header>
        <div class="settings-group">
          <div class="setting-card setting-choice"><div><strong>{{ t('settings.language') }}</strong><span>{{ t('settings.languageHint') }}</span></div><div class="segmented"><button :class="{ selected: data.settings.language === 'zh-CN' }" type="button" @click="updateSetting('language', 'zh-CN')">{{ t('settings.zh') }}</button><button :class="{ selected: data.settings.language === 'en' }" type="button" @click="updateSetting('language', 'en')">{{ t('settings.en') }}</button></div></div>
          <label class="setting-card setting-toggle"><div><strong>{{ t('settings.autostart') }}</strong><span>{{ t('settings.autostartHint') }}</span></div><input :checked="data.settings.autostart" type="checkbox" @change="updateAutostart(($event.target as HTMLInputElement).checked)" /></label>
          <label class="setting-card setting-toggle"><div><strong>{{ t('settings.startHidden') }}</strong><span>{{ t('settings.startHiddenHint') }}</span></div><input :checked="data.settings.minimizeToTray" type="checkbox" @change="updateSetting('minimizeToTray', ($event.target as HTMLInputElement).checked)" /></label>
          <label class="setting-card setting-toggle"><div><strong>{{ t('settings.alwaysOnTop') }}</strong><span>{{ t('settings.alwaysOnTopHint') }}</span></div><input :checked="data.settings.popupAlwaysOnTop" type="checkbox" @change="updateSetting('popupAlwaysOnTop', ($event.target as HTMLInputElement).checked)" /></label>
          <div class="setting-card setting-choice"><div><strong>{{ t('settings.notificationMode') }}</strong><span>{{ t('settings.notificationModeHint') }}</span></div><div class="setting-control-row"><div class="segmented"><button :class="{ selected: data.settings.notificationMode === 'system' }" type="button" @click="updateSetting('notificationMode', 'system')">{{ t('settings.systemNotification') }}</button><button :class="{ selected: data.settings.notificationMode === 'popup' }" type="button" @click="updateSetting('notificationMode', 'popup')">{{ t('settings.softwareNotification') }}</button></div><button class="button" type="button" @click="testNotification"><Play :size="14" />{{ t('settings.testNotification') }}</button></div></div>
          <div class="setting-card setting-choice"><div><strong>{{ t('settings.notificationStyle') }}</strong><span>{{ t('settings.notificationStyleHint') }}</span></div><div class="segmented"><button :class="{ selected: data.settings.notificationStyle === 'compact' }" type="button" @click="updateSetting('notificationStyle', 'compact')">{{ t('settings.compact') }}</button><button :class="{ selected: data.settings.notificationStyle === 'standard' }" type="button" @click="updateSetting('notificationStyle', 'standard')">{{ t('settings.standard') }}</button><button :class="{ selected: data.settings.notificationStyle === 'prominent' }" type="button" @click="updateSetting('notificationStyle', 'prominent')">{{ t('settings.prominent') }}</button></div></div>
          <div class="setting-card setting-choice"><div><strong>{{ t('settings.appearance') }}</strong><span>{{ t('settings.appearanceHint') }}</span></div><div class="segmented"><button :class="{ selected: data.settings.theme === 'dark' }" type="button" @click="updateSetting('theme', 'dark')">{{ t('settings.dark') }}</button><button :class="{ selected: data.settings.theme === 'light' }" type="button" @click="updateSetting('theme', 'light')">{{ t('settings.light') }}</button><button :class="{ selected: data.settings.theme === 'system' }" type="button" @click="updateSetting('theme', 'system')">{{ t('settings.system') }}</button></div></div>
          <div class="setting-card color-setting"><div><strong>{{ t('settings.accent') }}</strong><span>{{ t('settings.accentHint') }}</span></div><div class="color-options"><button v-for="color in accentColors" :key="color" :class="['color-swatch', `swatch-${color}`, { selected: data.settings.accentColor === color }]" type="button" :aria-label="color" @click="updateSetting('accentColor', color)"></button></div></div>
        </div>
        <div class="data-actions"><div><strong>{{ t('settings.data') }}</strong><span>{{ t('settings.dataHint') }}</span></div><div class="action-row"><button class="button" type="button" @click="importData"><Upload :size="14" />{{ t('settings.import') }}</button><button class="button" type="button" @click="exportData"><Download :size="14" />{{ t('settings.export') }}</button></div><small v-if="actionMessage" class="status-message">{{ actionMessage }}</small></div>
      </section>

      <section v-else class="page-section narrow-section about-section">
        <div class="about-hero"><img :src="brandIcon" alt="" /><h1>RemindOn</h1><p>{{ t('app.tagline') }}</p><span>{{ t('about.version', { version: appVersion }) }}</span></div>
        <div class="about-meta"><span>{{ t('about.author') }}</span><strong>ChenHe · SimileHe</strong></div>
        <div class="donation-section"><div><BellRing :size="18" /><strong>{{ t('about.support') }}</strong><span>{{ t('about.supportHint') }}</span></div><div class="donation-code"><img :src="donationCode" alt="" /><img class="donation-logo" :src="brandIcon" alt="" /></div></div>
        <p class="about-copyright">{{ t('about.copyright') }}</p>
      </section>
    </main>
  </div>
</template>
