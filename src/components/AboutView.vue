<script setup lang="ts">
import { BellRing, Download, ExternalLink, Info, RotateCw } from '@lucide/vue'
import brandIcon from '../assets/remindon.svg'
import donationCode from '../assets/donate.png'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { Language } from '../types'

const props = defineProps<{
  language: Language
  appVersion: string
  isStoreBuild: boolean
  updateMode: string
  updateStatus: string
  updateStatusText: string
  newVersion: string | null
  updateProgress: number | null
  updateError: string
  updateBusy: boolean
}>()

const emit = defineEmits<{
  checkForUpdates: []
  openReleases: []
  openAuthorPage: []
}>()

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}
</script>

<template>
  <section class="page-section about-section">
    <div class="about-overview">
      <img :src="brandIcon" alt="" />
      <div class="about-product"><h1>RemindOn</h1><p>{{ t('app.tagline') }}</p></div>
      <span class="about-version">{{ t('about.version', { version: appVersion }) }}</span>
    </div>

    <div v-if="!isStoreBuild" class="update-panel" aria-live="polite">
      <div class="update-summary">
        <span :class="['update-icon', { checking: updateStatus === 'checking' }]"><RotateCw :size="18" /></span>
        <div><span>{{ t('update.title') }}</span><strong>{{ updateStatusText }}</strong></div>
      </div>
      <div class="update-actions">
        <button class="button" type="button" :disabled="updateBusy || updateMode === 'development' || updateMode === 'unsupported'" @click="emit('checkForUpdates')">
          <RotateCw :class="{ checking: updateStatus === 'checking' }" :size="14" />{{ t('update.check') }}
        </button>
        <button v-if="updateMode === 'unsupported' || updateError" class="button" type="button" @click="emit('openReleases')"><Download :size="14" />{{ t('update.download') }}</button>
      </div>
      <div v-if="updateStatus === 'downloading'" class="update-progress" role="progressbar" :aria-label="t('update.downloading')" :aria-valuemin="0" :aria-valuemax="100" :aria-valuenow="updateProgress ?? undefined">
        <div :class="['update-progress-track', { indeterminate: updateProgress === null }]"><span :style="updateProgress === null ? undefined : { width: `${updateProgress}%` }"></span></div>
        <span v-if="updateProgress !== null">{{ updateProgress }}%</span>
      </div>
      <p v-if="updateMode === 'portable' && updateStatus === 'available' && updateProgress === 100" class="update-download-note" role="status">{{ t('update.portableDownloaded') }}</p>
      <p v-if="updateError" class="update-error" role="alert">{{ t('update.failed') }}</p>
    </div>
    <div v-else class="update-panel" aria-live="polite">
      <div class="update-summary">
        <span class="update-icon"><Info :size="18" /></span>
        <div><span>{{ t('update.title') }}</span><strong>{{ t('update.storeManaged') }}</strong></div>
      </div>
    </div>

    <div class="support-section">
      <div class="support-copy">
        <span class="support-icon"><BellRing :size="19" /></span>
        <div>
          <strong>{{ t('about.support') }}</strong>
          <span>{{ t('about.author') }} <button class="author-link" type="button" @click="emit('openAuthorPage')">Smileher <ExternalLink :size="12" /></button></span>
        </div>
      </div>
      <div class="donation-code"><img :src="donationCode" alt="" /><img class="donation-logo" :src="brandIcon" alt="" /></div>
    </div>
    <p class="about-copyright">{{ t('about.copyright') }}</p>
  </section>
</template>
