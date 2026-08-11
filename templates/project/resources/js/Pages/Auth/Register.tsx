import { Form, Head, Link } from '@inertiajs/react'
import type { RegisterProps } from '../../generated/RegisterProps'
import type { SharedProps } from '../../generated/SharedProps'

type Props = RegisterProps & SharedProps

export default function Register({ email, errors }: Props) {
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-20 text-slate-100">
      <Head title="Create account" />
      <section className="mx-auto max-w-md rounded-3xl bg-slate-900 p-10">
        <h1 className="text-4xl font-bold">Create account</h1>
        <Form action="/register" method="post" className="mt-8 space-y-5">
          <label className="block">
            <span>Email</span>
            <input name="email" type="email" defaultValue={email ?? ''} required autoComplete="email" className="mt-2 w-full rounded-lg bg-slate-800 p-3" />
            {errors.email && <span className="mt-2 block text-red-400">{errors.email}</span>}
          </label>
          <label className="block">
            <span>Password</span>
            <input name="password" type="password" minLength={12} required autoComplete="new-password" className="mt-2 w-full rounded-lg bg-slate-800 p-3" />
            {errors.password && <span className="mt-2 block text-red-400">{errors.password}</span>}
          </label>
          <button className="w-full rounded-lg bg-amber-400 p-3 font-semibold text-slate-950">Register</button>
        </Form>
        <p className="mt-6 text-slate-400">Already registered? <Link href="/login" className="text-amber-400">Sign in</Link>.</p>
      </section>
    </main>
  )
}
