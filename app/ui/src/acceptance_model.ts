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

import {
  ContractError,
  sameBasis,
  type Basis,
  type Judgment,
  type ReadAcceptance,
  type ReadRequirements,
  type RequirementOutcome,
  type RequirementsReport,
  type WorkSnapshot,
} from "./contract";

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
  /**
   * The revisions of the work this is of: the snapshot it was read beside, which SCE was asked
   * about by name. The list, the report and the standing are all of these and of no others.
   */
  readonly basis: Basis;
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
  /** Something the report measured has not been read onto the screen yet, or could not be: the owner has not been shown it. */
  | "unread"
  /** What the screen shows is not what the report measured: the owner would accept what they have not read. */
  | "differs"
  /** This very design, text, list and answers are accepted already and the acceptance holds. */
  | "already";

/**
 * The revisions the screen is showing. Every part is stated, so a caller cannot leave one
 * out and be taken to have nothing to say: `undefined` is "not on the screen" (still being
 * read, or it could not be read), and `null` is "on the screen, and nothing is saved" (no
 * answers). The two are not the same thing and are never taken for each other.
 */
export interface Shown {
  readonly source: string | null | undefined;
  readonly model: string | null | undefined;
  readonly answers: string | null | undefined;
  /** The requirement list the screen read: the sentences it quotes are what the owner reads each requirement by. */
  readonly requirements: string | null | undefined;
}

/**
 * Whether what is shown is what the report measured, and if not, why. The report is of the
 * revisions the screen read the model in, but the text and the answers are the person's and
 * are read apart from it, and a text saved from another entrance moves under the screen: the
 * report can be of a newer text than the one the owner is reading. The core accepts what the
 * report names, so the screen is where the owner is kept from accepting what they did not read.
 *
 * ⚠ A part that is not on the screen is NOT a part that matches. It used to be taken for
 * one, and the saved answers could then be accepted before the owner had been shown them.
 */
function whatIsNotShown(shown: Shown, basis: Basis): "unread" | "differs" | null {
  if (
    shown.source === undefined ||
    shown.model === undefined ||
    shown.answers === undefined ||
    shown.requirements === undefined
  ) {
    return "unread";
  }
  const same = (a: string | null, b: string | null): boolean => a === b;
  return same(shown.source, basis.source) &&
    same(shown.model, basis.model) &&
    same(shown.answers, basis.answers) &&
    same(shown.requirements, basis.requirements)
    ? null
    : "differs";
}

/**
 * The revisions of the work a snapshot holds, which is what SCE is asked about; `null` when it
 * holds no text, no model or no list, since there is no design to measure against anything.
 */
export function basisOf(snapshot: WorkSnapshot): Basis | null {
  const { source, model, requirements } = snapshot;
  if (source === null || model === null || requirements === null) return null;
  return {
    source: source.revision,
    model: model.revision,
    requirements: requirements.revision,
    answers: snapshot.answers?.revision ?? null,
  };
}

/**
 * The panel for the work `snapshot` holds, from what SCE said of those revisions. Every part is
 * of the snapshot: the list is its list, the measure and the standing were asked of its revisions
 * by name, so nothing has to be compared and no part can be of another state of the work. SCE not
 * measuring is a state of the panel (`measureFailure`); SCE not judging the acceptance is a
 * failure of it, since whether the owner's acceptance holds is not something to guess at.
 */
export function panelOf(snapshot: WorkSnapshot, judgment: Judgment): AcceptancePanel {
  const list = snapshot.requirements;
  const source = snapshot.source;
  if (list === null || source === null) return { phase: "no-list" };
  const held = snapshot.acceptance;
  // A verdict is asked for exactly when an acceptance was made, so an answer that disagrees is a
  // caller that did not ask what it should have, which is not shown as if nothing was accepted.
  if ((held === null) !== (judgment.acceptance === null)) {
    throw new ContractError("judgment.acceptance", "a verdict exactly when the snapshot holds an acceptance");
  }
  let acceptance: ReadAcceptance;
  if (held === null || judgment.acceptance === null) {
    acceptance = { acceptance: null, standing: "none", lapse: null, now: null };
  } else if (!judgment.acceptance.said) {
    return { phase: "failed", message: judgment.acceptance.refusal.message };
  } else {
    const { standing, lapse } = judgment.acceptance.value;
    acceptance = { acceptance: held, standing, lapse, now: judgment.basis };
  }
  return {
    phase: "read",
    state: {
      basis: judgment.basis,
      list: { requirements: list, source_head: source.revision, standing: snapshot.requirements_standing },
      acceptance,
      report: judgment.report.said ? judgment.report.value : null,
      measureFailure: judgment.report.said ? null : judgment.report.refusal.message,
      accepting: false,
      refusal: null,
    },
  };
}

/**
 * Whether a panel read is of the work `snapshot` holds: the same revisions, and the same acceptance
 * (an acceptance made elsewhere moves nothing a basis names). A panel that is not is not shown
 * beside that snapshot's model, since a verdict of one design beside another is a false statement
 * about what the owner is looking at; it is asked for again.
 */
export function panelIsOf(state: AcceptanceState, snapshot: WorkSnapshot): boolean {
  const basis = basisOf(snapshot);
  return (
    basis !== null &&
    sameBasis(state.basis, basis) &&
    (state.acceptance.acceptance?.revision ?? null) === (snapshot.acceptance?.revision ?? null)
  );
}

/**
 * Whether the acceptance's standing was judged of the work the screen is showing. The panel is of
 * the snapshot the model was read in, so the design shown is the one judged; what can differ is
 * the text and the answers, which the person edits apart from it and which are read again
 * apart. A part still being read is not held against the standing, and where there is no
 * acceptance there is no standing to be of another work.
 */
export function standingIsOfWhatIsShown(state: AcceptanceState, shown: Shown): boolean {
  if (state.acceptance.acceptance === null) return true;
  const basis = state.basis;
  const same = (on: string | null | undefined, named: string | null): boolean => on === undefined || on === named;
  return (
    same(shown.source, basis.source) &&
    same(shown.model, basis.model) &&
    same(shown.answers, basis.answers) &&
    same(shown.requirements, basis.requirements)
  );
}

/**
 * Whether the owner may press accept now, and if not, why. The core refuses what is
 * not current and what moved; this keeps the button from offering what it will refuse,
 * and from offering what the owner is not looking at. `shown` has no default: a caller
 * that does not say what is on the screen cannot be told the button may be offered.
 */
export function gate(state: AcceptanceState, unsaved: boolean, shown: Shown): Withheld | null {
  if (state.accepting) return "accepting";
  if (unsaved) return "unsaved";
  const report = state.report;
  if (report === null) return "not-measured";
  if (report.model_standing !== "current" || report.requirements_standing !== "current") return "behind";
  const notShown = whatIsNotShown(shown, report.basis);
  if (notShown !== null) return notShown;
  const accepted = state.acceptance;
  if (accepted.standing === "holds" && accepted.acceptance !== null && sameBasis(accepted.acceptance.basis, report.basis)) {
    return "already";
  }
  return null;
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
