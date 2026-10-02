// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the editor knows about the text in front of the person, apart from how it
// is drawn. Pure functions over an immutable value, so every path a save can take
// (saved, unchanged, conflict, failure, typing while it is in flight) is a test.
//
// The one rule underneath: a save names the revision it was written from (`base`),
// and the core refuses it if that is no longer current. The model never decides to
// overwrite someone else's text; the person does, by choosing to save on top of the
// revision that turned up.

import type { Revision, Saved, SourceText } from "./contract";

export type Phase = "idle" | "saving" | "conflict" | "failed";

export interface EditorModel {
  readonly workId: string;
  /** The revision the buffer was loaded from, or last saved as. `null`: the work has no text yet. */
  readonly base: Revision | null;
  /** The text at `base` (empty when there is none). */
  readonly saved: string;
  /** What is in the editor. */
  readonly text: string;
  readonly phase: Phase;
  /** Set while `phase` is `conflict`: the revision now current in the core. */
  readonly current: Revision | null;
  /** Set while `phase` is `failed`. */
  readonly failure: string | null;
}

export interface SaveRequest {
  readonly id: string;
  readonly text: string;
  readonly base: Revision | null;
}

export function open(workId: string, source: SourceText | null): EditorModel {
  return {
    workId,
    base: source?.revision ?? null,
    saved: source?.text ?? "",
    text: source?.text ?? "",
    phase: "idle",
    current: null,
    failure: null,
  };
}

export function edit(model: EditorModel, text: string): EditorModel {
  // Typing after a failure or a conflict keeps that state: it is about the last
  // save, which has not been retried, not about the keystroke.
  return { ...model, text };
}

export function isDirty(model: EditorModel): boolean {
  return model.text !== model.saved;
}

/** The save to send now, or `null` when there is nothing to send or one is in flight. */
export function saveRequest(model: EditorModel): SaveRequest | null {
  if (model.phase === "saving" || model.phase === "conflict" || !isDirty(model)) return null;
  return { id: model.workId, text: model.text, base: model.base };
}

export function saving(model: EditorModel): EditorModel {
  return { ...model, phase: "saving", failure: null };
}

/** The core took `sentText`. Anything typed since stays in the buffer, still unsaved. */
export function saved(model: EditorModel, sentText: string, outcome: Saved): EditorModel {
  return {
    ...model,
    base: outcome.revision,
    saved: sentText,
    phase: "idle",
    current: null,
    failure: null,
  };
}

/** The core refused: `base` is not current any more. */
export function conflicted(model: EditorModel, current: Revision | null): EditorModel {
  return { ...model, phase: "conflict", current, failure: null };
}

export function failed(model: EditorModel, message: string): EditorModel {
  return { ...model, phase: "failed", failure: message };
}

/** Discard the buffer for the text that was saved over it. */
export function takeTheirs(model: EditorModel, theirs: SourceText | null): EditorModel {
  return open(model.workId, theirs);
}

/**
 * Keep the buffer and write it on top of the revision that turned up. The next
 * save names that revision as its base, so it is accepted, and the history keeps
 * both (the other person's text stays a revision of its own).
 */
export function keepMine(model: EditorModel): EditorModel {
  if (model.phase !== "conflict") return model;
  return { ...model, base: model.current, phase: "idle", current: null };
}
