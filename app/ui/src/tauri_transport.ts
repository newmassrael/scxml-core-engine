// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The window's transport. Kept apart from `ipc.ts` and loaded only inside a Tauri
// window, so a browser never evaluates the Tauri API.

import { invoke } from "@tauri-apps/api/core";

import type { Desktop } from "./desktop";
import { failureFromInvoke, type Transport } from "./ipc";

/**
 * The window's own commands: whether the screen holds unsaved changes, and that the
 * person decided it may close. A report that cannot be delivered is dropped: the
 * shell then believes the last one it heard, and a window it cannot hear from is
 * closed after a wait rather than held for ever (`close_gate.rs`).
 */
export const tauriDesktop: Desktop = {
  unsaved(unsaved) {
    invoke("sce_unsaved", { unsaved }).catch(() => undefined);
  },
  async close() {
    await invoke("sce_close");
  },
};

export const tauriTransport: Transport = {
  async call(name, args = {}) {
    try {
      return await invoke("sce_call", { name, args });
    } catch (rejection) {
      throw failureFromInvoke(rejection);
    }
  },
};
