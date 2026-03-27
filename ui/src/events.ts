import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  ConfigLoadedEvent,
  PanelFrameEvent,
  PanelStatusEvent,
  SecurityNoticeEvent,
  SnapshotFailedEvent,
  SnapshotSavedEvent
} from './types'

export interface EventClient {
  onPanelStatus(cb: (payload: PanelStatusEvent) => void): Promise<UnlistenFn>
  onPanelFrame(cb: (payload: PanelFrameEvent) => void): Promise<UnlistenFn>
  onConfigLoaded(cb: (payload: ConfigLoadedEvent) => void): Promise<UnlistenFn>
  onSnapshotSaved(cb: (payload: SnapshotSavedEvent) => void): Promise<UnlistenFn>
  onSnapshotFailed(cb: (payload: SnapshotFailedEvent) => void): Promise<UnlistenFn>
  onSecurityNotice(cb: (payload: SecurityNoticeEvent) => void): Promise<UnlistenFn>
}

export const tauriEventClient: EventClient = {
  onPanelStatus(cb) {
    return listen<PanelStatusEvent>('panel_status', (event) => cb(event.payload))
  },
  onPanelFrame(cb) {
    return listen<PanelFrameEvent>('panel_frame', (event) => cb(event.payload))
  },
  onConfigLoaded(cb) {
    return listen<ConfigLoadedEvent>('config_loaded', (event) => cb(event.payload))
  },
  onSnapshotSaved(cb) {
    return listen<SnapshotSavedEvent>('snapshot_saved', (event) => cb(event.payload))
  },
  onSnapshotFailed(cb) {
    return listen<SnapshotFailedEvent>('snapshot_failed', (event) => cb(event.payload))
  },
  onSecurityNotice(cb) {
    return listen<SecurityNoticeEvent>('security_notice', (event) => cb(event.payload))
  }
}
