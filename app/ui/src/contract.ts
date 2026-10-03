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
export const SUPPORTED_COMMAND_SET_VERSION = 5;

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

/**
 * How a model stands to the text now, as the core words it: `current` was written
 * for the text as it is, `behind` for an earlier one, `unstated` its writer did
 * not say. One definition, in the core; the screen only shows it.
 */
export type Standing = "current" | "behind" | "unstated";

/** A saved model, and the text revision it was written for. */
export interface ModelText {
  readonly revision: Revision;
  readonly written_for: Revision | null;
  readonly text: string;
}

/** `read_model`: the model, or `null` for a work with none yet. */
export interface ReadModel {
  readonly model: ModelText | null;
  readonly source_head: Revision | null;
  readonly standing: Standing | null;
}

/** One sheet SCE drew: the file name it gave it, and the SVG. */
export interface DrawnSheet {
  readonly name: string;
  readonly svg: string;
}

/** `figures`: what SCE drew of a model, in the order it wrote it. */
export interface Figures {
  readonly model: { readonly revision: Revision; readonly written_for: Revision | null };
  readonly source_head: Revision | null;
  readonly standing: Standing;
  /** The generator's own version line, when it gave one. */
  readonly generator: string | null;
  readonly sheets: readonly DrawnSheet[];
}

/** SCE's verdict on a document. It is the product's verdict, not a claim that the model matches the text. */
export type Verdict = "accepted" | "refused";

/** One question the model marks as not decided, and where. */
export interface Unresolved {
  readonly id: string;
  readonly node_path: string;
  readonly line: number | null;
  /** The question in the words the model asked it; `null` when it gave none. */
  readonly reason: string | null;
}

/** One answer of the owner's: their words, and when those words last changed. */
export interface AnswerEntry {
  readonly answer: string;
  readonly answered_at: string;
}

/** The owner's answers to the questions a model leaves open, by the id of each question. */
export interface Answers {
  readonly revision: Revision;
  readonly entries: Readonly<Record<string, AnswerEntry>>;
}

/** One record SCE wrote about the document, in its words. */
export interface CheckRecord {
  readonly code: string;
  readonly message: string;
  readonly stage: string | null;
  readonly line: number | null;
}

/** SCE's check of a model. */
export interface Check {
  readonly verdict: Verdict;
  /** The kind SCE read the document as; `null` for one it refused. */
  readonly kind: string | null;
  /** What an accepted model still leaves to a person, in SCE's sentences. */
  readonly open: readonly string[];
  readonly unresolved: readonly Unresolved[];
  readonly records: readonly CheckRecord[];
}

/** Why SCE did not write the page of a model it accepted. */
export interface PageRefusal {
  readonly code: string;
  readonly message: string;
}

/** `review`: what SCE says of a model, for the person who reads it against the text. */
export interface Review {
  readonly model: { readonly revision: Revision; readonly written_for: Revision | null };
  readonly source_head: Revision | null;
  readonly standing: Standing;
  readonly generator: string | null;
  readonly check: Check;
  /** The pseudocode page, as SCE wrote it; `null` when there is none (see `page_refusal`, or a refused model). */
  readonly page: string | null;
  readonly page_refusal: PageRefusal | null;
}

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

