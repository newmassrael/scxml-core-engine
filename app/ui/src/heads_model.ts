// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Whether the work moved under what the screen shows, apart from how the screen
// reads it again. Pure functions over the heads the core says (`read_work_heads`)
// and what the screen holds, so each way a part can differ is a test.
//
// The comparison is between the core and what is ON SCREEN, not between two answers of
// the core. That is what keeps the screen from reading again what it already has: its
// own save moves a head and the screen shows the saved text a moment later, and a
// comparison with the previous answer would call that a change from elsewhere.

import type { ClaimedHead, RequestHead, Revision, WorkHeads } from "./contract";

/** The parts of a work that are read apart from each other, and so are read again apart. */
export type Part = "source" | "model" | "answers" | "requirements" | "acceptance";

/**
 * A part the screen tried to read and could not. It is not "nothing on screen" (that is
 * `null`, which says the work has none) and it is not "left out" (`undefined`): there is
 * something to read, the screen does not have it, and so it differs from whatever the core
 * says, until a read has worked.
 */
export const UNREAD: unique symbol = Symbol("unread");
export type Unread = typeof UNREAD;

/**
 * What the screen shows of one part of the work. `undefined` means there is nothing to
 * compare now: the part is being read or saved, or holds what the person typed, and a part
 * that is not compared is not read again over their head. A part that could not be read is
 * `UNREAD`, which is compared: leaving it out would leave it unread for good.
 */
export type OnScreen<T> = T | null | undefined | Unread;

/** A model or a requirement list on screen, and the head of the source it was read beside. */
export interface ClaimedOnScreen {
  readonly head: ClaimedHead;
  /** Where it stands to the text is that text's head against what it was written for. */
  readonly sourceHead: Revision | null;
}

export interface WorkOnScreen {
  /** The revision of the text the editor holds as saved. */
  readonly source: OnScreen<Revision>;
  readonly model: OnScreen<ClaimedOnScreen>;
  readonly answers: OnScreen<Revision>;
  readonly requirements: OnScreen<ClaimedOnScreen>;
  readonly acceptance: OnScreen<Revision>;
}

function sameHead(a: ClaimedHead | null, b: ClaimedHead | null): boolean {
  if (a === null || b === null) return a === b;
  return a.revision === b.revision && a.written_for === b.written_for;
}

/** Whether a part on screen as `shown` differs from the `head` the core says. */
function revisionMoved(shown: OnScreen<Revision>, head: Revision | null): boolean {
  if (shown === undefined) return false;
  if (shown === UNREAD) return true;
  return shown !== head;
}

/**
 * Whether a model or list on screen as `shown` differs from the `head` the core says: in
 * which revision it is, in what it was written for, or in the text it now stands beside.
 */
function claimedMoved(shown: OnScreen<ClaimedOnScreen>, head: ClaimedHead | null, sourceHead: Revision | null): boolean {
  if (shown === undefined) return false;
  if (shown === UNREAD) return true;
  if (shown === null) return head !== null;
  return !sameHead(shown.head, head) || shown.sourceHead !== sourceHead;
}

/** The parts of the work the core now says differently from what the screen shows, in the order they are read again. */
export function movedParts(shown: WorkOnScreen, heads: WorkHeads): Part[] {
  const moved: Part[] = [];
  if (revisionMoved(shown.source, heads.source)) moved.push("source");
  if (revisionMoved(shown.answers, heads.answers)) moved.push("answers");
  if (claimedMoved(shown.model, heads.model, heads.source)) moved.push("model");
  if (claimedMoved(shown.requirements, heads.requirements, heads.source)) moved.push("requirements");
  if (revisionMoved(shown.acceptance, heads.acceptance)) moved.push("acceptance");
  return moved;
}

/** Whether two answers of the core say the latest request is at the same place. */
export function sameRequest(a: RequestHead | null, b: RequestHead | null): boolean {
  if (a === null || b === null) return a === b;
  return a.id === b.id && a.state === b.state && a.attempt === b.attempt;
}

