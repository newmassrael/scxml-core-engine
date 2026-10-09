// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the revision panel is in, and the few pure facts it needs.
//
// The screen judges nothing about a revision. The core says what each requirement's words did
// (carried, changed, new, retired) beside what SCE says the design's evidence did, and whether
// the revision stayed within the reach of what changed; and it says when it cannot compare yet
// (the design or the list was written for an earlier text, or the list says nothing of its
// words). What is decided HERE is only which of those the panel is in, and that a report is
// shown only for the revisions the screen is showing. Pure, like the acceptance model, so each
// state is a test.

import { sameBasis, type Basis, type ReadRevisionReport, type RevisionReport } from "./contract";
import { CommandFailure } from "./ipc";

/**
 * The refusals that mean "not compared, and here is why, in a sentence": the core's own words are
 * shown as they are. Any other failure is the panel's own message, as it is for the acceptance.
 */
const HELD_KINDS: readonly string[] = ["revision-not-current", "revision-not-judged"];

/** Where the panel is. A work nobody accepted has no panel at all. */
export type RevisionPanel =
  | { readonly phase: "reading" }
  /** The core cannot compare yet, and said why. */
  | { readonly phase: "held"; readonly kind: string; readonly message: string }
  /** The report could not be read at all. */
  | { readonly phase: "failed"; readonly message: string }
  | { readonly phase: "read"; readonly report: RevisionReport };

/** The panel an answer makes; `null` for a work nobody accepted. */
export function panelOfRead(read: ReadRevisionReport): RevisionPanel | null {
  return read.report === null ? null : { phase: "read", report: read.report };
}

/**
 * The panel a failure makes. A refusal the core gave in a sentence is held as it is; anything else
 * (the server unreachable, an answer in a shape this screen does not know) is the panel's own
 * message, in the words `explain` gives it.
 */
export function panelOfFailure(error: unknown, explain: (error: unknown) => string): RevisionPanel {
  if (error instanceof CommandFailure && HELD_KINDS.includes(error.kind)) {
    return { phase: "held", kind: error.kind, message: error.message };
  }
  return { phase: "failed", message: explain(error) };
}

/**
 * Whether a report is of the revisions the screen is showing. The report names the revisions it
 * compared; one that names others than the screen's is of a work that has moved since, and is not
 * shown as if it were of this one.
 */
export function reportIsOf(report: RevisionReport, shown: Basis): boolean {
  return sameBasis(report.of.now, shown);
}

/** What the panel keeps of its last answer while the screen asks again about a changed work. */
export function stillShown(panel: RevisionPanel | null, shown: Basis): RevisionPanel | null {
  if (panel === null || panel.phase !== "read") return panel;
  return reportIsOf(panel.report, shown) ? panel : { phase: "reading" };
}

/** A number the product put in the summary, or `null` when it is not one (never guessed at). */
export function countOf(summary: Readonly<Record<string, unknown>>, key: string): number | null {
  const value = summary[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

/** The verdicts the product gives; any other word is shown as it was said. */
export type Verdict = "within-reach" | "outside-reach";

export function verdictOf(report: RevisionReport): Verdict | null {
  return report.verdict === "within-reach" || report.verdict === "outside-reach" ? report.verdict : null;
}
