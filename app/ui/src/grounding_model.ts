// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Which sentence of the specification a question, or a requirement, is about.
//
// The screen guesses nothing from words. The product already says where in the design each
// requirement is carried (`RequirementOutcome.node_paths`, in SCE's path syntax) and the list
// keeps the sentence each requirement quotes (the sidecar). A question the model asks says where
// in the design it was asked (`Unresolved.node_path`, the same syntax). A question that was asked
// inside a part of the design a requirement is carried by is about that requirement's sentence:
// the most specific such part names it. That is all this does, and it says nothing when it cannot.

import type { RequirementOutcome } from "./contract";

/** The requirement a question or a place of the design belongs to, and the sentence it quotes. */
export interface Ground {
  readonly requirement: string;
  /** The sentence of the specification the requirement quotes; `null` for a list without a sidecar. */
  readonly quote: string | null;
  readonly section: string | null;
}

/** The sentences a list quotes, by requirement id: what a sidecar holds, or nothing for one that cannot be read. */
export function quotesOf(sidecar: string | null): Readonly<Record<string, string>> {
  if (sidecar === null) return {};
  try {
    const parsed: unknown = JSON.parse(sidecar);
    if (typeof parsed !== "object" || parsed === null) return {};
    const text = (parsed as Record<string, unknown>)["text"];
    if (typeof text !== "object" || text === null) return {};
    return Object.fromEntries(
      Object.entries(text as Record<string, unknown>).filter(
        (pair): pair is [string, string] => typeof pair[1] === "string",
      ),
    );
  } catch {
    return {};
  }
}

/**
 * Whether `prefix` names `path` or a part that contains it: the same path, or one that
 * continues it at a segment (`.state`) or an index (`[0]`). `states.a` contains `states.a.b` and
 * `states.a[0]`, and not `states.ab`.
 */
export function containsPath(prefix: string, path: string): boolean {
  if (prefix === "") return false;
  if (path === prefix) return true;
  return path.startsWith(prefix) && (path[prefix.length] === "." || path[prefix.length] === "[");
}

/**
 * The requirement that carries the part of the design at `nodePath`, with the sentence it quotes.
 * Of the requirements whose carrying place contains it, the most specific (the longest) wins, so a
 * question asked inside a state is about the requirement of that state and not of the machine that
 * contains it. `null` when no requirement carries it.
 */
export function groundOf(
  nodePath: string | null,
  outcomes: readonly RequirementOutcome[],
  quotes: Readonly<Record<string, string>>,
): Ground | null {
  if (nodePath === null || nodePath === "") return null;
  let best: { outcome: RequirementOutcome; length: number } | null = null;
  for (const outcome of outcomes) {
    for (const carried of outcome.node_paths) {
      if (!containsPath(carried, nodePath)) continue;
      if (best === null || carried.length > best.length) best = { outcome, length: carried.length };
    }
  }
  if (best === null) return null;
  return {
    requirement: best.outcome.id,
    quote: quotes[best.outcome.id] ?? null,
    section: best.outcome.section,
  };
}

/**
 * The states a requirement is carried by, as the paths of its places name them: each name that
 * follows a `states.` segment (`states.closed.states.lockedOut.transitions[1]` names `closed` and
 * `lockedOut`). In the order they are first named.
 */
export function statesOf(nodePaths: readonly string[]): string[] {
  const names: string[] = [];
  for (const path of nodePaths) {
    for (const found of path.matchAll(/(?:^|\.)states\.([^.[\]]+)/g)) {
      const name = found[1];
      if (name !== undefined && !names.includes(name)) names.push(name);
    }
  }
  return names;
}

/**
 * The lines of a page that name one of `states`, as the whole word: `lockedOut` is named by
 * `state lockedOut:` and not by `lockedOutAgain`. The page is the product's own rendering and
 * carries no address of the part of the design a line is for, so a state's name is what ties a
 * line to a place, and the screen says that is what it did.
 */
export function linesNaming(page: string, states: readonly string[]): number[] {
  if (states.length === 0) return [];
  const escaped = states.map((s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const named = new RegExp(`(?<![\\w-])(?:${escaped.join("|")})(?![\\w-])`);
  const lines: number[] = [];
  page.split("\n").forEach((line, index) => {
    if (named.test(line)) lines.push(index);
  });
  return lines;
}

/** Where a quoted sentence is in the text, as a range, or `null` when it is not there. */
export interface Place {
  readonly start: number;
  readonly end: number;
}

/**
 * Find `quote` in `text`. Exactly where it is, and else where it is when each run of white space
 * is one space: a sentence broken across lines in the text is the same sentence.
 */
export function findQuote(quote: string, text: string): Place | null {
  const wanted = quote.trim();
  if (wanted === "") return null;
  const exact = text.indexOf(wanted);
  if (exact >= 0) return { start: exact, end: exact + wanted.length };
  // Match with runs of white space equal: the pattern is the quote's words, apart by any white space.
  const words = wanted.split(/\s+/).map((w) => w.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const found = new RegExp(words.join("\\s+")).exec(text);
  return found === null ? null : { start: found.index, end: found.index + found[0].length };
}
