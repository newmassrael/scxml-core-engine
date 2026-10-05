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
  listIsTheOneMeasured,
  OUTCOME_WORDS,
  readsAgree,
  standingIsOfWhatWasMeasured,
  tally,
  type AcceptanceState,
  type Shown,
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

// What the screen shows when it shows what `report()` measured: the text, the design, the list, and no answers.
const shown: Shown = { source: basis.source, model: basis.model, answers: null, requirements: basis.requirements };

describe("a requirement list and SCE's measure of it", () => {
  const listed = (revision: string): AcceptanceState["list"] => ({
    requirements: { revision, written_for: basis.source, manifest: "{}", sidecar: null },
    source_head: basis.source,
    standing: "current",
  });

  it("are the same list when the measure's basis names the revision that was read", () => {
    expect(listIsTheOneMeasured(state({ list: listed(basis.requirements) }))).toBe(true);
  });

  it("are not when a list was saved between reading it and measuring it", () => {
    expect(listIsTheOneMeasured(state({ list: listed(hex(8)) }))).toBe(false);
  });

  it("say nothing against a work with no list, or a design SCE did not measure", () => {
    expect(listIsTheOneMeasured(state())).toBe(true);
    expect(listIsTheOneMeasured(state({ list: listed(hex(8)), report: null, measureFailure: "too slow" }))).toBe(true);
  });
});

describe("the accept button", () => {
  it("is offered for a measured design written for the text as it is", () => {
    expect(gate(state(), false, shown)).toBeNull();
  });

  it("is not withheld for a gap: a design that misses a requirement is the owner's to accept", () => {
    const gapped = state({ report: report({ outcomes: [outcome("R1", "missing"), outcome("R2", "needs-scenario")] }) });
    expect(gate(gapped, false, shown)).toBeNull();
  });

  it("is withheld while what is typed is not saved, because what is accepted is what is saved", () => {
    expect(gate(state(), true, shown)).toBe("unsaved");
  });

  it("is withheld while an acceptance is on its way, whatever else is true", () => {
    expect(gate(accepting(state()), false, shown)).toBe("accepting");
    expect(gate(accepting(state()), true, shown)).toBe("accepting");
  });

  it("is withheld when SCE did not measure the design: the owner has not been shown what they would accept", () => {
    expect(gate(state({ report: null, measureFailure: "SCE took too long" }), false, shown)).toBe("not-measured");
  });

  it("is withheld for a design or a list written for an earlier text, or for none that says", () => {
    expect(gate(state({ report: report({ model_standing: "behind" }) }), false, shown)).toBe("behind");
    expect(gate(state({ report: report({ requirements_standing: "behind" }) }), false, shown)).toBe("behind");
    expect(gate(state({ report: report({ model_standing: "unstated" }) }), false, shown)).toBe("behind");
  });

  it("is withheld for exactly what is accepted and still holds, and offered again for anything else", () => {
    const record = { revision: hex(9), accepted_at: "t", channel: "direct", basis, open: [] };
    const holds = state({ acceptance: { acceptance: record, standing: "holds", lapse: null, now: basis } });
    expect(gate(holds, false, shown)).toBe("already");

    const lapsed = state({
      acceptance: { acceptance: record, standing: "lapsed", lapse: "design/model.scxml moved", now: basis },
    });
    expect(gate(lapsed, false, shown)).toBeNull();
    // Answers given since are a different basis, which holds no longer says; the screen shows them.
    // `now` is the work as it is, which is what the report measured: both name the answers.
    const answered = state({
      acceptance: { acceptance: record, standing: "holds", lapse: null, now: { ...basis, answers: hex(4) } },
      report: report({ basis: { ...basis, answers: hex(4) } }),
    });
    expect(gate(answered, false, { ...shown, answers: hex(4) })).toBeNull();
  });

  it("is withheld while the standing was judged of another state of the work than the one measured", () => {
    // `read_acceptance` and the report are asked for apart, and the model saved between the two
    // leaves a "holds" beside a design it was not judged of: whether this design is accepted already
    // is not said, and the button waits for a pair that agrees.
    const record = { revision: hex(9), accepted_at: "t", channel: "direct", basis, open: [] };
    const apart = state({
      acceptance: { acceptance: record, standing: "holds", lapse: null, now: { ...basis, model: hex(8) } },
    });
    expect(gate(apart, false, shown)).toBe("unread");
    expect(standingIsOfWhatWasMeasured(apart)).toBe(false);
    expect(readsAgree(apart)).toBe(false);
    // Where there is no acceptance there is no standing to be of another state.
    expect(standingIsOfWhatWasMeasured(state())).toBe(true);
    expect(readsAgree(state())).toBe(true);
  });

  it("is withheld while what the screen shows is not what the report measured, whichever part differs", () => {
    expect(gate(state(), false, { ...shown, source: hex(8) })).toBe("differs");
    expect(gate(state(), false, { ...shown, model: hex(8) })).toBe("differs");
    // Answers on screen that are not the ones measured, and answers measured that are not on screen.
    expect(gate(state(), false, { ...shown, answers: hex(8) })).toBe("differs");
    expect(gate(state({ report: report({ basis: { ...basis, answers: hex(4) } }) }), false, shown)).toBe("differs");
    // The list the screen read is not the list SCE measured: its sentences are not the ones the outcomes are about.
    expect(gate(state(), false, { ...shown, requirements: hex(8) })).toBe("differs");
    expect(gate(state({ report: report({ basis: { ...basis, requirements: hex(8) } }) }), false, shown)).toBe(
      "differs",
    );
  });

  it("is withheld for a part that is not on the screen, which is not a part that matches", () => {
    for (const part of ["source", "model", "answers", "requirements"] as const) {
      expect(gate(state(), false, { ...shown, [part]: undefined })).toBe("unread");
    }
    expect(
      gate(state(), false, { source: undefined, model: undefined, answers: undefined, requirements: undefined }),
    ).toBe("unread");
    // Not even when the report measured no answers: the screen has not said there are none.
    expect(gate(state(), false, { ...shown, answers: undefined })).toBe("unread");
  });

  it("takes 'no answers saved' on the screen and 'no answers' in the report for the same thing", () => {
    expect(gate(state(), false, { ...shown, answers: null })).toBeNull();
  });

  it("says a design for an earlier text is behind before it says what is not on the screen", () => {
    expect(gate(state({ report: report({ model_standing: "behind" }) }), false, { ...shown, source: hex(8) })).toBe(
      "behind",
    );
    expect(
      gate(state({ report: report({ model_standing: "behind" }) }), false, { ...shown, answers: undefined }),
    ).toBe("behind");
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
    expect(gate(refused, false, shown)).toBeNull();
    // A new press clears the last refusal.
    expect(accepting(refused).refusal).toBeNull();
  });
});
