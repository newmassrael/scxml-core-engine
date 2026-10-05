// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import { comparePages, previousOf } from "../src/change_model";
import type { HistoryEntry } from "../src/contract";

const rev = (n: number): string => n.toString(16).padStart(64, "0");
const entry = (revision: number, parent: number | null): HistoryEntry => ({
  revision: rev(revision),
  parent: parent === null ? null : rev(parent),
  saved_at: "2026-10-05T09:00:00Z",
});

describe("the model a model replaced", () => {
  it("is the one before it in the history", () => {
    const history = [entry(1, null), entry(2, 1), entry(3, 2)];
    expect(previousOf(history, rev(3))).toBe(rev(2));
    expect(previousOf(history, rev(2))).toBe(rev(1));
  });

  it("is nothing for the first model", () => {
    expect(previousOf([entry(1, null)], rev(1))).toBeNull();
  });

  it("is nothing for a model the history does not list", () => {
    expect(previousOf([entry(1, null)], rev(9))).toBeNull();
    expect(previousOf([], rev(1))).toBeNull();
  });

  it("is not the same model: one published again unchanged replaced the one before it", () => {
    const history = [entry(1, null), entry(2, 1), entry(2, 2)];
    expect(previousOf(history, rev(2))).toBe(rev(1));
  });

  it("is the one the first entry says it followed, when the history begins there", () => {
    // The first bundle of a work that had a model before: the earlier chain is not listed here.
    expect(previousOf([entry(5, 4)], rev(5))).toBe(rev(4));
    expect(previousOf([entry(5, 5)], rev(5))).toBeNull();
  });
});

describe("how a page compares with the one before it", () => {
  it("is unavailable when either page could not be had, and says which", () => {
    expect(comparePages(null, "a\n")).toEqual({ phase: "unavailable", reason: "before" });
    expect(comparePages("a\n", null)).toEqual({ phase: "unavailable", reason: "after" });
  });

  it("is unchanged for the same page", () => {
    expect(comparePages("a\nb\n", "a\nb\n")).toEqual({ phase: "unchanged" });
  });

  it("is the changed lines with their context, and how many", () => {
    const panel = comparePages("a\nb\nc\nd\ne\nf\n", "a\nb\nc\nX\ne\nf\n");
    expect(panel.phase).toBe("changed");
    if (panel.phase !== "changed") return;
    expect(panel.added).toBe(1);
    expect(panel.removed).toBe(1);
    expect(panel.hunks).toHaveLength(1);
    expect(panel.hunks[0]?.map((l) => `${l.kind}:${l.text}`)).toEqual([
      "same:b",
      "same:c",
      "removed:d",
      "added:X",
      "same:e",
      "same:f",
    ]);
  });
});
