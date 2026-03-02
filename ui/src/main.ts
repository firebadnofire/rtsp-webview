import '@fontsource/space-grotesk/400.css'
import '@fontsource/space-grotesk/600.css'
import '@fontsource/ibm-plex-mono/400.css'
import './styles.css'
import { createRtspViewerApp } from './app'

const root = document.getElementById('app')
if (!root) {
  throw new Error('Missing #app root element')
}

const app = createRtspViewerApp(root)
void app.start().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error)
  root.innerHTML = `<main class="shell"><div class="loading">Startup error: ${message}</div></main>`
})
