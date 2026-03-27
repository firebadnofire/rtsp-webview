import { invoke } from '@tauri-apps/api/tauri'
import type { AutoPopulateTool, GetStateResponse, PanelConfigPatch, StreamDefaultsPatch } from './types'

export interface IpcClient {
  setActiveScreen(screenId: number): Promise<void>
  setActivePanel(screenId: number, panelId: number): Promise<void>
  getState(): Promise<GetStateResponse>
  updatePanelConfig(screenId: number, panelId: number, patch: PanelConfigPatch): Promise<void>
  updateStreamDefaults(patch: StreamDefaultsPatch): Promise<void>
  setPanelSecret(screenId: number, panelId: number, username: string, password: string): Promise<void>
  autoPopulateCameras(tool: AutoPopulateTool): Promise<void>
  startStream(screenId: number, panelId: number): Promise<void>
  stopStream(screenId: number, panelId: number): Promise<void>
  startScreen(screenId: number): Promise<void>
  stopScreen(screenId: number): Promise<void>
  startAllGlobal(): Promise<void>
  stopAllGlobal(): Promise<void>
  saveConfig(path: string | null): Promise<string>
  loadConfig(path: string | null): Promise<string>
  loadStartupConfig(): Promise<string | null>
  snapshot(screenId: number, panelId: number, path: string | null): Promise<string>
  toggleRecording(screenId: number, panelId: number, path: string | null): Promise<string | null>
  toggleFullscreen(enabled: boolean): Promise<void>
  createScreen(): Promise<number>
  deleteScreen(screenId: number): Promise<void>
}

export const tauriIpcClient: IpcClient = {
  setActiveScreen(screenId) {
    return invoke('set_active_screen', { screenId })
  },
  setActivePanel(screenId, panelId) {
    return invoke('set_active_panel', { screenId, panelId })
  },
  getState() {
    return invoke<GetStateResponse>('get_state')
  },
  updatePanelConfig(screenId, panelId, patch) {
    return invoke('update_panel_config', { screenId, panelId, patch })
  },
  updateStreamDefaults(patch) {
    return invoke('update_stream_defaults', { patch })
  },
  setPanelSecret(screenId, panelId, username, password) {
    return invoke('set_panel_secret', { screenId, panelId, username, password })
  },
  autoPopulateCameras(tool) {
    return invoke('auto_populate_cameras', { tool })
  },
  startStream(screenId, panelId) {
    return invoke('start_stream', { screenId, panelId })
  },
  stopStream(screenId, panelId) {
    return invoke('stop_stream', { screenId, panelId })
  },
  startScreen(screenId) {
    return invoke('start_screen', { screenId })
  },
  stopScreen(screenId) {
    return invoke('stop_screen', { screenId })
  },
  startAllGlobal() {
    return invoke('start_all_global')
  },
  stopAllGlobal() {
    return invoke('stop_all_global')
  },
  saveConfig(path) {
    return invoke<string>('save_config', { path })
  },
  loadConfig(path) {
    return invoke<string>('load_config', { path })
  },
  loadStartupConfig() {
    return invoke<string | null>('load_startup_config')
  },
  snapshot(screenId, panelId, path) {
    return invoke<string>('snapshot', { screenId, panelId, path })
  },
  toggleRecording(screenId, panelId, path) {
    return invoke<string | null>('toggle_recording', { screenId, panelId, path })
  },
  toggleFullscreen(enabled) {
    return invoke('toggle_fullscreen', { enabled })
  },
  createScreen() {
    return invoke<number>('create_screen')
  },
  deleteScreen(screenId) {
    return invoke('delete_screen', { screenId })
  }
}
