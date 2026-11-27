// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Run the browser node's kit under Node.
//
//   node run_node.mjs <wasm-bindgen --target nodejs output dir> <https_echo binary>
//
// Node is a host of the same module with its own fetch, WebCrypto, timers and BroadcastChannel,
// and no IndexedDB or Web Locks: the store and the lock are the kit's in memory (`memory.mjs`),
// held to the contracts the shell reads of them. The browsers run the kit on their own
// (`run_browser.mjs`).
//
// Exit status: 0 every case passed, 1 a case failed, 2 the kit could not run.

import { createRequire } from "node:module";
import path from "node:path";
import { memoryLocks, memoryStores } from "./memory.mjs";
import { printReport, runKit } from "./kit.mjs";
import { serveKit } from "./serve.mjs";

const glue = path.resolve(process.argv[2] ?? "");
const echo = path.resolve(process.argv[3] ?? "");
const require = createRequire(import.meta.url);

let served;
let status = 2;
try {
  const { Client } = require(path.join(glue, "sce_rust_browser_node.js"));
  served = await serveKit({ echo });
  const openStore = memoryStores();
  const locks = memoryLocks();
  const platform = () => ({
    origin: served.origin,
    fetch: globalThis.fetch,
    crypto: globalThis.crypto,
    locks,
    storage: undefined,
    openStore,
    BroadcastChannel: globalThis.BroadcastChannel,
    setTimeout: globalThis.setTimeout,
    clearTimeout: globalThis.clearTimeout,
    now: () => Date.now(),
  });
  const report = await runKit({
    host: `node ${process.version}`,
    Client,
    platform,
    origin: served.origin,
    server: served.server,
    control: {
      proxy: async (open) => served.setOpen(open),
    },
  });
  status = printReport(report);
} catch (error) {
  console.log(`the kit could not run under Node: ${error?.stack ?? error}`);
} finally {
  await served?.close();
}
process.exit(status);
