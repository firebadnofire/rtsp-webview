import type { PanelRuntimeStatus } from '../types'

export const escapeHtml = (value: string): string =>
  value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')

export const statusClass = (status: PanelRuntimeStatus): string => `status-chip status-${status.state}`

export const statusText = (status: PanelRuntimeStatus): string => {
  const code = status.code ? ` (${status.code})` : ''
  return `${status.state.toUpperCase()}: ${status.message}${code}`
}
