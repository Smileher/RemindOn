<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { setTheme } from '@tauri-apps/api/app'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { Check, Clock3 } from '@lucide/vue'
import brandIcon from '../assets/remindon.svg'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { AppData, ReminderTriggeredEvent } from '../types'
import { defaultData } from '../types'

const current = ref<ReminderTriggeredEvent | null>(null)
const settings = ref<AppData['settings']>(defaultData().settings)
const triggeredAt = ref('')
const snoozeMinutes = ref(5)
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

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(settings.value.language, key, params)
}

const category = computed(() => {
  if (current.value?.isRest) return t('popup.rest')
  if (current.value?.isShutdown) return t('popup.power')
  return t('popup.event')
})

const isAutomaticPower = computed(() =>
  current.value?.powerAction === 'shutdown'
    || current.value?.powerAction === 'lock'
    || current.value?.powerAction === 'restart',
)

const powerVerb = computed(() => {
  if (current.value?.powerAction === 'restart') return t('popup.restart')
  if (current.value?.powerAction === 'lock') return t('popup.lock')
  return t('popup.shutdown')
})

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
  if (current.value) await invoke('snooze_reminder', { id: current.value.id, minutes: snoozeMinutes.value })
  await closePopup()
}

async function executePowerAction() {
  clearPowerTimer()
  const action = current.value?.powerAction
  if (action !== 'shutdown' && action !== 'restart') return
  try {
    await invoke('execute_power_action', { action })
  } catch (error) {
    powerError.value = t('popup.actionFailed', { action: powerVerb.value, error: String(error) })
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
  snoozeMinutes.value = 5
  powerError.value = ''
  try {
    settings.value = (await invoke<AppData>('load_data')).settings
  } catch {
    // Keep the last known settings if the backend is unavailable for a moment.
  }
  triggeredAt.value = new Intl.DateTimeFormat(settings.value.language, { hour: '2-digit', minute: '2-digit' }).format(new Date())
  try {
    await setTheme(settings.value.theme === 'system' ? null : settings.value.theme)
  } catch {
    // Theme synchronization must not prevent a due notification from opening.
  }
  const window = getCurrentWindow()
  await window.setAlwaysOnTop(settings.value.popupAlwaysOnTop)
  await window.center()
  await window.show()
  await window.setFocus()
  if (isAutomaticPower.value) startPowerCountdown()
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
      <div class="popup-identity"><img :src="brandIcon" alt="" /><div><strong>RemindOn</strong><span>{{ t('popup.time', { category, time: triggeredAt || t('common.now') }) }}</span></div></div>
    </header>
    <section class="popup-content">
      <p class="popup-label">{{ t('popup.label') }}</p>
      <h1>{{ current?.title || t('popup.defaultTitle') }}</h1>
      <div v-if="isAutomaticPower" class="power-countdown"><strong>{{ powerCountdown }}</strong><span>{{ t('popup.secondsUntil', { action: powerVerb }) }}</span></div>
      <p v-else class="popup-hint">{{ t('popup.hint') }}</p>
      <p v-if="powerError" class="popup-error">{{ powerError }}</p>
    </section>
    <footer class="popup-actions">
      <template v-if="isAutomaticPower"><button class="button" type="button" @click="dismiss">{{ t('popup.cancelAction', { action: powerVerb }) }}</button><button class="button button-danger" type="button" @click="executePowerAction">{{ t('popup.executeNow', { action: powerVerb }) }}</button></template>
      <template v-else><label class="snooze-select"><Clock3 :size="15" /><select v-model.number="snoozeMinutes" :aria-label="t('popup.snooze')"><option v-for="minutes in [5, 10, 30]" :key="minutes" :value="minutes">{{ t('popup.minutes', { minutes }) }}</option></select></label><button class="button" type="button" @click="snooze">{{ t('popup.snooze') }}</button><button class="button button-primary" type="button" @click="dismiss"><Check :size="15" />{{ t('popup.done') }}</button></template>
    </footer>
  </main>
</template>
