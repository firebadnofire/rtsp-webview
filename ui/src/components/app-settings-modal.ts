import type { AppSettingsModalState } from '../types'
import { escapeHtml } from './common'

interface AppSettingsModalInput {
  modal: AppSettingsModalState
}

export const renderAppSettingsModal = ({ modal }: AppSettingsModalInput): string => {
  const fpsLabel = modal.form.autoManagePreviewFps ? 'Inherited preview FPS cap' : 'Default preview FPS'
  return `<div class="modal-backdrop" data-action="close-app-settings">
    <div class="modal modal-compact" data-modal="true" data-persist-scroll="app-settings-modal">
      <header class="modal-header">
        <h3>App Settings</h3>
      </header>
      <form class="settings-form settings-form-compact" data-action="submit-app-settings">
        <label class="checkbox-row">
          <input
            data-app-field="autoManagePreviewFps"
            type="checkbox"
            ${modal.form.autoManagePreviewFps ? 'checked' : ''}
          />
          Automatically manage inherited preview FPS from active cameras
        </label>
        <label>${fpsLabel}
          <input
            data-app-field="previewFps"
            type="number"
            min="1"
            max="30"
            value="${escapeHtml(modal.form.previewFps)}"
          />
        </label>
        <footer class="modal-actions">
          <button type="button" data-action="close-app-settings">Cancel</button>
          <button type="submit">Save</button>
        </footer>
      </form>
    </div>
  </div>`
}
