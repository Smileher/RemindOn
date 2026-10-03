<script setup lang="ts">
import { computed } from 'vue'
import { Download, Image, RotateCcw, Upload } from '@lucide/vue'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { AccentColor, AppSettings, Language, PopupBackgroundFit, Theme } from '../types'

const props = defineProps<{
  language: Language
  settings: AppSettings
  accentColors: AccentColor[]
  autostartError: string
  notificationError: string
  actionMessage: string
  canReset: boolean
  backgroundPreview: string
}>()

const emit = defineEmits<{
  'update:setting': [key: keyof AppSettings, value: AppSettings[keyof AppSettings]]
  importData: []
  exportData: []
  resetSettings: []
  pickPopupImage: []
  clearPopupImage: []
}>()

const fitOptions = computed<{ value: PopupBackgroundFit; label: string }[]>(() => [
  { value: 'cover', label: t('settings.fitCover') },
  { value: 'contain', label: t('settings.fitContain') },
  { value: 'stretch', label: t('settings.fitStretch') },
  { value: 'repeat', label: t('settings.fitRepeat') },
])

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
      <div class="setting-card stacked-setting">
        <div><strong>{{ t('settings.popupBackground') }}</strong><span>{{ t('settings.popupBackgroundHint') }}</span></div>
        <div class="background-picker">
          <img v-if="backgroundPreview" class="background-thumb" :src="backgroundPreview" alt="" />
          <span v-else class="background-thumb placeholder" aria-hidden="true"></span>
          <div class="action-row">
            <button class="button" type="button" @click="emit('pickPopupImage')"><Image :size="14" />{{ t('settings.popupChooseImage') }}</button>
            <button v-if="backgroundPreview" class="button" type="button" @click="emit('clearPopupImage')"><RotateCcw :size="14" />{{ t('settings.popupClearImage') }}</button>
          </div>
        </div>
      </div>
      <div v-if="backgroundPreview" class="setting-card stacked-setting">
        <div><strong>{{ t('settings.popupBackgroundFit') }}</strong><span>{{ t('settings.popupBackgroundFitHint') }}</span></div>
        <div class="segmented fit-segments">
          <button v-for="option in fitOptions" :key="option.value" :class="{ selected: settings.popupBackgroundFit === option.value }" type="button" @click="emit('update:setting', 'popupBackgroundFit', option.value)">{{ option.label }}</button>
        </div>
        <label class="slider-row">
          <span>{{ t('settings.popupOverlay') }}</span>
          <input :value="settings.popupOverlayOpacity" type="range" min="0" max="90" step="5" @input="emit('update:setting', 'popupOverlayOpacity', Number(($event.target as HTMLInputElement).value))" />
          <em>{{ settings.popupOverlayOpacity }}%</em>
        </label>
      </div>
      <div class="setting-card">
        <div><strong>{{ t('settings.popupTextColor') }}</strong><span>{{ t('settings.popupTextColorHint') }}</span></div>
        <span class="color-control">
          <input type="color" :value="settings.popupTextColor || '#f3f4f6'" @input="emit('update:setting', 'popupTextColor', ($event.target as HTMLInputElement).value)" />
          <button v-if="settings.popupTextColor" class="button" type="button" @click="emit('update:setting', 'popupTextColor', '')">{{ t('settings.popupResetColor') }}</button>
        </span>
      </div>
      <label class="setting-card">
        <div><strong>{{ t('settings.popupTitleSize') }}</strong><span>{{ t('settings.popupTitleSizeHint') }}</span></div>
        <label class="number-field"><input :value="settings.popupTitleSize" type="number" min="20" max="72" step="1" @change="emit('update:setting', 'popupTitleSize', Number(($event.target as HTMLInputElement).value))" /><span>px</span></label>
      </label>
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
          <button v-if="canReset" class="button danger" type="button" @click="emit('resetSettings')"><RotateCcw :size="14" />{{ t('settings.reset') }}</button>
        </div>
        <small v-if="actionMessage" class="status-message">{{ actionMessage }}</small>
      </div>
    </div>
  </section>
</template>
