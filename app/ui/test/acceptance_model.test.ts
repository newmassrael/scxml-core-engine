// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// When the accept button is offered, and what the owner is told before pressing it.
// SCE classifies the requirements and the core refuses what moved; what is decided
// here is only whether to offer the button, so each way of withholding it is a case.

import { describe, expect, it } from "vitest";

import {
  acceptRefused,
  accepting,
  basisOf,
  gate,
  isUnsettled,
  OUTCOME_WORDS,
  panelIsOf,
  panelOf,
  standingIsOfWhatIsShown,
  tally,
  type AcceptanceState,
  type Shown,
} from "../src/acceptance_model";
import type {
  AcceptanceRecord,
  Basis,
  Judgment,
  RequirementOutcome,
  RequirementsReport,
  WorkSnapshot,
} from "../src/contract";

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
  basis,
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

const record: AcceptanceRecord = { revision: hex(9), accepted_at: "t", channel: "direct", basis, open: [] };

/** A work with a text, a design and a list, and nothing accepted, read as one state. */
const snapshot = (over: Partial<WorkSnapshot> = {}): WorkSnapshot => ({
  work: { id: "door", title: "Door lock", created_at: "t" },
  source: { revision: basis.source, text: "The door opens." },
  model: { revision: basis.model, written_for: basis.source, text: "<scxml/>", entry: "model.scxml", documents: [] },
  model_standing: "current",
  answers: null,
  requirements: { revision: basis.requirements, written_for: basis.source, manifest: "{}", sidecar: "{}" },
  requirements_standing: "current",
  acceptance: null,
  bundle: null,
  ...over,
});

/** What SCE said of the revisions `snapshot()` holds. */
const judgment = (over: Partial<Judgment> = {}): Judgment => ({
  basis,
  acceptance: null,
  report: { said: true, value: report() },
  ...over,
});

describe("the revisions SCE is asked about", () => {
  it("are those of the text, the design, the list and the answers a snapshot holds", () => {
    expect(basisOf(snapshot())).toEqual(basis);
    const answered = snapshot({ answers: { revision: hex(4), entries: {} } });
    expect(basisOf(answered)).toEqual({ ...basis, answers: hex(4) });
  });

  it("are none for a work with no text, no design or no list, since there is nothing to measure", () => {
    expect(basisOf(snapshot({ source: null }))).toBeNull();
    expect(basisOf(snapshot({ model: null, model_standing: null }))).toBeNull();
    expect(basisOf(snapshot({ requirements: null, requirements_standing: null }))).toBeNull();
  });
});

describe("the panel beside a snapshot", () => {
  it("has no list to accept a design against for a work with none", () => {
    expect(panelOf(snapshot({ requirements: null, requirements_standing: null }), judgment())).toEqual({
      phase: "no-list",
    });
  });

  it("is of the snapshot: its list, the measure SCE gave, and no acceptance when none was made", () => {
    const read = snapshot();
    const panel = panelOf(read, judgment());
    expect(panel).toEqual({
      phase: "read",
      state: {
        basis,
        list: { requirements: read.requirements, source_head: basis.source, standing: "current" },
        acceptance: { acceptance: null, standing: "none", lapse: null, now: null },
        report: report(),
        measureFailure: null,
        accepting: false,
        refusal: null,
      },
    });
  });

  it("carries the acceptance, whether SCE says it holds, and the revisions it was judged of", () => {
    const read = snapshot({ acceptance: record });
    const holds = panelOf(read, judgment({ acceptance: { said: true, value: { standing: "holds", lapse: null } } }));
    expect(holds).toMatchObject({
      state: { acceptance: { acceptance: record, standing: "holds", lapse: null, now: basis } },
    });
    const lapsed = panelOf(
      read,
      judgment({ acceptance: { said: true, value: { standing: "lapsed", lapse: "design/model.scxml moved" } } }),
    );
    expect(lapsed).toMatchObject({
      state: { acceptance: { acceptance: record, standing: "lapsed", lapse: "design/model.scxml moved" } },
    });
  });

  it("says SCE did not measure, and still shows an acceptance the owner made", () => {
    const refusal = { kind: "sce-timeout", message: "SCE took too long", code: null };
    const panel = panelOf(
      snapshot({ acceptance: record }),
      judgment({
        acceptance: { said: true, value: { standing: "holds", lapse: null } },
        report: { said: false, refusal },
      }),
    );
    expect(panel).toMatchObject({
      phase: "read",
      state: { report: null, measureFailure: "SCE took too long", acceptance: { acceptance: record } },
    });
  });

  it("is a failure when SCE did not say whether the acceptance holds, which is not guessed at", () => {
    const refusal = { kind: "sce-unavailable", message: "no generator", code: null };
    const panel = panelOf(snapshot({ acceptance: record }), judgment({ acceptance: { said: false, refusal } }));
    expect(panel).toEqual({ phase: "failed", message: "no generator" });
  });

  it("refuses a verdict that was not asked for, or one that was asked for nothing", () => {
    // Shown as 'nothing accepted' it would be a false statement about what the owner has done.
    expect(() => panelOf(snapshot({ acceptance: record }), judgment())).toThrow(/judgment\.acceptance/);
    expect(() =>
      panelOf(snapshot(), judgment({ acceptance: { said: true, value: { standing: "holds", lapse: null } } })),
    ).toThrow(/judgment\.acceptance/);
  });
});

