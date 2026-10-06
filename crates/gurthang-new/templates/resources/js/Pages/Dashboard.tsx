import { Head } from '@inertiajs/react'
import { router } from '@inertiajs/react'
import type { DashboardProps } from '../generated/DashboardProps'

export default function Dashboard({ title, status, user }: DashboardProps) {
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-20 text-slate-100">
      <Head title={title} />
      <section className="mx-auto max-w-3xl rounded-3xl border border-slate-800 bg-slate-900 p-10 shadow-2xl">
        <p className="text-sm font-semibold uppercase tracking-[0.2em] text-amber-400">Gurthang</p>
        <h1 className="mt-3 text-5xl font-bold tracking-tight">{title}</h1>
        <p className="mt-5 text-lg text-slate-300">{status}</p>
        <p className="mt-3 text-slate-400">Signed in as {user.email}</p>
        <button
          type="button"
          className="mt-8 rounded-lg bg-amber-400 px-4 py-2 font-semibold text-slate-950"
          onClick={() => router.delete('/logout')}
        >
          Sign out
        </button>
      </section>
    </main>
  )
}
