// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the revision panel is in. The core says what a revision did, or why it cannot compare yet;
// what is decided here is only which state the panel is in and that a report is shown only for the
// revisions the screen is showing. The words and the shapes are the core's own, read from the
// replies it actually gives (`app-core/contract/replies.json`).

import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { asCommandError, parseReadRevisionReport, type Basis, type RevisionReport } from "../src/contract";
import { CommandFailure } from "../src/ipc";
import {
  comparedNothing,
  countOf,
  panelOfFailure,
  panelOfRead,
  reportIsOf,
  stillShown,
  verdictOf,
  type RevisionPanel,
} from "../src/revision_model";

const replies = JSON.parse(
  readFileSync(new URL("../../../app-core/contract/replies.json", import.meta.url), "utf8"),
) as { answers: Record<string, unknown>; refusals: Record<string, unknown> };

const read = parseReadRevisionReport(replies.answers["read_revision_report"]);
const report = (): RevisionReport => {
  if (read.report === null) throw new Error("the contract file has a report");
  return read.report;
};

const explain = (error: unknown): string => `explained: ${String(error)}`;

/** The refusal the core gave, as the failure a call to it throws. */
function failureOf(name: string): CommandFailure {
  const body = asCommandError(replies.refusals[name]);
  if (body === null) throw new Error(`the contract file has no refusal ${name}`);
  return new CommandFailure(body.kind, body.message, body.detail);
}

describe("the revision panel", () => {
  it("has no panel for a work nobody accepted, and shows the report of one that did", () => {
    expect(panelOfRead(parseReadRevisionReport(replies.answers["read_revision_report_none"]))).toBeNull();
    expect(panelOfRead(read)).toEqual({ phase: "read", report: report() });
  });

  it("holds a refusal the core gave in a sentence as it is, in the core's own words", () => {
    for (const kind of ["revision-not-current", "revision-not-judged"]) {
      const failure = failureOf(kind);
      expect(panelOfFailure(failure, explain)).toEqual({ phase: "held", kind, message: failure.message });
    }
    // The sentence says what is behind and what to do: the screen adds nothing to it.
    expect(failureOf("revision-not-current").message).toContain("generate the requirement list and the model");
  });

  it("makes any other failure the panel's own message, and never a held comparison", () => {
    const other = new CommandFailure("transport", "the server did not answer");
    expect(panelOfFailure(other, explain)).toEqual({ phase: "failed", message: explain(other) });
    expect(panelOfFailure(new Error("broken"), explain).phase).toBe("failed");
    // A refusal of another command is not this panel's to hold.
    expect(panelOfFailure(failureOf("invalid-model"), explain).phase).toBe("failed");
  });

  it("takes a report to be of the revisions it names and of no others", () => {
    const shown: Basis = report().of.now;
    expect(reportIsOf(report(), shown)).toBe(true);
    for (const part of ["source", "model", "requirements"] as const) {
      expect(reportIsOf(report(), { ...shown, [part]: "f".repeat(64) })).toBe(false);
    }
  });

  it("keeps the last report only while it is of what the screen shows", () => {
    const shown = report().of.now;
    const panel: RevisionPanel = { phase: "read", report: report() };
    expect(stillShown(panel, shown)).toBe(panel);
    expect(stillShown(panel, { ...shown, model: "f".repeat(64) })).toEqual({ phase: "reading" });
    // What is not a report is not about any revisions: it stays as it was.
    expect(stillShown(null, shown)).toBeNull();
    const held: RevisionPanel = { phase: "held", kind: "revision-not-current", message: "m" };
    expect(stillShown(held, shown)).toBe(held);
  });

  it("says a report compared nothing only when the product's own count of what it saw is zero", () => {
    expect(comparedNothing(report())).toBe(false);
    expect(comparedNothing({ ...report(), summary: { ...report().summary, seen: 0 } })).toBe(true);
    // A count the product did not give is not a zero: nothing is guessed from its absence.
    const { seen: _seen, ...without } = report().summary as Record<string, unknown>;
    expect(comparedNothing({ ...report(), summary: without })).toBe(false);
  });

  it("reads only the numbers the product gave, and a verdict only in its two words", () => {
    const summary = report().summary;
    expect(countOf(summary, "requirements")).toBe(5);
    expect(countOf(summary, "violations")).toBe(0);
    expect(countOf(summary, "no-such-count")).toBeNull();
    expect(countOf({ look: "1" }, "look")).toBeNull();
    expect(countOf({ look: Number.NaN }, "look")).toBeNull();
    expect(verdictOf(report())).toBe("within-reach");
    expect(verdictOf({ ...report(), verdict: "outside-reach" })).toBe("outside-reach");
    expect(verdictOf({ ...report(), verdict: "perhaps" })).toBeNull();
  });
});