describe("a panel and the snapshot it is shown beside", () => {
  const panel = (read: WorkSnapshot, said: Judgment): AcceptanceState => {
    const made = panelOf(read, said);
    if (made.phase !== "read") throw new Error("a panel was expected");
    return made.state;
  };

  it("are of one work when they name the same revisions and the same acceptance", () => {
    expect(panelIsOf(panel(snapshot(), judgment()), snapshot())).toBe(true);
  });

  it("are not when the design, the text, the list or the answers moved, whatever the acceptance says", () => {
    const state = panel(snapshot(), judgment());
    expect(panelIsOf(state, snapshot({ model: { ...snapshot().model!, revision: hex(8) } }))).toBe(false);
    expect(panelIsOf(state, snapshot({ source: { revision: hex(8), text: "x" } }))).toBe(false);
    expect(panelIsOf(state, snapshot({ requirements: { ...snapshot().requirements!, revision: hex(8) } }))).toBe(false);
    expect(panelIsOf(state, snapshot({ answers: { revision: hex(4), entries: {} } }))).toBe(false);
  });

  it("are not when an acceptance was made or replaced, which no revision of the work names", () => {
    const state = panel(snapshot(), judgment());
    expect(panelIsOf(state, snapshot({ acceptance: record }))).toBe(false);
    const accepted = panel(
      snapshot({ acceptance: record }),
      judgment({ acceptance: { said: true, value: { standing: "holds", lapse: null } } }),
    );
    expect(panelIsOf(accepted, snapshot({ acceptance: record }))).toBe(true);
    expect(panelIsOf(accepted, snapshot({ acceptance: { ...record, revision: hex(7) } }))).toBe(false);
    expect(panelIsOf(accepted, snapshot())).toBe(false);
  });

  it("are not when the work has nothing to measure now", () => {
    expect(panelIsOf(panel(snapshot(), judgment()), snapshot({ requirements: null, requirements_standing: null }))).toBe(
      false,
    );
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

  it("says a standing of the work on screen only when every part the screen has is the one judged", () => {
    // The panel is of the snapshot the design was read in; what can differ is the text and the
    // answers, which the person edits and which are read apart.
    const holds = state({ acceptance: { acceptance: record, standing: "holds", lapse: null, now: basis } });
    expect(standingIsOfWhatIsShown(holds, shown)).toBe(true);
    for (const part of ["source", "model", "answers", "requirements"] as const) {
      expect(standingIsOfWhatIsShown(holds, { ...shown, [part]: hex(8) })).toBe(false);
    }
    // A part still being read is not held against it; it is the gate that waits for what is unread.
    expect(standingIsOfWhatIsShown(holds, { ...shown, model: undefined })).toBe(true);
    // Where there is no acceptance there is no standing to be of another work.
    expect(standingIsOfWhatIsShown(state(), { ...shown, model: hex(8) })).toBe(true);
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
