// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Where an answer stands: typed, saved, being written into a model, or in the model that is
// shown. Four different facts that a screen that says only "saved" runs together.
//
// What the screen can say is what the core records. A model made by a request carries the answers
// the request was made about (its bundle, `Bundle.answers`), so whether the model shown was made
// from THIS answer is a comparison of what was answered then with what is answered now. A model
// that was saved some other way says nothing of what it was made from, and an answer is then
// only saved: nothing here claims more than that.
//
// "In the model" is not "the model means what the owner meant". It says the model was made after
// the answer was given and no longer asks the question, which is what the core can know; the owner
// still reads the behaviour it led to.

import type { GenerationRequest, RequestHead, Revision } from "./contract";
import { isOpenRequest } from "./contract";

export type AnswerState =
  /** Typed, and not saved. */
  | "unsaved"
  /** Saved, and no model made from it that the screen can tell of. */
  | "saved"
  /** A request made about this version of the answers is open: the AI is writing it into a model. */
  | "writing"
  /** The model shown was made after this answer was given, and it no longer asks the question. */
  | "in-model"
  /** The model shown was made after this answer was given, and it still asks the question. */
  | "ignored";

/** What the model shown was made from, when a request made it. */
export interface MadeFrom {
  /** The answers at the revision the request was about: question id to words. */
  readonly answers: Readonly<Record<string, string>>;
}

export interface Inputs {
  /** Words in the field now; `undefined` for a question not answered here. */
  readonly typed: string;
  /** Words the core holds, when it holds some. */
  readonly saved: string | null;
  /** The model shown asks this question. */
  readonly asked: boolean;
  /** What the model shown was made from; `null` when no request made it (or it is not yet read). */
  readonly madeFrom: MadeFrom | null;
  /** The latest request and what was read of it, and the revision of the answers now. */
  readonly head: RequestHead | null;
  readonly detail: GenerationRequest | null;
  readonly answersNow: Revision | null;
}

/**
 * The state of the answer to question `id`, or `null` when there is no answer to speak of (none
 * typed and none saved).
 */
export function answerState(id: string, inputs: Inputs): AnswerState | null {
  const typed = inputs.typed;
  if (typed.trim() === "" && inputs.saved === null) return null;
  if (inputs.saved === null || typed !== inputs.saved) return "unsaved";
  // A request that is open and was made about the answers as they are now is writing them in.
  const asking =
    inputs.head !== null &&
    isOpenRequest(inputs.head.state) &&
    inputs.detail !== null &&
    inputs.detail.id === inputs.head.id &&
    inputs.detail.inputs.answers !== null &&
    inputs.detail.inputs.answers === inputs.answersNow;
  if (asking) return "writing";
  const then = inputs.madeFrom?.answers[id];
  // The model was made from this very answer, or from another one, or from none.
  if (then !== undefined && then === inputs.saved) return inputs.asked ? "ignored" : "in-model";
  return "saved";
}