/** `remove_work`: the work that was taken out of the list. */
export function parseRemoved(value: unknown): Work {
  return parseWork(record(value, "remove_work")["removed"], "remove_work.removed");
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

function standing(value: unknown, where: string): Standing {
  if (value === "current" || value === "behind" || value === "unstated") return value;
  throw new ContractError(where, '"current", "behind" or "unstated"');
}

/** `read_model`. */
export function parseReadModel(value: unknown): ReadModel {
  const r = record(value, "read_model");
  const model = r["model"];
  const source_head = nullableRevision(r["source_head"], "read_model.source_head");
  if (model === null) {
    if (r["standing"] !== null) throw new ContractError("read_model.standing", "null when there is no model");
    return { model: null, source_head, standing: null };
  }
  const m = record(model, "read_model.model");
  return {
    model: {
      revision: revision(m["revision"], "read_model.model.revision"),
      written_for: nullableRevision(m["written_for"], "read_model.model.written_for"),
      text: text(m, "text", "read_model.model"),
    },
    source_head,
    standing: standing(r["standing"], "read_model.standing"),
  };
}

/** `figures`. */
export function parseFigures(value: unknown): Figures {
  const r = record(value, "figures");
  const model = record(r["model"], "figures.model");
  const generator = r["generator"];
  if (generator !== null && typeof generator !== "string") {
    throw new ContractError("figures.generator", "a string or null");
  }
  return {
    model: {
      revision: revision(model["revision"], "figures.model.revision"),
      written_for: nullableRevision(model["written_for"], "figures.model.written_for"),
    },
    source_head: nullableRevision(r["source_head"], "figures.source_head"),
    standing: standing(r["standing"], "figures.standing"),
    generator,
    sheets: list(r, "sheets", "figures").map((s, i) => {
      const where = `figures.sheets[${i}]`;
      const sheet = record(s, where);
      return { name: text(sheet, "name", where), svg: text(sheet, "svg", where) };
    }),
  };
}

function nullableText(value: Obj, key: string, where: string): string | null {
  const field = value[key];
  if (field === null || field === undefined) return null;
  if (typeof field !== "string") throw new ContractError(`${where}.${key}`, "a string or null");
  return field;
}

function nullableNumber(value: Obj, key: string, where: string): number | null {
  const field = value[key];
  if (field === null || field === undefined) return null;
  if (typeof field !== "number") throw new ContractError(`${where}.${key}`, "a number or null");
  return field;
}

function parseCheck(value: unknown): Check {
  const where = "review.check";
  const r = record(value, where);
  const verdict = r["verdict"];
  if (verdict !== "accepted" && verdict !== "refused") {
    throw new ContractError(`${where}.verdict`, '"accepted" or "refused"');
  }
  return {
    verdict,
    kind: nullableText(r, "kind", where),
    open: list(r, "open", where).map((m, i) => {
      if (typeof m !== "string") throw new ContractError(`${where}.open[${i}]`, "a string");
      return m;
    }),
    unresolved: list(r, "unresolved", where).map((u, i) => {
      const at = `${where}.unresolved[${i}]`;
      const entry = record(u, at);
      return {
        id: text(entry, "id", at),
        node_path: text(entry, "node_path", at),
        line: nullableNumber(entry, "line", at),
        reason: nullableText(entry, "reason", at),
      };
    }),
    records: list(r, "records", where).map((rec, i) => {
      const at = `${where}.records[${i}]`;
      const entry = record(rec, at);
      return {
        code: text(entry, "code", at),
        message: text(entry, "message", at),
        stage: nullableText(entry, "stage", at),
        line: nullableNumber(entry, "line", at),
      };
    }),
  };
}

/** `review`. */
export function parseReview(value: unknown): Review {
  const r = record(value, "review");
  const model = record(r["model"], "review.model");
  const generator = r["generator"];
  if (generator !== null && typeof generator !== "string") {
    throw new ContractError("review.generator", "a string or null");
  }
  const refusal = r["page_refusal"];
  let page_refusal: PageRefusal | null = null;
  if (refusal !== null) {
    const at = "review.page_refusal";
    const entry = record(refusal, at);
    page_refusal = { code: text(entry, "code", at), message: text(entry, "message", at) };
  }
  return {
    model: {
      revision: revision(model["revision"], "review.model.revision"),
      written_for: nullableRevision(model["written_for"], "review.model.written_for"),
    },
    source_head: nullableRevision(r["source_head"], "review.source_head"),
    standing: standing(r["standing"], "review.standing"),
    generator,
    check: parseCheck(r["check"]),
    page: nullableText(r, "page", "review"),
    page_refusal,
  };
}

/** `read_answers`: the owner's answers, or `null` when they have answered nothing. */
export function parseReadAnswers(value: unknown): Answers | null {
  const r = record(value, "read_answers");
  const answers = r["answers"];
  if (answers === null) return null;
  const where = "read_answers.answers";
  const a = record(answers, where);
  const entries = record(a["entries"], `${where}.entries`);
  const parsed: Record<string, AnswerEntry> = {};
  for (const [id, entry] of Object.entries(entries)) {
    const at = `${where}.entries.${id}`;
    const e = record(entry, at);
    parsed[id] = { answer: text(e, "answer", at), answered_at: text(e, "answered_at", at) };
  }
  return { revision: revision(a["revision"], `${where}.revision`), entries: parsed };
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
