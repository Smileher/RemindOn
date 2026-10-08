<script setup lang="ts">
import { CalendarClock, Coffee, Globe, Info, Settings2 } from '@lucide/vue'
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

const emit = defineEmits<{ navigate: [view: View]; openWebsite: [] }>()

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <img class="brand-mark" :src="brandIcon" alt="" />
      <div class="brand-copy"><strong>RemindOn</strong></div>
    </div>
    <nav class="nav-list" aria-label="Navigation">
      <button :class="['nav-item', { active: currentView === 'rest' }]" :aria-current="currentView === 'rest' ? 'page' : undefined" @click="emit('navigate', 'rest')"><Coffee :size="17" /><span>{{ t('nav.rest') }}</span></button>
      <button :class="['nav-item', { active: currentView === 'events' }]" :aria-current="currentView === 'events' ? 'page' : undefined" @click="emit('navigate', 'events')"><CalendarClock :size="17" /><span>{{ t('nav.events') }}</span></button>
      <button :class="['nav-item', { active: currentView === 'settings' }]" :aria-current="currentView === 'settings' ? 'page' : undefined" @click="emit('navigate', 'settings')"><Settings2 :size="17" /><span>{{ t('nav.settings') }}</span></button>
      <button :class="['nav-item', { active: currentView === 'about' }]" :aria-current="currentView === 'about' ? 'page' : undefined" @click="emit('navigate', 'about')"><Info :size="17" /><span>{{ t('nav.about') }}</span><span v-if="hasUpdate" class="update-dot" :aria-label="t('update.available', { version: '' })"></span></button>
    </nav>
    <div class="sidebar-footer"><span>v{{ appVersion }}</span><button class="icon-button" type="button" :aria-label="t('about.website')" :title="t('about.website')" @click="emit('openWebsite')"><Globe :size="16" /></button></div>
  </aside>
</template>
