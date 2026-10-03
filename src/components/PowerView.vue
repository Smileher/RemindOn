<script setup lang="ts">
import { Clock3, Play, Power } from '@lucide/vue'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { AppSettings, Language, PowerAction } from '../types'

const props = defineProps<{
  language: Language
  subtitle: string
  settings: AppSettings
  status: string
  actionMessage: string
  powerActionOptions: { value: PowerAction; label: string; icon: unknown }[]
}>()

const emit = defineEmits<{
  testNotification: []
  'update:enabled': [value: boolean]
  'update:powerAction': [value: PowerAction]
  'update:time': [value: string]
  'update:message': [value: string]
}>()

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}
</script>

<template>
  <section class="page-section narrow-section">
    <header class="page-header compact-header">
      <div>
        <p class="eyebrow">SYSTEM</p>
        <h1>{{ t('power.title') }}</h1>
        <p class="page-subtitle">{{ subtitle }}</p>
      </div>
      <button class="button" type="button" @click="emit('testNotification')">
        <Play :size="14" />{{ t('settings.testNotification') }}
      </button>
    </header>

    <div class="status-panel power-status">
      <div class="status-panel-top">
        <span class="status-icon"><Power :size="18" /></span>
        <div>
          <span class="card-label">{{ t('power.next') }}</span>
          <strong>{{ status }}</strong>
        </div>
      </div>
    </div>

    <div class="settings-group">
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('power.enable') }}</strong><span>{{ t('power.enableHint') }}</span></div>
        <input :checked="settings.shutdownReminderEnabled" type="checkbox" @change="emit('update:enabled', ($event.target as HTMLInputElement).checked)" />
      </label>
      <div class="setting-card setting-choice power-choice">
        <div><strong>{{ t('power.action') }}</strong><span>{{ t('power.actionHint') }}</span></div>
        <div class="segmented power-segments">
          <button
            v-for="option in powerActionOptions"
            :key="option.value"
            :class="{ selected: settings.powerAction === option.value }"
            type="button"
            @click="emit('update:powerAction', option.value)"
          >
            <component :is="option.icon" :size="14" />{{ option.label }}
          </button>
        </div>
      </div>
      <label class="setting-card">
        <div><strong>{{ t('power.dailyTime') }}</strong><span>{{ t('power.dailyTimeHint') }}</span></div>
        <span class="time-control">
          <Clock3 :size="15" />
          <input class="time-input" :value="settings.shutdownReminderTime" type="time" @input="emit('update:time', ($event.target as HTMLInputElement).value)" />
        </span>
      </label>
      <label class="setting-card stacked-setting">
        <div><strong>{{ t('power.message') }}</strong><span>{{ t('power.messageHint') }}</span></div>
        <input :value="settings.shutdownReminderMessage" type="text" maxlength="120" @change="emit('update:message', ($event.target as HTMLInputElement).value)" />
      </label>
    </div>
    <small v-if="actionMessage" class="status-message page-message">{{ actionMessage }}</small>
  </section>
</template>
