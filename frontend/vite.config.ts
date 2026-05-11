import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    lib: {
      entry: "src/index.tsx",
      formats: ["iife"],
      name: "HfPluginSwitchboard",
      fileName: () => "index.js",
      cssFileName: "styles",
    },
    outDir: "dist",
    rollupOptions: {
      // React is provided by the host app as window globals.
      // @tauri-apps/api and @haloforge/plugin-sdk are bundled inline.
      external: ["react", "react-dom", "react/jsx-runtime"],
      output: {
        globals: {
          react: "React",
          "react-dom": "ReactDOM",
          "react/jsx-runtime": "jsxRuntime",
        },
      },
    },
  },
});
