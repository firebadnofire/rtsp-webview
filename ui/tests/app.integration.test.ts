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

const flushFrames = async (): Promise<void> => {
  await flush()
  await new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => resolve())
  })
  await flush()
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
  updateStreamDefaults: vi.fn().mockResolvedValue(undefined),
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
  loadStartupConfig: vi.fn().mockResolvedValue(null),
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

  it('attempts to load the startup config on app start', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    expect(ipc.loadStartupConfig).toHaveBeenCalledTimes(1)

    await app.destroy()
  })

  it('enters in-window fullscreen for a panel and exits by click or Escape', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const state = makeState(1)
    state.screens[0].panels[0].status.state = 'playing'
    state.screens[0].panels[0].status.message = 'Playing'

    const ipc = createMockIpc()
    ;(ipc.getState as ReturnType<typeof vi.fn>).mockImplementation(async () => state)
    ;(ipc.toggleFullscreen as ReturnType<typeof vi.fn>).mockImplementation(async (enabled: boolean) => {
      state.fullscreen = enabled
    })
    ;(ipc.setActivePanel as ReturnType<typeof vi.fn>).mockImplementation(async (screenId: number, panelId: number) => {
      state.active_panel_per_screen[screenId] = panelId
    })

    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const enter = root.querySelector('[data-action="enter-fullscreen"]') as HTMLButtonElement
    enter.click()
    await flush()

    expect(ipc.toggleFullscreen).toHaveBeenCalledWith(true)
    expect(root.querySelector('.toolbar')).toBeNull()
    expect(root.querySelector('.fullscreen-panel')).not.toBeNull()

    const fullscreen = root.querySelector('.fullscreen-panel') as HTMLElement
    fullscreen.click()
    await flush()

    expect(ipc.toggleFullscreen).toHaveBeenLastCalledWith(false)
    expect(root.querySelector('.toolbar')).not.toBeNull()

    const reenter = root.querySelector('[data-action="enter-fullscreen"]') as HTMLButtonElement
    reenter.click()
    await flush()
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await flush()

    expect(ipc.toggleFullscreen).toHaveBeenLastCalledWith(false)
    expect(root.querySelector('.fullscreen-panel')).toBeNull()

    await app.destroy()
  })

  it('updates visible panel frames without replacing unrelated DOM nodes', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flushFrames()

    const startAll = root.querySelector('[data-action="start-all"]') as HTMLButtonElement
    const frameImage = root.querySelector(
      '[data-frame-image="true"][data-screen-id="0"][data-panel-id="0"]'
    ) as HTMLImageElement
    const placeholder = root.querySelector(
      '[data-frame-placeholder="true"][data-screen-id="0"][data-panel-id="0"]'
    ) as HTMLElement

    events.panelFrame?.({
      ipc_version: '1',
      screen_id: 0,
      panel_id: 0,
      mime: 'image/jpeg',
      data_base64: 'ZmFrZQ==',
      width: 1,
      height: 1,
      pts_ms: 10,
      seq: 1
    })
    await flushFrames()

    expect(root.querySelector('[data-action="start-all"]')).toBe(startAll)
    expect(root.querySelector('[data-frame-image="true"][data-screen-id="0"][data-panel-id="0"]')).toBe(frameImage)
    expect(frameImage.getAttribute('src')).toBe('data:image/jpeg;base64,ZmFrZQ==')
    expect(placeholder.classList.contains('hidden')).toBe(true)

    await app.destroy()
  })

  it('shows cached frame immediately when the active screen changes', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flushFrames()

    events.panelFrame?.({
      ipc_version: '1',
      screen_id: 1,
      panel_id: 0,
      mime: 'image/jpeg',
      data_base64: 'c2NyZWVuMg==',
      width: 1,
      height: 1,
      pts_ms: 11,
      seq: 2
    })

    const screenTwoState = makeState(2)
    screenTwoState.active_screen = 1
    events.configLoaded?.({
      ipc_version: '1',
      state: screenTwoState
    })
    await flushFrames()

    const frameImage = root.querySelector(
      '[data-frame-image="true"][data-screen-id="1"][data-panel-id="0"]'
    ) as HTMLImageElement
    expect(frameImage.getAttribute('src')).toBe('data:image/jpeg;base64,c2NyZWVuMg==')

    await app.destroy()
  })

  it('updates the visible fps label from incoming frames and refreshes it on resize', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flushFrames()

    const fps = root.querySelector('[data-frame-fps="true"][data-screen-id="0"][data-panel-id="0"]') as HTMLElement
    expect(fps.textContent).toBe('-- FPS')

    events.panelFrame?.({
      ipc_version: '1',
      screen_id: 0,
      panel_id: 0,
      mime: 'image/jpeg',
      data_base64: 'ZmFrZQ==',
      width: 1,
      height: 1,
      pts_ms: 0,
      seq: 1
    })
    events.panelFrame?.({
      ipc_version: '1',
      screen_id: 0,
      panel_id: 0,
      mime: 'image/jpeg',
      data_base64: 'YmFy',
      width: 1,
      height: 1,
      pts_ms: 100,
      seq: 2
    })
    await flushFrames()

    expect(fps.textContent).toBe('10 FPS')

    window.dispatchEvent(new Event('resize'))
    await flushFrames()

    expect(fps.textContent).toBe('10 FPS')

    await app.destroy()
  })

  it('shows empty-workspace actions and disables screen-only controls when no screens exist', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    ;(ipc.getState as ReturnType<typeof vi.fn>).mockResolvedValue(makeState(0))

    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const emptyManual = root.querySelector('[data-action="empty-manual-setup"]')
    const emptyBulk = root.querySelector('[data-action="empty-open-auto-populate"]')
    const startScreen = root.querySelector('[data-action="start-screen"]') as HTMLButtonElement
    const startAll = root.querySelector('[data-action="start-all"]') as HTMLButtonElement

    expect(emptyManual).not.toBeNull()
    expect(emptyBulk).not.toBeNull()
    expect(startScreen.disabled).toBe(true)
    expect(startAll.disabled).toBe(true)

    await app.destroy()
  })

  it('creates first screen and opens settings from empty manual setup action', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    let state = makeState(0)
    ;(ipc.getState as ReturnType<typeof vi.fn>).mockImplementation(async () => state)
    ;(ipc.createScreen as ReturnType<typeof vi.fn>).mockImplementation(async () => {
      state = makeState(1)
      return 0
    })

    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const createFirst = root.querySelector('[data-action="empty-manual-setup"]') as HTMLButtonElement
    createFirst.click()
    await flush()
    await flush()

    expect(ipc.createScreen).toHaveBeenCalledTimes(1)
    expect(root.querySelector('[data-action="submit-settings"]')).not.toBeNull()

    await app.destroy()
  })

  it('updates panel config and credentials when subtype is changed from panel dropdown', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const state = makeState(1)
    state.auto_populate_tool.ip = '127.0.0.1'
    state.auto_populate_tool.username = 'admin'
    state.auto_populate_tool.password = 'pw'
    state.screens[0].panels[0].config.camera_num = 1
    state.screens[0].panels[0].config.sub_num = 0
    state.screens[0].panels[0].config.title = 'Camera 1'

    const ipc = createMockIpc()
    ;(ipc.getState as ReturnType<typeof vi.fn>).mockResolvedValue(state)

    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const picker = root.querySelector('[data-subtype-picker="true"][data-panel-id="0"]') as HTMLSelectElement
    picker.value = '1'
    picker.dispatchEvent(new Event('change', { bubbles: true }))
    await flush()
    await flush()

    expect(ipc.updatePanelConfig).toHaveBeenCalledWith(
      0,
      0,
      expect.objectContaining({
        camera_num: 1,
        sub_num: 1,
        host: '127.0.0.1',
        port: 554
      })
    )
    expect(ipc.setPanelSecret).toHaveBeenCalledWith(0, 0, 'admin', 'pw')

    await app.destroy()
  })

  it('preserves settings modal focus, scroll, and advanced section while typing', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const state = makeState(1)
    state.screens[0].panels[0].config.advanced.preview_fps_override = 12

    const ipc = createMockIpc()
    ;(ipc.getState as ReturnType<typeof vi.fn>).mockResolvedValue(state)

    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const openSettings = root.querySelector('[data-action="open-settings"]') as HTMLButtonElement
    openSettings.click()
    await flush()

    const modal = root.querySelector('[data-persist-scroll="settings-modal"]') as HTMLElement
    const details = root.querySelector(
      'details[data-persist-details="advanced-settings"]'
    ) as HTMLDetailsElement
    details.open = true
    modal.scrollTop = 220

    const input = root.querySelector('[data-field="previewFpsOverride"]') as HTMLInputElement
    expect(input.getAttribute('type')).toBe('text')
    input.focus()
    input.value = '9'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    await flush()

    const nextModal = root.querySelector('[data-persist-scroll="settings-modal"]') as HTMLElement
    const nextDetails = root.querySelector(
      'details[data-persist-details="advanced-settings"]'
    ) as HTMLDetailsElement
    const nextInput = root.querySelector('[data-field="previewFpsOverride"]') as HTMLInputElement

    expect(nextDetails.open).toBe(true)
    expect(nextModal.scrollTop).toBe(220)
    expect(document.activeElement).toBe(nextInput)
    expect(nextInput.value).toBe('9')

    await app.destroy()
  })

  it('updates automatic preview fps settings from app settings', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const ipc = createMockIpc()
    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const open = root.querySelector('[data-action="open-app-settings"]') as HTMLButtonElement
    open.click()
    await flush()

    const autoManage = root.querySelector('[data-app-field="autoManagePreviewFps"]') as HTMLInputElement
    autoManage.click()
    await flush()

    const input = root.querySelector('[data-app-field="previewFps"]') as HTMLInputElement
    input.value = '15'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    await flush()

    const form = root.querySelector('[data-action="submit-app-settings"]') as HTMLFormElement
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flush()

    expect(ipc.updateStreamDefaults).toHaveBeenCalledWith({
      preview_fps: 15,
      auto_manage_preview_fps: true
    })

    await app.destroy()
  })

  it('shows the auto-managed inherited preview fps in panel settings when override is off', async () => {
    const root = document.createElement('div')
    document.body.appendChild(root)

    const state = makeState(2)
    state.stream_defaults.auto_manage_preview_fps = true
    state.screens[0].panels[0].status.state = 'playing'
    state.screens[0].panels[1].status.state = 'playing'
    state.screens[0].panels[2].status.state = 'playing'
    state.screens[0].panels[3].status.state = 'playing'
    state.screens[1].panels[0].status.state = 'playing'

    const ipc = createMockIpc()
    ;(ipc.getState as ReturnType<typeof vi.fn>).mockResolvedValue(state)

    const events = new MockEventClient()
    const app = createRtspViewerApp(root, { ipc, events })
    await app.start()
    await flush()

    const openSettings = root.querySelector('[data-action="open-settings"]') as HTMLButtonElement
    openSettings.click()
    await flush()

    const input = root.querySelector('[data-field="previewFpsOverride"]') as HTMLInputElement
    expect(input.value).toBe('12')

    const close = root.querySelector('[data-action="close-modal"]') as HTMLButtonElement
    close.click()
    await flush()

    const secondOpenSettings = root.querySelector(
      '[data-action="open-settings"][data-screen-id="0"][data-panel-id="1"]'
    ) as HTMLButtonElement
    secondOpenSettings.click()
    await flush()

    const secondInput = root.querySelector('[data-field="previewFpsOverride"]') as HTMLInputElement
    expect(secondInput.value).toBe('8')

    await app.destroy()
  })
})
