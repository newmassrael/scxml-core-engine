// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the command layer answers, and the guards that check an answer is that.
//
// The Rust side is the authority (`app-core/src/commands.rs`). An answer is not
// trusted because the screen was written against it: it is checked as it comes in,
// so a screen and a core of different versions fail with a sentence that says so
// instead of with `undefined` somewhere in a view. Extra fields are tolerated (a
// later core may add some); missing or mistyped ones are not.

/** The command set this screen was written for (`COMMAND_SET_VERSION` in the core). */
export const SUPPORTED_COMMAND_SET_VERSION = 1;

/** A revision: the SHA-256 of a saved text, as 64 lowercase hex digits. */
export type Revision = string;

export interface Work {
  readonly id: string;
  readonly title: string;
  readonly created_at: string;
}

export interface Unreadable {
  readonly id: string;
  readonly reason: string;
}

export interface Listing {
  readonly works: readonly Work[];
  readonly unreadable: readonly Unreadable[];
}

export interface SourceText {
  readonly revision: Revision;
  readonly text: string;
}

export interface HistoryEntry {
  readonly revision: Revision;
  readonly parent: Revision | null;
  readonly saved_at: string;
}

export type Saved =
  | { readonly outcome: "saved"; readonly revision: Revision; readonly parent: Revision | null }
  | { readonly outcome: "unchanged"; readonly revision: Revision };

export interface Described {
  readonly command_set_version: number;
  readonly commands: readonly string[];
  readonly root: string;
}

export interface WorkAndHead {
  readonly work: Work;
  readonly head: Revision | null;
}

/** A command that did not do what was asked, as the core words it. */
export interface CommandErrorBody {
  readonly kind: string;
  readonly message: string;
  readonly detail?: unknown;
}

/** An answer that is not the shape the core promises. */
export class ContractError extends Error {
  constructor(where: string, expected: string) {
    super(`${where}: expected ${expected}`);
    this.name = "ContractError";
  }
}

const REVISION = /^[0-9a-f]{64}$/;

type Obj = Record<string, unknown>;

function record(value: unknown, where: string): Obj {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ContractError(where, "an object");
  }
  return value as Obj;
}

function text(value: Obj, key: string, where: string): string {
  const field = value[key];
  if (typeof field !== "string") throw new ContractError(`${where}.${key}`, "a string");
  return field;
}

function revision(value: unknown, where: string): Revision {
  if (typeof value !== "string" || !REVISION.test(value)) {
    throw new ContractError(where, "a revision (64 lowercase hex digits)");
  }
  return value;
}

function nullableRevision(value: unknown, where: string): Revision | null {
  return value === null ? null : revision(value, where);
}

function list(value: Obj, key: string, where: string): unknown[] {
  const field = value[key];
  if (!Array.isArray(field)) throw new ContractError(`${where}.${key}`, "a list");
  return field;
}

export function parseWork(value: unknown, where = "work"): Work {
  const r = record(value, where);
  return {
    id: text(r, "id", where),
    title: text(r, "title", where),
    created_at: text(r, "created_at", where),
  };
}

export function parseListing(value: unknown): Listing {
  const r = record(value, "listing");
  return {
    works: list(r, "works", "listing").map((w, i) => parseWork(w, `listing.works[${i}]`)),
    unreadable: list(r, "unreadable", "listing").map((u, i) => {
      const where = `listing.unreadable[${i}]`;
      const entry = record(u, where);
      return { id: text(entry, "id", where), reason: text(entry, "reason", where) };
    }),
  };
}

export function parseWorkAndHead(value: unknown): WorkAndHead {
  const r = record(value, "read_work");
  return { work: parseWork(r["work"], "read_work.work"), head: nullableRevision(r["head"], "read_work.head") };
}

export function parseSourceText(value: unknown, where = "source"): SourceText {
  const r = record(value, where);
  return { revision: revision(r["revision"], `${where}.revision`), text: text(r, "text", where) };
}

/** `read_source`: the work's text, or `null` for a work that has none yet. */
export function parseReadSource(value: unknown): SourceText | null {
  const r = record(value, "read_source");
  return r["source"] === null ? null : parseSourceText(r["source"], "read_source.source");
}

export function parseSaved(value: unknown): Saved {
  const r = record(value, "save_source");
  const outcome = r["outcome"];
  if (outcome === "saved") {
    return {
      outcome,
      revision: revision(r["revision"], "save_source.revision"),
      parent: nullableRevision(r["parent"], "save_source.parent"),
    };
  }
  if (outcome === "unchanged") {
    return { outcome, revision: revision(r["revision"], "save_source.revision") };
  }
  throw new ContractError("save_source.outcome", '"saved" or "unchanged"');
}

export function parseHistory(value: unknown): HistoryEntry[] {
  const r = record(value, "history");
  return list(r, "entries", "history").map((e, i) => {
    const where = `history.entries[${i}]`;
    const entry = record(e, where);
    return {
      revision: revision(entry["revision"], `${where}.revision`),
      parent: nullableRevision(entry["parent"], `${where}.parent`),
      saved_at: text(entry, "saved_at", where),
    };
  });
}

export function parseDescribed(value: unknown): Described {
  const r = record(value, "describe");
  const version = r["command_set_version"];
  if (typeof version !== "number") throw new ContractError("describe.command_set_version", "a number");
  return {
    command_set_version: version,
    commands: list(r, "commands", "describe").map((c, i) => {
      if (typeof c !== "string") throw new ContractError(`describe.commands[${i}]`, "a string");
      return c;
    }),
    root: text(r, "root", "describe"),
  };
}

/** Whether `value` is a refusal in the core's shape (what a rejected call carries). */
export function asCommandError(value: unknown): CommandErrorBody | null {
  if (typeof value !== "object" || value === null) return null;
  const r = value as Obj;
  if (typeof r["kind"] !== "string" || typeof r["message"] !== "string") return null;
  return r["detail"] === undefined
    ? { kind: r["kind"], message: r["message"] }
    : { kind: r["kind"], message: r["message"], detail: r["detail"] };
}

/** The two revisions a `conflict` carries: the one the caller wrote from, and the current one. */
export function conflictRevisions(detail: unknown): { base: Revision | null; current: Revision | null } {
  const r = record(detail, "conflict.detail");
  return {
    base: nullableRevision(r["base"] ?? null, "conflict.detail.base"),
    current: nullableRevision(r["current"] ?? null, "conflict.detail.current"),
  };
}
