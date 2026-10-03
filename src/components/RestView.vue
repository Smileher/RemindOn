<script setup lang="ts">
import { Coffee, Play } from '@lucide/vue'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { Language } from '../types'

const props = defineProps<{
  language: Language
  subtitle: string
  enabled: boolean
  intervalMinutes: number
  message: string
  progress: number
  status: string
  actionMessage: string
}>()

const emit = defineEmits<{
  testNotification: []
  'update:enabled': [value: boolean]
  'update:interval': [value: string]
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
        <p class="eyebrow">BREAK</p>
        <h1>{{ t('rest.title') }}</h1>
        <p class="page-subtitle">{{ subtitle }}</p>
      </div>
      <button class="button" type="button" @click="emit('testNotification')">
        <Play :size="14" />{{ t('settings.testNotification') }}
      </button>
    </header>

    <div class="status-panel">
      <div class="status-panel-top">
        <span class="status-icon"><Coffee :size="18" /></span>
        <div>
          <span class="card-label">{{ t('rest.next') }}</span>
          <strong>{{ status }}</strong>
        </div>
        <label class="setting-toggle compact-toggle">
          <input :checked="enabled" type="checkbox" @change="emit('update:enabled', ($event.target as HTMLInputElement).checked)" />
        </label>
      </div>
      <div class="progress-track"><span :style="{ width: `${progress}%` }"></span></div>
      <p>{{ t('rest.scheduleHint') }}</p>
    </div>

    <div class="settings-group">
      <div class="setting-card">
        <div><strong>{{ t('rest.interval') }}</strong><span>{{ t('rest.intervalHint') }}</span></div>
        <label class="number-field">
          <input :value="intervalMinutes" type="number" min="1" max="1440" @input="emit('update:interval', ($event.target as HTMLInputElement).value)" />
          <span>{{ t('common.minutes') }}</span>
        </label>
      </div>
      <label class="setting-card stacked-setting">
        <div><strong>{{ t('rest.message') }}</strong><span>{{ t('rest.messageHint') }}</span></div>
        <input :value="message" type="text" maxlength="120" @change="emit('update:message', ($event.target as HTMLInputElement).value)" />
      </label>
    </div>
    <small v-if="actionMessage" class="status-message page-message">{{ actionMessage }}</small>
  </section>
</template>
