import type { GetStateResponse } from '../types'

export const renderTabs = (state: GetStateResponse): string => {
  const tabs = state.screens
    .map((screen, index) => {
      const active = screen.id === state.active_screen ? 'tab active' : 'tab'
      return `<div class="${active}">
        <button data-action="switch-screen" data-screen-id="${screen.id}">Screen ${index + 1}</button>
        <button class="tab-delete" data-action="delete-screen" data-screen-id="${screen.id}">×</button>
      </div>`
    })
    .join('')

  return `<div class="tab-bar">
    <div class="tab-list">${tabs}</div>
    <button class="add-screen" data-action="create-screen">+ Add Screen</button>
  </div>`
}
