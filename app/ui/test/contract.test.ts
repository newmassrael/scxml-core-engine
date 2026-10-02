// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The screen's guards against the replies the core actually gives.
//
// `app-core/contract/replies.json` is produced by running the core's commands
// (`app-core/tests/contract.rs` fails if it is stale). Here it is read back and
// held against the guards, so the screen cannot expect a shape the core does not
// produce, and the core cannot change a shape without this test seeing it.

import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import {
  asCommandError,
  conflictRevisions,
  ContractError,
  parseDescribed,
  parseHistory,
  parseListing,
  parseReadSource,
  parseSaved,
  parseWork,
  parseWorkAndHead,
  SUPPORTED_COMMAND_SET_VERSION,
} from "../src/contract";

interface Replies {
  command_set_version: number;
  answers: Record<string, unknown>;
  refusals: Record<string, unknown>;
}

const replies = JSON.parse(
  readFileSync(new URL("../../../app-core/contract/replies.json", import.meta.url), "utf8"),
) as Replies;

const parsers: Record<string, (value: unknown) => unknown> = {
  describe: parseDescribed,
  list_works: parseListing,
  create_work: (v) => parseWork(v),
  read_work: parseWorkAndHead,
  read_source: parseReadSource,
  save_source: parseSaved,
  history: parseHistory,
};

/** The command an answer's name belongs to: the longest command name it starts with. */
function commandOf(name: string): string {
  const matches = Object.keys(parsers).filter((c) => name === c || name.startsWith(`${c}_`));
  const longest = matches.sort((a, b) => b.length - a.length)[0];
  if (longest === undefined) throw new Error(`${name} belongs to no command the screen knows`);
  return longest;
}

describe("the replies the core gives", () => {
  it("are of the command set this screen was written for", () => {
    expect(replies.command_set_version).toBe(SUPPORTED_COMMAND_SET_VERSION);
  });

  it("are all accepted by the guard of their command", () => {
    for (const [name, reply] of Object.entries(replies.answers)) {
      const parse = parsers[commandOf(name)];
      expect(parse, name).toBeDefined();
      expect(() => parse?.(reply), name).not.toThrow();
    }
  });

  it("cover every command the core lists", () => {
    const described = parseDescribed(replies.answers["describe"]);
    for (const command of described.commands) {
      expect(parsers[command], `the screen has no guard for \`${command}\``).toBeDefined();
      const covered = Object.keys(replies.answers).some((name) => commandOf(name) === command);
      expect(covered, `no reply of \`${command}\` is written down`).toBe(true);
    }
  });

  it("carry the facts the screen branches on", () => {
    expect(parseReadSource(replies.answers["read_source_none"])).toBeNull();
    expect(parseReadSource(replies.answers["read_source"])?.text).toContain("Three misses");
    expect(parseWorkAndHead(replies.answers["read_work_no_text"]).head).toBeNull();
    expect(parseSaved(replies.answers["save_source_first"])).toMatchObject({ outcome: "saved", parent: null });
    expect(parseSaved(replies.answers["save_source_unchanged"]).outcome).toBe("unchanged");
    expect(parseHistory(replies.answers["history"])).toHaveLength(2);
  });

  it("include a refusal for every kind the screen handles, each in the shape of a refusal", () => {
    for (const kind of ["conflict", "not-found", "invalid-id", "invalid-title", "bad-request", "unknown-command"]) {
      const refusal = asCommandError(replies.refusals[kind]);
      expect(refusal, kind).not.toBeNull();
      expect(refusal?.kind).toBe(kind);
    }
  });

  it("give a conflict the two revisions the screen offers the person", () => {
    const refusal = asCommandError(replies.refusals["conflict"]);
    const { base, current } = conflictRevisions(refusal?.detail);
    expect(base).toMatch(/^[0-9a-f]{64}$/);
    expect(current).toMatch(/^[0-9a-f]{64}$/);
    expect(current).not.toBe(base);
  });
});

describe("a reply that is not the promised shape", () => {
  const listing = replies.answers["list_works"] as { works: Record<string, unknown>[] };

  it("is refused with the place it went wrong", () => {
    expect(() => parseListing({ works: [{ id: "a", title: 1, created_at: "x" }], unreadable: [] })).toThrow(
      /listing\.works\[0\]\.title: expected a string/,
    );
    expect(() => parseListing({ works: [] })).toThrow(/listing\.unreadable: expected a list/);
    expect(() => parseListing(null)).toThrow(ContractError);
    expect(() => parseListing([])).toThrow(ContractError);
  });

  it("is refused when a revision is not a digest", () => {
    expect(() => parseSaved({ outcome: "saved", revision: "abc", parent: null })).toThrow(/revision/);
    expect(() => parseSaved({ outcome: "saved", revision: "A".repeat(64), parent: null })).toThrow(/revision/);
    expect(() => parseSaved({ outcome: "merged", revision: "a".repeat(64) })).toThrow(/outcome/);
  });

  it("is refused when a field the screen reads is missing", () => {
    const work = { ...(listing.works[0] ?? {}) };
    delete work["title"];
    expect(() => parseWork(work)).toThrow(/work\.title/);
    expect(() => parseWorkAndHead({ work: listing.works[0] })).toThrow(/read_work\.head/);
    expect(() => parseReadSource({})).toThrow(/read_source\.source/);
  });

  it("is accepted when the core has added a field the screen does not read", () => {
    expect(() => parseWork({ ...(listing.works[0] ?? {}), colour: "blue" })).not.toThrow();
  });

  it("is not mistaken for a refusal unless it has a kind and a message", () => {
    expect(asCommandError({ kind: "conflict" })).toBeNull();
    expect(asCommandError("conflict")).toBeNull();
    expect(asCommandError(null)).toBeNull();
    expect(asCommandError({ kind: "busy", message: "wait" })).toEqual({ kind: "busy", message: "wait" });
  });
});
