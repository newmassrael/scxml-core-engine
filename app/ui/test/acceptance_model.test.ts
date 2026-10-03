// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// When the accept button is offered, and what the owner is told before pressing it.
// SCE classifies the requirements and the core refuses what moved; what is decided
// here is only whether to offer the button, so each way of withholding it is a case.

import { describe, expect, it } from "vitest";

import {
  acceptRefused,
  accepting,
  gate,
  isUnsettled,
  OUTCOME_WORDS,
  tally,
  type AcceptanceState,
} from "../src/acceptance_model";
import type { Basis, RequirementOutcome, RequirementsReport } from "../src/contract";

const hex = (n: number): string => n.toString(16).padStart(64, "0");

const basis: Basis = { source: hex(1), model: hex(2), requirements: hex(3), answers: null };

const outcome = (id: string, word: string): RequirementOutcome => ({ id, outcome: word, section: null, node_paths: [] });

const report = (over: Partial<RequirementsReport> = {}): RequirementsReport => ({
  basis,
  source_head: basis.source,
  model_standing: "current",
  requirements_standing: "current",
  generator: null,
  denominator: null,
  outcomes: [outcome("R1", "implemented")],
  page: null,
  page_refusal: null,
  ...over,
});

const state = (over: Partial<AcceptanceState> = {}): AcceptanceState => ({
  list: { requirements: null, source_head: null, standing: null },
  acceptance: { acceptance: null, standing: "none", lapse: null, now: null },
  report: report(),
  measureFailure: null,
  accepting: false,
  refusal: null,
  ...over,
});

describe("the accept button", () => {
  it("is offered for a measured design written for the text as it is", () => {
    expect(gate(state(), false)).toBeNull();
  });

  it("is not withheld for a gap: a design that misses a requirement is the owner's to accept", () => {
    const gapped = state({ report: report({ outcomes: [outcome("R1", "missing"), outcome("R2", "needs-scenario")] }) });
    expect(gate(gapped, false)).toBeNull();
  });

  it("is withheld while what is typed is not saved, because what is accepted is what is saved", () => {
    expect(gate(state(), true)).toBe("unsaved");
  });

  it("is withheld while an acceptance is on its way, whatever else is true", () => {
    expect(gate(accepting(state()), false)).toBe("accepting");
    expect(gate(accepting(state()), true)).toBe("accepting");
  });

  it("is withheld when SCE did not measure the design: the owner has not been shown what they would accept", () => {
    expect(gate(state({ report: null, measureFailure: "SCE took too long" }), false)).toBe("not-measured");
  });

  it("is withheld for a design or a list written for an earlier text, or for none that says", () => {
    expect(gate(state({ report: report({ model_standing: "behind" }) }), false)).toBe("behind");
    expect(gate(state({ report: report({ requirements_standing: "behind" }) }), false)).toBe("behind");
    expect(gate(state({ report: report({ model_standing: "unstated" }) }), false)).toBe("behind");
  });

  it("is withheld for exactly what is accepted and still holds, and offered again for anything else", () => {
    const record = { revision: hex(9), accepted_at: "t", channel: "direct", basis, open: [] };
    const holds = state({ acceptance: { acceptance: record, standing: "holds", lapse: null, now: basis } });
    expect(gate(holds, false)).toBe("already");

    const lapsed = state({
      acceptance: { acceptance: record, standing: "lapsed", lapse: "design/model.scxml moved", now: basis },
    });
    expect(gate(lapsed, false)).toBeNull();
    // Answers given since are a different basis, which holds no longer says.
    const answered = state({
      acceptance: { acceptance: record, standing: "holds", lapse: null, now: basis },
      report: report({ basis: { ...basis, answers: hex(4) } }),
    });
    expect(gate(answered, false)).toBeNull();
  });
});

describe("what SCE finds of the requirements", () => {
  it("is counted by SCE's own words, the known ones first in their order", () => {
    const outcomes = [
      outcome("R1", "missing"),
      outcome("R2", "implemented"),
      outcome("R3", "needs-scenario"),
      outcome("R4", "implemented"),
    ];
    expect(tally(outcomes)).toEqual([
      ["implemented", 2],
      ["missing", 1],
      ["needs-scenario", 1],
    ]);
  });

  it("keeps a word a later SCE writes as it was spelled, after the known ones", () => {
    const outcomes = [outcome("R1", "waived"), outcome("R2", "implemented"), outcome("R3", "deferred")];
    expect(tally(outcomes)).toEqual([
      ["implemented", 1],
      ["deferred", 1],
      ["waived", 1],
    ]);
    expect(tally([])).toEqual([]);
  });

  it("knows every word the product writes, what the design settles first", () => {
    // The eleven of `Outcome` in the product's requirement_manifest.rs: a word added there
    // and not here would be shown as spelled, never guessed at, and this list is where to say it.
    expect([...OUTCOME_WORDS].sort()).toEqual(
      [
        "contradicted",
        "dangling",
        "delegated",
        "implemented",
        "missing",
        "needs-scenario",
        "out-of-scope",
        "scenario-failed",
        "scenario-passed",
        "system-level",
        "unresolved",
      ],
    );
    expect(OUTCOME_WORDS.slice(0, 2)).toEqual(["implemented", "scenario-passed"]);
  });

  it("marks as unsettled what the design leaves open, never what it settles or the list places elsewhere", () => {
    for (const word of ["missing", "unresolved", "dangling", "contradicted", "scenario-failed", "needs-scenario"]) {
      expect(isUnsettled(word), word).toBe(true);
    }
    for (const word of ["implemented", "scenario-passed", "delegated", "out-of-scope", "system-level"]) {
      expect(isUnsettled(word), word).toBe(false);
    }
    // A word SCE adds later is not guessed at.
    expect(isUnsettled("waived")).toBe(false);
  });
});

describe("a refused acceptance", () => {
  it("keeps the core's words, and the button is offered again", () => {
    const refused = acceptRefused(accepting(state()), "model changed after you were shown it");
    expect(refused.refusal).toBe("model changed after you were shown it");
    expect(refused.accepting).toBe(false);
    expect(gate(refused, false)).toBeNull();
    // A new press clears the last refusal.
    expect(accepting(refused).refusal).toBeNull();
  });
});
