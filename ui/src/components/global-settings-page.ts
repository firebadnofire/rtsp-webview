import type { AutoPopulateToolFormState } from '../types'
import { escapeHtml } from './common'

interface AutoPopulateToolInput {
  form: AutoPopulateToolFormState
  cameraCount: number
  subtypeCount: number
}

export const renderGlobalSettingsPage = ({
  form,
  cameraCount,
  subtypeCount
}: AutoPopulateToolInput): string => {
  const panelCount = cameraCount
  const screenCount = panelCount === 0 ? 0 : Math.ceil(panelCount / 4)
  const summary = `${cameraCount} cameras with ${subtypeCount} subtype options = ${panelCount} panels (${screenCount} screens)`

  return `<section class="global-settings-page">
    <header class="global-settings-header">
      <h2>Auto-population Tool</h2>
      <button type="button" data-action="close-auto-populate-tool">Back to Grid</button>
    </header>
    <form class="global-settings-form" data-action="run-auto-populate-tool">
      <label>Base RTSP URL Template
        <input data-tool-field="baseUrlTemplate" value="${escapeHtml(form.baseUrlTemplate)}" />
      </label>
      <div class="global-grid-2">
        <label>Username Token Value
          <input data-tool-field="username" value="${escapeHtml(form.username)}" />
        </label>
        <label>Password Token Value
          <input data-tool-field="password" type="password" value="${escapeHtml(form.password)}" />
        </label>
        <label>IP Token Value
          <input data-tool-field="ip" value="${escapeHtml(form.ip)}" />
        </label>
        <label>Port Token Value
          <input data-tool-field="port" value="${escapeHtml(form.port)}" />
        </label>
        <label>Camera Channel Range Start
          <input data-tool-field="cameraNumStart" type="number" min="0" value="${escapeHtml(form.cameraNumStart)}" />
        </label>
        <label>Camera Channel Range End
          <input data-tool-field="cameraNumEnd" type="number" min="0" value="${escapeHtml(form.cameraNumEnd)}" />
        </label>
        <label>Subtype Range Start
          <input data-tool-field="subNumStart" type="number" min="0" value="${escapeHtml(form.subNumStart)}" />
        </label>
        <label>Subtype Range End
          <input data-tool-field="subNumEnd" type="number" min="0" value="${escapeHtml(form.subNumEnd)}" />
        </label>
      </div>
      <div class="global-settings-summary">${escapeHtml(summary)}</div>
      <footer class="global-settings-actions">
        <button type="submit">Run Auto-population</button>
      </footer>
    </form>
  </section>`
}
