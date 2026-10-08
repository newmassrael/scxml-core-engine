// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the two hosts of the scenario module (`run.mjs` under Node, `run_chrome.mjs`
// in Chrome) agree on: which scenarios the module has to hold, and how a result is
// said. Kept in one place so the two cannot come to mean different things by
// "the module holds every scenario" or by a pass.

import { readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));

// The committed scenario files every backend replays.
const scenarioDir = path.resolve(
  here,
  "../../../../sce-build/tests/fixtures/static_datamodel/scenarios",
);

/** The scenarios of the directory, by name, sorted. */
export function scenariosOnDisk() {
  return readdirSync(scenarioDir)
    .filter((file) => file.endsWith(".json"))
    .map((file) => file.slice(0, -".json".length))
    .sort();
}

/**
 * Why the module's scenarios are not the directory's, or null when they are.
 *
 * The module must hold every scenario of the directory, and only those: a scenario
 * the module does not hold is one this target never replays. The floor on the
 * directory is there because a directory that read as empty would be held by an
 * empty module.
 */
export function heldMismatch(names) {
  const onDisk = scenariosOnDisk();
  const held = [...names].sort();
  const missing = onDisk.filter((name) => !held.includes(name));
  const extra = held.filter((name) => !onDisk.includes(name));
  if (onDisk.length < 40 || missing.length || extra.length) {
    return (
      `the module's scenarios are not the directory's (directory ${onDisk.length}, module ${held.length}): ` +
      `not in the module: ${missing.join(",") || "-"}; not in the directory: ${extra.join(",") || "-"}`
    );
  }
  return null;
}

/**
 * Say the result and return the process's exit status: 0 when every selected
 * scenario passed, 1 otherwise. A run that selected nothing judged nothing, so it
 * is not a pass.
 */
export function report({ host, selected, pass, failures, seconds }) {
  console.log(
    `[${host}] scenarios: ${selected}  pass: ${pass}  fail: ${failures.length}  (${seconds.toFixed(1)}s)`,
  );
  for (const failure of failures) {
    console.log(`FAIL ${failure.name}: ${failure.panic || failure.trap}`);
  }
  if (selected === 0) {
    console.log(`[${host}] no scenario was selected, so nothing was judged`);
    return 1;
  }
  return failures.length ? 1 : 0;
}
