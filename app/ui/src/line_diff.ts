// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What changed between two pages of pseudocode, line by line.
//
// The page SCE writes of a model is the reading the owner reviews, and the lines of one page
// against the other are what a person compares. This is the plain difference of the two
// (the longest run of lines they share is kept, everything else is added or removed) and the
// screen shows only that, with a few lines around each change so that it has its context. It
// says nothing of what a change MEANS: a condition, a signal, a value or a time that moved shows
// up as the lines that hold it, and the owner reads them.

/** One line of the difference. */
export interface Line {
  readonly kind: "same" | "added" | "removed";
  readonly text: string;
}

/** The difference of `before` and `after`, in the order of the lines. */
export function diffLines(before: string, after: string): Line[] {
  const a = splitLines(before);
  const b = splitLines(after);
  // The longest common subsequence of the lines, by the classic table: pages are tens of lines, so
  // the quadratic table is small, and nothing cleverer is worth being wrong.
  const n = a.length;
  const m = b.length;
  const table: number[][] = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      const row = table[i] as number[];
      row[j] =
        a[i] === b[j]
          ? ((table[i + 1] as number[])[j + 1] as number) + 1
          : Math.max((table[i + 1] as number[])[j] as number, row[j + 1] as number);
    }
  }
  const lines: Line[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      lines.push({ kind: "same", text: a[i] as string });
      i += 1;
      j += 1;
    } else if (((table[i + 1] as number[])[j] as number) >= ((table[i] as number[])[j + 1] as number)) {
      lines.push({ kind: "removed", text: a[i] as string });
      i += 1;
    } else {
      lines.push({ kind: "added", text: b[j] as string });
      j += 1;
    }
  }
  for (; i < n; i += 1) lines.push({ kind: "removed", text: a[i] as string });
  for (; j < m; j += 1) lines.push({ kind: "added", text: b[j] as string });
  return lines;
}

/** How many lines were added and removed. */
export function tallyOf(lines: readonly Line[]): { readonly added: number; readonly removed: number } {
  return {
    added: lines.filter((l) => l.kind === "added").length,
    removed: lines.filter((l) => l.kind === "removed").length,
  };
}

/** A run of the difference worth showing: the changes, and the lines around them. */
export type Hunk = readonly Line[];

/**
 * The parts of a difference that hold a change, each with `context` lines of what did not change
 * on either side. Runs that would touch or overlap are one hunk. Nothing at all for a difference
 * with no change in it.
 */
export function hunksOf(lines: readonly Line[], context = 2): Hunk[] {
  const changed = lines.map((l, index) => (l.kind === "same" ? -1 : index)).filter((index) => index >= 0);
  if (changed.length === 0) return [];
  const hunks: Line[][] = [];
  let from = -1;
  let to = -1;
  for (const index of changed) {
    const start = Math.max(0, index - context);
    const end = Math.min(lines.length - 1, index + context);
    if (from === -1) {
      from = start;
      to = end;
    } else if (start <= to + 1) {
      to = Math.max(to, end);
    } else {
      hunks.push(lines.slice(from, to + 1));
      from = start;
      to = end;
    }
  }
  hunks.push(lines.slice(from, to + 1));
  return hunks;
}

/** A page's lines, without the empty one a final newline leaves. */
function splitLines(page: string): string[] {
  const lines = page.split("\n");
  if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}
