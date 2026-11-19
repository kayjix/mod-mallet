// @ts-expect-error type error without @types/node package
import process from "node:process";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react()],

  // The app's version and which commit it is built from, shown in its
  // settings (ADR 0057). npm passes the package's version; `cargo xtask
  // app` passes the commit.
  define: {
    __MALLET_VERSION__: JSON.stringify(process.env.npm_package_version ?? "0.0.0"),
    __MALLET_COMMIT__: JSON.stringify(process.env.MALLET_COMMIT ?? null),
    __MALLET_COMMIT_DATE__: JSON.stringify(process.env.MALLET_COMMIT_DATE ?? null),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
