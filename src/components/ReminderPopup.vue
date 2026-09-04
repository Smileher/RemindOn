<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { setTheme } from '@tauri-apps/api/app'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { confirm } from '@tauri-apps/plugin-dialog'
import { Check, ChevronDown, Clock3 } from '@lucide/vue'
import brandIcon from '../assets/remindon.svg'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { AppData, ReminderTriggeredEvent } from '../types'
import { defaultData } from '../types'

const current = ref<ReminderTriggeredEvent | null>(null)
const settings = ref<AppData['settings']>(defaultData().settings)
const triggeredAt = ref('')
const snoozeSeconds = ref(300)
const snoozeMenu = ref<HTMLDetailsElement | null>(null)
const restElapsedSeconds = ref(0)
const powerCountdown = ref(60)
const powerError = ref('')
let unlisten: (() => void) | undefined
let unlistenClose: (() => void) | undefined
let restTimer: number | undefined
let powerTimer: number | undefined
let restStartedAt = 0
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

const snoozeOptions = computed(() => [
  { seconds: 30, label: t('popup.seconds', { seconds: 30 }) },
  { seconds: 60, label: t('popup.minutes', { minutes: 1 }) },
  { seconds: 300, label: t('popup.minutes', { minutes: 5 }) },
  { seconds: 600, label: t('popup.minutes', { minutes: 10 }) },
  { seconds: 1800, label: t('popup.minutes', { minutes: 30 }) },
  { seconds: 3600, label: t('popup.hours', { hours: 1 }) },
  { seconds: 7200, label: t('popup.hours', { hours: 2 }) },
  { seconds: 10800, label: t('popup.hours', { hours: 3 }) },
  { seconds: 14400, label: t('popup.hours', { hours: 4 }) },
])

const selectedSnoozeLabel = computed(() =>
  snoozeOptions.value.find((option) => option.seconds === snoozeSeconds.value)?.label
    || t('popup.minutes', { minutes: 5 }),
)

const restElapsed = computed(() => t('popup.rested', {
  minutes: Math.floor(restElapsedSeconds.value / 60),
  seconds: restElapsedSeconds.value % 60,
}))

async function closePopup() {
  await getCurrentWindow().hide()
}

function clearPowerTimer() {
  if (powerTimer) window.clearInterval(powerTimer)
  powerTimer = undefined
}

function clearRestTimer() {
  if (restTimer) window.clearInterval(restTimer)
  restTimer = undefined
}

function startRestTimer() {
  clearRestTimer()
  restElapsedSeconds.value = 0
  restStartedAt = Date.now()
  restTimer = window.setInterval(() => {
    restElapsedSeconds.value = Math.floor((Date.now() - restStartedAt) / 1000)
  }, 1000)
}

function selectSnooze(seconds: number) {
  snoozeSeconds.value = seconds
  snoozeMenu.value?.removeAttribute('open')
}

async function dismiss() {
  clearPowerTimer()
  clearRestTimer()
  if (current.value) await invoke('dismiss_reminder', { id: current.value.id })
  await closePopup()
}

async function snooze(seconds: number) {
  clearPowerTimer()
  clearRestTimer()
  if (current.value) await invoke('snooze_reminder', { id: current.value.id, seconds })
  await closePopup()
}

async function executePowerAction() {
  clearPowerTimer()
  const action = current.value?.powerAction
  if (action !== 'shutdown' && action !== 'lock' && action !== 'restart') return
  try {
    if (current.value?.isTest) {
      const confirmed = await confirm(t('popup.testPowerConfirm', { action: powerVerb.value }), {
        title: t('popup.testMode'),
        kind: 'warning',
        okLabel: t('popup.confirmExecute'),
        cancelLabel: t('common.cancel'),
      })
      if (!confirmed) {
        await closePopup()
        return
      }
    }
    await invoke('execute_power_action', { action })
    await closePopup()
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
  clearPowerTimer()
  clearRestTimer()
  current.value = event
  snoozeSeconds.value = 300
  restElapsedSeconds.value = 0
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
  else if (event.isRest) startRestTimer()
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
  clearRestTimer()
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
      <p v-if="current?.isRest" class="rest-elapsed">{{ restElapsed }}</p>
      <h1>{{ current?.title || t('popup.defaultTitle') }}</h1>
      <div v-if="isAutomaticPower" class="power-countdown"><strong>{{ powerCountdown }}</strong><span>{{ t('popup.secondsUntil', { action: powerVerb }) }}</span></div>
      <p v-if="powerError" class="popup-error">{{ powerError }}</p>
    </section>
    <footer :class="['popup-actions', { 'split-actions': !isAutomaticPower }]">
      <template v-if="isAutomaticPower"><button class="button" type="button" @click="dismiss">{{ t('popup.cancelAction', { action: powerVerb }) }}</button><button class="button button-danger" type="button" @click="executePowerAction">{{ t('popup.executeNow', { action: powerVerb }) }}</button></template>
      <template v-else>
        <div class="popup-action-group">
          <details ref="snoozeMenu" class="snooze-picker">
            <summary><Clock3 :size="15" /><span>{{ selectedSnoozeLabel }}</span><ChevronDown :size="15" class="select-chevron" /></summary>
            <div class="snooze-options">
              <button v-for="option in snoozeOptions" :key="option.seconds" :class="{ selected: snoozeSeconds === option.seconds }" type="button" @click="selectSnooze(option.seconds)"><span>{{ option.label }}</span><Check v-if="snoozeSeconds === option.seconds" :size="14" /></button>
            </div>
          </details>
          <button class="button" type="button" @click="snooze(snoozeSeconds)">{{ t('popup.snooze') }}</button>
        </div>
        <button v-if="current?.isRest" class="button button-primary" type="button" @click="snooze(60)">{{ t('popup.remindInOneMinute') }}</button>
        <button v-else class="button button-primary" type="button" @click="dismiss"><Check :size="15" />{{ t('popup.done') }}</button>
      </template>
    </footer>
  </main>
</template>
