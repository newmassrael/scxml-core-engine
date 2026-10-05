// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Whether the work moved under what the screen shows: each way a part can differ, and
// the parts that are not compared at all.

import { describe, expect, it } from "vitest";

import type { Basis, WorkHeads } from "../src/contract";
import { movedParts, sameRequest, UNREAD, type WorkOnScreen } from "../src/heads_model";
import { nextDelay, WATCH_MAX_MS, WATCH_MS } from "../src/watch";

const hex = (n: number): string => n.toString(16).padStart(64, "0");
const [T1, T2, M1, M2, A1, R1, R2, C1] = [1, 2, 3, 4, 5, 6, 7, 8].map(hex) as [
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
];

/** A work with every chain filled, the text at T2 and the model and list written for T2. */
const heads: WorkHeads = {
  source: T2,
  model: { revision: M1, written_for: T2 },
  answers: A1,
  requirements: { revision: R1, written_for: T2 },
  acceptance: C1,
  bundle: null,
  request: null,
};

/** The work those heads say, as a basis names it: what the acceptance panel's reads are of when they are current. */
const basisNow: Basis = { source: T2, model: M1, requirements: R1, answers: A1 };

/** The screen showing exactly that. */
const shown: WorkOnScreen = {
  source: T2,
  model: { head: { revision: M1, written_for: T2 }, sourceHead: T2 },
  answers: A1,
  requirements: { head: { revision: R1, written_for: T2 }, sourceHead: T2 },
  acceptance: C1,
  judged: [basisNow, basisNow],
};

