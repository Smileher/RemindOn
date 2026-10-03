<script setup lang="ts">
import { Download, RotateCcw, Upload } from '@lucide/vue'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { AccentColor, AppSettings, Language, Theme } from '../types'

const props = defineProps<{
  language: Language
  settings: AppSettings
  accentColors: AccentColor[]
  autostartError: string
  notificationError: string
  actionMessage: string
  canReset: boolean
}>()

const emit = defineEmits<{
  'update:setting': [key: keyof AppSettings, value: AppSettings[keyof AppSettings]]
  importData: []
  exportData: []
  resetSettings: []
}>()

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}
</script>

<template>
  <section class="page-section narrow-section">
    <header class="page-header compact-header">
      <div>
        <p class="eyebrow">PREFERENCES</p>
        <h1>{{ t('settings.title') }}</h1>
        <p class="page-subtitle">{{ t('settings.subtitle') }}</p>
      </div>
    </header>

    <div class="settings-group">
      <p class="settings-group-title">{{ t('settings.groupGeneral') }}</p>
      <div class="setting-card setting-choice">
        <div><strong>{{ t('settings.language') }}</strong><span>{{ t('settings.languageHint') }}</span></div>
        <div class="segmented">
          <button :class="{ selected: settings.language === 'zh-CN' }" type="button" @click="emit('update:setting', 'language', 'zh-CN')">{{ t('settings.zh') }}</button>
          <button :class="{ selected: settings.language === 'en' }" type="button" @click="emit('update:setting', 'language', 'en')">{{ t('settings.en') }}</button>
        </div>
      </div>
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.autostart') }}</strong><span>{{ t('settings.autostartHint') }}</span><small v-if="autostartError" class="setting-error">{{ autostartError }}</small></div>
        <input :checked="settings.autostart" type="checkbox" @change="emit('update:setting', 'autostart', ($event.target as HTMLInputElement).checked)" />
      </label>
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.startHidden') }}</strong><span>{{ t('settings.startHiddenHint') }}</span></div>
        <input :checked="settings.minimizeToTray" type="checkbox" @change="emit('update:setting', 'minimizeToTray', ($event.target as HTMLInputElement).checked)" />
      </label>
    </div>

    <div class="settings-group">
      <p class="settings-group-title">{{ t('settings.groupAppearance') }}</p>
      <div class="setting-card setting-choice">
        <div><strong>{{ t('settings.appearance') }}</strong><span>{{ t('settings.appearanceHint') }}</span></div>
        <div class="segmented">
          <button :class="{ selected: settings.theme === 'dark' }" type="button" @click="emit('update:setting', 'theme', 'dark' as Theme)">{{ t('settings.dark') }}</button>
          <button :class="{ selected: settings.theme === 'light' }" type="button" @click="emit('update:setting', 'theme', 'light' as Theme)">{{ t('settings.light') }}</button>
          <button :class="{ selected: settings.theme === 'system' }" type="button" @click="emit('update:setting', 'theme', 'system' as Theme)">{{ t('settings.system') }}</button>
        </div>
      </div>
      <div class="setting-card color-setting">
        <div><strong>{{ t('settings.accent') }}</strong><span>{{ t('settings.accentHint') }}</span></div>
        <div class="color-options">
          <button
            v-for="color in accentColors"
            :key="color"
            :class="['color-swatch', `swatch-${color}`, { selected: settings.accentColor === color }]"
            type="button"
            :aria-label="color"
            @click="emit('update:setting', 'accentColor', color)"
          ></button>
        </div>
      </div>
    </div>

    <div class="settings-group">
      <p class="settings-group-title">{{ t('settings.groupNotification') }}</p>
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.alwaysOnTop') }}</strong><span>{{ t('settings.alwaysOnTopHint') }}</span></div>
        <input :checked="settings.popupAlwaysOnTop" type="checkbox" @change="emit('update:setting', 'popupAlwaysOnTop', ($event.target as HTMLInputElement).checked)" />
      </label>
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.fullscreenPopup') }}</strong><span>{{ t('settings.fullscreenPopupHint') }}</span></div>
        <input :checked="settings.popupFullscreen" type="checkbox" @change="emit('update:setting', 'popupFullscreen', ($event.target as HTMLInputElement).checked)" />
      </label>
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.systemNotification') }}</strong><span>{{ t('settings.systemNotificationHint') }}</span><small v-if="notificationError" class="setting-error">{{ notificationError }}</small></div>
        <input :checked="settings.systemNotificationEnabled" type="checkbox" @change="emit('update:setting', 'systemNotificationEnabled', ($event.target as HTMLInputElement).checked)" />
      </label>
    </div>

    <div class="settings-group">
      <p class="settings-group-title">{{ t('settings.groupData') }}</p>
      <div class="data-actions">
        <div><strong>{{ t('settings.data') }}</strong><span>{{ t('settings.dataHint') }}</span></div>
        <div class="action-row">
          <button class="button" type="button" @click="emit('importData')"><Upload :size="14" />{{ t('settings.import') }}</button>
          <button class="button" type="button" @click="emit('exportData')"><Download :size="14" />{{ t('settings.export') }}</button>
        </div>
        <small v-if="actionMessage" class="status-message">{{ actionMessage }}</small>
      </div>
      <div v-if="canReset" class="data-actions danger-actions">
        <div><strong>{{ t('settings.reset') }}</strong><span>{{ t('settings.resetHint') }}</span></div>
        <div class="action-row">
          <button class="button danger" type="button" @click="emit('resetSettings')"><RotateCcw :size="14" />{{ t('settings.reset') }}</button>
        </div>
      </div>
    </div>
  </section>
</template>
