<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import {
  CalendarClock, Check, Coffee, Download, Info, Pencil, Play, Plus,
  Power, Settings2, Trash2, Upload, X,
} from '@lucide/vue'
import ReminderPopup from './components/ReminderPopup.vue'
import brandIcon from './assets/remindon.svg'
import type { AppData, PowerAction, Reminder, ReminderTriggeredEvent, ReminderType } from './types'
import { defaultData } from './types'

type View = 'events' | 'rest' | 'settings' | 'about'
type EditableReminderType = Exclude<ReminderType, 'interval'>

const isPopup = window.location.hash === '#/reminder'
const data = ref<AppData>(defaultData())
const currentView = ref<View>('events')
const showForm = ref(false)
const editingId = ref<string | null>(null)
const actionMessage = ref('')
const now = ref(Date.now())
const nextRestTrigger = ref<string | null>(null)
const nextShutdownTrigger = ref<string | null>(null)
let unlisten: (() => void) | undefined
let clockTimer: number | undefined

const frequencyOptions: Array<{ value: EditableReminderType; label: string }> = [
  { value: 'once', label: '单次' },
  { value: 'daily', label: '每天' },
  { value: 'weekly', label: '每周' },
  { value: 'monthly', label: '每月' },
  { value: 'workday', label: '工作日' },
  { value: 'weekend', label: '休息日' },
]

const powerActionOptions: Array<{ value: PowerAction; label: string }> = [
  { value: 'shutdown', label: '自动关机' },
  { value: 'remindShutdown', label: '提醒关机' },
  { value: 'restart', label: '自动重启' },
  { value: 'remindRestart', label: '提醒重启' },
]

const defaultShutdownMessage = '时间不早了，记得关闭电脑。'
const defaultRestartMessage = '计划时间到了，记得重启电脑。'

const weekdayOptions = [
  { value: 1, label: '一' }, { value: 2, label: '二' }, { value: 3, label: '三' },
  { value: 4, label: '四' }, { value: 5, label: '五' }, { value: 6, label: '六' },
  { value: 7, label: '日' },
]

const typeLabels: Record<ReminderType, string> = {
  once: '单次', daily: '每天', weekly: '每周', monthly: '每月',
  workday: '工作日', weekend: '休息日', interval: '间隔',
}

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
  if (diff <= 0) return '即将提醒'
  const totalSeconds = Math.floor(diff / 1000)
  const days = Math.floor(totalSeconds / 86400)
  const hours = Math.floor((totalSeconds % 86400) / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  if (days > 0) return `还有 ${days} 天 ${hours} 小时`
  if (hours > 0) return `还有 ${hours} 小时 ${minutes} 分`
  if (minutes > 0) return `还有 ${minutes} 分 ${seconds} 秒`
  return `还有 ${seconds} 秒`
}

