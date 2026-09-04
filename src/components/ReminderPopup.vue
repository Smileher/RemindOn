<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { Check, Clock3, X } from '@lucide/vue'
import brandIcon from '../assets/remindon.svg'
import type { AppData, ReminderTriggeredEvent } from '../types'
import { defaultData } from '../types'

const current = ref<ReminderTriggeredEvent | null>(null)
const settings = ref<AppData['settings']>(defaultData().settings)
const triggeredAt = ref('')
const powerCountdown = ref(60)
const powerError = ref('')
let unlisten: (() => void) | undefined
let unlistenClose: (() => void) | undefined
let powerTimer: number | undefined
let powerDeadline = 0

const popupClass = computed(() => [
  `theme-${settings.value.theme}`,
  `accent-${settings.value.accentColor}`,
  `notification-${settings.value.notificationStyle}`,
])

const category = computed(() => {
  if (current.value?.isRest) return '休息提醒'
  if (current.value?.isShutdown) return '定时操作'
  return '事件提醒'
})

const isAutomaticPower = computed(() =>
  current.value?.powerAction === 'shutdown' || current.value?.powerAction === 'restart',
)

const powerVerb = computed(() => current.value?.powerAction === 'restart' ? '重启' : '关机')

function includesPopup(mode: AppData['settings']['notificationMode']) {
  return mode === 'popup' || mode === 'both'
}

function includesSystem(mode: AppData['settings']['notificationMode']) {
  return mode === 'system' || mode === 'both'
}

async function closePopup() {
  await getCurrentWindow().hide()
}

function clearPowerTimer() {
  if (powerTimer) window.clearInterval(powerTimer)
  powerTimer = undefined
}

async function dismiss() {
  clearPowerTimer()
  if (current.value) await invoke('dismiss_reminder', { id: current.value.id })
  await closePopup()
}

async function snooze() {
  clearPowerTimer()
  if (current.value) await invoke('snooze_reminder', { id: current.value.id, minutes: 5 })
  await closePopup()
}

async function executePowerAction() {
  clearPowerTimer()
  const action = current.value?.powerAction
  if (action !== 'shutdown' && action !== 'restart') return
  try {
    await invoke('execute_power_action', { action })
  } catch (error) {
    powerError.value = `无法${powerVerb.value}：${String(error)}`
  }
}

function startPowerCountdown() {
  clearPowerTimer()
  powerCountdown.value = 60
  powerDeadline = Date.now() + 60_000
  powerTimer = window.setInterval(() => {
    const remaining = Math.ceil((powerDeadline - Date.now()) / 1000)
    powerCountdown.value = Math.max(0, remaining)
    if (remaining <= 0) {
      if (Date.now() - powerDeadline <= 5_000) void executePowerAction()
      else void dismiss()
    }
  }, 1000)
}

async function handleTrigger(event: ReminderTriggeredEvent) {
  current.value = event
  powerError.value = ''
  triggeredAt.value = new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit' }).format(new Date())
  try {
    settings.value = (await invoke<AppData>('load_data')).settings
  } catch {
    // Keep the last known settings if the backend is unavailable for a moment.
  }

  if (includesSystem(settings.value.notificationMode)) {
    try {
      let permission = await isPermissionGranted()
      if (!permission) permission = (await requestPermission()) === 'granted'
      if (permission) await sendNotification({ title: `RemindOn · ${category.value}`, body: event.title })
    } catch {
      // Software notification remains available when native notification permission is unavailable.
    }
  }
  if (includesPopup(settings.value.notificationMode) || isAutomaticPower.value) {
    const window = getCurrentWindow()
    await window.setAlwaysOnTop(settings.value.popupAlwaysOnTop)
    await window.center()
    await window.show()
    await window.setFocus()
    if (isAutomaticPower.value) startPowerCountdown()
  }
}

onMounted(async () => {
  try {
    settings.value = (await invoke<AppData>('load_data')).settings
  } catch {
    // The standalone Vite preview has no Tauri command bridge.
  }
  try {
    unlisten = await listen<ReminderTriggeredEvent>('reminder-triggered', (event) => void handleTrigger(event.payload))
  } catch {
    // The standalone Vite preview has no Tauri event bridge.
  }
  unlistenClose = await getCurrentWindow().onCloseRequested((event) => {
    event.preventDefault()
    void dismiss()
  })
})

onUnmounted(() => {
  clearPowerTimer()
  unlisten?.()
  unlistenClose?.()
})
</script>

<template>
  <main :class="['popup-shell', ...popupClass]">
    <header class="popup-header">
      <div class="popup-identity"><img :src="brandIcon" alt="" /><div><strong>RemindOn</strong><span>{{ category }} · {{ triggeredAt || '现在' }}</span></div></div>
      <button class="icon-button popup-close" type="button" aria-label="关闭通知" title="关闭" @click="dismiss"><X :size="18" /></button>
    </header>
    <section class="popup-content">
      <p class="popup-label">现在是提醒时间</p>
      <h1>{{ current?.title || '你有一条新提醒' }}</h1>
      <div v-if="isAutomaticPower" class="power-countdown"><strong>{{ powerCountdown }}</strong><span>秒后自动{{ powerVerb }}</span></div>
      <p v-else class="popup-hint">可以立即完成，或稍后 5 分钟再次提醒。</p>
      <p v-if="powerError" class="popup-error">{{ powerError }}</p>
    </section>
    <footer class="popup-actions">
      <template v-if="isAutomaticPower"><button class="button" type="button" @click="dismiss">取消{{ powerVerb }}</button><button class="button button-danger" type="button" @click="executePowerAction">立即{{ powerVerb }}</button></template>
      <template v-else><button class="button" type="button" @click="snooze"><Clock3 :size="15" />稍后 5 分钟</button><button class="button button-primary" type="button" @click="dismiss"><Check :size="15" />完成</button></template>
    </footer>
  </main>
</template>
