import type {
  AutoPopulateTool,
  AutoPopulateToolFormState,
  GetStateResponse,
  NotificationItem,
  NotificationLevel,
  PanelConfig,
  PanelFrameEvent,
  PanelStatusEvent,
  SettingsModalState
} from './types'

export interface UiStoreState {
  data: GetStateResponse | null
  frames: Record<string, PanelFrameEvent>
  revealPassword: Record<string, boolean>
  modal: SettingsModalState | null
  notifications: NotificationItem[]
  autoPopulateToolOpen: boolean
  autoPopulateToolForm: AutoPopulateToolFormState | null
}

type Listener = (state: UiStoreState) => void

export const panelKey = (screenId: number, panelId: number): string => `${screenId}:${panelId}`

export const buildRtspPreview = (
  config: PanelConfig,
  username: string,
  password: string,
  revealPassword: boolean
): string => {
  const user = username.trim()
  const pass = password.trim()
  let credential = ''
  if (user.length > 0) {
    credential = user
    if (pass.length > 0) {
      credential += `:${revealPassword ? pass : '***'}`
    }
    credential += '@'
  }

  const parts = [config.path, config.channel ?? '', config.subtype ?? '']
    .map((value) => value.trim())
    .filter((value) => value.length > 0)
  const joinedPath = parts.join('/')

  return `rtsp://${credential}${config.host}:${config.port}/${joinedPath}`
}

export const rangeInclusive = (start: number, end: number): number[] => {
  if (!Number.isFinite(start) || !Number.isFinite(end)) {
    return []
  }
  const normalizedStart = Math.floor(start)
  const normalizedEnd = Math.floor(end)
  if (normalizedEnd < normalizedStart) {
    return []
  }
  return Array.from({ length: normalizedEnd - normalizedStart + 1 }, (_, index) => normalizedStart + index)
}

const replaceToken = (input: string, token: string, value: string): string => input.split(token).join(value)

export const resolveAutoPopulateRtspUrl = (
  tool: AutoPopulateTool,
  cameraNum: number,
  subNum: number
): string => {
  let url = tool.base_url_template
  url = replaceToken(url, '$cameraNum', String(cameraNum))
  url = replaceToken(url, '$subNum', String(subNum))
  url = replaceToken(url, '$USERNAME', tool.username)
  url = replaceToken(url, '$PASSWORD', tool.password)
  url = replaceToken(url, '$IP', tool.ip)
  url = replaceToken(url, '$PORT', tool.port)
  return url
}

export const redactRtspPassword = (url: string, revealPassword: boolean): string => {
  if (revealPassword) {
    return url
  }
  try {
    const parsed = new URL(url)
    if (parsed.password.length > 0) {
      parsed.password = '***'
      return parsed.toString()
    }
    return parsed.toString()
  } catch {
    return url.replace(/(rtsp:\/\/[^:/@]+:)([^@]*)(@)/i, '$1***$3')
  }
}

export interface ParsedRtspUrl {
  host: string
  port: number
  path: string
  username: string
  password: string
}

export const parseRtspUrl = (url: string): ParsedRtspUrl => {
  const parsed = new URL(url)
  const path = `${parsed.pathname.replace(/^\/+/, '')}${parsed.search}`
  return {
    host: parsed.hostname,
    port: parsed.port.length > 0 ? Number(parsed.port) : 554,
    path,
    username: decodeURIComponent(parsed.username),
    password: decodeURIComponent(parsed.password)
  }
}

export const toAutoPopulateToolFormState = (state: GetStateResponse): AutoPopulateToolFormState => ({
  baseUrlTemplate: state.auto_populate_tool.base_url_template,
  username: state.auto_populate_tool.username,
  password: state.auto_populate_tool.password,
  ip: state.auto_populate_tool.ip,
  port: state.auto_populate_tool.port,
  cameraNumStart: String(state.auto_populate_tool.camera_num_start),
  cameraNumEnd: String(state.auto_populate_tool.camera_num_end),
  subNumStart: String(state.auto_populate_tool.sub_num_start),
  subNumEnd: String(state.auto_populate_tool.sub_num_end)
})

export class UiStore {
  private state: UiStoreState
  private listeners = new Set<Listener>()
  private notificationCounter = 0

