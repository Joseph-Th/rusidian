import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../crates/pkm-app/dist',
    emptyOutDir: true,
  },
})
