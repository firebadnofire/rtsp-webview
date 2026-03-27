export const renderFullscreenHint = (enabled: boolean): string => {
  if (!enabled) {
    return ''
  }
  return '<div class="fullscreen-hint">Fullscreen active: click anywhere or press Esc to exit.</div>'
}
