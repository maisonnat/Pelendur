import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@tauri-apps/api/tauri': path.resolve(__dirname, 'src/lib/tauri-api-stub.ts'),
    },
  },
  optimizeDeps: {
    exclude: ['@tauri-apps/api', '@tauri-apps/api/tauri'],
  },
  build: {
    outDir: 'dist',
    rollupOptions: {
      external: ['@tauri-apps/api/tauri', '@tauri-apps/api'],
      output: {
        entryFileNames: 'assets/[name].[hash].js',
        chunkFileNames: 'assets/[name].[hash].js',
      },
    },
  },
})
