import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Tauri yerel bir pencere açar; geliştirme sunucusu yalnız localhost'ta dinler.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    // Tasarım token'ları depo kökündeki design/ klasöründen (değiştirilmeden) okunur.
    fs: { allow: ["..", "../../design"] },
  },
  css: {
    preprocessorOptions: {
      scss: {
        // Bootstrap 5.3 SCSS'i Dart Sass'ın kaldırılacak @import/renk işlevlerini
        // kullanıyor; uyarılar Bootstrap kaynaklı ve derlemeyi etkilemiyor.
        quietDeps: true,
        silenceDeprecations: ["import", "global-builtin", "color-functions", "if-function"],
      },
    },
  },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: false,
  },
});
