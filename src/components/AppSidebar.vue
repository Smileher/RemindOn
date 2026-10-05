<script setup lang="ts">
import { CalendarClock, Coffee, Info, Settings2 } from '@lucide/vue'
import brandIcon from '../assets/remindon.svg'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { Language } from '../types'

type View = 'events' | 'rest' | 'settings' | 'about'

const props = defineProps<{
  currentView: View
  language: Language
  appVersion: string
  hasUpdate: boolean
}>()

const emit = defineEmits<{ navigate: [view: View] }>()

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <img class="brand-mark" :src="brandIcon" alt="" />
      <div class="brand-copy"><strong>RemindOn</strong><span>{{ t('app.tagline') }}</span></div>
    </div>
    <nav class="nav-list" aria-label="Navigation">
      <button :class="['nav-item', { active: currentView === 'rest' }]" @click="emit('navigate', 'rest')"><Coffee :size="17" /><span>{{ t('nav.rest') }}</span></button>
      <button :class="['nav-item', { active: currentView === 'events' }]" @click="emit('navigate', 'events')"><CalendarClock :size="17" /><span>{{ t('nav.events') }}</span></button>
      <button :class="['nav-item', { active: currentView === 'settings' }]" @click="emit('navigate', 'settings')"><Settings2 :size="17" /><span>{{ t('nav.settings') }}</span></button>
      <button :class="['nav-item', { active: currentView === 'about' }]" @click="emit('navigate', 'about')"><Info :size="17" /><span>{{ t('nav.about') }}</span><span v-if="hasUpdate" class="update-dot" :aria-label="t('update.available', { version: '' })"></span></button>
    </nav>
    <div class="sidebar-footer">RemindOn v{{ appVersion }}</div>
  </aside>
</template>
