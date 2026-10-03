// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The owner's answers apart from how they are drawn: what counts as a change, what
// a save sends, and what each way a save can end leaves the person with.

import { describe, expect, it } from "vitest";

import {
  answeredIds,
  answersConflicted,
  answersFailed,
  answersRequest,
  answersSaved,
  answersSaving,
  editAnswer,
  isAnswersDirty,
  openAnswers,
  wordsOf,
} from "../src/answers_model";
import type { Answers } from "../src/contract";

const hex = (n: number): string => n.toString(16).padStart(64, "0");

const held: Answers = {
  revision: hex(1),
  entries: {
    a: { answer: "yes", answered_at: "2026-10-03T09:00:00Z" },
    b: { answer: "no", answered_at: "2026-10-03T09:00:00Z" },
  },
};

describe("answers as opened", () => {
  it("show what is saved, and nothing is a change", () => {
    const model = openAnswers(held);
    expect(wordsOf(model, "a")).toBe("yes");
    expect(wordsOf(model, "unasked")).toBe("");
    expect(isAnswersDirty(model)).toBe(false);
    expect(answersRequest(model)).toBeNull();
    expect(model.base).toBe(hex(1));
  });

  it("are nothing for a work the owner answered nothing of, and the first save names no base", () => {
    const model = editAnswer(openAnswers(null), "a", "yes");
    expect(model.base).toBeNull();
    expect(answersRequest(model)).toEqual({ answers: { a: "yes" }, base: null });
  });
});

describe("typing an answer", () => {
  it("is a change only where it differs from what is saved", () => {
    let model = editAnswer(openAnswers(held), "a", "yes, always");
    expect(isAnswersDirty(model)).toBe(true);
    model = editAnswer(model, "a", "yes");
    expect(isAnswersDirty(model)).toBe(false);
    expect(model.drafts).toEqual({});
  });

  it("is not a change when it differs only by spaces at its ends, which the core does not keep", () => {
    const model = editAnswer(openAnswers(held), "a", "  yes \n");
    expect(isAnswersDirty(model)).toBe(false);
    expect(wordsOf(model, "a")).toBe("  yes \n");
    expect(answersRequest(model)).toBeNull();
  });

  it("sends every answer the owner has, not only the one just typed", () => {
    const model = editAnswer(openAnswers(held), "a", "maybe");
    expect(answersRequest(model)).toEqual({ answers: { a: "maybe", b: "no" }, base: hex(1) });
  });

  it("adds an answer to a question that had none, in a stable order", () => {
    let model = editAnswer(openAnswers(held), "z", "last");
    model = editAnswer(model, "c", "middle");
    expect(Object.keys(answersRequest(model)?.answers ?? {})).toEqual(["a", "b", "c", "z"]);
  });

  it("takes an answer back when its field is cleared, and a blank one is no answer at all", () => {
    const cleared = editAnswer(openAnswers(held), "b", "");
    expect(isAnswersDirty(cleared)).toBe(true);
    expect(answersRequest(cleared)?.answers).toEqual({ a: "yes" });
    expect(answeredIds(editAnswer(openAnswers(null), "x", "   "))).toEqual([]);
    expect(answersRequest(editAnswer(openAnswers(null), "x", "   "))).toBeNull();
  });
});

describe("a save", () => {
  it("is not offered while one is on its way", () => {
    const model = answersSaving(editAnswer(openAnswers(held), "a", "maybe"));
    expect(model.phase).toBe("saving");
    expect(answersRequest(model)).toBeNull();
  });

  it("that took leaves the core's answers shown and the typing the person did meanwhile", () => {
    const sent = { a: "maybe", b: "no" };
    let model = answersSaving(editAnswer(openAnswers(held), "a", "maybe"));
    // Typed while the save was in flight.
    model = editAnswer(model, "b", "never");
    const read: Answers = {
      revision: hex(2),
      entries: {
        a: { answer: "maybe", answered_at: "2026-10-03T09:00:05Z" },
        b: held.entries["b"] as Answers["entries"][string],
      },
    };
    const after = answersSaved(model, read, sent);
    expect(after.phase).toBe("idle");
    expect(after.base).toBe(hex(2));
    expect(after.saved["a"]?.answered_at).toBe("2026-10-03T09:00:05Z");
    expect(after.drafts).toEqual({ b: "never" });
    expect(isAnswersDirty(after)).toBe(true);
  });

  it("that took drops every draft that was what was sent, and a taken-back answer with it", () => {
    const sent = { a: "yes" };
    const model = answersSaving(editAnswer(openAnswers(held), "b", ""));
    const after = answersSaved(model, { revision: hex(2), entries: { a: held.entries["a"] as never } }, sent);
    expect(after.drafts).toEqual({});
    expect(isAnswersDirty(after)).toBe(false);
  });

  it("that conflicted loads what is saved now and keeps what the person typed", () => {
    const model = answersSaving(editAnswer(openAnswers(held), "a", "mine"));
    const current: Answers = { revision: hex(9), entries: { a: { answer: "theirs", answered_at: "2026-10-03T10:00:00Z" } } };
    const after = answersConflicted(model, current);
    expect(after.phase).toBe("conflict");
    expect(after.base).toBe(hex(9));
    expect(wordsOf(after, "a")).toBe("mine");
    expect(after.saved["a"]?.answer).toBe("theirs");
    // Saved again, it goes on top of the revision that turned up and not over it unseen.
    expect(answersRequest(after)).toEqual({ answers: { a: "mine" }, base: hex(9) });
  });

  it("that failed keeps the typing and says why, and can be tried again", () => {
    const model = answersFailed(answersSaving(editAnswer(openAnswers(held), "a", "mine")), "the disk is full");
    expect(model.phase).toBe("failed");
    expect(model.failure).toBe("the disk is full");
    expect(model.drafts).toEqual({ a: "mine" });
    expect(answersRequest(model)).toEqual({ answers: { a: "mine", b: "no" }, base: hex(1) });
  });
});
