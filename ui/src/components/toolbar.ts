import type { GetStateResponse } from '../types'

export const renderToolbar = (state: GetStateResponse, autoPopulateToolOpen: boolean): string => {
  const hasScreens = state.screens.length > 0
  const screenActionDisabled = hasScreens ? '' : 'disabled'
  const toolLabel = autoPopulateToolOpen ? 'Close Auto-population Tool' : 'Auto-population Tool'
  return `<header class="toolbar">
    <div class="toolbar-primary">
      <button data-action="start-screen" ${screenActionDisabled}>Start Screen</button>
      <button data-action="stop-screen" ${screenActionDisabled}>Stop Screen</button>
      <button data-action="start-all" ${screenActionDisabled}>Start All Cameras</button>
      <button data-action="stop-all" ${screenActionDisabled}>Stop All Cameras</button>
    </div>
    <div class="toolbar-secondary">
      <button data-action="open-app-settings">Settings</button>
      <button data-action="toggle-auto-populate-tool">${toolLabel}</button>
      <button data-action="save-config">Save Config</button>
      <button data-action="load-config">Load Config</button>
    </div>
  </header>`
}
