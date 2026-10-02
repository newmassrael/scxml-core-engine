// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The window's transport. Kept apart from `ipc.ts` and loaded only inside a Tauri
// window, so a browser never evaluates the Tauri API.

import { invoke } from "@tauri-apps/api/core";

import { failureFromInvoke, type Transport } from "./ipc";

export const tauriTransport: Transport = {
  async call(name, args = {}) {
    try {
      return await invoke("sce_call", { name, args });
    } catch (rejection) {
      throw failureFromInvoke(rejection);
    }
  },
};
