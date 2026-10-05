// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import type { RequirementOutcome } from "../src/contract";
import { containsPath, findQuote, groundOf, linesNaming, quotesOf, statesOf } from "../src/grounding_model";

const outcome = (id: string, node_paths: string[], section: string | null = null): RequirementOutcome => ({
  id,
  outcome: "implemented",
  section,
  node_paths,
});

const QUOTES = {
  R1: "The door is closed until somebody presents a card.",
  R4: "the controller counts three misses in a row, after which it ignores every card for one minute",
};

describe("the sentences a list quotes", () => {
  it("are what the sidecar holds, by requirement", () => {
    const sidecar = JSON.stringify({ doc_id: "door", rev: "1", text: QUOTES });
    expect(quotesOf(sidecar)).toEqual(QUOTES);
  });

  it("are nothing for a list that has no sidecar, or one that is not what it should be", () => {
    expect(quotesOf(null)).toEqual({});
    expect(quotesOf("not json")).toEqual({});
    expect(quotesOf("[]")).toEqual({});
    expect(quotesOf(JSON.stringify({ text: "x" }))).toEqual({});
    expect(quotesOf(JSON.stringify({ text: { R1: 3, R2: "two" } }))).toEqual({ R2: "two" });
  });
});

describe("a part of the design that contains another", () => {
  it("is the same path, or one that continues it at a segment or an index", () => {
    expect(containsPath("states.a", "states.a")).toBe(true);
    expect(containsPath("states.a", "states.a.transitions[0]")).toBe(true);
    expect(containsPath("states.a", "states.a[0]")).toBe(true);
  });

  it("is not a path that only starts with the same letters", () => {
    expect(containsPath("states.a", "states.ab")).toBe(false);
    expect(containsPath("states.a", "states")).toBe(false);
    expect(containsPath("", "states.a")).toBe(false);
  });
});

describe("the requirement a question is about", () => {
  const outcomes = [
    outcome("R1", ["states.closed"], "S2"),
    outcome("R4", ["states.closed.states.lockedOut"], "S3"),
    outcome("R9", []),
  ];

  it("is the one whose part of the design contains the place the question was asked", () => {
    expect(groundOf("states.closed.states.ready.transitions[0]", outcomes, QUOTES)).toEqual({
      requirement: "R1",
      quote: QUOTES.R1,
      section: "S2",
    });
  });

  it("is the most specific one when several contain it", () => {
    expect(groundOf("states.closed.states.lockedOut.transitions[1]", outcomes, QUOTES)).toEqual({
      requirement: "R4",
      quote: QUOTES.R4,
      section: "S3",
    });
  });

  it("has no sentence when the list quotes none for it, and still names the requirement", () => {
    expect(groundOf("states.closed.states.lockedOut", outcomes, {})).toEqual({
      requirement: "R4",
      quote: null,
      section: "S3",
    });
  });

  it("is nothing when no requirement carries the place, or the question says no place", () => {
    expect(groundOf("states.opened.transitions[0]", outcomes, QUOTES)).toBeNull();
    expect(groundOf(null, outcomes, QUOTES)).toBeNull();
    expect(groundOf("", outcomes, QUOTES)).toBeNull();
    expect(groundOf("states.closed", [], QUOTES)).toBeNull();
  });
});

describe("the states a requirement is carried by", () => {
  it("are the names that follow a states segment, in the order they are first named", () => {
    expect(statesOf(["states.closed.states.lockedOut.transitions[1]", "states.opened"])).toEqual([
      "closed",
      "lockedOut",
      "opened",
    ]);
    expect(statesOf(["states.closed", "states.closed.transitions[0]"])).toEqual(["closed"]);
  });

  it("are none for places that name no state", () => {
    expect(statesOf([])).toEqual([]);
    expect(statesOf(["datamodel.misses", "transitions[0]"])).toEqual([]);
  });
});

describe("the lines of a page that name a state", () => {
  const page = [
    "machine door (lexicon: en)",
    "  state closed:",
    "    on open   -> opened",
    "  state lockedOutAgain:",
    "  state lockedOut:",
    "    on lockout.end -> closed",
  ].join("\n");

  it("are those that hold the name as a whole word", () => {
    expect(linesNaming(page, ["lockedOut"])).toEqual([4]);
    expect(linesNaming(page, ["closed"])).toEqual([1, 5]);
    expect(linesNaming(page, ["opened", "lockedOut"])).toEqual([2, 4]);
  });

  it("are none for no state, or one the page does not name", () => {
    expect(linesNaming(page, [])).toEqual([]);
    expect(linesNaming(page, ["missing"])).toEqual([]);
  });

  it("read a state's name as a name and not as a pattern", () => {
    expect(linesNaming("state a.b:\nstate axb:", ["a.b"])).toEqual([0]);
  });
});

describe("where a quoted sentence is in the text", () => {
  const text = "A door controller.\nThe door is closed until\nsomebody presents a card.\nIt opens.";

  it("is where it is, exactly", () => {
    expect(findQuote("It opens.", text)).toEqual({ start: text.indexOf("It opens."), end: text.length });
  });

  it("is where it is when the text broke the sentence across lines", () => {
    const place = findQuote("The door is closed until somebody presents a card.", text);
    expect(place).not.toBeNull();
    expect(text.slice(place?.start, place?.end)).toBe("The door is closed until\nsomebody presents a card.");
  });

  it("is nowhere for a sentence the text does not hold, or an empty one", () => {
    expect(findQuote("The window opens.", text)).toBeNull();
    expect(findQuote("   ", text)).toBeNull();
  });

  it("treats the sentence as words, not as a pattern", () => {
    expect(findQuote("(a+b)?", "no (a+b)? here")).toEqual({ start: 3, end: 9 });
    expect(findQuote("a.c", "abc a.c")).toEqual({ start: 4, end: 7 });
  });
});
