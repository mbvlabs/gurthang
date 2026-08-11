import { Form, Head, Link } from '@inertiajs/react'
import type { LoginProps } from '../../generated/LoginProps'
import type { SharedProps } from '../../generated/SharedProps'

type Props = LoginProps & SharedProps

export default function Login({ email, errors, flash }: Props) {
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-20 text-slate-100">
      <Head title="Sign in" />
      <section className="mx-auto max-w-md rounded-3xl bg-slate-900 p-10">
        <h1 className="text-4xl font-bold">Sign in</h1>
        {flash.success && <p className="mt-4 text-emerald-400">{flash.success}</p>}
        {errors.credentials && <p className="mt-4 text-red-400">{errors.credentials}</p>}
        <Form action="/login" method="post" className="mt-8 space-y-5">
          <label className="block">
            <span>Email</span>
            <input name="email" type="email" defaultValue={email ?? ''} required autoComplete="email" className="mt-2 w-full rounded-lg bg-slate-800 p-3" />
          </label>
          <label className="block">
            <span>Password</span>
            <input name="password" type="password" required autoComplete="current-password" className="mt-2 w-full rounded-lg bg-slate-800 p-3" />
          </label>
          <button className="w-full rounded-lg bg-amber-400 p-3 font-semibold text-slate-950">Sign in</button>
        </Form>
        <p className="mt-6 text-slate-400">New here? <Link href="/register" className="text-amber-400">Create an account</Link>.</p>
      </section>
    </main>
  )
}
