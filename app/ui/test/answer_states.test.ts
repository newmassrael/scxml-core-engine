// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import { answerState, type Inputs } from "../src/answer_states";
import type { GenerationRequest, RequestState } from "../src/contract";

const A1 = "a".repeat(64);
const A2 = "b".repeat(64);

const request = (state: RequestState, answers: string | null): { head: Inputs["head"]; detail: GenerationRequest } => ({
  head: { id: "req-1", state, attempt: 1 },
  detail: {
    id: "req-1",
    work: "door",
    seq: 1,
    key: "k",
    origin: "gui",
    state,
    stored_state: state,
    attempt: 1,
    inputs: { source: "c".repeat(64), answers },
    created_at: "2026-10-05T09:00:00Z",
    lease: null,
    candidate: null,
    outcome: null,
    ended_at: null,
    note: null,
  },
});

const base: Inputs = {
  typed: "Any card on the list opens it.",
  saved: "Any card on the list opens it.",
  asked: true,
  madeFrom: null,
  head: null,
  detail: null,
  answersNow: A2,
};

describe("where an answer stands", () => {
  it("is nothing for a question nobody has answered", () => {
    expect(answerState("q", { ...base, typed: "", saved: null })).toBeNull();
    expect(answerState("q", { ...base, typed: "   ", saved: null })).toBeNull();
  });

  it("is unsaved when words are typed that the core does not hold, or differ from what it holds", () => {
    expect(answerState("q", { ...base, saved: null })).toBe("unsaved");
    expect(answerState("q", { ...base, typed: "Only cards on the list." })).toBe("unsaved");
  });

  it("is saved when it is held and no model that the screen can tell of was made from it", () => {
    expect(answerState("q", base)).toBe("saved");
    // A model made from other answers, or from none, says nothing of this one.
    expect(answerState("q", { ...base, madeFrom: { answers: {} } })).toBe("saved");
    expect(answerState("q", { ...base, madeFrom: { answers: { q: "Another answer." } } })).toBe("saved");
  });

  it("is being written while a request about these answers is open", () => {
    for (const state of ["queued", "running", "interrupted"] as const) {
      const r = request(state, A2);
      expect(answerState("q", { ...base, ...r }), state).toBe("writing");
    }
  });

  it("is not being written by a request made about older answers, or one that is over", () => {
    expect(answerState("q", { ...base, ...request("running", A1) })).toBe("saved");
    expect(answerState("q", { ...base, ...request("running", null) })).toBe("saved");
    for (const state of ["completed", "failed", "cancelled", "superseded"] as const) {
      expect(answerState("q", { ...base, ...request(state, A2) }), state).toBe("saved");
    }
  });

  it("does not use what was read of another request", () => {
    const r = request("running", A2);
    expect(answerState("q", { ...base, head: { ...r.head!, id: "req-2" }, detail: r.detail })).toBe("saved");
  });

  it("is in the model when the model was made from this very answer and no longer asks the question", () => {
    const madeFrom = { answers: { q: "Any card on the list opens it." } };
    expect(answerState("q", { ...base, madeFrom, asked: false })).toBe("in-model");
  });

  it("is ignored when the model was made from this answer and still asks the question", () => {
    const madeFrom = { answers: { q: "Any card on the list opens it." } };
    expect(answerState("q", { ...base, madeFrom, asked: true })).toBe("ignored");
  });

  it("is being written, not ignored, when the owner asked again", () => {
    const madeFrom = { answers: { q: "Any card on the list opens it." } };
    expect(answerState("q", { ...base, madeFrom, ...request("running", A2) })).toBe("writing");
  });
});
