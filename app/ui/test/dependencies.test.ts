// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What of other people's code reaches a user through the screen.
//
// The screen is public and ships inside the application, so the closure of its
// runtime npm dependencies, with each licence, is written down here. A new
// dependency (or a transitive one a bump brings in) fails this test until it is
// added on purpose, with its licence read, instead of arriving unnoticed in a
// bundle. Build tools (vite, typescript, vitest) are not shipped and are not in
// the closure.

import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

/** Every runtime npm package the screen may ship, and the licence it is under. */
const ALLOWED: Record<string, string> = {
  "@tauri-apps/api": "Apache-2.0 OR MIT",
};

interface PackageJson {
  name: string;
  license?: string;
  dependencies?: Record<string, string>;
}

function read(path: URL): PackageJson {
  return JSON.parse(readFileSync(path, "utf8")) as PackageJson;
}

/** The runtime closure, found the way node resolves it: `node_modules/<name>/package.json`. */
function closure(): Map<string, string> {
  const found = new Map<string, string>();
  const queue = Object.keys(read(new URL("../package.json", import.meta.url)).dependencies ?? {});
  while (queue.length > 0) {
    const name = queue.pop();
    if (name === undefined || found.has(name)) continue;
    const manifest = read(new URL(`../node_modules/${name}/package.json`, import.meta.url));
    found.set(name, manifest.license ?? "(none declared)");
    queue.push(...Object.keys(manifest.dependencies ?? {}));
  }
  return found;
}

describe("the screen's runtime dependencies", () => {
  it("are exactly the ones written down, under the licences written down", () => {
    expect(Object.fromEntries(closure())).toEqual(ALLOWED);
  });
});
