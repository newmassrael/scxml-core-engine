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
  parseFigures,
  parseHistory,
  parseListing,
  parseReadAnswers,
  parseReadModel,
  parseReadSource,
  parseRemoved,
  parseReview,
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
  save_model: parseSaved,
  read_model: parseReadModel,
  model_history: parseHistory,
  figures: parseFigures,
  remove_work: parseRemoved,
  review: parseReview,
  read_answers: parseReadAnswers,
  save_answers: parseSaved,
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

  it("carry what the screen shows of a model: where it stands, and the sheets in SCE's order", () => {
    expect(parseReadModel(replies.answers["read_model_none"])).toMatchObject({ model: null, standing: null });
    // The same model, for the text before and after it moved on.
    expect(parseReadModel(replies.answers["read_model"]).standing).toBe("behind");
    expect(parseReadModel(replies.answers["read_model_kept"]).standing).toBe("current");
    const model = parseReadModel(replies.answers["read_model"]);
    expect(model.model?.written_for).not.toBe(model.source_head);
    const drawn = parseFigures(replies.answers["figures"]);
    expect(drawn.sheets.map((s) => s.name)).toEqual(["picture.svg", "fields-1.svg"]);
    expect(drawn.sheets[0]?.svg.startsWith("<svg")).toBe(true);
    expect(drawn.generator).toBe("fake-sce 0");
  });

  it("carry what the screen shows of SCE's review: the verdict, what is left open, the page or why not", () => {
    const accepted = parseReview(replies.answers["review"]);
    expect(accepted.check).toMatchObject({ verdict: "accepted", kind: "statechart", records: [] });
    expect(accepted.check.open).toHaveLength(1);
    expect(accepted.check.unresolved[0]).toEqual({
      id: "open-guard",
      node_path: "states.closed.transitions[0]",
      line: 3,
      reason: "Which card values open the door?",
    });
    expect(accepted.page).toContain("machine");
    expect(accepted.page_refusal).toBeNull();
    // The same standing the figures and the model read carry: one word, from the core.
    expect(accepted.standing).toBe("behind");

    const refused = parseReview(replies.answers["review_refused"]);
    expect(refused.check.verdict).toBe("refused");
    expect(refused.check.kind).toBeNull();
    expect(refused.page).toBeNull();
    expect(refused.check.records[0]).toMatchObject({ code: "validation/invalid-reference", line: 3 });

    const noPage = parseReview(replies.answers["review_no_page"]);
    expect(noPage.check.verdict).toBe("accepted");
    expect(noPage.page).toBeNull();
    expect(noPage.page_refusal?.code).toBe("cli/pseudo-unsupported");
  });

  it("carry the owner's answers by question, each with its words and when they changed", () => {
    expect(parseReadAnswers(replies.answers["read_answers_none"])).toBeNull();
    const held = parseReadAnswers(replies.answers["read_answers"]);
    expect(held?.revision).toMatch(/^[0-9a-f]{64}$/);
    expect(held?.entries["open-guard"]?.answer).toBe("Any card on the list opens it.");
    expect(held?.entries["open-guard"]?.answered_at).toBe("2026-10-03T09:00:00Z");
    expect(parseSaved(replies.answers["save_answers_first"]).outcome).toBe("saved");
    expect(parseSaved(replies.answers["save_answers_unchanged"]).outcome).toBe("unchanged");
    expect(asCommandError(replies.refusals["invalid-answers"])?.message).toContain("is empty");
    // The question's own words are what the screen shows the owner.
    const review = parseReview(replies.answers["review"]);
    expect(review.check.unresolved[0]?.reason).toBe("Which card values open the door?");
  });

  it("say which work was removed, and that it is no longer listed or readable", () => {
    expect(parseRemoved(replies.answers["remove_work"]).title).toBe("Window blind");
    const after = parseListing(replies.answers["list_works_after_removal"]);
    expect(after.works.map((w) => w.title)).toEqual(["Door lock"]);
    expect(after.unreadable).toEqual([]);
    const refusal = asCommandError(replies.refusals["removed-work"]);
    expect(refusal?.kind).toBe("not-found");
    expect(refusal?.message).toContain("removed");
  });

  it("include a refusal for every kind the screen handles, each in the shape of a refusal", () => {
    for (const kind of [
      "conflict",
      "not-found",
      "invalid-id",
      "invalid-title",
      "bad-request",
      "unknown-command",
      "sce-refused",
      "sce-unavailable",
    ]) {
      const refusal = asCommandError(replies.refusals[kind]);
      expect(refusal, kind).not.toBeNull();
      expect(refusal?.kind).toBe(kind);
    }
  });

  it("give SCE's refusal the product's own code, for the screen to show beside its sentence", () => {
    const refusal = asCommandError(replies.refusals["sce-refused"]);
    expect(refusal?.detail).toEqual({ code: "cli/diagram-does-not-fit" });
    expect(refusal?.message).toContain("cli/diagram-does-not-fit");
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
    expect(() => parseRemoved({})).toThrow(/remove_work\.removed/);
    expect(() => parseReadAnswers({})).toThrow(/read_answers/);
    expect(() => parseReadAnswers({ answers: { revision: "abc", entries: {} } })).toThrow(/revision/);
    expect(() =>
      parseReadAnswers({ answers: { revision: "a".repeat(64), entries: { q: { answer: 3, answered_at: "t" } } } }),
    ).toThrow(/entries\.q\.answer/);
  });

  it("is refused when a review's verdict is not a word the screen knows, or a record has no code", () => {
    const review = replies.answers["review"] as { check: Record<string, unknown> } & Record<string, unknown>;
    expect(() => parseReview({ ...review, check: { ...review.check, verdict: "maybe" } })).toThrow(
      /review\.check\.verdict/,
    );
    expect(() =>
      parseReview({ ...review, check: { ...review.check, records: [{ message: "m", stage: null, line: null }] } }),
    ).toThrow(/review\.check\.records\[0\]\.code/);
    expect(() => parseReview({ ...review, page: 3 })).toThrow(/review\.page/);
    expect(() => parseReview({ ...review, page_refusal: { code: "c" } })).toThrow(/review\.page_refusal\.message/);
  });

  it("is refused when a model's standing is a word the screen does not know, or a sheet has no svg", () => {
    const read = replies.answers["read_model"] as Record<string, unknown>;
    expect(() => parseReadModel({ ...read, standing: "stale" })).toThrow(/read_model\.standing/);
    expect(() => parseReadModel({ ...read, model: null })).toThrow(/null when there is no model/);
    const figures = replies.answers["figures"] as { sheets: Record<string, unknown>[] };
    expect(() => parseFigures({ ...figures, sheets: [{ name: "a.svg" }] })).toThrow(/figures\.sheets\[0\]\.svg/);
    expect(() => parseFigures({ ...figures, generator: 3 })).toThrow(/figures\.generator/);
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
