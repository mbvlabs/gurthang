import { createInterface } from 'node:readline'
import { createInertiaApp } from '@inertiajs/react'
import type { Page } from '@inertiajs/core'
import type { ComponentType } from 'react'
import ReactDOMServer from 'react-dom/server'

const protocolPrefix = 'gurthang:ssr:'
const pages = import.meta.glob<{ default: ComponentType }>('./Pages/**/*.tsx', {
  eager: true,
})

async function render(page: Page) {
  return createInertiaApp({
    page,
    render: ReactDOMServer.renderToString,
    resolve: (name) => {
      const path = `./Pages/${name}.tsx`
      const component = pages[path]
      if (!component) {
        throw new Error(`No Inertia page is registered for ${name} (${path})`)
      }
      return component.default
    },
    setup: ({ App, props }) => <App {...props} />,
  })
}

function send(message: unknown) {
  process.stdout.write(`${protocolPrefix}${JSON.stringify(message)}\n`)
}

const input = createInterface({ input: process.stdin, terminal: false })
let queue = Promise.resolve()

input.on('line', (line) => {
  queue = queue.then(async () => {
    try {
      const page = JSON.parse(line) as Page
      send({ ok: true, result: await render(page) })
    } catch (error) {
      send({
        ok: false,
        error: error instanceof Error ? error.stack ?? error.message : String(error),
      })
    }
  })
})

send({ ready: true })
