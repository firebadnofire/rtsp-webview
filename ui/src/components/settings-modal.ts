import type { PanelStateView, SettingsModalState } from '../types'
import { escapeHtml } from './common'

interface SettingsModalInput {
  modal: SettingsModalState
  panel: PanelStateView
  previewUrl: string
  revealPassword: boolean
  subtypeOptions: number[]
}

const valueOrEmpty = (value: string): string => escapeHtml(value)

const boolAttr = (value: boolean): string => (value ? 'checked' : '')

const renderOptions = (values: number[], selected: string): string =>
  values
    .map((value) => `<option value="${value}" ${String(value) === selected ? 'selected' : ''}>${value}</option>`)
    .join('')

export const renderSettingsModal = ({
  modal,
  panel,
  previewUrl,
  revealPassword,
  subtypeOptions
}: SettingsModalInput): string => {
  const secretStatus = panel.secret_present ? 'Credentials stored' : 'No credentials stored'
  return `<div class="modal-backdrop" data-action="close-modal">
    <div class="modal" data-modal="true" data-persist-scroll="settings-modal">
      <header class="modal-header">
        <h3>Panel Settings</h3>
      </header>
      <form class="settings-form" data-action="submit-settings">
        <label>Title<input data-field="title" value="${valueOrEmpty(modal.form.title)}" /></label>
        <label>Camera Number<input data-field="cameraNum" type="number" min="0" value="${valueOrEmpty(modal.form.cameraNum)}" /></label>
        <label>Subtype Number
          <select data-field="subNum">${renderOptions(subtypeOptions, modal.form.subNum)}</select>
        </label>
        <label>Host/IP<input data-field="host" value="${valueOrEmpty(modal.form.host)}" /></label>
        <label>Port<input data-field="port" type="number" min="1" max="65535" value="${valueOrEmpty(modal.form.port)}" /></label>
        <label>Path<input data-field="path" value="${valueOrEmpty(modal.form.path)}" /></label>
        <label>Channel<input data-field="channel" value="${valueOrEmpty(modal.form.channel)}" /></label>
        <label>Subtype Label<input data-field="subtype" value="${valueOrEmpty(modal.form.subtype)}" /></label>
        <label>Transport
          <select data-field="transport">
            <option value="tcp" ${modal.form.transport === 'tcp' ? 'selected' : ''}>TCP</option>
            <option value="udp" ${modal.form.transport === 'udp' ? 'selected' : ''}>UDP</option>
          </select>
        </label>
        <label>Latency (ms)<input data-field="latencyMs" type="number" min="0" max="5000" value="${valueOrEmpty(modal.form.latencyMs)}" /></label>
        <div class="secret-status">${escapeHtml(secretStatus)}</div>
        <label>Username<input data-field="username" value="${valueOrEmpty(modal.form.username)}" autocomplete="off" /></label>
        <label>Password<input data-field="password" type="password" value="${valueOrEmpty(modal.form.password)}" autocomplete="off" /></label>
        <label class="checkbox-row"><input data-field="clearSecret" type="checkbox" ${boolAttr(modal.form.clearSecret)} /> Clear stored credentials</label>
        <label class="checkbox-row"><input data-action="toggle-reveal" type="checkbox" ${boolAttr(revealPassword)} /> Reveal password in preview</label>
        <div class="preview-row"><strong>RTSP Preview:</strong><code>${escapeHtml(previewUrl)}</code></div>
        <details class="advanced-settings" data-persist-details="advanced-settings">
          <summary>Advanced Stream Settings</summary>
          <label>Connection timeout (ms)<input data-field="connectionTimeoutMs" type="number" min="100" value="${valueOrEmpty(modal.form.connectionTimeoutMs)}" /></label>
          <label>Stall timeout (ms)<input data-field="stallTimeoutMs" type="number" min="100" value="${valueOrEmpty(modal.form.stallTimeoutMs)}" /></label>
          <label>Retry base (ms)<input data-field="retryBaseMs" type="number" min="100" value="${valueOrEmpty(modal.form.retryBaseMs)}" /></label>
          <label>Retry max (ms)<input data-field="retryMaxMs" type="number" min="100" value="${valueOrEmpty(modal.form.retryMaxMs)}" /></label>
          <label>Retry jitter (ms)<input data-field="retryJitterMs" type="number" min="0" value="${valueOrEmpty(modal.form.retryJitterMs)}" /></label>
          <label>Max failures<input data-field="maxFailures" type="number" min="1" value="${valueOrEmpty(modal.form.maxFailures)}" /></label>
          <label class="checkbox-row"><input data-field="previewFpsOverrideEnabled" type="checkbox" ${boolAttr(modal.form.previewFpsOverrideEnabled)} /> Override inherited preview FPS</label>
          <label>Inherited preview FPS override
            <input
              data-field="previewFpsOverride"
              type="text"
              inputmode="numeric"
              pattern="[0-9]*"
              autocomplete="off"
              spellcheck="false"
              value="${valueOrEmpty(modal.form.previewFpsOverride)}"
              ${modal.form.previewFpsOverrideEnabled ? '' : 'disabled'}
            />
          </label>
        </details>
        <footer class="modal-actions">
          <button type="button" data-action="close-modal">Cancel</button>
          <button type="submit">Save</button>
        </footer>
      </form>
    </div>
  </div>`
}
