import { defineConfig, loadEnv, mergeConfig } from 'vite';
import baseConfig from './vite.config.base';

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  const proxy = { '/api': { target: env.VITE_DEV_API_TARGET || 'http://127.0.0.1:8080', changeOrigin: true } };
  return mergeConfig(baseConfig, {
    server: { host: '127.0.0.1', port: 5173, strictPort: true, proxy },
    preview: { host: '127.0.0.1', port: 4173, strictPort: true, proxy },
  });
});
