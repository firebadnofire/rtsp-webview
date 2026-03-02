export type Transport = 'tcp' | 'udp'

export type PanelState = 'idle' | 'connecting' | 'playing' | 'retrying' | 'error' | 'stopped'

export interface AutoPopulateTool {
  base_url_template: string
  username: string
  password: string
  ip: string
  port: string
  camera_num_start: number
  camera_num_end: number
  sub_num_start: number
  sub_num_end: number
}

export interface AdvancedConfig {
  connection_timeout_ms: number
  stall_timeout_ms: number
  retry_base_ms: number
  retry_max_ms: number
  retry_jitter_ms: number
  max_failures: number
}

export interface AdvancedConfigPatch {
  connection_timeout_ms?: number
  stall_timeout_ms?: number
  retry_base_ms?: number
  retry_max_ms?: number
  retry_jitter_ms?: number
  max_failures?: number
}

export interface SecretRef {
  key: string
}

export interface PanelConfig {
  title: string
  host: string
  port: number
  path: string
  channel: string | null
  subtype: string | null
  camera_num: number | null
  sub_num: number | null
  transport: Transport
  latency_ms: number
  secret_ref: SecretRef
  advanced: AdvancedConfig
}

export interface PanelConfigPatch {
  title?: string
  host?: string
  port?: number
  path?: string
  channel?: string | null
  subtype?: string | null
  camera_num?: number | null
  sub_num?: number | null
  transport?: Transport
  latency_ms?: number
  advanced?: AdvancedConfigPatch
}

export interface PanelRuntimeStatus {
  state: PanelState
  message: string
  code: string | null
}

export interface PanelStateView {
  config: PanelConfig
  status: PanelRuntimeStatus
  secret_present: boolean
  is_recording: boolean
}

export interface ScreenStateView {
  id: number
  panels: [PanelStateView, PanelStateView, PanelStateView, PanelStateView]
}

export interface GetStateResponse {
  ipc_version: string
  schema_version: number
  active_screen: number
  active_panel_per_screen: number[]
  fullscreen: boolean
  screens: ScreenStateView[]
  auto_populate_tool: AutoPopulateTool
}

export interface PanelStatusEvent {
  ipc_version: string
  screen_id: number
  panel_id: number
  state: PanelState
  message: string
  code: string | null
}

export interface PanelFrameEvent {
  ipc_version: string
  screen_id: number
  panel_id: number
  mime: string
  data_base64: string
  width: number | null
  height: number | null
  pts_ms: number | null
  seq: number
}

export interface ConfigLoadedEvent {
  ipc_version: string
  state: GetStateResponse
}

export interface SnapshotSavedEvent {
  ipc_version: string
  screen_id: number
  panel_id: number
  path: string
}

export interface SnapshotFailedEvent {
  ipc_version: string
  screen_id: number
  panel_id: number
  code: string
  message: string
}

export interface SecurityNoticeEvent {
  ipc_version: string
  code: string
  message: string
}

export interface CommandError {
  code: string
  message: string
}

export type NotificationLevel = 'info' | 'success' | 'error'

export interface NotificationItem {
  id: number
  level: NotificationLevel
  message: string
}

export interface SettingsModalState {
  screenId: number
  panelId: number
  form: {
    title: string
    host: string
    port: string
    path: string
    channel: string
    subtype: string
    cameraNum: string
    subNum: string
    transport: Transport
    latencyMs: string
    username: string
    password: string
    connectionTimeoutMs: string
    stallTimeoutMs: string
    retryBaseMs: string
    retryMaxMs: string
    retryJitterMs: string
    maxFailures: string
    clearSecret: boolean
  }
}

export interface AutoPopulateToolFormState {
  baseUrlTemplate: string
  username: string
  password: string
  ip: string
  port: string
  cameraNumStart: string
  cameraNumEnd: string
  subNumStart: string
  subNumEnd: string
}