  constructor() {
    this.state = {
      data: null,
      frames: {},
      revealPassword: {},
      modal: null,
      notifications: [],
      autoPopulateToolOpen: false,
      autoPopulateToolForm: null
    }
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener)
    listener(this.snapshot())
    return () => {
      this.listeners.delete(listener)
    }
  }

  snapshot(): UiStoreState {
    return {
      data: this.state.data,
      frames: { ...this.state.frames },
      revealPassword: { ...this.state.revealPassword },
      modal: this.state.modal ? { ...this.state.modal, form: { ...this.state.modal.form } } : null,
      notifications: [...this.state.notifications],
      autoPopulateToolOpen: this.state.autoPopulateToolOpen,
      autoPopulateToolForm: this.state.autoPopulateToolForm ? { ...this.state.autoPopulateToolForm } : null
    }
  }

  setData(data: GetStateResponse): void {
    this.state.data = data
    if (this.state.autoPopulateToolOpen) {
      this.state.autoPopulateToolForm = toAutoPopulateToolFormState(data)
    }
    this.emit()
  }

  openModal(modal: SettingsModalState): void {
    this.state.modal = modal
    this.emit()
  }

  updateModalField<K extends keyof SettingsModalState['form']>(field: K, value: SettingsModalState['form'][K]): void {
    if (!this.state.modal) {
      return
    }
    this.state.modal = {
      ...this.state.modal,
      form: {
        ...this.state.modal.form,
        [field]: value
      }
    }
    this.emit()
  }

  closeModal(): void {
    this.state.modal = null
    this.emit()
  }

  openAutoPopulateTool(): void {
    if (!this.state.data) {
      return
    }
    this.state.autoPopulateToolOpen = true
    this.state.autoPopulateToolForm = toAutoPopulateToolFormState(this.state.data)
    this.emit()
  }

  closeAutoPopulateTool(): void {
    this.state.autoPopulateToolOpen = false
    this.state.autoPopulateToolForm = null
    this.emit()
  }

  updateAutoPopulateToolField<K extends keyof AutoPopulateToolFormState>(
    field: K,
    value: AutoPopulateToolFormState[K]
  ): void {
    if (!this.state.autoPopulateToolForm) {
      return
    }
    this.state.autoPopulateToolForm = {
      ...this.state.autoPopulateToolForm,
      [field]: value
    }
    this.emit()
  }

  setRevealPassword(screenId: number, panelId: number, reveal: boolean): void {
    this.state.revealPassword[panelKey(screenId, panelId)] = reveal
    this.emit()
  }

  revealPassword(screenId: number, panelId: number): boolean {
    return this.state.revealPassword[panelKey(screenId, panelId)] ?? false
  }

  applyPanelStatus(event: PanelStatusEvent): void {
    if (!this.state.data) {
      return
    }
    const screen = this.state.data.screens[event.screen_id]
    if (!screen) {
      return
    }
    const panel = screen.panels[event.panel_id]
    if (!panel) {
      return
    }
    panel.status = {
      state: event.state,
      message: event.message,
      code: event.code
    }
    this.emit()
  }

  applyPanelFrame(event: PanelFrameEvent): void {
    const key = panelKey(event.screen_id, event.panel_id)
    const previous = this.state.frames[key]
    if (previous && previous.seq >= event.seq) {
      return
    }
    this.state.frames[key] = event
    this.emit()
  }

  frameDataUrl(screenId: number, panelId: number): string | null {
    const frame = this.state.frames[panelKey(screenId, panelId)]
    if (!frame) {
      return null
    }
    return `data:${frame.mime};base64,${frame.data_base64}`
  }

  addNotification(level: NotificationLevel, message: string): number {
    const id = this.notificationCounter++
    this.state.notifications = [...this.state.notifications, { id, level, message }]
    this.emit()
    return id
  }

  dismissNotification(id: number): void {
    this.state.notifications = this.state.notifications.filter((notification) => notification.id !== id)
    this.emit()
  }

  activePanelKey(): { screenId: number; panelId: number } | null {
    if (!this.state.data) {
      return null
    }
    const screenId = this.state.data.active_screen
    const panelId = this.state.data.active_panel_per_screen[screenId] ?? 0
    return { screenId, panelId }
  }

  private emit(): void {
    const snapshot = this.snapshot()
    for (const listener of this.listeners) {
      listener(snapshot)
    }
  }
}
