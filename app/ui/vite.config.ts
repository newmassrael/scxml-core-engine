// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { defineConfig } from "vitest/config";

// The browser shell (`sce-web-shell`) listens here by default; the dev server
// forwards `/api` to it so the screen on :5173 and the commands on :5174 are one
// origin to the page.
const WEB_SHELL = process.env["SCE_WEB_SHELL"] ?? "http://127.0.0.1:5174";

export default defineConfig({
  // The Tauri window loads the build from disk, so assets must be relative.
  base: "./",
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    proxy: { "/api": WEB_SHELL },
  },
  build: {
    target: "es2022",
    sourcemap: true,
  },
  test: {
    environment: "node",
    include: ["test/**/*.test.ts"],
  },
});
