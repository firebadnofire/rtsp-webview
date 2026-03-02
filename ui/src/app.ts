import type { UnlistenFn } from '@tauri-apps/api/event'
import { tauriEventClient, type EventClient } from './events'
import { tauriIpcClient, type IpcClient } from './ipc'
import { renderFullscreenHint } from './components/fullscreen-hint'
import { renderGlobalSettingsPage } from './components/global-settings-page'
import { renderNotifications } from './components/notifications'
import { renderPanelCard } from './components/panel-card'
import { renderSettingsModal } from './components/settings-modal'
import { renderTabs } from './components/tabs'
import { renderToolbar } from './components/toolbar'
import {
  buildRtspPreview,
  parseRtspUrl,
  rangeInclusive,
  redactRtspPassword,
  resolveAutoPopulateRtspUrl,
  UiStore,
  type UiStoreState
} from './store'
import type {
  AutoPopulateTool,
  AutoPopulateToolFormState,
  CommandError,
  GetStateResponse,
  PanelConfig,
  PanelConfigPatch,
  SettingsModalState
} from './types'

interface AppDeps {
  ipc: IpcClient
  events: EventClient
}

interface FocusSnapshot {
  selector: string
  start: number | null
  end: number | null
}

const parseNumber = (value: string, fallback: number): number => {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

const clampInt = (value: number, min: number, max: number): number => Math.max(min, Math.min(max, Math.round(value)))

const toErrorMessage = (error: unknown): string => {
  if (typeof error === 'string') {
    return error
  }
  if (error && typeof error === 'object') {
    const candidate = error as Partial<CommandError>
    if (typeof candidate.message === 'string') {
      if (typeof candidate.code === 'string') {
        return `${candidate.code}: ${candidate.message}`
      }
      return candidate.message
    }
  }
  return 'Unknown error'
}

const actionTarget = (event: Event): HTMLElement | null => {
  const target = event.target
  if (!(target instanceof HTMLElement)) {
    return null
  }
  return target.closest<HTMLElement>('[data-action]')
}

const createFallbackState = (): GetStateResponse => ({
  ipc_version: '1',
  schema_version: 2,
  active_screen: 0,
  active_panel_per_screen: [],
  fullscreen: false,
  auto_populate_tool: {
    base_url_template: 'rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor?channel=$cameraNum&subtype=$subNum',
    username: '',
    password: '',
    ip: '',
    port: '554',
    camera_num_start: 1,
    camera_num_end: 16,
    sub_num_start: 0,
    sub_num_end: 1
  },
  screens: []
})

const toolRanges = (state: GetStateResponse): { cameraNumbers: number[]; subtypeNumbers: number[] } => ({
  cameraNumbers: rangeInclusive(state.auto_populate_tool.camera_num_start, state.auto_populate_tool.camera_num_end),
  subtypeNumbers: rangeInclusive(state.auto_populate_tool.sub_num_start, state.auto_populate_tool.sub_num_end)
})

export class RtspViewerApp {
  private root: HTMLElement
  private deps: AppDeps
  private store = new UiStore()
  private unlistenFns: UnlistenFn[] = []
  private unsubscribeStore: (() => void) | null = null

  constructor(root: HTMLElement, deps: AppDeps) {
    this.root = root
    this.deps = deps
  }

  async start(): Promise<void> {
    this.unsubscribeStore = this.store.subscribe((state) => {
      this.render(state)
    })

    this.root.addEventListener('click', this.handleClick)
    this.root.addEventListener('input', this.handleInput)
    this.root.addEventListener('change', this.handleChange)
    this.root.addEventListener('submit', this.handleSubmit)
    window.addEventListener('keydown', this.handleKeydown)

    const startupErrors: string[] = []
    try {
      await this.attachBackendListeners()
    } catch (error) {
      startupErrors.push(`Event channel unavailable: ${toErrorMessage(error)}`)
    }

    try {
      await this.syncState()
    } catch (error) {
      startupErrors.push(`Backend state unavailable: ${toErrorMessage(error)}`)
    }

    if (!this.store.snapshot().data) {
      this.store.setData(createFallbackState())
    }

    for (const message of startupErrors) {
      this.notify('info', message)
    }
  }

  async destroy(): Promise<void> {
    this.root.removeEventListener('click', this.handleClick)
    this.root.removeEventListener('input', this.handleInput)
    this.root.removeEventListener('change', this.handleChange)
    this.root.removeEventListener('submit', this.handleSubmit)
    window.removeEventListener('keydown', this.handleKeydown)
    for (const unlisten of this.unlistenFns) {
      await unlisten()
    }
    this.unlistenFns = []
    if (this.unsubscribeStore) {
      this.unsubscribeStore()
      this.unsubscribeStore = null
    }
  }

  private readonly handleKeydown = async (event: KeyboardEvent): Promise<void> => {
    const state = this.store.snapshot()
    if (!state.data?.fullscreen) {
      return
    }
    const key = event.key.toLowerCase()
    if (key === 'escape' || key === 'f11' || key === 'q') {
      event.preventDefault()
      await this.execute(async () => {
        await this.deps.ipc.toggleFullscreen(false)
        await this.syncState()
      })
    }
  }

  private readonly handleSubmit = async (event: Event): Promise<void> => {
    const form = event.target
    if (!(form instanceof HTMLFormElement)) {
      return
    }
    const action = form.dataset.action
    if (action === 'submit-settings') {
      event.preventDefault()
      await this.saveModal()
      return
    }
    if (action === 'run-auto-populate-tool') {
      event.preventDefault()
      await this.runAutoPopulateTool()
    }
  }

  private readonly handleInput = (event: Event): void => {
    const target = event.target
    if (!(target instanceof HTMLInputElement || target instanceof HTMLSelectElement)) {
      return
    }

    const toolField = target.dataset.toolField as keyof AutoPopulateToolFormState | undefined
    if (toolField) {
      this.store.updateAutoPopulateToolField(toolField, target.value)
      return
    }

    const field = target.dataset.field as keyof SettingsModalState['form'] | undefined
    if (!field) {
      return
    }
    if (target instanceof HTMLInputElement && target.type === 'checkbox') {
      this.store.updateModalField(field, target.checked)
    } else {
      this.store.updateModalField(field, target.value)
    }
  }

  private readonly handleChange = (event: Event): void => {
    const target = event.target
    if (!(target instanceof HTMLInputElement || target instanceof HTMLSelectElement)) {
      return
    }

    if (target.dataset.action === 'toggle-reveal') {
      const modal = this.store.snapshot().modal
      if (!modal || !(target instanceof HTMLInputElement)) {
        return
      }
      this.store.setRevealPassword(modal.screenId, modal.panelId, target.checked)
      return
    }

    const toolField = target.dataset.toolField as keyof AutoPopulateToolFormState | undefined
    if (toolField) {
      this.store.updateAutoPopulateToolField(toolField, target.value)
      return
    }

    const field = target.dataset.field as keyof SettingsModalState['form'] | undefined
    if (!field) {
      return
    }

    if (target instanceof HTMLInputElement && target.type === 'checkbox') {
      this.store.updateModalField(field, target.checked)
      return
    }

    this.store.updateModalField(field, target.value)
  }

  private readonly handleClick = async (event: Event): Promise<void> => {
    const target = actionTarget(event)
    if (!target) {
      return
    }

    const action = target.dataset.action
    if (!action) {
      return
    }

    const screenId = Number(target.dataset.screenId)
    const panelId = Number(target.dataset.panelId)

    if (action === 'dismiss-notice') {
      const id = Number(target.dataset.id)
      if (Number.isFinite(id)) {
        this.store.dismissNotification(id)
      }
      return
    }

    if (action === 'close-modal') {
      if (target.classList.contains('modal-backdrop')) {
        const originalTarget = event.target
        if (originalTarget instanceof HTMLElement && originalTarget.closest('.modal')) {
          return
        }
      }
      this.store.closeModal()
      return
    }

    await this.execute(async () => {
      switch (action) {
        case 'switch-screen':
          await this.deps.ipc.setActiveScreen(screenId)
          await this.syncState()
          break
        case 'select-panel':
          await this.deps.ipc.setActivePanel(screenId, panelId)
          await this.syncState()
          break
        case 'start-stream':
          await this.deps.ipc.startStream(screenId, panelId)
          await this.syncState()
          break
        case 'stop-stream':
          await this.deps.ipc.stopStream(screenId, panelId)
          await this.syncState()
          break
        case 'snapshot': {
          const path = await this.deps.ipc.snapshot(screenId, panelId, null)
          this.notify('success', `Snapshot saved: ${path}`)
          break
        }
        case 'toggle-recording': {
          const path = await this.deps.ipc.toggleRecording(screenId, panelId, null)
          await this.syncState()
          if (path) {
            this.notify('success', `Recording saved: ${path}`)
          } else {
            this.notify('success', `Recording started on Screen ${screenId + 1} Panel ${panelId + 1}`)
          }
          break
        }
        case 'open-settings':
          this.openSettings(screenId, panelId)
          break
        case 'start-screen': {
          const snapshot = this.store.snapshot()
          if (snapshot.data && snapshot.data.screens.length > 0) {
            await this.deps.ipc.startScreen(snapshot.data.active_screen)
            await this.syncState()
          }
          break
        }
        case 'stop-screen': {
          const snapshot = this.store.snapshot()
          if (snapshot.data && snapshot.data.screens.length > 0) {
            await this.deps.ipc.stopScreen(snapshot.data.active_screen)
            await this.syncState()
          }
          break
        }
        case 'start-all':
          await this.deps.ipc.startAllGlobal()
          await this.syncState()
          break
        case 'stop-all':
          await this.deps.ipc.stopAllGlobal()
          await this.syncState()
          break
        case 'save-config': {
          const path = await this.deps.ipc.saveConfig(null)
          this.notify('success', `Config saved: ${path}`)
          await this.syncState()
          break
        }
        case 'load-config': {
          const path = await this.deps.ipc.loadConfig(null)
          this.notify('success', `Config loaded: ${path}`)
          await this.syncState()
          break
        }
        case 'toggle-fullscreen': {
          const snapshot = this.store.snapshot()
          const next = !(snapshot.data?.fullscreen ?? false)
          await this.deps.ipc.toggleFullscreen(next)
          await this.syncState()
          break
        }
        case 'create-screen':
          await this.deps.ipc.createScreen()
          await this.syncState()
          break
        case 'delete-screen':
          await this.deps.ipc.deleteScreen(screenId)
          await this.syncState()
          break
        case 'toggle-auto-populate-tool': {
          const snapshot = this.store.snapshot()
          if (snapshot.autoPopulateToolOpen) {
            this.store.closeAutoPopulateTool()
          } else {
            this.store.openAutoPopulateTool()
          }
          break
        }
        case 'close-auto-populate-tool':
          this.store.closeAutoPopulateTool()
          break
        default:
          break
      }
    })
  }

  private async attachBackendListeners(): Promise<void> {
    const listeners = await Promise.all([
      this.deps.events.onPanelStatus((payload) => this.store.applyPanelStatus(payload)),
      this.deps.events.onPanelFrame((payload) => this.store.applyPanelFrame(payload)),
      this.deps.events.onConfigLoaded((payload) => this.store.setData(payload.state)),
      this.deps.events.onSnapshotSaved((payload) => {
        this.notify('success', `Snapshot saved: ${payload.path}`)
      }),
      this.deps.events.onSnapshotFailed((payload) => {
        this.notify('error', `${payload.code}: ${payload.message}`)
      }),
      this.deps.events.onSecurityNotice((payload) => {
        this.notify('info', `${payload.code}: ${payload.message}`)
      })
    ])
    this.unlistenFns = listeners
  }

  private async syncState(): Promise<void> {
    const state = await this.deps.ipc.getState()
    this.store.setData(state)
  }

  private openSettings(screenId: number, panelId: number): void {
    const snapshot = this.store.snapshot()
    if (!snapshot.data) {
      return
    }
    const screen = snapshot.data.screens[screenId]
    if (!screen) {
      return
    }
    const panel = screen.panels[panelId]
    if (!panel) {
      return
    }

    const ranges = toolRanges(snapshot.data)

    this.store.openModal({
      screenId,
      panelId,
      form: {
        title: panel.config.title,
        host: panel.config.host,
        port: String(panel.config.port),
        path: panel.config.path,
        channel: panel.config.channel ?? '',
        subtype: panel.config.subtype ?? '',
        cameraNum: panel.config.camera_num !== null ? String(panel.config.camera_num) : '',
        subNum: panel.config.sub_num !== null ? String(panel.config.sub_num) : String(ranges.subtypeNumbers[0] ?? 0),
        transport: panel.config.transport,
        latencyMs: String(panel.config.latency_ms),
        username: '',
        password: '',
        connectionTimeoutMs: String(panel.config.advanced.connection_timeout_ms),
        stallTimeoutMs: String(panel.config.advanced.stall_timeout_ms),
        retryBaseMs: String(panel.config.advanced.retry_base_ms),
        retryMaxMs: String(panel.config.advanced.retry_max_ms),
        retryJitterMs: String(panel.config.advanced.retry_jitter_ms),
        maxFailures: String(panel.config.advanced.max_failures),
        clearSecret: false
      }
    })
  }

  private createAutoPopulateTool(form: AutoPopulateToolFormState): AutoPopulateTool {
    const cameraNumStart = clampInt(parseNumber(form.cameraNumStart, 1), 0, 9999)
    const cameraNumEndRaw = clampInt(parseNumber(form.cameraNumEnd, cameraNumStart), 0, 9999)
    const subNumStart = clampInt(parseNumber(form.subNumStart, 0), 0, 9999)
    const subNumEndRaw = clampInt(parseNumber(form.subNumEnd, subNumStart), 0, 9999)
    const cameraNumEnd = Math.max(cameraNumStart, cameraNumEndRaw)
    const subNumEnd = Math.max(subNumStart, subNumEndRaw)

    return {
      base_url_template: form.baseUrlTemplate,
      username: form.username,
      password: form.password,
      ip: form.ip,
      port: form.port.trim().length > 0 ? form.port.trim() : '554',
      camera_num_start: cameraNumStart,
      camera_num_end: cameraNumEnd,
      sub_num_start: subNumStart,
      sub_num_end: subNumEnd
    }
  }

  private async runAutoPopulateTool(): Promise<void> {
    const snapshot = this.store.snapshot()
    if (!snapshot.autoPopulateToolForm) {
      return
    }

    const tool = this.createAutoPopulateTool(snapshot.autoPopulateToolForm)

    await this.execute(async () => {
      await this.deps.ipc.autoPopulateCameras(tool)
      await this.syncState()
      this.store.openAutoPopulateTool()
      this.notify('success', 'Auto-population complete')
    })
  }

  private createAdvancedPatch(current: PanelConfig, modal: SettingsModalState): PanelConfigPatch['advanced'] {
    return {
      connection_timeout_ms: clampInt(
        parseNumber(modal.form.connectionTimeoutMs, current.advanced.connection_timeout_ms),
        100,
        120000
      ),
      stall_timeout_ms: clampInt(parseNumber(modal.form.stallTimeoutMs, current.advanced.stall_timeout_ms), 100, 120000),
      retry_base_ms: clampInt(parseNumber(modal.form.retryBaseMs, current.advanced.retry_base_ms), 100, 120000),
      retry_max_ms: clampInt(parseNumber(modal.form.retryMaxMs, current.advanced.retry_max_ms), 100, 120000),
      retry_jitter_ms: clampInt(parseNumber(modal.form.retryJitterMs, current.advanced.retry_jitter_ms), 0, 120000),
      max_failures: clampInt(parseNumber(modal.form.maxFailures, current.advanced.max_failures), 1, 1000)
    }
  }

  private async saveModal(): Promise<void> {
    const snapshot = this.store.snapshot()
    const modal = snapshot.modal
    const data = snapshot.data
    if (!modal || !data) {
      return
    }

    const panel = data.screens[modal.screenId]?.panels[modal.panelId]
    if (!panel) {
      return
    }

    const cameraNumProvided = modal.form.cameraNum.trim().length > 0
    const cameraNum = cameraNumProvided ? clampInt(parseNumber(modal.form.cameraNum, 0), 0, 9999) : null
    const subNum = clampInt(parseNumber(modal.form.subNum, 0), 0, 9999)

    await this.execute(async () => {
      let host = modal.form.host.trim()
      let port = clampInt(parseNumber(modal.form.port, panel.config.port), 1, 65535)
      let path = modal.form.path.trim()
      let username = modal.form.username
      let password = modal.form.password

      if (cameraNum !== null) {
        const resolved = resolveAutoPopulateRtspUrl(data.auto_populate_tool, cameraNum, subNum)
        const parsed = parseRtspUrl(resolved)
        host = parsed.host
        port = clampInt(parsed.port, 1, 65535)
        path = parsed.path
        username = parsed.username
        password = parsed.password
      }

      const patch: PanelConfigPatch = {
        title: modal.form.title,
        host,
        port,
        path,
        channel: modal.form.channel.trim().length === 0 ? null : modal.form.channel.trim(),
        subtype: modal.form.subtype.trim().length === 0 ? null : modal.form.subtype.trim(),
        camera_num: cameraNum,
        sub_num: cameraNum !== null ? subNum : null,
        transport: modal.form.transport,
        latency_ms: clampInt(parseNumber(modal.form.latencyMs, panel.config.latency_ms), 0, 5000),
        advanced: this.createAdvancedPatch(panel.config, modal)
      }

      await this.deps.ipc.updatePanelConfig(modal.screenId, modal.panelId, patch)

      const shouldUpdateSecret =
        modal.form.clearSecret || username.trim().length > 0 || password.trim().length > 0
      if (shouldUpdateSecret) {
        if (modal.form.clearSecret) {
          await this.deps.ipc.setPanelSecret(modal.screenId, modal.panelId, '', '')
        } else {
          await this.deps.ipc.setPanelSecret(modal.screenId, modal.panelId, username, password)
        }
      }

      this.store.closeModal()
      await this.syncState()
      this.notify('success', `Updated panel ${modal.panelId + 1} on Screen ${modal.screenId + 1}`)
    })
  }

  private notify(level: 'info' | 'success' | 'error', message: string): void {
    const id = this.store.addNotification(level, message)
    window.setTimeout(() => {
      this.store.dismissNotification(id)
    }, 5000)
  }

  private captureFocusSnapshot(): FocusSnapshot | null {
    const active = document.activeElement
    if (
      !(active instanceof HTMLInputElement) &&
      !(active instanceof HTMLTextAreaElement) &&
      !(active instanceof HTMLSelectElement)
    ) {
      return null
    }
    if (!this.root.contains(active)) {
      return null
    }

    const toolField = active.dataset.toolField
    if (toolField) {
      return {
        selector: `[data-tool-field="${toolField}"]`,
        start:
          active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement
            ? active.selectionStart
            : null,
        end:
          active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement ? active.selectionEnd : null
      }
    }

    const field = active.dataset.field
    if (!field) {
      return null
    }

    return {
      selector: `[data-field="${field}"]`,
      start:
        active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement ? active.selectionStart : null,
      end:
        active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement ? active.selectionEnd : null
    }
  }

  private restoreFocusSnapshot(snapshot: FocusSnapshot | null): void {
    if (!snapshot) {
      return
    }
    const target = this.root.querySelector(snapshot.selector)
    if (
      !(target instanceof HTMLInputElement) &&
      !(target instanceof HTMLTextAreaElement) &&
      !(target instanceof HTMLSelectElement)
    ) {
      return
    }

    target.focus()
    if (
      snapshot.start !== null &&
      snapshot.end !== null &&
      (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement)
    ) {
      target.setSelectionRange(snapshot.start, snapshot.end)
    }
  }

  private async execute(fn: () => Promise<void>): Promise<void> {
    try {
      await fn()
    } catch (error) {
      this.notify('error', toErrorMessage(error))
    }
  }

  private render(state: UiStoreState): void {
    if (!state.data) {
      this.root.innerHTML = '<main class="shell"><div class="loading">Loading state...</div></main>'
      return
    }

    const activeScreen = state.data.screens[state.data.active_screen]
    const activePanelId = state.data.active_panel_per_screen[state.data.active_screen] ?? 0

    const cards = activeScreen
      ? activeScreen.panels
          .map((panel, panelId) =>
            renderPanelCard({
              screenId: activeScreen.id,
              panelId,
              panel,
              active: panelId === activePanelId,
              frameUrl: this.store.frameDataUrl(activeScreen.id, panelId)
            })
          )
          .join('')
      : ''

    let modalMarkup = ''
    if (state.modal) {
      const panel = state.data.screens[state.modal.screenId]?.panels[state.modal.panelId]
      if (panel) {
        const reveal = this.store.revealPassword(state.modal.screenId, state.modal.panelId)
        const cameraNumProvided = state.modal.form.cameraNum.trim().length > 0
        let previewUrl = ''

        if (cameraNumProvided) {
          const cameraNum = clampInt(parseNumber(state.modal.form.cameraNum, 0), 0, 9999)
          const subNum = clampInt(parseNumber(state.modal.form.subNum, 0), 0, 9999)
          const resolved = resolveAutoPopulateRtspUrl(state.data.auto_populate_tool, cameraNum, subNum)
          previewUrl = redactRtspPassword(resolved, reveal)
        } else {
          const configForPreview: PanelConfig = {
            ...panel.config,
            title: state.modal.form.title,
            host: state.modal.form.host,
            port: clampInt(parseNumber(state.modal.form.port, panel.config.port), 1, 65535),
            path: state.modal.form.path,
            channel: state.modal.form.channel.trim().length === 0 ? null : state.modal.form.channel,
            subtype: state.modal.form.subtype.trim().length === 0 ? null : state.modal.form.subtype,
            camera_num: null,
            sub_num: null,
            transport: state.modal.form.transport,
            latency_ms: clampInt(parseNumber(state.modal.form.latencyMs, panel.config.latency_ms), 0, 5000)
          }
          previewUrl = buildRtspPreview(
            configForPreview,
            state.modal.form.username,
            state.modal.form.password,
            reveal
          )
        }

        const ranges = toolRanges(state.data)
        modalMarkup = renderSettingsModal({
          modal: state.modal,
          panel,
          previewUrl,
          revealPassword: reveal,
          subtypeOptions: ranges.subtypeNumbers.length > 0 ? ranges.subtypeNumbers : [0]
        })
      }
    }

    let contentMarkup = `${renderTabs(state.data)}
      ${renderFullscreenHint(state.data.fullscreen)}
      ${
        cards.length > 0
          ? `<section class="screen-grid">${cards}</section>`
          : '<section class="loading">No screens yet. Use Auto-population Tool or create a screen manually.</section>'
      }`

    if (state.autoPopulateToolOpen && state.autoPopulateToolForm) {
      const cameraStart = clampInt(parseNumber(state.autoPopulateToolForm.cameraNumStart, 0), 0, 9999)
      const cameraEnd = clampInt(parseNumber(state.autoPopulateToolForm.cameraNumEnd, cameraStart), 0, 9999)
      const subStart = clampInt(parseNumber(state.autoPopulateToolForm.subNumStart, 0), 0, 9999)
      const subEnd = clampInt(parseNumber(state.autoPopulateToolForm.subNumEnd, subStart), 0, 9999)
      const cameraCount = Math.max(0, cameraEnd - cameraStart + 1)
      const subtypeCount = Math.max(0, subEnd - subStart + 1)
      contentMarkup = renderGlobalSettingsPage({
        form: state.autoPopulateToolForm,
        cameraCount,
        subtypeCount
      })
    }

    const focusSnapshot = this.captureFocusSnapshot()
    this.root.innerHTML = `<main class="shell">
      ${renderNotifications(state.notifications)}
      ${renderToolbar(state.data, state.autoPopulateToolOpen)}
      ${contentMarkup}
      ${modalMarkup}
    </main>`
    this.restoreFocusSnapshot(focusSnapshot)
  }
}

export const createRtspViewerApp = (
  root: HTMLElement,
  deps: Partial<AppDeps> = {}
): RtspViewerApp => {
  return new RtspViewerApp(root, {
    ipc: deps.ipc ?? tauriIpcClient,
    events: deps.events ?? tauriEventClient
  })
}
