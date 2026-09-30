import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { openUrl } from '@tauri-apps/plugin-opener'

function logUpdaterError(context: string, error: unknown) {
  console.error(`[RemindOn] ${context}`, error)
}

type UpdateMode = 'unknown' | 'installed' | 'portable' | 'development' | 'unsupported'
type UpdatePhase = 'idle' | 'checking' | 'available' | 'downloading' | 'installing' | 'error'
type UpdateStatus = UpdatePhase | 'upToDate'
type UpdateRuntimeState = { phase: UpdatePhase; version: string | null; error: string | null }
type UpdateProgress = { downloadedBytes: number; totalBytes: number | null; percentage: number | null }

export function useUpdater() {
  const mode = ref<UpdateMode>('unknown')
  const status = ref<UpdateStatus>('idle')
  const newVersion = ref('')
  const progress = ref<number | null>(null)
  const errorMessage = ref('')
  const busy = computed(() => ['checking', 'downloading', 'installing'].includes(status.value))
  let disposed = false
  let statusListener: Promise<UnlistenFn> | undefined
  let progressListener: Promise<UnlistenFn> | undefined

  function applyRuntimeState(runtime: UpdateRuntimeState) {
    newVersion.value = runtime.version ?? ''
    status.value = runtime.phase === 'idle' ? 'upToDate' : runtime.phase
    errorMessage.value = runtime.error ? 'update-failed' : ''
  }

  function applyProgress(runtime: UpdateProgress) {
    progress.value = runtime.percentage
  }

  async function ensureListeners() {
    if (!statusListener) {
      statusListener = listen<UpdateRuntimeState>('update-status', (event) => {
        if (!disposed) applyRuntimeState(event.payload)
      }).catch((error) => {
        logUpdaterError('listen for update status', error)
        throw error
      })
    }
    if (!progressListener) {
      progressListener = listen<UpdateProgress>('update-progress', (event) => {
        if (!disposed) applyProgress(event.payload)
      }).catch((error) => {
        logUpdaterError('listen for update progress', error)
        throw error
      })
    }
    await Promise.allSettled([statusListener, progressListener])
  }

  async function loadStatus(): Promise<boolean> {
    if (disposed) return false
    if (typeof __REMINDON_STORE_BUILD__ !== 'undefined' && __REMINDON_STORE_BUILD__) return true
    try {
      await ensureListeners()
      mode.value = await invoke<UpdateMode>('get_update_mode')
      if (disposed) return false
      if (mode.value !== 'installed' && mode.value !== 'portable') {
        status.value = 'idle'
        newVersion.value = ''
        return true
      }
      const runtime = await invoke<UpdateRuntimeState>('get_update_status')
      const runtimeProgress = await invoke<UpdateProgress | null>('get_update_progress')
      if (disposed) return false
      applyRuntimeState(runtime)
      progress.value = runtimeProgress?.percentage ?? null
      return true
    } catch (error) {
      logUpdaterError('load update status', error)
      return false
    }
  }

  async function checkForUpdates(silent = false): Promise<boolean> {
    if (disposed || busy.value) return false
    if (typeof __REMINDON_STORE_BUILD__ !== 'undefined' && __REMINDON_STORE_BUILD__) return true
    status.value = 'checking'
    errorMessage.value = ''
    progress.value = null
    try {
      await ensureListeners()
      mode.value = await invoke<UpdateMode>('get_update_mode')
      if (disposed) return false
      if (mode.value !== 'installed' && mode.value !== 'portable') {
        status.value = 'idle'
        newVersion.value = ''
        return true
      }
      const runtime = await invoke<UpdateRuntimeState>('check_for_updates', { force: !silent })
      if (disposed) return false
      applyRuntimeState(runtime)
      const runtimeProgress = await invoke<UpdateProgress | null>('get_update_progress')
      progress.value = runtimeProgress?.percentage ?? null
      return true
    } catch (error) {
      logUpdaterError('check for updates', error)
      if (silent) {
        status.value = 'idle'
        errorMessage.value = ''
      } else {
        status.value = 'error'
        errorMessage.value = 'update-failed'
      }
      return false
    }
  }

  async function openReleases() {
    errorMessage.value = ''
    try {
      await openUrl('https://github.com/Smileher/RemindOn/releases/latest')
    } catch (error) {
      logUpdaterError('open release page', error)
      errorMessage.value = 'update-failed'
    }
  }

  async function openAuthorPage() {
    try {
      await openUrl('https://smileher.github.io/RemindOn/')
    } catch (error) {
      logUpdaterError('open author page', error)
    }
  }

  function dispose() {
    disposed = true
    void statusListener?.then((unlisten) => unlisten(), () => {})
    void progressListener?.then((unlisten) => unlisten(), () => {})
    statusListener = undefined
    progressListener = undefined
  }

  return {
    mode, status, newVersion, progress, errorMessage, busy,
    loadStatus, checkForUpdates, openReleases, openAuthorPage, dispose,
  }
}
