import { describe, expect, it, vi } from 'vitest'
import { createRtspViewerApp } from '../src/app'
import type { EventClient } from '../src/events'
import type { IpcClient } from '../src/ipc'
import type {
  ConfigLoadedEvent,
  PanelFrameEvent,
  PanelStatusEvent,
  SecurityNoticeEvent,
  SnapshotFailedEvent,
  SnapshotSavedEvent
} from '../src/types'
import { makeState } from './fixtures'

const flush = async (): Promise<void> => {
  await new Promise((resolve) => setTimeout(resolve, 0))
}

class MockEventClient implements EventClient {
  panelStatus: ((payload: PanelStatusEvent) => void) | null = null
  panelFrame: ((payload: PanelFrameEvent) => void) | null = null
  configLoaded: ((payload: ConfigLoadedEvent) => void) | null = null
  snapshotSaved: ((payload: SnapshotSavedEvent) => void) | null = null
  snapshotFailed: ((payload: SnapshotFailedEvent) => void) | null = null
  securityNotice: ((payload: SecurityNoticeEvent) => void) | null = null

  onPanelStatus(cb: (payload: PanelStatusEvent) => void) {
    this.panelStatus = cb
    return Promise.resolve(() => {})
  }

  onPanelFrame(cb: (payload: PanelFrameEvent) => void) {
    this.panelFrame = cb
    return Promise.resolve(() => {})
  }

  onConfigLoaded(cb: (payload: ConfigLoadedEvent) => void) {
    this.configLoaded = cb
    return Promise.resolve(() => {})
  }

  onSnapshotSaved(cb: (payload: SnapshotSavedEvent) => void) {
    this.snapshotSaved = cb
    return Promise.resolve(() => {})
  }

  onSnapshotFailed(cb: (payload: SnapshotFailedEvent) => void) {
    this.snapshotFailed = cb
    return Promise.resolve(() => {})
  }

  onSecurityNotice(cb: (payload: SecurityNoticeEvent) => void) {
    this.securityNotice = cb
    return Promise.resolve(() => {})
  }
}

const createMockIpc = (): IpcClient => ({
  setActiveScreen: vi.fn().mockResolvedValue(undefined),
  setActivePanel: vi.fn().mockResolvedValue(undefined),
  getState: vi.fn().mockResolvedValue(makeState()),
  updatePanelConfig: vi.fn().mockResolvedValue(undefined),
  setPanelSecret: vi.fn().mockResolvedValue(undefined),
  autoPopulateCameras: vi.fn().mockResolvedValue(undefined),
  startStream: vi.fn().mockResolvedValue(undefined),
  stopStream: vi.fn().mockResolvedValue(undefined),
  startScreen: vi.fn().mockResolvedValue(undefined),
  stopScreen: vi.fn().mockResolvedValue(undefined),
  startAllGlobal: vi.fn().mockResolvedValue(undefined),
  stopAllGlobal: vi.fn().mockResolvedValue(undefined),
  saveConfig: vi.fn().mockResolvedValue('/tmp/config.json'),
  loadConfig: vi.fn().mockResolvedValue('/tmp/config.json'),
  snapshot: vi.fn().mockResolvedValue('/tmp/snapshot.jpg'),
  toggleRecording: vi.fn().mockResolvedValue(null),
  toggleFullscreen: vi.fn().mockResolvedValue(undefined),
  createScreen: vi.fn().mockResolvedValue(4),
  deleteScreen: vi.fn().mockResolvedValue(undefined)
})

describe('app integration', () => {
  it('dispatches toolbar and panel commands', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const startAll = root.querySelector('[data-action="start-all"]') as HTMLButtonElement
    startAll.click()
    await flush()
    expect(ipc.startAllGlobal).toHaveBeenCalledTimes(1)

    const startStream = root.querySelector('[data-action="start-stream"]') as HTMLButtonElement
    startStream.click()
    await flush()
    expect(ipc.startStream).toHaveBeenCalledWith(0, 0)

    const stopStream = root.querySelector('[data-action="stop-stream"]') as HTMLButtonElement
    stopStream.click()
    await flush()
    expect(ipc.stopStream).toHaveBeenCalledWith(0, 0)

    events.panelStatus?.({
      ipc_version: '1',
      screen_id: 0,
      panel_id: 0,
      state: 'playing',
      message: 'Playing',
      code: null
    })
    await flush()

    const record = root.querySelector('[data-action="toggle-recording"]') as HTMLButtonElement
    record.click()
    await flush()
    expect(ipc.toggleRecording).toHaveBeenCalledWith(0, 0, null)

    await app.destroy()
  })

  it('enables snapshot only when panel is playing', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const snapshotBefore = root.querySelector('[data-action="snapshot"]') as HTMLButtonElement
    expect(snapshotBefore.disabled).toBe(true)

    events.panelStatus?.({
      ipc_version: '1',
      screen_id: 0,
      panel_id: 0,
      state: 'playing',
      message: 'Playing',
      code: null
    })
    await flush()

    const snapshotAfter = root.querySelector('[data-action="snapshot"]') as HTMLButtonElement
    expect(snapshotAfter.disabled).toBe(false)

    await app.destroy()
  })

  it('updates visible screens on config_loaded event', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const twoScreenState = makeState(2)
    events.configLoaded?.({
      ipc_version: '1',
      state: twoScreenState
    })
    await flush()

    const tabs = Array.from(root.querySelectorAll('[data-action="switch-screen"]'))
    expect(tabs.length).toBe(2)

    await app.destroy()
  })
})
