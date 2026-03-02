export const renderFullscreenHint = (enabled: boolean): string => {
  if (!enabled) {
    return ''
  }
  return '<div class="fullscreen-hint">Fullscreen active: press Esc, F11, or Q to exit.</div>'
}
