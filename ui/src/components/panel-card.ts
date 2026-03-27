import type { PanelStateView } from '../types'
import { escapeHtml, statusClass, statusText } from './common'

interface PanelCardInput {
  screenId: number
  panelId: number
  panel: PanelStateView
  active: boolean
  frameUrl: string | null
  hasFrame: boolean
  fpsLabel: string
  subtypeOptions: number[]
}

const renderSubtypeOptions = (options: number[], selected: number | null): string =>
  options
    .map((option) => `<option value="${option}" ${selected === option ? 'selected' : ''}>${option}</option>`)
    .join('')

export const renderPanelCard = (input: PanelCardInput): string => {
  const { screenId, panelId, panel, active, frameUrl, hasFrame, fpsLabel, subtypeOptions } = input
  const playing = panel.status.state === 'playing'
  const recording = panel.is_recording
  const recordDisabled = !playing && !recording
  const recordLabel = recording ? 'Stop Recording' : 'Record'
  const cardClass = active ? 'panel-card active' : 'panel-card'
  const selectedSubtype = panel.config.sub_num
  const image = `<img
      class="panel-frame ${hasFrame ? '' : 'hidden'}"
      data-frame-image="true"
      data-screen-id="${screenId}"
      data-panel-id="${panelId}"
      src="${frameUrl ?? ''}"
      alt="Panel ${panelId + 1} stream"
    />
    <div
      class="panel-placeholder ${hasFrame ? 'hidden' : ''}"
      data-frame-placeholder="true"
      data-screen-id="${screenId}"
      data-panel-id="${panelId}"
    >No Frame</div>`
  const subtypePicker =
    panel.config.camera_num !== null && subtypeOptions.length > 0
      ? `<label class="subtype-picker">
          subtype
          <select data-subtype-picker="true" data-screen-id="${screenId}" data-panel-id="${panelId}">
            ${renderSubtypeOptions(subtypeOptions, selectedSubtype)}
          </select>
        </label>`
      : ''

  return `<article class="${cardClass}" data-action="select-panel" data-screen-id="${screenId}" data-panel-id="${panelId}">
    <div class="panel-header">
      <div class="panel-title-row">
        <h3>${escapeHtml(panel.config.title)}</h3>
        <span class="panel-fps" data-frame-fps="true" data-screen-id="${screenId}" data-panel-id="${panelId}">${escapeHtml(fpsLabel)}</span>
      </div>
      <div class="panel-header-actions">
        ${subtypePicker}
        <span class="${statusClass(panel.status)}">${escapeHtml(statusText(panel.status))}</span>
      </div>
    </div>
    <div class="panel-viewport">${image}</div>
    <div class="panel-controls">
      <button data-action="start-stream" data-screen-id="${screenId}" data-panel-id="${panelId}">Start</button>
      <button data-action="stop-stream" data-screen-id="${screenId}" data-panel-id="${panelId}">Stop</button>
      <button data-action="snapshot" data-screen-id="${screenId}" data-panel-id="${panelId}" ${playing ? '' : 'disabled'}>Snapshot</button>
      <button data-action="toggle-recording" data-screen-id="${screenId}" data-panel-id="${panelId}" ${recordDisabled ? 'disabled' : ''}>${recordLabel}</button>
      <button data-action="enter-fullscreen" data-screen-id="${screenId}" data-panel-id="${panelId}">Fullscreen</button>
      <button data-action="open-settings" data-screen-id="${screenId}" data-panel-id="${panelId}">Settings</button>
    </div>
  </article>`
}
