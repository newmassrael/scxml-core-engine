// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The page surface (docs/adr/0117, D7) held to what it promises, on the surface
// `sce-codegen surface` wrote for `static_optional`
// (sce-build/tests/fixtures/static_datamodel/static_optional.scxml):
//
//   - TypeScript compiles a page that uses it right (`pages/right.mts`) and refuses
//     one whose every wrong use must not compile (`pages/wrong.mts`): each line
//     marked `// wrong:` is an error, and nothing else is;
//   - its module, run under Node, hands the port exactly the JSON text of the
//     payload of each event it admits, its keys in the order the schema declares
//     them, and refuses every event a page in JavaScript can hand it wrong, naming
//     where, before anything reaches the port;
//   - a snapshot the port gives reaches the page parsed, and read-only.
//
//   node check.mjs <the directory sce-codegen surface wrote> <the TypeScript package>
//
// The TypeScript package is the directory of its package.json, the one
// scripts/install_typescript.sh unpacks; the compiler is driven through its own
// API, so each error is read with its file and its line. Exit 0 when every check
// holds, 1 otherwise, each failure printed.

import { copyFileSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const MACHINE = "static_optional";
const PAGES = ["right.mts", "wrong.mts"];
const WRONG = "// wrong:";

const [surfaceDir, typescriptDir] = process.argv.slice(2);
if (!surfaceDir || !typescriptDir) {
  console.error("usage: node check.mjs <surface directory> <TypeScript package directory>");
  process.exit(2);
}
const here = dirname(fileURLToPath(import.meta.url));
const failures = [];

function check(holds, what) {
  if (!holds) failures.push(what);
}

// ── The declaration, by TypeScript ─────────────────────────────────────────────────────────────

/** Each page, compiled beside the declaration: the right one clean, the wrong one wrong only where it says. */
function typeCheck() {
  const ts = createRequire(import.meta.url)(join(typescriptDir, "lib", "typescript.js"));
  const pages = PAGES.map((page) => {
    const to = join(surfaceDir, page);
    copyFileSync(join(here, "pages", page), to);
    return to;
  });
  const program = ts.createProgram(pages, {
    strict: true,
    noEmit: true,
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.NodeNext,
    moduleResolution: ts.ModuleResolutionKind.NodeNext,
    types: [],
    skipLibCheck: false,
  });
  const marked = new Set();
  readFileSync(pages[1], "utf8")
    .split("\n")
    .forEach((line, at) => {
      if (line.includes(WRONG)) marked.add(at + 1);
    });
  const erred = new Set();
  for (const diagnostic of ts.getPreEmitDiagnostics(program)) {
    const words = ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n");
    if (diagnostic.file === undefined || diagnostic.start === undefined) {
      failures.push("TypeScript, of no file: " + words);
      continue;
    }
    const line = diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start).line + 1;
    const where = basename(diagnostic.file.fileName) + ":" + line;
    if (basename(diagnostic.file.fileName) === PAGES[1] && marked.has(line)) {
      erred.add(line);
      continue;
    }
    failures.push("an error where the page is right, " + where + ": TS" + diagnostic.code + " " + words);
  }
  for (const line of marked) {
    check(erred.has(line), "a wrong use compiled, " + PAGES[1] + ":" + line);
  }
  check(marked.size >= 10, "the wrong page marks " + marked.size + " wrong uses; it was read wrong");
}

// ── The module, under Node ─────────────────────────────────────────────────────────────────────

