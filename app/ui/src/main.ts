// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import "./style.css";

import { App } from "./app";
import { httpTransport, insideTauri, type Transport } from "./ipc";
import { credentials, takeToken, type Credentials } from "./token";

interface Connection {
  readonly transport: Transport;
  /** Present when the screen talks to a server that wants a token. */
  readonly credentials?: Credentials;
}

async function connect(): Promise<Connection> {
  if (insideTauri(window)) {
    return { transport: (await import("./tauri_transport")).tauriTransport };
  }
  let storage: Storage | null = null;
  try {
    storage = window.sessionStorage;
  } catch {
    storage = null;
  }
  const { token, cleanHash } = takeToken(window.location.hash, storage);
  if (cleanHash !== window.location.hash) {
    history.replaceState(null, "", `${window.location.pathname}${window.location.search}${cleanHash}`);
  }
  const held = credentials(token, storage);
  return {
    transport: httpTransport({ fetch: (...args) => fetch(...args), token: () => held.token() }),
    credentials: held,
  };
}

const root = document.getElementById("app");
if (root === null) throw new Error("index.html has no #app element");

let storage: Storage | null = null;
try {
  storage = window.localStorage;
} catch {
  storage = null;
}

const connection = await connect();
const app = new App(root, {
  ...connection,
  storage,
  browserLanguage: navigator.language,
});
void app.start();

// A tab closed or reloaded while the editor holds text the core has not been
// given loses it, so the browser is asked to confirm first. A desktop window has
// no such event for its close button; that case is not covered yet.
if (!insideTauri(window)) {
  window.addEventListener("beforeunload", (event) => {
    if (app.hasUnsavedChanges()) event.preventDefault();
  });
}
