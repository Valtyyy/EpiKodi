import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The remote is served by the axum control API in production; this dev server
// uses 5174 so it never collides with the TV dev server (1420, HMR 1421).
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5174,
    strictPort: true,
  },
});
