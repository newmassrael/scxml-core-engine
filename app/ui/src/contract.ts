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
export const SUPPORTED_COMMAND_SET_VERSION = 21;

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

/**
 * SCE's measure of a design against a list, in its words, and the page the owner reads before
 * accepting. It is a function of the design and the list and says nothing of which text they
 * were written for: that is a claim made about them, which can change with the design the same.
 */
export interface Measure {
  readonly generator: string | null;
  /** What the list is a denominator OF, as SCE states it. */
  readonly denominator: string | null;
  readonly outcomes: readonly RequirementOutcome[];
  /** SCE's acceptance page; `null` when SCE did not write it (see `page_refusal`). */
  readonly page: string | null;
  readonly page_refusal: PageRefusal | null;
}

/**
 * `requirements_report`: the measure of the work as it stands, with the revisions it was made
 * against and where the design and the list stand to the text. The screen puts one together from
 * the snapshot it read and the measure SCE gave of that snapshot's revisions (`panelOf`).
 */
export interface RequirementsReport extends Measure {
  readonly basis: Basis;
  readonly source_head: Revision | null;
  readonly model_standing: Standing;
  readonly requirements_standing: Standing;
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

/** `read_acceptance_delta`: what moved since the owner accepted, as the product wrote it. */
export interface ReadAcceptanceDelta {
  readonly acceptance: AcceptanceRecord | null;
  /** The manifest the product's record pinned: where, whose, which revision, its digest. */
  readonly manifest: { readonly doc_id: string; readonly rev: string; readonly sha256: string } | null;
  /** The product's own lines, one JSON object each, in the order it wrote them. */
  readonly lines: readonly Record<string, unknown>[] | null;
  /** The revisions the comparison was made at. */
  readonly now: Basis | null;
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

/** Which door a command came in by (`Entrance` in the core). */
export type Entrance = "desktop" | "browser" | "tool";

export interface Described {
  readonly command_set_version: number;
  readonly commands: readonly string[];
  readonly root: string;
  /** Which entrance asked: what it may change depends on it. */
  readonly entrance: Entrance;
  /** Whether this entrance has a settings folder to read. */
  readonly settings: boolean;
  /** Whether it may change what is in it: only the desktop window does. */
  readonly writes_settings: boolean;
  /** Whether it may start a program of the person's to ask it something: only the desktop window does. */
  readonly starts_programs: boolean;
}

/** Which kind of client or server a connection reaches. */
export type AdapterKind = "claude-code" | "codex" | "local";

/** Where a connection's credential comes from. The credential itself is never in a connection. */
export type AuthSource = "official-login" | "app-store" | "env-api-key" | "server-key" | "none";

/** The settings of one way to reach a model. */
export interface Connection {
  readonly id: string;
  readonly adapter: AdapterKind;
  readonly display_name: string | null;
  readonly executable: string | null;
  readonly model: string | null;
  readonly auth: AuthSource;
  readonly server_url: string | null;
  readonly limits: { readonly turns: number | null; readonly seconds: number | null };
}

/** A connection as it is kept, with the revision it is kept under. */
export interface StoredConnection {
  readonly connection: Connection;
  readonly revision: Revision;
}

/** `list_connections`. */
export interface ConnectionListing {
  readonly connections: readonly StoredConnection[];
  readonly unreadable: readonly { readonly id: string; readonly reason: string }[];
  /** The connection a new request uses when the person does not choose one. */
  readonly default: string | null;
}

/** What the design says of a way of signing in. */
export type RouteStatus = "allowed" | "conditional" | "forbidden" | "unconfirmed";

/** Whether a way of signing in is used, and if not, why. */
export type RouteDecision =
  | { readonly decision: "use"; readonly status: RouteStatus }
  | {
      readonly decision: "refuse";
      readonly status: RouteStatus;
      readonly reason: "forbidden" | "unconfirmed" | "switched-off";
    };

/** `read_auth_policy`. */
export interface AuthPolicy {
  readonly routes: readonly {
    readonly route: string;
    readonly status: RouteStatus;
    readonly decision: RouteDecision;
  }[];
  readonly switched_off: readonly string[];
}

/** How a way of signing in is billed: against a plan, by use, or by a cloud provider. */
export type Billing = "subscription" | "usage" | "provider";

/** Whether Claude Code is there. `unverified` is a file that did not answer `--version`. */
export type ClaudeClient =
  | { readonly state: "installed"; readonly version: string; readonly path: string }
  | { readonly state: "unverified" }
  | { readonly state: "missing" };

/**
 * Who is signed in to it. `unknown` is a client that could not be asked, which is not nobody:
 * a screen that offered a sign-in for a question that failed would send a person to log in for nothing.
 */
export type ClaudeAccount =
  | {
      readonly state: "signed-in";
      readonly route: string;
      /** `null` for a way that is none of the three. */
      readonly billing: Billing | null;
      /** The variable that decided it, when the client said one did: a name, never a value. */
      readonly environment: string | null;
      /** Whether a generation would use it. */
      readonly usable: boolean;
      readonly decision: RouteDecision;
    }
  | { readonly state: "signed-out" }
  | { readonly state: "unknown"; readonly reason: string };

/** A command that signs in, as a person runs it in a terminal, and how it bills. */
export interface SignInCommand {
  readonly billing: Billing;
  readonly command: string;
}

/** `read_claude_status`. */
export interface ClaudeStatus {
  readonly client: ClaudeClient;
  readonly account: ClaudeAccount;
  readonly sign_in: readonly SignInCommand[];
}

/**
 * Whether this build verified the Codex that is there. Until a version is verified a generation
 * never runs, whoever is signed in, so this is said apart from the account. `unknown` is one
 * that could not be asked (no program, or it would not list its features): not "unverified".
 */
export type CodexSupport =
  | { readonly state: "verified" }
  | { readonly state: "unverified"; readonly reason: string }
  | { readonly state: "unknown"; readonly reason: string };

/** The three places a connection to Codex can take its credential from. */
export type CodexSource = "official-login" | "app-store" | "env-api-key";

/** Who is signed in to Codex by one source, in the words of `ClaudeAccount`. */
export type CodexAccount = ClaudeAccount & { readonly source: CodexSource };

/** A command that signs in to Codex, with the billing it signs in for. */
export interface CodexSignIn {
  readonly source: CodexSource;
  readonly billing: Billing;
  readonly command: string;
  /**
   * The folder to start the client with as `CODEX_HOME`, so that the login it makes is the one a
   * generation uses. `null` for the official client's own login. A folder, and not a shell's
   * words: which shell it is is not known.
   */
  readonly home: string | null;
}

/** `read_codex_status`. */
export interface CodexStatus {
  readonly client: ClaudeClient;
  readonly support: CodexSupport;
  readonly accounts: readonly CodexAccount[];
  readonly sign_in: readonly CodexSignIn[];
  /** The variable a connection that takes its key from the environment reads it from. */
  readonly key_variable: string;
}

/** `find_clients`: the programs of each client the application found. */
export interface FoundClients {
  readonly claude: readonly Candidate[];
  readonly codex: readonly Candidate[];
}

/** Where a model server is, by its address: what the address says and nothing more. */
export type ServerReach = "this-computer" | "network";

/** What a server answered when it was asked for its models. */
export type ServerState =
  /** It listed its models, in its order. None is a server with nothing loaded. */
  | { readonly state: "listed"; readonly models: readonly string[] }
  /** Nothing answered, or it broke or was too slow. */
  | { readonly state: "unreachable"; readonly reason: string }
  /** It answered over https and its certificate was refused. Nothing was sent to it. */
  | { readonly state: "certificate"; readonly reason: string }
  /** It wants a key, which this build has no place to keep. */
  | { readonly state: "needs-key"; readonly reason: string }
  /** It answered with something that is not a list of models: the address is not its root. */
  | { readonly state: "not-a-model-list"; readonly reason: string };

/** `read_server_status`: what is known of the server at an address. */
export type ServerStatus = {
  /** The address as it was given: what a connection would keep. */
  readonly address: string;
  readonly reach: ServerReach;
  /** Whether the way to the server is encrypted and the server is known by its certificate. */
  readonly tls: boolean;
} & ServerState;

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
  /**
   * The bundle the model and the list are the ones of, for a work that keeps bundles; `null`
   * for one whose model and list were saved by hand. A bundle is one pointer, so a model of
   * one generation is never shown beside a list of another.
   */
  readonly bundle: Revision | null;
}

/** SCE did not answer: the word a program branches on, its own words, and the code it refused with. */
export interface Refusal {
  readonly kind: string;
  readonly message: string;
  readonly code: string | null;
}

/** What SCE said, or that it did not answer. What was read is still what was read either way. */
export type Judged<T> =
  | { readonly said: true; readonly value: T }
  | { readonly said: false; readonly refusal: Refusal };

/** SCE's word on an acceptance that was made: whether it holds, and what moved when it does not. */
export interface AcceptanceVerdict {
  readonly standing: "holds" | "lapsed";
  /** SCE's one sentence of what moved; `null` exactly when it holds. */
  readonly lapse: string | null;
}

/**
 * `read_judgment`: what SCE says of the revisions it was asked about, and of no other. A
 * revision is never rewritten, so the answer is of the design that was read, whatever has been
 * saved since; a screen that shows a work and then asks about the revisions it shows has a verdict
 * that is of what it shows, and nothing to compare. `basis` is what was asked, echoed. It is a
 * function of the revisions' content alone: where the design stands to the text is a claim made
 * about it, which a later save can change without changing the design, and it is said by the
 * snapshot the revisions were read in.
 */
export interface Judgment {
  readonly basis: Basis;
  /** `null` exactly when no acceptance was named. */
  readonly acceptance: Judged<AcceptanceVerdict> | null;
  readonly report: Judged<Measure>;
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
  /** The bundle the model and the list are the ones of; `null` for a work that keeps none. */
  readonly bundle: Revision | null;
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
  /**
   * What the executor has written for it, by revision. It is the request's and not the
   * work's: the work's model is the one it was until the request completes. `null` until
   * the executor writes something.
   */
  readonly candidate: RequestCandidate | null;
  /** The bundle a completed request made; `null` for every request that did not complete. */
  readonly outcome: { readonly bundle: Revision } | null;
  /**
   * The connection it was made for, as the core copied it when the request was made, or `null`
   * for a request nobody chose a connection for. It says which client, which model and what a
   * run may spend, and nothing of where a server is or what the person calls it.
   */
  readonly pin: RequestPin | null;
  readonly ended_at: string | null;
  /** Why it ended or was let go of, in words. */
  readonly note: string | null;
}

/** The connection a request was made for (`Pin` in the core). */
export interface RequestPin {
  readonly connection: string;
  readonly revision: Revision;
  readonly adapter: AdapterKind;
  readonly model: string | null;
  readonly limits: { readonly turns: number | null; readonly seconds: number | null };
}

/** The halves of a candidate the executor has written; a bundle needs both. */
export interface RequestCandidate {
  readonly model: Revision | null;
  readonly requirements: Revision | null;
  /** The version of the working instructions the executor was given, in its own words; `null` when it did not say. */
  readonly instructions: string | null;
}

/** `read_request_candidate`: what the executor wrote, as texts. */
export interface RequestCandidateTexts {
  readonly request: string;
  readonly model: { readonly revision: Revision; readonly text: string } | null;
  readonly requirements: { readonly revision: Revision; readonly text: string } | null;
}

/** `complete_request`: the request, completed, and the bundle it made the work's. */
export interface CompletedRequest {
  readonly request: GenerationRequest;
  readonly bundle: Revision;
}

/** What checked a candidate: the core ran it, or the executor says it did. */
export type CheckedBy = "core" | "client";

/** One check of a candidate and how it came out. */
export interface BundleCheck {
  readonly by: CheckedBy;
  /** What was checked (`model`, `decisions`). */
  readonly name: string;
  readonly verdict: "accepted" | "refused";
  readonly generator: string | null;
  readonly digest: string | null;
  /** The revision the check ran on. */
  readonly subject: Revision | null;
}

/**
 * A published bundle: the model and the requirement list one generation made together,
 * the text and answers it was made from, and what was checked of it.
 */
export interface Bundle {
  readonly request: string;
  readonly attempt: number;
  readonly executor: string;
  readonly source: Revision;
  readonly answers: Revision | null;
  readonly model: Revision;
  readonly requirements: Revision;
  /** Set on a work's first bundle, when it had a model or a list before. */
  readonly previous: { readonly model: Revision | null; readonly requirements: Revision | null } | null;
  /** The bundle this one replaced as the work's model and list; `null` for a work's first. */
  readonly replaces: Revision | null;
  /** The version of the working instructions its executor was given; `null` when it did not say. */
  readonly instructions: string | null;
  readonly checks: readonly BundleCheck[];
  readonly published_at: string;
}

/** `read_bundle`: the bundle and the revision it is, or `null` for a work that keeps none. */
export interface BundleRead {
  readonly revision: Revision;
  readonly bundle: Bundle;
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

/** A request an executor could not run, which it left queued, and why. */
export interface HostWaiting {
  readonly work: string;
  readonly request: string;
  readonly connection: string;
  readonly reason: string;
}

/**
 * A shell's word about the executor it hosts: that one runs in it, or why none does, in words the
 * owner can act on (what to install, what to set).
 */
export interface HostStatus {
  /** Which shell: `desktop`, `web-shell`. */
  readonly name: string;
  readonly hosting: boolean;
  readonly reason: string | null;
  readonly client_version: string | null;
  /**
   * The requests its executor left queued because it could not run them, and why, each in one
   * sentence. Ids and words only: the works folder is shared, so nothing of the computer is in it.
   */
  readonly waiting: readonly HostWaiting[];
  readonly seen_at: string;
  /** The shell said it recently: one that went is not hosting, whatever it last said. */
  readonly live: boolean;
}

/** `read_host_status`: every shell that reported, and every record that could not be read. */
export interface HostListing {
  readonly hosts: readonly HostStatus[];
  readonly unreadable: readonly Unreadable[];
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

/** What `requirements_report` and `read_judgment` both say of a measure, in the same words. */
export function parseMeasure(value: unknown, where: string): Measure {
  const r = record(value, where);
  const generator = r["generator"];
  if (generator !== null && typeof generator !== "string") {
    throw new ContractError(`${where}.generator`, "a string or null");
  }
  return {
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

/** `requirements_report`. */
export function parseRequirementsReport(value: unknown): RequirementsReport {
  const where = "requirements_report";
  const r = record(value, where);
  return {
    basis: parseBasis(r["basis"], `${where}.basis`),
    source_head: nullableRevision(r["source_head"], `${where}.source_head`),
    model_standing: standing(r["model_standing"], `${where}.model_standing`),
    requirements_standing: standing(r["requirements_standing"], `${where}.requirements_standing`),
    ...parseMeasure(value, where),
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

/**
 * `read_acceptance_delta`. A report the screen does not show yet (the revision report of a work
 * is for the client that revises it), so this holds the shape and nothing is read out of the
 * lines: all four parts are there together or none is, which is what a work nobody accepted says.
 */
export function parseReadAcceptanceDelta(value: unknown): ReadAcceptanceDelta {
  const where = "read_acceptance_delta";
  const r = record(value, where);
  if (r["acceptance"] === null) {
    for (const part of ["manifest", "lines", "now"]) {
      if (r[part] !== null) throw new ContractError(`${where}.${part}`, "null when nobody accepted");
    }
    return { acceptance: null, manifest: null, lines: null, now: null };
  }
  const pin = record(r["manifest"], `${where}.manifest`);
  const lines = r["lines"];
  if (!Array.isArray(lines)) throw new ContractError(`${where}.lines`, "a list of the product's lines");
  return {
    acceptance: parseAcceptanceRecord(r["acceptance"], `${where}.acceptance`),
    manifest: {
      doc_id: text(pin, "doc_id", `${where}.manifest`),
      rev: text(pin, "rev", `${where}.manifest`),
      sha256: text(pin, "sha256", `${where}.manifest`),
    },
    lines: lines.map((line, i) => record(line, `${where}.lines[${i}]`)),
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
    bundle: nullableRevision(r["bundle"], `${where}.bundle`),
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
  const candidate = r["candidate"];
  const outcome = r["outcome"];
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
    candidate: candidate === null ? null : parseRequestCandidate(candidate, `${where}.candidate`),
    outcome:
      outcome === null
        ? null
        : { bundle: revision(record(outcome, `${where}.outcome`)["bundle"], `${where}.outcome.bundle`) },
    pin: r["pin"] === null || r["pin"] === undefined ? null : parseRequestPin(r["pin"], `${where}.pin`),
    ended_at: nullableText(r, "ended_at", where),
    note: nullableText(r, "note", where),
  };
}

function parseRequestPin(value: unknown, where: string): RequestPin {
  const r = record(value, where);
  const limits = r["limits"] === undefined ? {} : record(r["limits"], `${where}.limits`);
  return {
    connection: text(r, "connection", where),
    revision: revision(r["revision"], `${where}.revision`),
    adapter: oneOf(r, "adapter", where, ["claude-code", "codex", "local"] as const),
    model: nullableText(r, "model", where),
    limits: {
      turns: nullableCount(limits, "turns", `${where}.limits`),
      seconds: nullableCount(limits, "seconds", `${where}.limits`),
    },
  };
}

function parseRequestCandidate(value: unknown, where: string): RequestCandidate {
  const r = record(value, where);
  return {
    model: nullableRevision(r["model"], `${where}.model`),
    requirements: nullableRevision(r["requirements"], `${where}.requirements`),
    instructions: nullableText(r, "instructions", where),
  };
}

/** `read_request`, `claim_request`, `heartbeat_request`, `save_request_candidate`, `fail_request`, `cancel_request`: the request. */
export function parseRequestReply(value: unknown): GenerationRequest {
  return parseGenerationRequest(record(value, "request reply")["request"], "request reply.request");
}

/** `complete_request`. */
export function parseCompletedRequest(value: unknown): CompletedRequest {
  const where = "complete_request";
  const r = record(value, where);
  const request = parseGenerationRequest(r["request"], `${where}.request`);
  const bundle = revision(r["bundle"], `${where}.bundle`);
  if (request.outcome === null || request.outcome.bundle !== bundle) {
    throw new ContractError(`${where}.request.outcome`, "the bundle the request made");
  }
  return { request, bundle };
}

function candidateText(value: unknown, where: string): { revision: Revision; text: string } | null {
  if (value === null) return null;
  const r = record(value, where);
  return { revision: revision(r["revision"], `${where}.revision`), text: text(r, "text", where) };
}

/** `read_request_candidate`. */
export function parseRequestCandidateTexts(value: unknown): RequestCandidateTexts {
  const where = "read_request_candidate";
  const r = record(value, where);
  return {
    request: text(r, "request", where),
    model: candidateText(r["model"], `${where}.model`),
    requirements: candidateText(r["requirements"], `${where}.requirements`),
  };
}

function parseBundleCheck(value: unknown, where: string): BundleCheck {
  const r = record(value, where);
  const by = r["by"];
  if (by !== "core" && by !== "client") throw new ContractError(`${where}.by`, `"core" or "client"`);
  const verdict = r["verdict"];
  if (verdict !== "accepted" && verdict !== "refused") {
    throw new ContractError(`${where}.verdict`, `"accepted" or "refused"`);
  }
  return {
    by,
    name: text(r, "name", where),
    verdict,
    generator: nullableText(r, "generator", where),
    digest: nullableText(r, "digest", where),
    subject: nullableRevision(r["subject"], `${where}.subject`),
  };
}

function parseBundle(value: unknown, where: string): Bundle {
  const r = record(value, where);
  const previous = r["previous"];
  const earlier = previous === null ? null : record(previous, `${where}.previous`);
  return {
    request: text(r, "request", where),
    attempt: count(r, "attempt", where),
    executor: text(r, "executor", where),
    source: revision(r["source"], `${where}.source`),
    answers: nullableRevision(r["answers"], `${where}.answers`),
    model: revision(r["model"], `${where}.model`),
    requirements: revision(r["requirements"], `${where}.requirements`),
    previous:
      earlier === null
        ? null
        : {
            model: nullableRevision(earlier["model"], `${where}.previous.model`),
            requirements: nullableRevision(earlier["requirements"], `${where}.previous.requirements`),
          },
    replaces: nullableRevision(r["replaces"], `${where}.replaces`),
    instructions: nullableText(r, "instructions", where),
    checks: list(r, "checks", where).map((c, i) => parseBundleCheck(c, `${where}.checks[${i}]`)),
    published_at: text(r, "published_at", where),
  };
}

/** `read_bundle`: `null` for a work that keeps no bundle. */
export function parseBundleRead(value: unknown): BundleRead | null {
  const where = "read_bundle";
  const found = record(value, where)["bundle"];
  if (found === null) return null;
  const r = record(found, `${where}.bundle`);
  return {
    revision: revision(r["revision"], `${where}.bundle.revision`),
    bundle: parseBundle(r["bundle"], `${where}.bundle.bundle`),
  };
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

/** `read_host_status`. */
export function parseHostListing(value: unknown): HostListing {
  const where = "read_host_status";
  const r = record(value, where);
  return {
    hosts: list(r, "hosts", where).map((h, i) => {
      const at = `${where}.hosts[${i}]`;
      const host = record(h, at);
      if (typeof host["hosting"] !== "boolean") throw new ContractError(`${at}.hosting`, "true or false");
      if (typeof host["live"] !== "boolean") throw new ContractError(`${at}.live`, "true or false");
      return {
        name: text(host, "name", at),
        hosting: host["hosting"],
        reason: nullableText(host, "reason", at),
        client_version: nullableText(host, "client_version", at),
        waiting: list(host, "waiting", at).map((entry, k) => {
          const place = `${at}.waiting[${k}]`;
          const waiting = record(entry, place);
          return {
            work: text(waiting, "work", place),
            request: text(waiting, "request", place),
            connection: text(waiting, "connection", place),
            reason: text(waiting, "reason", place),
          };
        }),
        seen_at: text(host, "seen_at", at),
        live: host["live"],
      };
    }),
    unreadable: list(r, "unreadable", where).map((u, i) => {
      const at = `${where}.unreadable[${i}]`;
      const entry = record(u, at);
      return { id: text(entry, "id", at), reason: text(entry, "reason", at) };
    }),
  };
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
    bundle: nullableRevision(r["bundle"], `${where}.bundle`),
  };
}

function parseRefusal(value: unknown, where: string): Refusal {
  const r = record(value, where);
  return { kind: text(r, "kind", where), message: text(r, "message", where), code: nullableText(r, "code", where) };
}

/** SCE's word, or that it did not answer: `{"said": ...}` or `{"refused": ...}`, never both and never neither. */
function parseJudged<T>(value: unknown, where: string, said: (value: unknown, where: string) => T): Judged<T> {
  const r = record(value, where);
  const hasSaid = "said" in r;
  const hasRefused = "refused" in r;
  if (hasSaid === hasRefused) throw new ContractError(where, 'exactly one of "said" and "refused"');
  if (hasRefused) return { said: false, refusal: parseRefusal(r["refused"], `${where}.refused`) };
  return { said: true, value: said(r["said"], `${where}.said`) };
}

function parseAcceptanceVerdict(value: unknown, where: string): AcceptanceVerdict {
  const r = record(value, where);
  const state = r["standing"];
  if (state !== "holds" && state !== "lapsed") throw new ContractError(`${where}.standing`, '"holds" or "lapsed"');
  const lapse = nullableText(r, "lapse", where);
  if ((state === "lapsed") !== (lapse !== null)) {
    throw new ContractError(`${where}.lapse`, "SCE's sentence exactly when the acceptance lapsed");
  }
  return { standing: state, lapse };
}

/** Whether two bases name the same revision of the text, the model, the list and the answers. */
export function sameBasis(a: Basis, b: Basis): boolean {
  return a.source === b.source && a.model === b.model && a.requirements === b.requirements && a.answers === b.answers;
}

/**
 * `read_judgment`. That the judgment names the revisions that were asked about is the caller's
 * to hold it to (`Api.readJudgment`), since only the caller knows what it asked.
 */
export function parseJudgment(value: unknown): Judgment {
  const where = "read_judgment";
  const r = record(value, where);
  const accepted = r["acceptance"];
  return {
    basis: parseBasis(r["basis"], `${where}.basis`),
    acceptance: accepted === null ? null : parseJudged(accepted, `${where}.acceptance`, parseAcceptanceVerdict),
    report: parseJudged(r["report"], `${where}.report`, parseMeasure),
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
    entrance: oneOf(r, "entrance", "describe", ["desktop", "browser", "tool"] as const),
    settings: flag(r, "settings", "describe"),
    writes_settings: flag(r, "writes_settings", "describe"),
    starts_programs: flag(r, "starts_programs", "describe"),
  };
}

function flag(value: Obj, key: string, where: string): boolean {
  const field = value[key];
  if (typeof field !== "boolean") throw new ContractError(`${where}.${key}`, "true or false");
  return field;
}

function oneOf<T extends string>(value: Obj, key: string, where: string, words: readonly T[]): T {
  const field = value[key];
  const found = words.find((word) => word === field);
  if (found === undefined) {
    throw new ContractError(`${where}.${key}`, words.map((w) => `"${w}"`).join(", "));
  }
  return found;
}

function nullableCount(value: Obj, key: string, where: string): number | null {
  const field = value[key];
  if (field === undefined || field === null) return null;
  if (typeof field !== "number" || !Number.isInteger(field)) {
    throw new ContractError(`${where}.${key}`, "a whole number");
  }
  return field;
}

function parseConnection(value: unknown, where: string): Connection {
  const r = record(value, where);
  const limits = r["limits"] === undefined ? {} : record(r["limits"], `${where}.limits`);
  return {
    id: text(r, "id", where),
    adapter: oneOf(r, "adapter", where, ["claude-code", "codex", "local"] as const),
    display_name: nullableText(r, "display_name", where),
    executable: nullableText(r, "executable", where),
    model: nullableText(r, "model", where),
    auth: oneOf(r, "auth", where, ["official-login", "app-store", "env-api-key", "server-key", "none"] as const),
    server_url: nullableText(r, "server_url", where),
    limits: {
      turns: nullableCount(limits, "turns", `${where}.limits`),
      seconds: nullableCount(limits, "seconds", `${where}.limits`),
    },
  };
}

function parseStoredConnection(value: unknown, where: string): StoredConnection {
  const r = record(value, where);
  return {
    connection: parseConnection(r["connection"], `${where}.connection`),
    revision: revision(r["revision"], `${where}.revision`),
  };
}

/** `list_connections`. */
export function parseConnectionListing(value: unknown): ConnectionListing {
  const where = "list_connections";
  const r = record(value, where);
  const chosen = r["default"];
  if (chosen !== null && typeof chosen !== "string") {
    throw new ContractError(`${where}.default`, "a connection id or null");
  }
  return {
    connections: list(r, "connections", where).map((c, i) => parseStoredConnection(c, `${where}.connections[${i}]`)),
    unreadable: list(r, "unreadable", where).map((u, i) => {
      const at = `${where}.unreadable[${i}]`;
      const entry = record(u, at);
      return { id: text(entry, "id", at), reason: text(entry, "reason", at) };
    }),
    default: chosen,
  };
}

/** `read_connection`: the connection, or `null` for one that is not there. */
export function parseReadConnection(value: unknown): StoredConnection | null {
  const stored = record(value, "read_connection")["connection"];
  return stored === null ? null : parseStoredConnection(stored, "read_connection.connection");
}

/** `delete_connection`: the id of the connection that was taken out of the list. */
export function parseDeletedConnection(value: unknown): string {
  return text(record(value, "delete_connection"), "deleted", "delete_connection");
}

/** `set_default_connection`: the id that is the default now, or `null` for none. */
export function parseDefaultConnection(value: unknown): string | null {
  const chosen = record(value, "set_default_connection")["default"];
  if (chosen !== null && typeof chosen !== "string") {
    throw new ContractError("set_default_connection.default", "a connection id or null");
  }
  return chosen;
}

const ROUTE_STATUSES = ["allowed", "conditional", "forbidden", "unconfirmed"] as const;

function parseRouteDecision(value: unknown, where: string): RouteDecision {
  const r = record(value, where);
  const status = oneOf(r, "status", where, ROUTE_STATUSES);
  const decision = oneOf(r, "decision", where, ["use", "refuse"] as const);
  if (decision === "use") return { decision, status };
  return { decision, status, reason: oneOf(r, "reason", where, ["forbidden", "unconfirmed", "switched-off"] as const) };
}

/** `read_auth_policy`. */
export function parseAuthPolicy(value: unknown): AuthPolicy {
  const where = "read_auth_policy";
  const r = record(value, where);
  return {
    routes: list(r, "routes", where).map((entry, i) => {
      const at = `${where}.routes[${i}]`;
      const route = record(entry, at);
      return {
        route: text(route, "route", at),
        status: oneOf(route, "status", at, ROUTE_STATUSES),
        decision: parseRouteDecision(route["decision"], `${at}.decision`),
      };
    }),
    switched_off: stringList(r, "switched_off", where),
  };
}

const BILLINGS = ["subscription", "usage", "provider"] as const;

/** `read_claude_status`. */
export function parseClaudeStatus(value: unknown): ClaudeStatus {
  const where = "read_claude_status";
  const claude = record(record(value, where)["claude"], `${where}.claude`);
  const clientAt = `${where}.client`;
  const client = record(claude["client"], clientAt);
  const accountAt = `${where}.account`;
  const account = record(claude["account"], accountAt);
  return {
    client: parseClaudeClient(client, clientAt),
    account: parseClaudeAccount(account, accountAt),
    sign_in: list(claude, "sign_in", where).map((entry, i) => {
      const at = `${where}.sign_in[${i}]`;
      const command = record(entry, at);
      return { billing: oneOf(command, "billing", at, BILLINGS), command: text(command, "command", at) };
    }),
  };
}

function parseClaudeClient(client: Obj, where: string): ClaudeClient {
  const state = oneOf(client, "state", where, ["installed", "unverified", "missing"] as const);
  return state === "installed"
    ? { state, version: text(client, "version", where), path: text(client, "path", where) }
    : { state };
}

/** A program the application found that says it is Claude Code: what a connection may name. */
export interface Candidate {
  readonly path: string;
  readonly version: string;
  /** On the search path, or in a folder the official installer uses. */
  readonly found: "search-path" | "known-location";
}

/** `find_clients`: the programs of each client the application found, in the order it found them. */
export function parseFindClients(value: unknown): FoundClients {
  const where = "find_clients";
  const reply = record(value, where);
  const candidates = (client: "claude" | "codex"): Candidate[] =>
    list(reply, client, where).map((entry, i) => {
      const at = `${where}.${client}[${i}]`;
      const candidate = record(entry, at);
      return {
        path: text(candidate, "path", at),
        version: text(candidate, "version", at),
        found: oneOf(candidate, "found", at, ["search-path", "known-location"] as const),
      };
    });
  return { claude: candidates("claude"), codex: candidates("codex") };
}

/** `read_server_status`. */
export function parseServerStatus(value: unknown): ServerStatus {
  const where = "read_server_status";
  const at = `${where}.server`;
  const server = record(record(value, where)["server"], at);
  const common = {
    address: text(server, "address", at),
    reach: oneOf(server, "reach", at, ["this-computer", "network"] as const),
    tls: flag(server, "tls", at),
  };
  const state = oneOf(server, "state", at, [
    "listed",
    "unreachable",
    "certificate",
    "needs-key",
    "not-a-model-list",
  ] as const);
  if (state === "listed") {
    const models = list(server, "models", at).map((model, i) => {
      if (typeof model !== "string") throw new ContractError(`${at}.models[${i}]`, "text");
      return model;
    });
    return { ...common, state, models };
  }
  return { ...common, state, reason: text(server, "reason", at) };
}

const CODEX_SOURCES = ["official-login", "app-store", "env-api-key"] as const;

/** `read_codex_status`. */
export function parseCodexStatus(value: unknown): CodexStatus {
  const where = "read_codex_status";
  const codex = record(record(value, where)["codex"], `${where}.codex`);
  const clientAt = `${where}.client`;
  const supportAt = `${where}.support`;
  const support = record(codex["support"], supportAt);
  const state = oneOf(support, "state", supportAt, ["verified", "unverified", "unknown"] as const);
  return {
    client: parseClaudeClient(record(codex["client"], clientAt), clientAt),
    support: state === "verified" ? { state } : { state, reason: text(support, "reason", supportAt) },
    accounts: list(codex, "accounts", where).map((entry, i) => {
      const at = `${where}.accounts[${i}]`;
      const account = record(entry, at);
      return { ...parseClaudeAccount(account, at), source: oneOf(account, "source", at, CODEX_SOURCES) };
    }),
    sign_in: list(codex, "sign_in", where).map((entry, i) => {
      const at = `${where}.sign_in[${i}]`;
      const command = record(entry, at);
      return {
        source: oneOf(command, "source", at, CODEX_SOURCES),
        billing: oneOf(command, "billing", at, BILLINGS),
        command: text(command, "command", at),
        home: nullableText(command, "home", at),
      };
    }),
    key_variable: text(codex, "key_variable", where),
  };
}

function parseClaudeAccount(account: Obj, where: string): ClaudeAccount {
  const state = oneOf(account, "state", where, ["signed-in", "signed-out", "unknown"] as const);
  if (state === "signed-out") return { state };
  if (state === "unknown") return { state, reason: text(account, "reason", where) };
  const billing = account["billing"];
  return {
    state,
    route: text(account, "route", where),
    billing: billing === null || billing === undefined ? null : oneOf(account, "billing", where, BILLINGS),
    environment: nullableText(account, "environment", where),
    usable: flag(account, "usable", where),
    decision: parseRouteDecision(account["decision"], `${where}.decision`),
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

/**
 * What a `moved` refusal says moved (`source`, `answers`, `connection`), as the core names it.
 * A refusal that names nothing, or names it in another shape, names no part: it is not a guess
 * at which one it was.
 */
export function movedInRefusal(detail: unknown): string[] {
  if (typeof detail !== "object" || detail === null) return [];
  const moved = (detail as Obj)["moved"];
  return Array.isArray(moved) ? moved.filter((part): part is string => typeof part === "string") : [];
}

/** The two revisions a `conflict` carries: the one the caller wrote from, and the current one. */
export function conflictRevisions(detail: unknown): { base: Revision | null; current: Revision | null } {
  const r = record(detail, "conflict.detail");
  return {
    base: nullableRevision(r["base"] ?? null, "conflict.detail.base"),
    current: nullableRevision(r["current"] ?? null, "conflict.detail.current"),
  };
}
