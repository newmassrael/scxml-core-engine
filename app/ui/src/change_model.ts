// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the owner is shown of how a new model differs from the one it replaced.
//
// The comparison is of the two models' pseudocode pages, which SCE writes and the owner reads, and
// not of their SCXML: a change in a condition, a signal, a value or a time shows up as the lines
// of the page that hold it. Which model it is compared with is the one before it in the work's
// model history, which belongs to the core (a work whose models were made by requests lists them
// as bundles, after the ones it had before).

import type { HistoryEntry, Revision } from "./contract";
import { diffLines, hunksOf, tallyOf, type Hunk } from "./line_diff";

/** Where the comparison is, and what it found. */
export type ChangePanel =
  | { readonly phase: "reading" }
  /** There is no earlier model: this is the first. */
  | { readonly phase: "none" }
  | { readonly phase: "unchanged" }
  /** One of the two pages could not be had, so there is nothing to compare; `reason` says which and why. */
  | { readonly phase: "unavailable"; readonly reason: string }
  | { readonly phase: "changed"; readonly hunks: readonly Hunk[]; readonly added: number; readonly removed: number };

/**
 * The model that `current` replaced: the nearest one before its last appearance in the history
 * that is not `current` itself (a model published again unchanged replaces nothing), or the one
 * the first entry says it followed. `null` when there was none.
 */
export function previousOf(history: readonly HistoryEntry[], current: Revision): Revision | null {
  let at = -1;
  history.forEach((entry, index) => {
    if (entry.revision === current) at = index;
  });
  if (at < 0) return null;
  for (let index = at - 1; index >= 0; index -= 1) {
    const earlier = history[index]?.revision;
    if (earlier !== undefined && earlier !== current) return earlier;
  }
  const parent = history[at]?.parent ?? null;
  return parent !== null && parent !== current ? parent : null;
}

/**
 * Compare the page of the model that was with the page of the model that is. A page SCE did not
 * write (the model was refused, or the page was) cannot be compared, and is said to be so, with
 * which of the two it was.
 */
export function comparePages(before: string | null, after: string | null): ChangePanel {
  if (before === null) return { phase: "unavailable", reason: "before" };
  if (after === null) return { phase: "unavailable", reason: "after" };
  const lines = diffLines(before, after);
  const { added, removed } = tallyOf(lines);
  if (added === 0 && removed === 0) return { phase: "unchanged" };
  return { phase: "changed", hunks: hunksOf(lines), added, removed };
}
