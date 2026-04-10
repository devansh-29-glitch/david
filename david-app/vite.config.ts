import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  build: {
    rollupOptions: {
      input: {
        orb: resolve(__dirname, 'orb.html'),
        auth: resolve(__dirname, 'auth.html'),
      },
    },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
})
