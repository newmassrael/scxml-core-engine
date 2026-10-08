// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Replay every scenario of the Rust arm in a wasm32-unknown-unknown module, in
// Chrome.
//
//   node run_chrome.mjs <wasm-bindgen --target web output dir> [pattern]
//
// The web target is Chrome, so Chrome is the host that decides whether the module
// starts there; Node (`run.mjs`) is the faster host of the same module and not a
// stand-in for it. They differ where it matters here: Chrome compiles the module
// with its own WebAssembly engine, fetches it as a browser does, and gives the page
// no `process`, no `require` and no file system.
//
// No driver and no package: this starts a loopback server that serves the page
// (`page.html`) and the glue, launches a headless Chrome at it, and waits for the
// page to post its result back. Chrome is found on PATH (or in $CHROME_BIN); a
// machine without one exits 3, which the gate reports as "cannot run" and not as a
// fault in the module.
//
// Exit status: 0 every scenario passed, 1 a scenario failed, 2 the page or the
// module is not what it should be, 3 no Chrome.

import http from "node:http";
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { heldMismatch, report } from "./scenarios.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.resolve(process.argv[2] ?? "");
const only = process.argv[3] ?? "";
const WAIT_MS = 180_000;

function findChrome() {
  const named = process.env.CHROME_BIN;
  if (named) return existsSync(named) ? named : null;
  for (const dir of (process.env.PATH ?? "").split(path.delimiter)) {
    for (const name of ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]) {
      const candidate = path.join(dir, name);
      if (dir && existsSync(candidate)) return candidate;
    }
  }
  return null;
}

const chrome = findChrome();
if (!chrome) {
  console.log("no Chrome on PATH and none named by CHROME_BIN");
  process.exit(3);
}

const TYPES = { ".js": "text/javascript", ".wasm": "application/wasm" };

let received;
const posted = new Promise((resolve) => {
  received = resolve;
});

const server = http.createServer((request, response) => {
  const { pathname } = new URL(request.url, "http://127.0.0.1");
  if (request.method === "POST" && pathname === "/result") {
    const body = [];
    request.on("data", (chunk) => body.push(chunk));
    request.on("end", () => {
      response.writeHead(204).end();
      received(Buffer.concat(body).toString("utf8"));
    });
    return;
  }
  if (request.method === "GET" && pathname === "/") {
    response.writeHead(200, { "content-type": "text/html" });
    response.end(readFileSync(path.join(here, "page.html")));
    return;
  }
  // Only the files of the glue directory, by name: a path with a separator in it
  // is not one of them.
  const served = pathname.startsWith("/pkg/") ? pathname.slice("/pkg/".length) : null;
  if (request.method === "GET" && served && served === path.basename(served)) {
    const file = path.join(outDir, served);
    if (existsSync(file)) {
      response.writeHead(200, {
        "content-type": TYPES[path.extname(file)] ?? "application/octet-stream",
      });
      response.end(readFileSync(file));
      return;
    }
  }
  response.writeHead(404).end();
});

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const { port } = server.address();

const profile = mkdtempSync(path.join(os.tmpdir(), "sce-chrome-"));
// --no-sandbox: the sandbox needs user namespaces, which a CI container or a
// locked-down desktop does not always grant. What runs is this repository's own
// module, served from loopback to a profile that is deleted afterwards.
const browser = spawn(
  chrome,
  [
    "--headless=new",
    "--no-sandbox",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-extensions",
    "--disable-background-networking",
    `--user-data-dir=${profile}`,
    `http://127.0.0.1:${port}/${only ? `?only=${encodeURIComponent(only)}` : ""}`,
  ],
  { stdio: "ignore", detached: true },
);
const exited = new Promise((resolve) => browser.on("exit", (code) => resolve(code)));

let timer;
const outcome = await Promise.race([
  posted.then((text) => ({ text })),
  exited.then((code) => ({ gone: code })),
  new Promise((resolve) => {
    timer = setTimeout(() => resolve({ late: true }), WAIT_MS);
  }),
]);
clearTimeout(timer);

try {
  process.kill(-browser.pid, "SIGKILL");
} catch {
  // Already gone.
}
server.close();
rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });

if (outcome.late || outcome.gone !== undefined) {
  console.log(
    outcome.late
      ? `Chrome posted no result within ${WAIT_MS / 1000}s`
      : `Chrome exited (${outcome.gone}) before the page posted a result`,
  );
  process.exit(2);
}

const result = JSON.parse(outcome.text);
if (result.error) {
  console.log(`the page failed before it could report: ${result.error}`);
  process.exit(2);
}
const mismatch = heldMismatch(result.names);
if (mismatch) {
  console.log(mismatch);
  process.exit(2);
}
process.exit(
  report({
    host: "chrome",
    selected: result.selected,
    pass: result.pass,
    failures: result.failures,
    seconds: result.seconds,
  }),
);
