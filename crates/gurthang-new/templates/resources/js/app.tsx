import { StrictMode } from 'react'
import type { ComponentType } from 'react'
import { createRoot, hydrateRoot } from 'react-dom/client'
import { createInertiaApp } from '@inertiajs/react'
import '../../css/base.css'

const pages = import.meta.glob<{ default: ComponentType }>('./Pages/**/*.tsx')

void createInertiaApp({
  resolve: async (name) => {
    const path = `./Pages/${name}.tsx`
    const load = pages[path]
    if (!load) {
      throw new Error(`No Inertia page is registered for ${name} (${path})`)
    }
    return (await load()).default
  },
  setup({ el, App, props }) {
    const application = (
      <StrictMode>
        <App {...props} />
      </StrictMode>
    )
    if (el.hasChildNodes()) {
      hydrateRoot(el, application)
    } else {
      createRoot(el).render(application)
    }
  },
})
