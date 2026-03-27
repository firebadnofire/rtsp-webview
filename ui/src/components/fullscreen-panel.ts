import type { PanelStateView } from '../types'
import { escapeHtml, statusClass, statusText } from './common'

interface FullscreenPanelInput {
  screenId: number
  panelId: number
  panel: PanelStateView
  frameUrl: string | null
  hasFrame: boolean
  fpsLabel: string
}

export const renderFullscreenPanel = ({
  screenId,
  panelId,
  panel,
  frameUrl,
  hasFrame,
  fpsLabel
}: FullscreenPanelInput): string => {
  return `<section class="fullscreen-panel" data-action="exit-fullscreen">
    <div class="fullscreen-overlay">
      <div class="fullscreen-title-row">
        <h2>${escapeHtml(panel.config.title)}</h2>
        <span class="panel-fps panel-fps-fullscreen" data-frame-fps="true" data-screen-id="${screenId}" data-panel-id="${panelId}">${escapeHtml(
          fpsLabel
        )}</span>
      </div>
      <span class="${statusClass(panel.status)}">${escapeHtml(statusText(panel.status))}</span>
      <div class="fullscreen-exit-hint">Click anywhere or press Esc to exit fullscreen.</div>
    </div>
    <div class="fullscreen-viewport">
      <img
        class="panel-frame ${hasFrame ? '' : 'hidden'}"
        data-frame-image="true"
        data-screen-id="${screenId}"
        data-panel-id="${panelId}"
        src="${frameUrl ?? ''}"
        alt="${escapeHtml(panel.config.title)} stream"
      />
      <div
        class="panel-placeholder ${hasFrame ? 'hidden' : ''}"
        data-frame-placeholder="true"
        data-screen-id="${screenId}"
        data-panel-id="${panelId}"
      >No Frame</div>
    </div>
  </section>`
}
