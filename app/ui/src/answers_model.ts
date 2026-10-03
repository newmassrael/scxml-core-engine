// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the screen knows about the owner's answers apart from how they are drawn:
// the answers saved, what has been typed since, and every path a save can take.
// Pure functions over an immutable value, like the editor's (`editor_model.ts`), so
// typing while a save is in flight, a conflict and a failure are each a test.
//
// Answers are saved as one map (a question's id to the owner's words) on top of the
// revision it was read from, and the core refuses a revision that is no longer
// current. A draft is kept ONLY where it differs from what is saved: a field the
// person has not touched, or has typed back to what it was, is not a change.

import type { Answers, AnswerEntry, Revision } from "./contract";

export type AnswersPhase = "idle" | "saving" | "conflict" | "failed";

export interface AnswersModel {
  /** The revision the saved answers were read as; `null` when nothing was answered yet. */
  readonly base: Revision | null;
  readonly saved: Readonly<Record<string, AnswerEntry>>;
  /** What has been typed, by question id, where it differs from `saved`. An empty text is "take it back". */
  readonly drafts: Readonly<Record<string, string>>;
  readonly phase: AnswersPhase;
  /** Set while `phase` is `failed`. */
  readonly failure: string | null;
}

export interface AnswersRequest {
  readonly answers: Readonly<Record<string, string>>;
  readonly base: Revision | null;
}

/** What the person has typed, trimmed the way the core keeps it. */
function normal(text: string): string {
  return text.trim();
}

export function openAnswers(read: Answers | null): AnswersModel {
  return {
    base: read?.revision ?? null,
    saved: read?.entries ?? {},
    drafts: {},
    phase: "idle",
    failure: null,
  };
}

/** What the field of question `id` shows: what was typed, else what is saved, else nothing. */
export function wordsOf(model: AnswersModel, id: string): string {
  const draft = model.drafts[id];
  return draft !== undefined ? draft : (model.saved[id]?.answer ?? "");
}

/** The question ids the owner has an answer for, saved or typed, in a stable order. */
export function answeredIds(model: AnswersModel): string[] {
  return [...new Set([...Object.keys(model.saved), ...Object.keys(model.drafts)])]
    .filter((id) => normal(wordsOf(model, id)) !== "")
    .sort();
}

/**
 * The person typed `text` into the field of question `id`. Typing after a failure or
 * a conflict keeps that state: it is about the last save, not about the keystroke.
 */
export function editAnswer(model: AnswersModel, id: string, text: string): AnswersModel {
  const drafts = { ...model.drafts };
  if (text === (model.saved[id]?.answer ?? "")) {
    delete drafts[id];
  } else {
    // Kept as typed, even where it differs from what is saved only by spaces at
    // its ends: the field shows what was typed, and `isAnswersDirty` is what
    // compares the trimmed words the core keeps.
    drafts[id] = text;
  }
  return { ...model, drafts };
}

/** Whether anything typed differs from what is saved. */
export function isAnswersDirty(model: AnswersModel): boolean {
  return Object.entries(model.drafts).some(
    ([id, text]) => normal(text) !== (model.saved[id]?.answer ?? ""),
  );
}

/** The save to send now, or `null` when nothing differs or one is in flight. */
export function answersRequest(model: AnswersModel): AnswersRequest | null {
  if (model.phase === "saving" || !isAnswersDirty(model)) return null;
  const answers: Record<string, string> = {};
  for (const id of answeredIds(model)) answers[id] = normal(wordsOf(model, id));
  return { answers, base: model.base };
}

export function answersSaving(model: AnswersModel): AnswersModel {
  return { ...model, phase: "saving", failure: null };
}

/**
 * The save took, and `read` is what the core now holds. A draft is dropped only if
 * it is what was sent: whatever was typed while the save was in flight is the
 * person's, and stays.
 */
export function answersSaved(
  model: AnswersModel,
  read: Answers | null,
  sent: Readonly<Record<string, string>>,
): AnswersModel {
  const drafts: Record<string, string> = {};
  for (const [id, text] of Object.entries(model.drafts)) {
    if (normal(text) !== (sent[id] ?? "")) drafts[id] = text;
  }
  return { ...openAnswers(read), drafts };
}

/**
 * The save was refused because the answers moved on. What is now saved is loaded
 * and what the person typed is kept, so nothing they wrote is lost and nothing
 * saved elsewhere is overwritten unseen: they look, and save again on top.
 */
export function answersConflicted(model: AnswersModel, current: Answers | null): AnswersModel {
  return { ...openAnswers(current), drafts: model.drafts, phase: "conflict" };
}

export function answersFailed(model: AnswersModel, failure: string): AnswersModel {
  return { ...model, phase: "failed", failure };
}
