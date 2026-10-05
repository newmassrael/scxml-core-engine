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
export const SUPPORTED_COMMAND_SET_VERSION = 10;

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

/** One document of a model: the file name its imports know it by, and its text. */
export interface ModelDocument {
  readonly name: string;
  readonly text: string;
}

/**
 * A saved model, and the text revision it was written for. A model of several
 * documents (a statechart and the event schemas it imports) lists them all, the
 * entry first; a model of one document lists it under the name `model.scxml`.
 */
export interface ModelText {
  readonly revision: Revision;
  readonly written_for: Revision | null;
  /** The entry's text: what SCE is asked about. */
  readonly text: string;
  /** The entry's file name. */
  readonly entry: string;
  readonly documents: readonly ModelDocument[];
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

/**
 * The revisions of everything an acceptance is about: the text, the model, the
 * requirement list, and the owner's answers when they have given some. What the
 * owner is shown carries these, and what they accept is sent back with them, so the
 * core can tell that what they accepted is what they saw.
 */
export interface Basis {
  readonly source: Revision;
  readonly model: Revision;
  readonly requirements: Revision;
  /** `null` when the owner had answered nothing. */
  readonly answers: Revision | null;
}

/** The requirement list a text was read into: two JSON files, kept as the authoring client wrote them. */
export interface RequirementsList {
  readonly revision: Revision;
  readonly written_for: Revision | null;
  readonly manifest: string;
  readonly sidecar: string | null;
}

/** `read_requirements`: the work's list, or `null` for a work with none yet. */
export interface ReadRequirements {
  readonly requirements: RequirementsList | null;
  readonly source_head: Revision | null;
  readonly standing: Standing | null;
}

/** One requirement and how SCE finds the design to stand to it. */
export interface RequirementOutcome {
  readonly id: string;
  /** SCE's word: `implemented`, `missing`, `needs-scenario`, or one a later SCE adds. */
  readonly outcome: string;
  /** The place of the text the requirement is anchored in. */
  readonly section: string | null;
  /** Where in the design it is carried, in SCE's path syntax. */
  readonly node_paths: readonly string[];
}

/** `requirements_report`: SCE's measure of the design against the list, and the page the owner reads before accepting. */
export interface RequirementsReport {
  readonly basis: Basis;
  readonly source_head: Revision | null;
  readonly model_standing: Standing;
  readonly requirements_standing: Standing;
  readonly generator: string | null;
  /** What the list is a denominator OF, as SCE states it. */
  readonly denominator: string | null;
  readonly outcomes: readonly RequirementOutcome[];
  /** SCE's acceptance page; `null` when SCE did not write it (see `page_refusal`). */
  readonly page: string | null;
  readonly page_refusal: PageRefusal | null;
}

/** Whether the owner's acceptance still holds for the work as it is now: SCE's answer. */
export type AcceptanceStanding = "none" | "holds" | "lapsed";

/** What the owner accepted, and when, and on which surface it was stated. */
export interface AcceptanceRecord {
  readonly revision: Revision;
  readonly accepted_at: string;
  /** `direct` (stated in this application) or `relayed` (an authoring client's statement). */
  readonly channel: string;
  readonly basis: Basis;
  /** What SCE listed as left open when it was accepted, in its sentences. */
  readonly open: readonly string[];
}

/** `read_acceptance`. */
export interface ReadAcceptance {
  readonly acceptance: AcceptanceRecord | null;
  readonly standing: AcceptanceStanding;
  /** SCE's one sentence of what moved, when the acceptance lapsed. */
  readonly lapse: string | null;
  /** The revisions of the work as it is now, when there is an acceptance to compare them with. */
  readonly now: Basis | null;
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

/**
 * `read_work_snapshot`: the work and its five chains as they stood together. Each part
 * is what the command that reads that chain answers, in the same words, taken from one
 * state of the work and not from six reads a save can land between. Whether the
 * acceptance still holds is not here: that is SCE's to say (`read_acceptance`).
 */
export interface WorkSnapshot {
  readonly work: Work;
  readonly source: SourceText | null;
  readonly model: ModelText | null;
  /** `null` exactly when there is no model. */
  readonly model_standing: Standing | null;
  readonly answers: Answers | null;
  readonly requirements: RequirementsList | null;
  /** `null` exactly when there is no list. */
  readonly requirements_standing: Standing | null;
  readonly acceptance: AcceptanceRecord | null;
}

/**
 * The revision at the head of the model or of the requirement list, and the source it
 * was written for. The same text kept again for a later source is the same revision
 * with another `written_for`, and that moves where it stands to the text.
 */
export interface ClaimedHead {
  readonly revision: Revision;
  readonly written_for: Revision | null;
}

/**
 * `read_work_heads`: where each chain of the work stands, and nothing it holds. A screen
 * compares it with what it shows to know whether the work moved under it, and reads the
 * work only when it did.
 */
export interface WorkHeads {
  readonly source: Revision | null;
  readonly model: ClaimedHead | null;
  readonly answers: Revision | null;
  readonly requirements: ClaimedHead | null;
  readonly acceptance: Revision | null;
  /** Where the work's latest request stands as the clock says it now; `null` for a work never asked for a model. */
  readonly request: RequestHead | null;
}

/**
 * Where a generation request is, in the core's words. `interrupted` is what a running
 * request is read as once its lease ran out, though nothing was written: the core says
 * it by the clock, so it can change between two questions with nobody having acted.
 */
export type RequestState =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "interrupted"
  | "superseded";

/** The states in which a request can still produce a result. At most one request of a work is in one. */
export function isOpenRequest(state: RequestState): boolean {
  return state === "queued" || state === "running" || state === "interrupted";
}

/** The latest request of a work, as the heads say it. */
export interface RequestHead {
  readonly id: string;
  readonly state: RequestState;
  readonly attempt: number;
}

/** An executor's claim on a request: who, which attempt, and when it was given and runs out (RFC 3339). */
export interface RequestLease {
  readonly holder: string;
  readonly attempt: number;
  readonly granted_at: string;
  readonly expires_at: string;
}

/** One ask of an AI to write a model from the work's text. */
export interface GenerationRequest {
  readonly id: string;
  readonly work: string;
  /** The order the work's requests were made in, from 1. */
  readonly seq: number;
  readonly key: string;
  /** Where it was asked from: `gui`, or the name of a client. */
  readonly origin: string;
  /** Where the request stands as the clock reads it now. */
  readonly state: RequestState;
  /** What was last written; differs from `state` for a lease that ran out and nobody wrote down. */
  readonly stored_state: RequestState;
  /** How many times an executor has taken it. */
  readonly attempt: number;
  /** The revisions it was asked about; an executor working from others is working from something else. */
  readonly inputs: { readonly source: Revision; readonly answers: Revision | null };
  readonly created_at: string;
  readonly lease: RequestLease | null;
  readonly ended_at: string | null;
  /** Why it ended or was let go of, in words. */
  readonly note: string | null;
}

/** `request_generation`: the request, and whether this call made it or repeated a call that did. */
export interface RegisteredRequest {
  readonly request: GenerationRequest;
  readonly created: boolean;
}

/** An AI adapter, and whether it is there now. */
export interface AdapterStatus {
  readonly name: string;
  readonly kind: string;
  /** What it can do (`generate`, `cancel`, ...): the screen offers only what is here. */
  readonly capabilities: readonly string[];
  readonly version: string | null;
  readonly seen_at: string;
  readonly live: boolean;
}

/** `read_adapter_status`: every adapter that reported, and every record that could not be read. */
export interface AdapterListing {
  readonly adapters: readonly AdapterStatus[];
  readonly unreadable: readonly Unreadable[];
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

/** A standing that is `null` exactly when the part it is about is absent. */
function partStanding(value: unknown, absent: boolean, where: string): Standing | null {
  if (absent) {
    if (value !== null) throw new ContractError(where, "null when there is none");
    return null;
  }
  return standing(value, where);
}

/** A saved model as `read_model` and `read_work_snapshot` both give it. */
function parseModelText(value: unknown, where: string): ModelText {
  const m = record(value, where);
  const documents = list(m, "documents", where).map((d, i) => {
    const at = `${where}.documents[${i}]`;
    const document = record(d, at);
    return { name: text(document, "name", at), text: text(document, "text", at) };
  });
  const entry = text(m, "entry", where);
  if (!documents.some((d) => d.name === entry)) {
    throw new ContractError(`${where}.entry`, "the name of one of its documents");
  }
  return {
    revision: revision(m["revision"], `${where}.revision`),
    written_for: nullableRevision(m["written_for"], `${where}.written_for`),
    text: text(m, "text", where),
    entry,
    documents,
  };
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
  return {
    model: parseModelText(model, "read_model.model"),
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
    page_refusal: parsePageRefusal(r["page_refusal"], "review.page_refusal"),
  };
}

/** Why SCE did not write a page it was asked for, or `null` when it wrote it. */
function parsePageRefusal(value: unknown, where: string): PageRefusal | null {
  if (value === null || value === undefined) return null;
  const entry = record(value, where);
  return { code: text(entry, "code", where), message: text(entry, "message", where) };
}

/** The owner's answers as `read_answers` and `read_work_snapshot` both give them. */
function parseAnswers(value: unknown, where: string): Answers {
  const a = record(value, where);
  const entries = record(a["entries"], `${where}.entries`);
  const parsed: Record<string, AnswerEntry> = {};
  for (const [id, entry] of Object.entries(entries)) {
    const at = `${where}.entries.${id}`;
    const e = record(entry, at);
    parsed[id] = { answer: text(e, "answer", at), answered_at: text(e, "answered_at", at) };
  }
  return { revision: revision(a["revision"], `${where}.revision`), entries: parsed };
}

/** `read_answers`: the owner's answers, or `null` when they have answered nothing. */
export function parseReadAnswers(value: unknown): Answers | null {
  const r = record(value, "read_answers");
  const answers = r["answers"];
  return answers === null ? null : parseAnswers(answers, "read_answers.answers");
}

function parseBasis(value: unknown, where: string): Basis {
  const r = record(value, where);
  return {
    source: revision(r["source"], `${where}.source`),
    model: revision(r["model"], `${where}.model`),
    requirements: revision(r["requirements"], `${where}.requirements`),
    // Absent when the owner had answered nothing: the core leaves it out rather than say null.
    answers: nullableRevision(r["answers"] ?? null, `${where}.answers`),
  };
}

function stringList(value: Obj, key: string, where: string): string[] {
  return list(value, key, where).map((item, i) => {
    if (typeof item !== "string") throw new ContractError(`${where}.${key}[${i}]`, "a string");
    return item;
  });
}

/** A requirement list as `read_requirements` and `read_work_snapshot` both give it. */
function parseRequirementsList(value: unknown, where: string): RequirementsList {
  const entry = record(value, where);
  return {
    revision: revision(entry["revision"], `${where}.revision`),
    written_for: nullableRevision(entry["written_for"], `${where}.written_for`),
    manifest: text(entry, "manifest", where),
    sidecar: nullableText(entry, "sidecar", where),
  };
}

/** `read_requirements`. */
export function parseReadRequirements(value: unknown): ReadRequirements {
  const where = "read_requirements";
  const r = record(value, where);
  const source_head = nullableRevision(r["source_head"], `${where}.source_head`);
  const held = r["requirements"];
  if (held === null) {
    if (r["standing"] !== null) throw new ContractError(`${where}.standing`, "null when there is no list");
    return { requirements: null, source_head, standing: null };
  }
  return {
    requirements: parseRequirementsList(held, `${where}.requirements`),
    source_head,
    standing: standing(r["standing"], `${where}.standing`),
  };
}

/** `requirements_report`. */
export function parseRequirementsReport(value: unknown): RequirementsReport {
  const where = "requirements_report";
  const r = record(value, where);
  const generator = r["generator"];
  if (generator !== null && typeof generator !== "string") {
    throw new ContractError(`${where}.generator`, "a string or null");
  }
  return {
    basis: parseBasis(r["basis"], `${where}.basis`),
    source_head: nullableRevision(r["source_head"], `${where}.source_head`),
    model_standing: standing(r["model_standing"], `${where}.model_standing`),
    requirements_standing: standing(r["requirements_standing"], `${where}.requirements_standing`),
    generator,
    denominator: nullableText(r, "denominator", where),
    outcomes: list(r, "outcomes", where).map((o, i) => {
      const at = `${where}.outcomes[${i}]`;
      const outcome = record(o, at);
      return {
        id: text(outcome, "id", at),
        outcome: text(outcome, "outcome", at),
        section: nullableText(outcome, "section", at),
        node_paths: stringList(outcome, "node_paths", at),
      };
    }),
    page: nullableText(r, "page", where),
    page_refusal: parsePageRefusal(r["page_refusal"], `${where}.page_refusal`),
  };
}

/** An acceptance as `read_acceptance` and `read_work_snapshot` both give it. */
function parseAcceptanceRecord(value: unknown, where: string): AcceptanceRecord {
  const entry = record(value, where);
  return {
    revision: revision(entry["revision"], `${where}.revision`),
    accepted_at: text(entry, "accepted_at", where),
    channel: text(entry, "channel", where),
    basis: parseBasis(entry["basis"], `${where}.basis`),
    open: stringList(entry, "open", where),
  };
}

/** `read_acceptance`. */
export function parseReadAcceptance(value: unknown): ReadAcceptance {
  const where = "read_acceptance";
  const r = record(value, where);
  const state = r["standing"];
  if (state !== "none" && state !== "holds" && state !== "lapsed") {
    throw new ContractError(`${where}.standing`, '"none", "holds" or "lapsed"');
  }
  const held = r["acceptance"];
  if (held === null) {
    if (state !== "none") throw new ContractError(`${where}.acceptance`, "an acceptance unless the standing is none");
    return { acceptance: null, standing: state, lapse: null, now: null };
  }
  if (state === "none") throw new ContractError(`${where}.standing`, "holds or lapsed when there is an acceptance");
  const lapse = nullableText(r, "lapse", where);
  if ((state === "lapsed") !== (lapse !== null)) {
    throw new ContractError(`${where}.lapse`, "SCE's sentence exactly when the acceptance lapsed");
  }
  return {
    acceptance: parseAcceptanceRecord(held, `${where}.acceptance`),
    standing: state,
    lapse,
    now: parseBasis(r["now"], `${where}.now`),
  };
}

function parseClaimedHead(value: unknown, where: string): ClaimedHead {
  const r = record(value, where);
  return {
    revision: revision(r["revision"], `${where}.revision`),
    written_for: nullableRevision(r["written_for"], `${where}.written_for`),
  };
}

const REQUEST_STATES: readonly RequestState[] = [
  "queued",
  "running",
  "completed",
  "failed",
  "cancelled",
  "interrupted",
  "superseded",
];

function requestState(value: unknown, where: string): RequestState {
  if (typeof value === "string" && (REQUEST_STATES as readonly string[]).includes(value)) {
    return value as RequestState;
  }
  throw new ContractError(where, `one of ${REQUEST_STATES.map((s) => `"${s}"`).join(", ")}`);
}

function count(value: Obj, key: string, where: string): number {
  const field = value[key];
  if (typeof field !== "number" || !Number.isInteger(field) || field < 0) {
    throw new ContractError(`${where}.${key}`, "a count");
  }
  return field;
}

function parseRequestHead(value: unknown, where: string): RequestHead {
  const r = record(value, where);
  return {
    id: text(r, "id", where),
    state: requestState(r["state"], `${where}.state`),
    attempt: count(r, "attempt", where),
  };
}

/** `read_work_heads`. */
export function parseWorkHeads(value: unknown): WorkHeads {
  const where = "read_work_heads";
  const r = record(value, where);
  const model = r["model"];
  const requirements = r["requirements"];
  const request = r["request"];
  return {
    source: nullableRevision(r["source"], `${where}.source`),
    model: model === null ? null : parseClaimedHead(model, `${where}.model`),
    answers: nullableRevision(r["answers"], `${where}.answers`),
    requirements: requirements === null ? null : parseClaimedHead(requirements, `${where}.requirements`),
    acceptance: nullableRevision(r["acceptance"], `${where}.acceptance`),
    request: request === null ? null : parseRequestHead(request, `${where}.request`),
  };
}

function parseLease(value: unknown, where: string): RequestLease {
  const r = record(value, where);
  return {
    holder: text(r, "holder", where),
    attempt: count(r, "attempt", where),
    granted_at: text(r, "granted_at", where),
    expires_at: text(r, "expires_at", where),
  };
}

/** A request as every command about one answers it. */
export function parseGenerationRequest(value: unknown, where = "request"): GenerationRequest {
  const r = record(value, where);
  const inputs = record(r["inputs"], `${where}.inputs`);
  const lease = r["lease"];
  return {
    id: text(r, "id", where),
    work: text(r, "work", where),
    seq: count(r, "seq", where),
    key: text(r, "key", where),
    origin: text(r, "origin", where),
    state: requestState(r["state"], `${where}.state`),
    stored_state: requestState(r["stored_state"], `${where}.stored_state`),
    attempt: count(r, "attempt", where),
    inputs: {
      source: revision(inputs["source"], `${where}.inputs.source`),
      answers: nullableRevision(inputs["answers"], `${where}.inputs.answers`),
    },
    created_at: text(r, "created_at", where),
    lease: lease === null ? null : parseLease(lease, `${where}.lease`),
    ended_at: nullableText(r, "ended_at", where),
    note: nullableText(r, "note", where),
  };
}

/** `read_request`, `claim_request`, `heartbeat_request`, `complete_request`, `fail_request`, `cancel_request`: the request. */
export function parseRequestReply(value: unknown): GenerationRequest {
  return parseGenerationRequest(record(value, "request reply")["request"], "request reply.request");
}

/** `request_generation`. */
export function parseRegisteredRequest(value: unknown): RegisteredRequest {
  const where = "request_generation";
  const r = record(value, where);
  if (typeof r["created"] !== "boolean") throw new ContractError(`${where}.created`, "true or false");
  return { request: parseGenerationRequest(r["request"], `${where}.request`), created: r["created"] };
}

/** `list_requests`: the work's requests, the newest first. */
export function parseRequestList(value: unknown): GenerationRequest[] {
  const where = "list_requests";
  const r = record(value, where);
  return list(r, "requests", where).map((item, i) => parseGenerationRequest(item, `${where}.requests[${i}]`));
}

function parseAdapter(value: unknown, where: string): AdapterStatus {
  const r = record(value, where);
  if (typeof r["live"] !== "boolean") throw new ContractError(`${where}.live`, "true or false");
  return {
    name: text(r, "name", where),
    kind: text(r, "kind", where),
    capabilities: stringList(r, "capabilities", where),
    version: nullableText(r, "version", where),
    seen_at: text(r, "seen_at", where),
    live: r["live"],
  };
}

/** `report_adapter`. */
export function parseAdapterReport(value: unknown): AdapterStatus {
  return parseAdapter(record(value, "report_adapter")["adapter"], "report_adapter.adapter");
}

/** `read_adapter_status`. */
export function parseAdapterListing(value: unknown): AdapterListing {
  const where = "read_adapter_status";
  const r = record(value, where);
  return {
    adapters: list(r, "adapters", where).map((a, i) => parseAdapter(a, `${where}.adapters[${i}]`)),
    unreadable: list(r, "unreadable", where).map((u, i) => {
      const at = `${where}.unreadable[${i}]`;
      const entry = record(u, at);
      return { id: text(entry, "id", at), reason: text(entry, "reason", at) };
    }),
  };
}

/** `read_work_snapshot`. */
export function parseWorkSnapshot(value: unknown): WorkSnapshot {
  const where = "read_work_snapshot";
  const r = record(value, where);
  const source = r["source"];
  const model = r["model"];
  const answers = r["answers"];
  const requirements = r["requirements"];
  const acceptance = r["acceptance"];
  return {
    work: parseWork(r["work"], `${where}.work`),
    source: source === null ? null : parseSourceText(source, `${where}.source`),
    model: model === null ? null : parseModelText(model, `${where}.model`),
    model_standing: partStanding(r["model_standing"], model === null, `${where}.model_standing`),
    answers: answers === null ? null : parseAnswers(answers, `${where}.answers`),
    requirements: requirements === null ? null : parseRequirementsList(requirements, `${where}.requirements`),
    requirements_standing: partStanding(
      r["requirements_standing"],
      requirements === null,
      `${where}.requirements_standing`,
    ),
    acceptance: acceptance === null ? null : parseAcceptanceRecord(acceptance, `${where}.acceptance`),
  };
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
