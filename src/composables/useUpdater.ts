import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'

type UpdateMode = 'unknown' | 'installed' | 'portable' | 'development' | 'unsupported'
type UpdateStatus = 'idle' | 'checking' | 'available' | 'downloading' | 'installing' | 'ready' | 'upToDate' | 'error'

export function useUpdater() {
  const mode = ref<UpdateMode>('unknown')
  const status = ref<UpdateStatus>('idle')
  const newVersion = ref('')
  const progress = ref<number | null>(null)
  const errorMessage = ref('')
  const busy = computed(() => ['checking', 'downloading', 'installing'].includes(status.value))
  let pendingUpdate: Update | null = null
  let disposed = false

  async function checkForUpdates(silent = false) {
    if (disposed || busy.value || status.value === 'ready') return
    status.value = 'checking'
    errorMessage.value = ''

    try {
      mode.value = await invoke<UpdateMode>('get_update_mode')
      if (disposed) return
      if (mode.value !== 'installed') {
        status.value = 'idle'
        return
      }
      const update = await check({ timeout: 15000 })
      if (disposed) {
        await update?.close().catch(() => {})
        return
      }
      const previous = pendingUpdate
      pendingUpdate = update
      newVersion.value = update?.version ?? ''
      status.value = update ? 'available' : 'upToDate'
      // Each successful check owns a native resource, including repeated checks of the same version.
      await previous?.close().catch(() => {})
    } catch (error) {
      status.value = silent ? (pendingUpdate ? 'available' : 'idle') : 'error'
      if (!silent) errorMessage.value = String(error)
    }
  }

  async function restartApp() {
    if (status.value !== 'ready') return
    status.value = 'installing'
    errorMessage.value = ''
    try {
      await relaunch()
    } catch (error) {
      status.value = 'ready'
      errorMessage.value = String(error)
    }
  }

  async function installUpdate() {
    if (!pendingUpdate || mode.value !== 'installed' || busy.value || status.value === 'ready') return
    status.value = 'downloading'
    progress.value = null
    errorMessage.value = ''

    try {
      let downloaded = 0
      let total = 0
      await pendingUpdate.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? 0
          progress.value = total > 0 ? 0 : null
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength
          if (total > 0) progress.value = Math.min(100, Math.round(downloaded / total * 100))
        } else if (event.event === 'Finished') {
          status.value = 'installing'
        }
      }, { timeout: 300000 })
      // Windows exits through the installer; macOS needs an explicit relaunch.
      status.value = 'ready'
      await restartApp()
    } catch (error) {
      status.value = 'error'
      errorMessage.value = String(error)
    }
  }

  async function openReleases() {
    errorMessage.value = ''
    try {
      await openUrl('https://github.com/Smileher/RemindOn/releases/latest')
    } catch (error) {
      errorMessage.value = String(error)
    }
  }

  function dispose() {
    disposed = true
    const update = pendingUpdate
    pendingUpdate = null
    void update?.close().catch(() => {})
  }

  return {
    mode, status, newVersion, progress, errorMessage, busy,
    checkForUpdates, installUpdate, restartApp, openReleases, dispose,
  }
}
