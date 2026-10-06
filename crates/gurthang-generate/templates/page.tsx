import { Head } from '@inertiajs/react'

export default function {{ page }}() {
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-20 text-slate-100">
      <Head title="{{ title }}" />
      <section className="mx-auto max-w-3xl rounded-3xl border border-slate-800 bg-slate-900 p-10">
        <h1 className="text-4xl font-bold">{{ title }}</h1>
      </section>
    </main>
  )
}