function formatRule(reminder: Reminder) {
  if (reminder.type === 'once') {
    return reminder.triggerAt
      ? new Intl.DateTimeFormat('zh-CN', { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(reminder.triggerAt))
      : '未设置'
  }
  if (reminder.type === 'weekly') {
    const days = (reminder.weekdays || []).map((day) => `周${weekdayOptions[day - 1]?.label || day}`).join('、')
    return `${days} ${reminder.time}`
  }
  if (reminder.type === 'monthly') {
    return `每月 ${(reminder.monthDays || []).join('、')} 日 ${reminder.time}`
  }
  return `${typeLabels[reminder.type]} ${reminder.time}`
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
    actionMessage.value = '请填写完整的提醒内容和时间'
    return
  }
  if (form.type === 'weekly' && form.weekdays.length === 0) {
    actionMessage.value = '请至少选择一个星期'
    return
  }
  if (form.type === 'monthly' && form.monthDays.length === 0) {
    actionMessage.value = '请至少选择一个日期'
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

  const index = data.value.reminders.findIndex((item) => item.id === reminder.id)
  if (index >= 0) data.value.reminders.splice(index, 1, reminder)
  else data.value.reminders.push(reminder)
  await persist()
  showForm.value = false
  actionMessage.value = '提醒已保存'
}

async function toggleReminder(reminder: Reminder) {
  reminder.enabled = !reminder.enabled
  await persist()
}

async function removeReminder(id: string) {
  data.value.reminders = data.value.reminders.filter((item) => item.id !== id)
  await persist()
  actionMessage.value = '提醒已删除'
}

async function updateSetting(key: keyof AppData['settings'], value: boolean | number | string) {
  data.value.settings[key] = value as never
  await persist()
  await refreshTimers()
}

async function updateAutostart(value: boolean) {
  try {
    if (value) await enable()
    else await disable()
    await updateSetting('autostart', value)
  } catch (error) {
    actionMessage.value = `开机启动设置失败：${String(error)}`
  }
}

async function updatePowerAction(value: PowerAction) {
  const message = data.value.settings.shutdownReminderMessage
  data.value.settings.powerAction = value
  if (message === defaultShutdownMessage || message === defaultRestartMessage) {
    data.value.settings.shutdownReminderMessage = value === 'restart' || value === 'remindRestart'
      ? defaultRestartMessage
      : defaultShutdownMessage
  }
  await persist()
  await refreshTimers()
}

async function importData() {
  actionMessage.value = ''
  try {
    const path = await open({ multiple: false, directory: false, filters: [{ name: 'RemindOn 数据备份', extensions: ['json'] }] })
    if (typeof path === 'string') {
      data.value = await invoke<AppData>('import_data', { path })
      actionMessage.value = '数据已导入'
      await refreshTimers()
    }
  } catch (error) {
    actionMessage.value = `数据导入失败：${String(error)}`
  }
}

async function exportData() {
  actionMessage.value = ''
  try {
    const path = await save({ defaultPath: 'RemindOn-数据备份.json', filters: [{ name: 'RemindOn 数据备份', extensions: ['json'] }] })
    if (typeof path === 'string') {
      await invoke('export_data', { path })
      actionMessage.value = '数据已导出'
    }
  } catch (error) {
    actionMessage.value = `数据导出失败：${String(error)}`
  }
}

async function testNotification() {
  try {
    await invoke('test_reminder')
    actionMessage.value = '测试通知已发送'
  } catch (error) {
    actionMessage.value = `测试通知失败：${String(error)}`
  }
}

async function refreshTimers() {
  if (isPopup) return
  const [rest, shutdown] = await Promise.allSettled([
    invoke<string | null>('get_next_rest_trigger'),
    invoke<string | null>('get_next_shutdown_trigger'),
  ])
  nextRestTrigger.value = rest.status === 'fulfilled' ? rest.value : null
  nextShutdownTrigger.value = shutdown.status === 'fulfilled' ? shutdown.value : null
}

onMounted(async () => {
  if (isPopup) return
  try {
    data.value = await invoke<AppData>('load_data')
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
    clockTimer = window.setInterval(() => {
      now.value = Date.now()
      void refreshTimers()
    }, 1000)
  } catch (error) {
    actionMessage.value = `读取数据失败：${String(error)}`
  }
})

onUnmounted(() => {
  unlisten?.()
  if (clockTimer) window.clearInterval(clockTimer)
})
</script>

<template>
  <ReminderPopup v-if="isPopup" />
  <div v-else :class="['app-shell', `theme-${data.settings.theme}`, `accent-${data.settings.accentColor}`]">
    <aside class="sidebar">
      <div class="brand">
        <img class="brand-mark" :src="brandIcon" alt="" />
        <div class="brand-copy"><strong>RemindOn</strong><span>轻量定时提醒</span></div>
      </div>
      <nav class="nav-list" aria-label="主导航">
        <button :class="['nav-item', { active: currentView === 'events' }]" @click="currentView = 'events'"><CalendarClock :size="17" /><span>事件提醒</span></button>
        <button :class="['nav-item', { active: currentView === 'rest' }]" @click="currentView = 'rest'"><Coffee :size="17" /><span>休息提醒</span></button>
        <button :class="['nav-item', { active: currentView === 'settings' }]" @click="currentView = 'settings'"><Settings2 :size="17" /><span>参数设置</span></button>
        <button :class="['nav-item', { active: currentView === 'about' }]" @click="currentView = 'about'"><Info :size="17" /><span>关于</span></button>
      </nav>
      <div class="sidebar-footer"><span class="status-dot"></span><div><strong>后台提醒已准备</strong><span>RemindOn 0.2.0</span></div></div>
    </aside>

    <main class="content">
      <section v-if="currentView === 'events'" class="page-section">
        <header class="page-header"><div><p class="eyebrow">REMINDERS</p><h1>事件提醒</h1><p class="page-subtitle">安排单次或重复的提醒。</p></div><button class="button button-primary" type="button" @click="openAddForm"><Plus :size="15" />添加提醒</button></header>

        <div v-if="showForm" class="form-panel">
          <div class="form-heading"><div><p class="eyebrow">REMINDER</p><h2>{{ editingId ? '编辑提醒' : '添加提醒' }}</h2></div><button class="icon-button" type="button" aria-label="关闭" title="关闭" @click="showForm = false"><X :size="18" /></button></div>
          <label class="field"><span>提醒内容</span><input v-model="form.title" type="text" maxlength="120" placeholder="例如：提交周报" /></label>
          <div class="field"><span>提醒频率</span><div class="segmented frequency-segments"><button v-for="option in frequencyOptions" :key="option.value" :class="{ selected: form.type === option.value }" type="button" @click="form.type = option.value">{{ option.label }}</button></div></div>
          <div class="field-row"><label v-if="form.type === 'once'" class="field"><span>提醒时间</span><input v-model="form.triggerAt" type="datetime-local" /></label><label v-else class="field"><span>具体时间</span><input v-model="form.time" type="time" /></label></div>
          <div v-if="form.type === 'weekly'" class="field"><span>选择星期</span><div class="choice-grid weekday-grid"><button v-for="day in weekdayOptions" :key="day.value" :class="{ selected: form.weekdays.includes(day.value) }" type="button" @click="toggleNumber(form.weekdays, day.value)">周{{ day.label }}</button></div></div>
          <div v-if="form.type === 'monthly'" class="field"><span>选择日期</span><div class="choice-grid month-grid"><button v-for="day in 31" :key="day" :class="{ selected: form.monthDays.includes(day) }" type="button" @click="toggleNumber(form.monthDays, day)">{{ day }}</button></div><small>当月没有所选日期时，该日期自动跳过。</small></div>
          <small v-if="actionMessage" class="status-message form-message">{{ actionMessage }}</small>
          <div class="form-actions"><button class="button" type="button" @click="showForm = false">取消</button><button class="button button-primary" type="button" @click="saveReminder"><Check :size="15" />保存提醒</button></div>
        </div>

        <div v-if="sortedReminders.length" class="reminder-list">
          <article v-for="reminder in sortedReminders" :key="reminder.id" :class="['reminder-row', { disabled: !reminder.enabled }]">
            <div :class="['reminder-status', { enabled: reminder.enabled }]"></div>
            <div class="reminder-main"><div class="reminder-title"><strong>{{ reminder.title }}</strong><span class="type-chip">{{ typeLabels[reminder.type] }}</span></div><span>{{ formatNext(reminder) }}</span></div>
            <button class="switch" :class="{ on: reminder.enabled }" type="button" :aria-label="reminder.enabled ? '停用' : '启用'" @click="toggleReminder(reminder)"><span></span></button>
            <button class="icon-button row-action" type="button" aria-label="编辑" title="编辑" @click="editReminder(reminder)"><Pencil :size="15" /></button>
            <button class="icon-button row-action danger" type="button" aria-label="删除" title="删除" @click="removeReminder(reminder.id)"><Trash2 :size="15" /></button>
          </article>
        </div>
        <div v-else-if="!showForm" class="empty-state"><div class="empty-icon"><CalendarClock :size="22" /></div><h2>还没有提醒</h2><p>添加一个需要记住的时间，RemindOn 会在后台提醒你。</p><button class="button" type="button" @click="openAddForm"><Plus :size="15" />添加第一个提醒</button></div>
      </section>

      <section v-else-if="currentView === 'rest'" class="page-section narrow-section">
        <header class="page-header compact-header"><div><p class="eyebrow">BREAK</p><h1>休息提醒</h1><p class="page-subtitle">按固定间隔提醒你离开屏幕片刻。</p></div><label class="switch large-switch" :class="{ on: data.settings.restEnabled }"><input :checked="data.settings.restEnabled" type="checkbox" @change="updateSetting('restEnabled', ($event.target as HTMLInputElement).checked)" /><span></span></label></header>
        <div class="rest-card"><div class="rest-card-heading"><div><span class="card-label">下次休息</span><strong>{{ data.settings.restEnabled ? (nextRestTrigger ? formatCountdown(nextRestTrigger) : '正在计算') : '已暂停' }}</strong></div><span class="rest-ring"><Coffee :size="18" /></span></div><div class="progress-track"><span :style="{ width: `${restProgress}%` }"></span></div><p>提醒完成或关闭后，会重新开始计时。</p></div>
        <div class="settings-group"><div class="setting-card"><div><strong>提醒间隔</strong><span>建议每隔一段时间离开屏幕。</span></div><label class="number-field"><input :value="data.settings.restIntervalMinutes" type="number" min="1" max="1440" @change="updateSetting('restIntervalMinutes', Number(($event.target as HTMLInputElement).value))" /><span>分钟</span></label></div><label class="setting-card stacked-setting"><div><strong>提醒内容</strong><span>显示在休息通知中。</span></div><input :value="data.settings.restMessage" type="text" maxlength="120" @change="updateSetting('restMessage', ($event.target as HTMLInputElement).value)" /></label></div>
        <div class="subsection-heading"><div><p class="eyebrow">POWER</p><h2>定时关机与重启</h2></div><label class="switch" :class="{ on: data.settings.shutdownReminderEnabled }"><input :checked="data.settings.shutdownReminderEnabled" type="checkbox" @change="updateSetting('shutdownReminderEnabled', ($event.target as HTMLInputElement).checked)" /><span></span></label></div>
        <div class="rest-card shutdown-card"><div class="rest-card-heading"><div><span class="card-label">下次定时操作</span><strong>{{ data.settings.shutdownReminderEnabled ? (nextShutdownTrigger ? formatCountdown(nextShutdownTrigger) : '正在计算') : '已暂停' }}</strong></div><span class="rest-ring"><Power :size="18" /></span></div><p>自动模式会先弹出 60 秒倒计时，可随时取消；错过计划时间超过 1 分钟时不会补执行。</p></div>
        <div class="settings-group power-settings"><div class="setting-card setting-choice"><div><strong>操作方式</strong><span>提醒模式不会改变电脑状态。</span></div><div class="segmented power-segments"><button v-for="option in powerActionOptions" :key="option.value" :class="{ selected: data.settings.powerAction === option.value }" type="button" @click="updatePowerAction(option.value)">{{ option.label }}</button></div></div><label class="setting-card"><div><strong>每日时间</strong><span>设置每天触发的时间。</span></div><input class="time-input" :value="data.settings.shutdownReminderTime" type="time" @change="updateSetting('shutdownReminderTime', ($event.target as HTMLInputElement).value)" /></label><label class="setting-card stacked-setting"><div><strong>通知内容</strong><span>显示在定时操作通知中。</span></div><input :value="data.settings.shutdownReminderMessage" type="text" maxlength="120" @change="updateSetting('shutdownReminderMessage', ($event.target as HTMLInputElement).value)" /></label></div>
      </section>

      <section v-else-if="currentView === 'settings'" class="page-section narrow-section">
        <header class="page-header compact-header"><div><p class="eyebrow">PREFERENCES</p><h1>参数设置</h1><p class="page-subtitle">调整通知、启动和外观。</p></div></header>
        <div class="settings-group">
          <label class="setting-card setting-toggle"><div><strong>开机自动运行</strong><span>登录系统后自动启动 RemindOn。</span></div><input :checked="data.settings.autostart" type="checkbox" @change="updateAutostart(($event.target as HTMLInputElement).checked)" /></label>
          <label class="setting-card setting-toggle"><div><strong>启动后自动最小化</strong><span>启动时直接隐藏到托盘，关闭主窗口后也继续运行。</span></div><input :checked="data.settings.minimizeToTray" type="checkbox" @change="updateSetting('minimizeToTray', ($event.target as HTMLInputElement).checked)" /></label>
          <label class="setting-card setting-toggle"><div><strong>软件通知保持置顶</strong><span>让软件通知显示在其他窗口上方。</span></div><input :checked="data.settings.popupAlwaysOnTop" type="checkbox" @change="updateSetting('popupAlwaysOnTop', ($event.target as HTMLInputElement).checked)" /></label>
          <div class="setting-card setting-choice"><div><strong>通知方式</strong><span>默认使用居中的软件通知。</span></div><div class="segmented"><button :class="{ selected: data.settings.notificationMode === 'system' }" type="button" @click="updateSetting('notificationMode', 'system')">系统通知</button><button :class="{ selected: data.settings.notificationMode === 'popup' }" type="button" @click="updateSetting('notificationMode', 'popup')">软件通知</button></div></div>
          <div class="setting-card setting-choice"><div><strong>通知样式</strong><span>选择软件通知的视觉强度。</span></div><div class="segmented"><button :class="{ selected: data.settings.notificationStyle === 'compact' }" type="button" @click="updateSetting('notificationStyle', 'compact')">简洁</button><button :class="{ selected: data.settings.notificationStyle === 'standard' }" type="button" @click="updateSetting('notificationStyle', 'standard')">标准</button><button :class="{ selected: data.settings.notificationStyle === 'prominent' }" type="button" @click="updateSetting('notificationStyle', 'prominent')">醒目</button></div></div>
          <div class="setting-card setting-choice"><div><strong>外观</strong><span>选择界面的明暗模式。</span></div><div class="segmented"><button :class="{ selected: data.settings.theme === 'dark' }" type="button" @click="updateSetting('theme', 'dark')">深色</button><button :class="{ selected: data.settings.theme === 'light' }" type="button" @click="updateSetting('theme', 'light')">浅色</button><button :class="{ selected: data.settings.theme === 'system' }" type="button" @click="updateSetting('theme', 'system')">跟随系统</button></div></div>
          <div class="setting-card color-setting"><div><strong>强调色</strong><span>用于导航、开关和提醒状态。</span></div><div class="color-options"><button v-for="color in ['mint', 'blue', 'violet', 'amber']" :key="color" :class="['color-swatch', `swatch-${color}`, { selected: data.settings.accentColor === color }]" type="button" :aria-label="`${color} 强调色`" @click="updateSetting('accentColor', color)"></button></div></div>
        </div>
        <div class="data-actions"><div><strong>数据管理</strong><span>导出备份，或在其他设备上恢复。</span></div><div class="action-row"><button class="button" type="button" @click="testNotification"><Play :size="14" />测试通知</button><button class="button" type="button" @click="importData"><Upload :size="14" />导入数据</button><button class="button" type="button" @click="exportData"><Download :size="14" />导出数据</button></div><small v-if="actionMessage" class="status-message">{{ actionMessage }}</small></div>
      </section>

      <section v-else class="page-section narrow-section about-section"><p class="eyebrow">ABOUT</p><h1>关于 RemindOn</h1><p class="about-lead">轻量定时提醒。</p><div class="about-note"><img :src="brandIcon" alt="" /><div><strong>RemindOn 0.2.0</strong><p>本地优先，不需要账号，不连接服务器。</p><p class="author-line">Created by SimileHe</p></div></div></section>
    </main>
  </div>
</template>
