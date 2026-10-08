// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Replay every scenario of the Rust arm in a wasm32-unknown-unknown module, under
// Node.
//
//   node run.mjs <wasm-bindgen --target nodejs output dir> [pattern]
//
// A panic on that target stops the module and cannot be caught inside it, so each
// scenario gets an instance of its own: one failing scenario cannot take the
// others down, and the trap carries the panic message, which says why. The
// scenarios are the committed files (`sce-build/tests/fixtures/static_datamodel/
// scenarios/*.json`) that every backend replays; the module holds their text, and
// the directory is read here only to check that the module holds all of them.
//
// The same module runs in Chrome through `run_chrome.mjs`, which is the host the
// web target actually has.

import { createRequire } from "node:module";
import path from "node:path";
import { heldMismatch, report } from "./scenarios.mjs";

const outDir = path.resolve(process.argv[2] ?? "");
const only = process.argv[3] ? new RegExp(process.argv[3]) : null;
const glue = path.join(outDir, "sce_rust_scenarios.js");
const require = createRequire(import.meta.url);

const names = require(glue).scenario_names().split(",").filter(Boolean);

const mismatch = heldMismatch(names);
if (mismatch) {
  console.log(mismatch);
  process.exit(2);
}

let pass = 0;
const failures = [];
const started = Date.now();
const selected = names.filter((name) => !only || only.test(name));
for (const name of selected) {
  delete require.cache[require.resolve(glue)];
  const instance = require(glue);
  const original = console.error;
  const panics = [];
  console.error = (...args) => panics.push(args.join(" "));
  try {
    instance.run_scenario(name);
    pass += 1;
  } catch (trap) {
    failures.push({
      name,
      trap: String(trap?.message ?? trap).split("\n")[0],
      panic: panics.join(" | ").replace(/\s+/g, " "),
    });
  } finally {
    console.error = original;
  }
}

process.exit(
  report({
    host: "node",
    selected: selected.length,
    pass,
    failures,
    seconds: (Date.now() - started) / 1000,
  }),
);
