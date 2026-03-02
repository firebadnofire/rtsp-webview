import type { GetStateResponse } from '../types'

export const renderToolbar = (state: GetStateResponse, autoPopulateToolOpen: boolean): string => {
  const fullscreenLabel = state.fullscreen ? 'Exit Fullscreen' : 'Fullscreen Active Panel'
  const toolLabel = autoPopulateToolOpen ? 'Close Auto-population Tool' : 'Auto-population Tool'
  return `<header class="toolbar">
    <div class="toolbar-primary">
      <button data-action="start-screen">Start Screen</button>
      <button data-action="stop-screen">Stop Screen</button>
      <button data-action="start-all">Start All Cameras</button>
      <button data-action="stop-all">Stop All Cameras</button>
    </div>
    <div class="toolbar-secondary">
      <button data-action="toggle-auto-populate-tool">${toolLabel}</button>
      <button data-action="save-config">Save Config</button>
      <button data-action="load-config">Load Config</button>
      <button data-action="toggle-fullscreen">${fullscreenLabel}</button>
    </div>
  </header>`
}
