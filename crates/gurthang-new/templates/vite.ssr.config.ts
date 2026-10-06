import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  build: {
    ssr: 'resources/js/ssr.tsx',
    outDir: 'dist-ssr',
    emptyOutDir: true,
    minify: true,
    rollupOptions: {
      output: {
        entryFileNames: 'ssr.mjs',
        inlineDynamicImports: true,
      },
    },
  },
  ssr: {
    noExternal: true,
  },
})
