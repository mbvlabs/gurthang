import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: '/assets/dist/',
  build: {
    manifest: 'manifest.json',
    assetsDir: '',
    outDir: 'assets/dist',
    emptyOutDir: true,
    rollupOptions: {
      input: 'resources/js/app.tsx',
    },
  },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    cors: {
      origin: [
        'http://127.0.0.1:3000',
        'http://localhost:3000',
        'http://[::1]:3000',
      ],
    },
  },
})