describe("a work that moved under the screen", () => {
  it("is nothing when the screen shows what the core says", () => {
    expect(movedParts(shown, heads)).toEqual([]);
  });

  it("is each part whose revision the core says differently, in the order they are read again", () => {
    const moved: WorkHeads = {
      source: T1,
      model: { revision: M2, written_for: T2 },
      answers: null,
      requirements: { revision: R2, written_for: T2 },
      acceptance: hex(9),
      bundle: null,
      request: null,
    };
    expect(movedParts(shown, moved)).toEqual(["source", "answers", "model", "requirements", "acceptance"]);
  });

  it("includes a model that is the same revision but was written for another text", () => {
    // The authoring client kept the model for the text that came after it: nothing about
    // the model changed but where it stands, and that is what the screen says.
    const kept: WorkHeads = { ...heads, model: { revision: M1, written_for: T1 } };
    expect(movedParts(shown, kept)).toEqual(["model"]);
  });

  it("includes a model whose text moved on beside it", () => {
    // Another window saved the text: the model on screen was read beside T2 and is now
    // behind, though neither the model nor what it was written for changed.
    // The acceptance panel's reads name the work as it was (T2), so they are read again too.
    const moved: WorkHeads = { ...heads, source: T1 };
    expect(movedParts({ ...shown, source: T1 }, moved)).toEqual(["model", "requirements", "acceptance"]);
  });

  it("includes the acceptance when what its reads name is not the work as the core says it is", () => {
    // The acceptance record keeps its revision while the model changes, and the panel's reads
    // agree with each other: both are of the model before. Only the core's heads can say so.
    const newer: WorkHeads = { ...heads, model: { revision: M2, written_for: T2 } };
    const modelRead: WorkOnScreen = {
      ...shown,
      model: { head: { revision: M2, written_for: T2 }, sourceHead: T2 },
    };
    expect(movedParts(modelRead, newer)).toEqual(["acceptance"]);
    // Either read naming another work is enough; the other agreeing with the heads is not.
    expect(movedParts({ ...shown, judged: [basisNow, { ...basisNow, answers: null }] }, heads)).toEqual(["acceptance"]);
    expect(movedParts({ ...shown, judged: [{ ...basisNow, requirements: R2 }] }, heads)).toEqual(["acceptance"]);
  });

  it("is nothing for acceptance reads of the work as it is, or when the panel has none that name one", () => {
    expect(movedParts({ ...shown, judged: [] }, heads)).toEqual([]);
    expect(movedParts({ ...shown, judged: undefined }, { ...heads, model: { revision: M2, written_for: T2 } })).toEqual(
      ["model"],
    );
  });

  it("includes a part that appeared, and one that went", () => {
    const none: WorkOnScreen = { ...shown, model: null, answers: null, acceptance: null };
    expect(movedParts(none, heads)).toEqual(["answers", "model", "acceptance"]);
    const gone: WorkHeads = { ...heads, requirements: null, acceptance: null };
    expect(movedParts(shown, gone)).toEqual(["requirements", "acceptance"]);
  });

  it("is every part the screen could not read, whatever the core says", () => {
    // A read that failed leaves nothing on screen to compare, and "nothing to compare" would
    // mean the part is never read again. It is not the same as a work that has none (`null`).
    const unread: WorkOnScreen = {
      source: shown.source,
      model: UNREAD,
      answers: UNREAD,
      requirements: UNREAD,
      acceptance: UNREAD,
      judged: undefined,
    };
    expect(movedParts(unread, heads)).toEqual(["answers", "model", "requirements", "acceptance"]);
    const empty: WorkHeads = { ...heads, model: null, answers: null, requirements: null, acceptance: null };
    expect(movedParts(unread, empty)).toEqual(["answers", "model", "requirements", "acceptance"]);
  });

  it("is nothing for a part the screen cannot compare now", () => {
    // Being read, being saved, or holding what the person typed: the screen does not read
    // such a part again over their head, whatever the core says.
    const uncompared: WorkOnScreen = {
      source: undefined,
      model: undefined,
      answers: undefined,
      requirements: undefined,
      acceptance: undefined,
      judged: undefined,
    };
    const moved: WorkHeads = {
      source: T1,
      model: null,
      answers: null,
      requirements: null,
      acceptance: null,
      bundle: null,
      request: null,
    };
    expect(movedParts(uncompared, moved)).toEqual([]);
    // The text is held for what is typed, but the acceptance's reads are compared with the core, not with the screen.
    expect(movedParts({ ...shown, source: undefined }, { ...heads, source: T1 })).toEqual([
      "model",
      "requirements",
      "acceptance",
    ]);
  });

  it("is nothing for a work with nothing in it, shown as nothing", () => {
    const empty: WorkHeads = {
      source: null,
      model: null,
      answers: null,
      requirements: null,
      acceptance: null,
      bundle: null,
      request: null,
    };
    const blank: WorkOnScreen = {
      source: null,
      model: null,
      answers: null,
      requirements: null,
      acceptance: null,
      judged: undefined,
    };
    expect(movedParts(blank, empty)).toEqual([]);
  });
});

describe("two answers of the core about the latest request", () => {
  const running = { id: "req-0123456789ab", state: "running", attempt: 1 } as const;

  it("are the same when it is at the same place, or when there is none in both", () => {
    expect(sameRequest(null, null)).toBe(true);
    expect(sameRequest({ ...running }, running)).toBe(true);
    expect(sameRequest(null, running)).toBe(false);
  });

  it("differ when it is another request, or is in another state or attempt", () => {
    expect(sameRequest({ ...running, state: "interrupted" }, running)).toBe(false);
    expect(sameRequest({ ...running, attempt: 2 }, running)).toBe(false);
    expect(sameRequest({ ...running, id: "req-ba9876543210" }, running)).toBe(false);
  });
});

describe("how long the screen waits between questions", () => {
  it("doubles after a failure and stops at the longest", () => {
    expect(nextDelay(WATCH_MS)).toBe(WATCH_MS * 2);
    expect(nextDelay(WATCH_MAX_MS - 1)).toBe(WATCH_MAX_MS);
    expect(nextDelay(WATCH_MAX_MS)).toBe(WATCH_MAX_MS);
  });
});
