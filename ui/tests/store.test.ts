import { describe, expect, it } from 'vitest'
import {
  buildRtspPreview,
  panelKey,
  redactRtspPassword,
  resolveAutoPopulateRtspUrl,
  UiStore
} from '../src/store'
import { makeState } from './fixtures'

describe('store', () => {
  it('hides password in preview when reveal is false', () => {
    const state = makeState(1)
    const panel = state.screens[0].panels[0].config
    panel.host = '10.0.0.15'
    panel.path = 'live'
    const preview = buildRtspPreview(panel, 'admin', 'secret', false)
    expect(preview).toContain('admin:***@')
    expect(preview).not.toContain('secret')
  })

  it('keeps latest frame by seq', () => {
    const store = new UiStore()
    store.setData(makeState(1))
    store.applyPanelFrame({
      ipc_version: '1',
      screen_id: 0,
      panel_id: 0,
      mime: 'image/jpeg',
      data_base64: 'older',
      width: 1,
      height: 1,
      pts_ms: 10,
      seq: 2
    })
    store.applyPanelFrame({
      ipc_version: '1',
      screen_id: 0,
      panel_id: 0,
      mime: 'image/jpeg',
      data_base64: 'newer',
      width: 1,
      height: 1,
      pts_ms: 11,
      seq: 3
    })
    store.applyPanelFrame({
      ipc_version: '1',
      screen_id: 0,
      panel_id: 0,
      mime: 'image/jpeg',
      data_base64: 'stale',
      width: 1,
      height: 1,
      pts_ms: 12,
      seq: 1
    })

    const snapshot = store.snapshot()
    expect(snapshot.frames[panelKey(0, 0)].data_base64).toBe('newer')
  })

  it('updates status payload into nested state', () => {
    const store = new UiStore()
    store.setData(makeState(1))
    store.applyPanelStatus({
      ipc_version: '1',
      screen_id: 0,
      panel_id: 1,
      state: 'playing',
      message: 'Playing',
      code: null
    })

    const snapshot = store.snapshot()
    expect(snapshot.data?.screens[0].panels[1].status.state).toBe('playing')
  })

  it('resolves auto-populate template values and masks password', () => {
    const state = makeState(1)
    state.auto_populate_tool.base_url_template =
      'rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor?channel=$cameraNum&subtype=$subNum'
    state.auto_populate_tool.username = 'william'
    state.auto_populate_tool.password = 'Plumbing1@'
    state.auto_populate_tool.ip = '192.168.86.66'
    state.auto_populate_tool.port = '554'

    const resolved = resolveAutoPopulateRtspUrl(state.auto_populate_tool, 1, 0)
    expect(resolved).toContain('channel=1')
    expect(resolved).toContain('subtype=0')

    const redacted = redactRtspPassword(resolved, false)
    expect(redacted).toContain('***')
    expect(redacted).not.toContain('Plumbing1')
  })
})
