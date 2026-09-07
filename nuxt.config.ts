import { readFileSync } from 'node:fs';

const serialportNativeModules = ['serialport', '@serialport/bindings-cpp', '@serialport/bindings-interface', '@serialport/parser-byte-length'];
const { version: snifferVersion } = JSON.parse(readFileSync('package.json', 'utf8')) as { version: string };

export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  ssr: false,
  pages: false,
  components: false,
  devtools: { enabled: false },
  telemetry: false,
  runtimeConfig: {
    public: {
      snifferVersion,
    },
  },

  devServer: {
    host: '0.0.0.0',
    port: 3010,
  },

  nitro: {
    preset: 'node-server',
    routeRules: {
      '/**': { cors: true },
    },
    externals: {
      external: serialportNativeModules,
    },
  },
});
