import type { PanelStateView } from '../types'
import { escapeHtml, statusClass, statusText } from './common'

interface PanelCardInput {
  screenId: number
  panelId: number
  panel: PanelStateView
  active: boolean
  frameUrl: string | null
}

export const renderPanelCard = (input: PanelCardInput): string => {
  const { screenId, panelId, panel, active, frameUrl } = input
  const playing = panel.status.state === 'playing'
  const recording = panel.is_recording
  const recordDisabled = !playing && !recording
  const recordLabel = recording ? 'Stop Recording' : 'Record'
  const cardClass = active ? 'panel-card active' : 'panel-card'
  const image = frameUrl
    ? `<img src="${frameUrl}" alt="Panel ${panelId + 1} stream" />`
    : '<div class="panel-placeholder">No Frame</div>'

  return `<article class="${cardClass}" data-action="select-panel" data-screen-id="${screenId}" data-panel-id="${panelId}">
    <div class="panel-header">
      <h3>${escapeHtml(panel.config.title)}</h3>
      <span class="${statusClass(panel.status)}">${escapeHtml(statusText(panel.status))}</span>
    </div>
    <div class="panel-viewport">${image}</div>
    <div class="panel-controls">
      <button data-action="start-stream" data-screen-id="${screenId}" data-panel-id="${panelId}">Start</button>
      <button data-action="stop-stream" data-screen-id="${screenId}" data-panel-id="${panelId}">Stop</button>
      <button data-action="snapshot" data-screen-id="${screenId}" data-panel-id="${panelId}" ${playing ? '' : 'disabled'}>Snapshot</button>
      <button data-action="toggle-recording" data-screen-id="${screenId}" data-panel-id="${panelId}" ${recordDisabled ? 'disabled' : ''}>${recordLabel}</button>
      <button data-action="open-settings" data-screen-id="${screenId}" data-panel-id="${panelId}">Settings</button>
    </div>
  </article>`
}
