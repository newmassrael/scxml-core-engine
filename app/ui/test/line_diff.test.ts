// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import { diffLines, hunksOf, tallyOf, type Line } from "../src/line_diff";

const kinds = (lines: readonly Line[]): string[] =>
  lines.map((l) => `${l.kind === "same" ? " " : l.kind === "added" ? "+" : "-"}${l.text}`);

describe("what changed between two pages", () => {
  it("is nothing for the same page", () => {
    const page = "machine door\n  state closed:\n";
    expect(kinds(diffLines(page, page))).toEqual([" machine door", "   state closed:"]);
    expect(tallyOf(diffLines(page, page))).toEqual({ added: 0, removed: 0 });
  });

  it("is the line that was added, and the line that was removed", () => {
    const before = "a\nb\nc\n";
    expect(kinds(diffLines(before, "a\nb\nx\nc\n"))).toEqual([" a", " b", "+x", " c"]);
    expect(kinds(diffLines(before, "a\nc\n"))).toEqual([" a", "-b", " c"]);
  });

  it("is a removal and an addition for a line that was changed", () => {
    expect(kinds(diffLines("a\n    on open -> opened\nc\n", "a\n    on open -> locked\nc\n"))).toEqual([
      " a",
      "-    on open -> opened",
      "+    on open -> locked",
      " c",
    ]);
  });

  it("keeps the longest run of lines the pages share", () => {
    expect(kinds(diffLines("a\nb\nc\nd\n", "b\nc\nx\nd\n"))).toEqual(["-a", " b", " c", "+x", " d"]);
  });

  it("is every line added for a page that had none, and every line removed for one that has none", () => {
    expect(kinds(diffLines("", "a\nb\n"))).toEqual(["+a", "+b"]);
    expect(kinds(diffLines("a\nb\n", ""))).toEqual(["-a", "-b"]);
    expect(diffLines("", "")).toEqual([]);
  });

  it("does not take a final newline for a line", () => {
    expect(kinds(diffLines("a", "a\n"))).toEqual([" a"]);
  });

  it("counts what was added and removed", () => {
    expect(tallyOf(diffLines("a\nb\nc\n", "a\nx\ny\n"))).toEqual({ added: 2, removed: 2 });
  });
});

describe("the parts worth showing", () => {
  const lines = (text: string): Line[] => diffLines("", "").concat(parse(text));

  /** `=` same, `+` added, `-` removed, one per line, to write a difference by hand. */
  function parse(text: string): Line[] {
    return text.split(/\s+/).filter(Boolean).map((token, i) => ({
      kind: token[0] === "+" ? "added" : token[0] === "-" ? "removed" : "same",
      text: `${token.slice(1)}${i}`,
    }));
  }

  it("are none for a page that did not change", () => {
    expect(hunksOf(lines("=a =b =c"))).toEqual([]);
  });

  it("are the change with the lines around it", () => {
    const hunks = hunksOf(lines("=a =b =c =d +e =f =g =h =i"), 2);
    expect(hunks).toHaveLength(1);
    expect(hunks[0]?.map((l) => l.kind)).toEqual(["same", "same", "added", "same", "same"]);
  });

  it("are one when two changes are near enough that their lines meet", () => {
    const hunks = hunksOf(lines("=a +b =c =d =e +f =g"), 2);
    expect(hunks).toHaveLength(1);
    expect(hunks[0]).toHaveLength(7);
  });

  it("are two when what lies between the changes is more than they show", () => {
    const hunks = hunksOf(lines("+a =b =c =d =e =f =g =h +i"), 1);
    expect(hunks).toHaveLength(2);
    expect(hunks[0]?.map((l) => l.kind)).toEqual(["added", "same"]);
    expect(hunks[1]?.map((l) => l.kind)).toEqual(["same", "added"]);
  });
});