/** A port that records what it is handed, and the listeners it is given. */
function recordingPort() {
  const handed = [];
  const listeners = new Set();
  return {
    handed,
    listeners,
    raise(name, data) {
      handed.push([name, data]);
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

const ACUTE = "\u00e9";

const ADMITTED = [
  [
    "every field, given in another order than the schema declares",
    { name: "due.set", data: { status: "in_progress", title: "soon", due: "1700000000000", id: 7 } },
    ["due.set", '{"id":7,"due":"1700000000000","title":"soon","status":"in_progress"}'],
  ],
  [
    "every value that may be absent, absent",
    { name: "due.set", data: { id: 0, due: null, title: null, status: null } },
    ["due.set", '{"id":0,"due":null,"title":null,"status":null}'],
  ],
  [
    "each bound at its end: the largest uint32, the least int64, 16 bytes of text in 8 characters",
    { name: "due.set", data: { id: 4294967295, due: "-9223372036854775808", title: ACUTE.repeat(8), status: "done" } },
    ["due.set", '{"id":4294967295,"due":"-9223372036854775808","title":"' + ACUTE.repeat(8) + '","status":"done"}'],
  ],
  [
    "the event the machine sends itself, raised by the page",
    { name: "due.sent", data: { due: "1", title: null, status: "open" } },
    ["due.sent", '{"due":"1","title":null,"status":"open"}'],
  ],
  ["an event that carries no data", { name: "due.clear" }, ["due.clear", null]],
  ["another", { name: "finish" }, ["finish", null]],
];

const DUE = { id: 1, due: null, title: null, status: null };

const REFUSED = [
  ["no object", "due.clear", ""],
  ["a list", ["due.clear"], ""],
  ["an event the machine does not take", { name: "due.sett", data: DUE }, "/name"],
  ["an event the platform raises", { name: "error.execution" }, "/name"],
  ["a name that is no text", { name: 7 }, "/name"],
  ["no name", { data: DUE }, "/name"],
  ["an event that carries data, without it", { name: "due.set" }, "/data"],
  ["an event that carries none, with some", { name: "due.clear", data: {} }, "/data"],
  ["a key beside the name and the data", { name: "relay", at: 1 }, "/at"],
  ["data that is a list", { name: "due.set", data: [DUE] }, "/data"],
  ["data of a class", { name: "due.set", data: new Map([["id", 1]]) }, "/data"],
  ["a field left out", { name: "due.set", data: { id: 1, title: null, status: null } }, "/data/due"],
  ["a field the payload does not have", { name: "due.set", data: { ...DUE, extra: 1 } }, "/data/extra"],
  ["a field left undefined", { name: "due.set", data: { ...DUE, status: undefined } }, "/data/status"],
  ["a number written as text", { name: "due.set", data: { ...DUE, id: "1" } }, "/data/id"],
  ["a number with a fraction", { name: "due.set", data: { ...DUE, id: 1.5 } }, "/data/id"],
  ["a number that is none", { name: "due.set", data: { ...DUE, id: Number.NaN } }, "/data/id"],
  ["a uint32 past its end", { name: "due.set", data: { ...DUE, id: 4294967296 } }, "/data/id"],
  ["a uint32 below zero", { name: "due.set", data: { ...DUE, id: -1 } }, "/data/id"],
  ["a 64-bit integer as a number", { name: "due.set", data: { ...DUE, due: 5 } }, "/data/due"],
  ["a 64-bit integer past its end", { name: "due.set", data: { ...DUE, due: "9223372036854775808" } }, "/data/due"],
  ["a 64-bit integer that is no number", { name: "due.set", data: { ...DUE, due: "soon" } }, "/data/due"],
  ["a text of more characters than its bound", { name: "due.set", data: { ...DUE, title: "x".repeat(17) } }, "/data/title"],
  ["a text of more bytes than its bound", { name: "due.set", data: { ...DUE, title: ACUTE.repeat(9) } }, "/data/title"],
  ["a text with a lone surrogate", { name: "due.set", data: { ...DUE, title: "\ud800" } }, "/data/title"],
  ["a value the enum does not declare", { name: "due.set", data: { ...DUE, status: "closed" } }, "/data/status"],
  ["a value of the enum by its number", { name: "due.set", data: { ...DUE, status: 2 } }, "/data/status"],
];

async function runtime() {
  const surface = await import(pathToFileURL(join(surfaceDir, MACHINE + ".mjs")).href);
  check(surface.MACHINE === MACHINE, "the module names the machine " + surface.MACHINE);
  const events = ["due.clear", "due.fallback", "due.read", "due.sent", "due.set", "finish", "relay"];
  check(
    JSON.stringify(surface.EVENTS) === JSON.stringify(events),
    "the module takes " + JSON.stringify(surface.EVENTS) + ", not the events the machine takes",
  );
  check(Object.isFrozen(surface.EVENTS), "a page can change the events the module takes");

  let bound = false;
  try {
    surface.bind({ raise() {} });
    bound = true;
  } catch (error) {
    check(error instanceof TypeError, "a port that cannot subscribe is refused as " + error);
  }
  check(!bound, "the module was bound to a port that cannot subscribe");

  const port = recordingPort();
  const page = surface.bind(port);
  for (const [what, event, want] of ADMITTED) {
    const before = port.handed.length;
    let answered;
    try {
      answered = await page.raise(event);
    } catch (error) {
      failures.push("refused " + what + ": " + error);
      continue;
    }
    check(answered === undefined, what + ": the raise answers " + answered);
    const got = port.handed.slice(before);
    check(
      JSON.stringify(got) === JSON.stringify([want]),
      what + ": the port was handed " + JSON.stringify(got) + ", not " + JSON.stringify([want]),
    );
  }
  for (const [what, event, path] of REFUSED) {
    const before = port.handed.length;
    let raised = false;
    try {
      page.raise(event);
      raised = true;
    } catch (error) {
      check(
        error instanceof surface.SurfaceRefused,
        what + ": refused as " + error + ", not as a SurfaceRefused",
      );
      check(
        error.path === path,
        what + ": refused at " + JSON.stringify(error.path) + ", not at " + JSON.stringify(path),
      );
    }
    check(!raised, what + ": admitted");
    check(port.handed.length === before, what + ": the port was handed something");
  }

  const seen = [];
  const stop = page.subscribe((snapshot) => seen.push(snapshot));
  check(port.listeners.size === 1, "the subscription of the page is not the port's");
  const snapshot = {
    configuration: ["tracking"],
    variables: { due: null, title: "soon", item: { id: 7, due: "3", title: null, status: "open" }, items: [] },
    truncated: false,
  };
  for (const listener of port.listeners) listener(JSON.stringify(snapshot));
  check(seen.length === 1, "the page was told " + seen.length + " snapshots of one");
  check(
    seen.length === 1 && JSON.stringify(seen[0]) === JSON.stringify(snapshot),
    "the page was told " + JSON.stringify(seen[0]) + " of the snapshot the port gave",
  );
  check(
    seen.length === 1 &&
      Object.isFrozen(seen[0]) &&
      Object.isFrozen(seen[0].variables) &&
      Object.isFrozen(seen[0].variables.item) &&
      Object.isFrozen(seen[0].variables.items),
    "a page can change the snapshot it was told",
  );
  stop();
  check(port.listeners.size === 0, "stopping the subscription left the port listening");
}

typeCheck();
await runtime();
for (const failure of failures) console.log("FAIL " + failure);
console.log(
  "page surface of " + MACHINE + ": " + (failures.length === 0 ? "every check holds" : failures.length + " failed"),
);
process.exit(failures.length === 0 ? 0 : 1);
