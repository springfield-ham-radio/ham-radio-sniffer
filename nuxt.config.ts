const serialportNativeModules = ['serialport', '@serialport/bindings-cpp', '@serialport/bindings-interface', '@serialport/parser-byte-length'];

export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  ssr: false,
  pages: false,
  components: false,
  devtools: { enabled: false },
  telemetry: false,

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
