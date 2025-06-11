import { defineConfig } from "vite";

export default defineConfig({
  // Prevent vite from obscuring rust errors
  clearScreen: false,
  // Tauri expects a fixed port, fail if that's not available
  server: {
    port: 1420,
    strictPort: true,
  },
  // To access the Tauri environment variables set by the CLI during development
  envPrefix: ["VITE_", "TAURI_PLATFORM", "TAURI_ARCH", "TAURI_FAMILY", "TAURI_PLATFORM_VERSION", "TAURI_PLATFORM_TYPE", "TAURI_DEBUG"],
  build: {
    // Tauri supports es2021
    target: process.env.TAURI_PLATFORM == "windows" ? "chrome105" : "safari13",
    // don't minify for debug builds
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    // produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
