// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the acceptance panel is in, and the few pure facts it needs.
//
// The screen decides nothing about a design. SCE classifies each requirement
// (`implemented`, `missing`, `needs-scenario`), SCE says whether an acceptance still
// holds and what moved, and the core refuses an acceptance of anything but what the
// owner was shown. What is decided HERE is only whether the button is offered, and
// what the owner is told before they press it: a design with a gap is theirs to
// accept, and is not refused for it. Pure, like the answers' model, so each way the
// button is withheld is a test.

import type { Basis, ReadAcceptance, ReadRequirements, RequirementOutcome, RequirementsReport } from "./contract";

/** Where the panel is. */
export type AcceptancePanel =
  | { readonly phase: "reading" }
  /** The work has no requirement list: there is nothing to accept a design against. */
  | { readonly phase: "no-list" }
  /** The list or the acceptance could not be read at all. */
  | { readonly phase: "failed"; readonly message: string }
  | { readonly phase: "read"; readonly state: AcceptanceState };

/** A work that has a list, and what is known of it. */
export interface AcceptanceState {
  readonly list: ReadRequirements;
  readonly acceptance: ReadAcceptance;
  /** SCE's measure; `null` when SCE did not answer (`measureFailure` says why). */
  readonly report: RequirementsReport | null;
  readonly measureFailure: string | null;
  /** The accept is on its way. */
  readonly accepting: boolean;
  /** Why the last accept took nothing, in the core's words. */
  readonly refusal: string | null;
}

/** Why the accept button is not offered; `null` from `gate` means it is. */
export type Withheld =
  /** What is accepted is what is saved, and something typed is not. */
  | "unsaved"
  | "accepting"
  /** SCE did not measure the design, so the owner has not been shown what they would accept. */
  | "not-measured"
  /** The design or the list was written for an earlier text. */
  | "behind"
  /** This very design, text, list and answers are accepted already and the acceptance holds. */
  | "already";

/**
 * Whether the owner may press accept now, and if not, why. The core refuses what is
 * not current and what moved; this keeps the button from offering what it will refuse.
 */
export function gate(state: AcceptanceState, unsaved: boolean): Withheld | null {
  if (state.accepting) return "accepting";
  if (unsaved) return "unsaved";
  const report = state.report;
  if (report === null) return "not-measured";
  if (report.model_standing !== "current" || report.requirements_standing !== "current") return "behind";
  const accepted = state.acceptance;
  if (accepted.standing === "holds" && accepted.acceptance !== null && sameBasis(accepted.acceptance.basis, report.basis)) {
    return "already";
  }
  return null;
}

function sameBasis(a: Basis, b: Basis): boolean {
  return a.source === b.source && a.model === b.model && a.requirements === b.requirements && a.answers === b.answers;
}

/**
 * SCE's words for how a design stands to a requirement (`Outcome` in the product's
 * `requirement_manifest.rs`), in the order a reader meets them: what the design settles,
 * then what it does not, then where the list places a requirement outside the design.
 */
export const OUTCOME_WORDS: readonly string[] = [
  "implemented",
  "scenario-passed",
  "missing",
  "unresolved",
  "dangling",
  "contradicted",
  "scenario-failed",
  "needs-scenario",
  "delegated",
  "out-of-scope",
  "system-level",
];

/**
 * The words that say the design leaves a requirement unsettled: nothing carries it
 * (`missing`), only an open node does (`unresolved`), the design cites what the list
 * does not have (`dangling`) or what the list places elsewhere (`contradicted`), a
 * scenario failed, or only a scenario can say (`needs-scenario`). The owner is shown
 * these before they accept, and is not refused for them.
 */
const UNSETTLED: ReadonlySet<string> = new Set([
  "missing",
  "unresolved",
  "dangling",
  "contradicted",
  "scenario-failed",
  "needs-scenario",
]);

/** Whether `word` is one of SCE's that says a requirement is left unsettled. A word SCE adds later is not guessed at. */
export function isUnsettled(word: string): boolean {
  return UNSETTLED.has(word);
}

/**
 * How many requirements fall in each of SCE's words, in `OUTCOME_WORDS` order and then
 * any other word a later SCE writes, alphabetically. A word is counted as SCE spelled
 * it; the screen does not fold an unknown one into another.
 */
export function tally(outcomes: readonly RequirementOutcome[]): ReadonlyArray<readonly [string, number]> {
  const counts = new Map<string, number>();
  for (const { outcome } of outcomes) counts.set(outcome, (counts.get(outcome) ?? 0) + 1);
  const known = OUTCOME_WORDS.filter((word) => counts.has(word));
  const others = [...counts.keys()].filter((word) => !OUTCOME_WORDS.includes(word)).sort();
  return [...known, ...others].map((word) => [word, counts.get(word) ?? 0] as const);
}

export function accepting(state: AcceptanceState): AcceptanceState {
  return { ...state, accepting: true, refusal: null };
}

/** The accept took nothing: what the core said, and what is there now. */
export function acceptRefused(state: AcceptanceState, refusal: string): AcceptanceState {
  return { ...state, accepting: false, refusal };
}
