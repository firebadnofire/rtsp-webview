import type { GetStateResponse, PanelState } from '../src/types'

const makePanel = (screenId: number, panelId: number, state: PanelState = 'idle') => ({
  config: {
    title: `Screen ${screenId + 1} Panel ${panelId + 1}`,
    host: '',
    port: 554,
    path: '',
    channel: null,
    subtype: null,
    camera_num: null,
    sub_num: null,
    transport: 'tcp' as const,
    latency_ms: 200,
    secret_ref: {
      key: `screen_${screenId}_panel_${panelId}`
    },
    advanced: {
      connection_timeout_ms: 5000,
      stall_timeout_ms: 5000,
      retry_base_ms: 500,
      retry_max_ms: 10000,
      retry_jitter_ms: 250,
      max_failures: 30,
      preview_fps_override: null
    }
  },
  status: {
    state,
    message: state === 'playing' ? 'Playing' : 'Idle',
    code: null
  },
  secret_present: false,
  is_recording: false
})

export const makeState = (screenCount = 4): GetStateResponse => ({
  ipc_version: '1',
  schema_version: 2,
  active_screen: 0,
  active_panel_per_screen: Array.from({ length: screenCount }, () => 0),
  fullscreen: false,
  stream_defaults: {
    preview_fps: 12,
    auto_manage_preview_fps: false
  },
  auto_populate_tool: {
    base_url_template:
      'rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor?channel=$cameraNum&subtype=$subNum',
    username: '',
    password: '',
    ip: '',
    port: '554',
    camera_num_start: 1,
    camera_num_end: 16,
    sub_num_start: 0,
    sub_num_end: 1
  },
  screens: Array.from({ length: screenCount }, (_, screenId) => ({
    id: screenId,
    panels: [
      makePanel(screenId, 0),
      makePanel(screenId, 1),
      makePanel(screenId, 2),
      makePanel(screenId, 3)
    ]
  }))
})
