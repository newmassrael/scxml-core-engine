// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Run the browser node's kit in a headless browser of one engine.
//
//   node run_browser.mjs chrome|firefox <wasm-bindgen --target web output dir> <https_echo binary>
//
// The engines are Chromium's and Firefox's (docs/adr/0117, open question 3). No driver and no
// package: this starts the origin (`serve.mjs`), launches the browser at its page with a profile
// of its own, and waits for the page to post its report. The browser is the one CHROME_BIN or
// FIREFOX_BIN names, else the first on PATH; a machine without one exits 3, which the gate
// reports as "cannot run" and not as a fault of the node.
//
// The page is of http://127.0.0.1, a secure context (a loopback origin is potentially
// trustworthy), so WebCrypto, IndexedDB and Web Locks are the ones a page served over HTTPS has.
// Firefox asks the person before it makes a bucket persistent; its profile here answers yes, as
// the person would, so the kit reads what the node publishes once it is asked.
//
// Exit status: 0 every case passed, 1 a case failed, 2 the page or the module is not what it
// should be, 3 no such browser.

import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { printReport } from "./kit.mjs";
import { serveKit } from "./serve.mjs";

const engine = process.argv[2] ?? "";
const glue = path.resolve(process.argv[3] ?? "");
const echo = path.resolve(process.argv[4] ?? "");
const WAIT_MS = 300_000;

const ENGINES = {
  chrome: {
    variable: "CHROME_BIN",
    names: ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"],
    // --no-sandbox: the sandbox needs user namespaces, which a CI container or a locked-down
    // desktop does not always grant. What runs is this repository's own module, served from
    // loopback to a profile that is deleted afterwards.
    args: (profile, url) => [
      "--headless=new",
      "--no-sandbox",
      "--disable-gpu",
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-extensions",
      "--disable-background-networking",
      `--user-data-dir=${profile}`,
      url,
    ],
  },
  firefox: {
    variable: "FIREFOX_BIN",
    names: ["firefox", "firefox-esr"],
    args: (profile, url) => ["--headless", "--no-remote", "--profile", profile, url],
    prepare: (profile) =>
      writeFileSync(
        path.join(profile, "user.js"),
        [
          // The answer the person gives to the question of a persistent bucket.
          'user_pref("dom.storageManager.prompt.testing", true);',
          'user_pref("dom.storageManager.prompt.testing.allow", true);',
          // Nothing of a first run, and nothing fetched from anywhere but the page's origin.
          'user_pref("browser.shell.checkDefaultBrowser", false);',
          'user_pref("browser.startup.homepage_override.mstone", "ignore");',
          'user_pref("browser.aboutwelcome.enabled", false);',
          'user_pref("datareporting.policy.dataSubmissionEnabled", false);',
          'user_pref("datareporting.healthreport.uploadEnabled", false);',
          'user_pref("toolkit.telemetry.reportingpolicy.firstRun", false);',
          'user_pref("app.update.disabledForTesting", true);',
          'user_pref("network.proxy.type", 0);',
          "",
        ].join("\n"),
      ),
  },
};

function findBrowser({ variable, names }) {
  const named = process.env[variable];
  if (named) return existsSync(named) ? named : null;
  for (const dir of (process.env.PATH ?? "").split(path.delimiter)) {
    for (const name of names) {
      const candidate = path.join(dir, name);
      if (dir && existsSync(candidate)) return candidate;
    }
  }
  return null;
}

const chosen = ENGINES[engine];
if (!chosen) {
  console.log(`no engine '${engine}': chrome or firefox`);
  process.exit(2);
}
const binary = findBrowser(chosen);
if (!binary) {
  console.log(`no ${engine} on PATH and none named by ${chosen.variable}`);
  process.exit(3);
}

let served;
try {
  served = await serveKit({ echo, glue });
} catch (error) {
  console.log(`the kit's origin could not start: ${error?.stack ?? error}`);
  process.exit(2);
}
const profile = mkdtempSync(path.join(os.tmpdir(), `sce-kit-${engine}-`));
chosen.prepare?.(profile);
const browser = spawn(binary, chosen.args(profile, `${served.origin}/`), {
  stdio: "ignore",
  detached: true,
});
const exited = new Promise((resolve) => browser.on("exit", (code) => resolve(code)));

let timer;
const outcome = await Promise.race([
  served.result.then((text) => ({ text })),
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
await served.close();
rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });

if (outcome.late || outcome.gone !== undefined) {
  console.log(
    outcome.late
      ? `${engine} posted no report within ${WAIT_MS / 1000}s`
      : `${engine} exited (${outcome.gone}) before the page posted a report`,
  );
  process.exit(2);
}
const report = JSON.parse(outcome.text);
if (report.error) {
  console.log(`the page failed before it could run the kit: ${report.error}`);
  process.exit(2);
}
process.exit(printReport(report));
