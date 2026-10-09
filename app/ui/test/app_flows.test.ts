// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// @vitest-environment jsdom

// The screen's own order of events: what happens when an answer arrives after the
// person has moved on, when text is not saved, and when a save succeeds and the
// read after it does not.
//
// The pure editor model is tested elsewhere and cannot see any of this: these
// failures live in the gaps between a request and its answer, which only the real
// `App`, driven through its buttons over a core whose answers can be held back,
// has. They were found by an external review of the first version, each by a
// sequence a person can perform, and each is a rule over the editor and not over a
// pair of works, so the cases below vary the sequence and not only the names.

import { beforeEach, describe, expect, it } from "vitest";

import { App } from "../src/app";
import { SUPPORTED_COMMAND_SET_VERSION } from "../src/contract";
import { CommandFailure, UNAUTHORIZED, type Args, type Transport } from "../src/ipc";
import type { Ticker } from "../src/watch";

const hex = (n: number): string => n.toString(16).padStart(64, "0");

interface Revision {
  revision: string;
  text: string;
}

/** A request as the fake core keeps it. */
interface FakeRequest {
  id: string;
  seq: number;
  key: string;
  state: string;
  attempt: number;
  source: string;
  answers: string | null;
  holder: string | null;
  note: string | null;
  /** The connection it was made for, as the core would copy it, or none. */
  pin: { connection: string; revision: string; model: string | null } | null;
}

interface Gate {
  name: string;
  match: (args: Args) => boolean;
  opened: Promise<void>;
  used: boolean;
}

/** A core in memory, whose answers the test can hold back and let go of. */
class FakeCore implements Transport {
  readonly calls: Array<{ name: string; args: Args }> = [];
  private readonly works = new Map<string, { title: string; revisions: Revision[] }>();
  private readonly revisionOf = new Map<string, string>();
  private readonly gates: Gate[] = [];
  private readonly failures: Array<{ name: string; error: CommandFailure }> = [];
  private readonly models = new Map<
    string,
    { text: string; writtenFor: string | null; others: ReadonlyArray<{ name: string; text: string }> }
  >();
  private readonly answersOf = new Map<
    string,
    { revision: string; entries: Record<string, { answer: string; answered_at: string }> }
  >();
  private answerSaves = 0;
  private readonly lists = new Map<string, { revision: string; writtenFor: string | null; ids: string[] }>();
  /** Every list a work had, by its revision: a revision is never rewritten, so SCE can be asked about an earlier one. */
  private readonly listsByRevision = new Map<string, { revision: string; writtenFor: string | null; ids: string[] }>();
  private readonly acceptances = new Map<
    string,
    { revision: string; basis: Record<string, string>; channel: string; open: string[] }
  >();
  private readonly acceptancesByRevision = new Map<
    string,
    { revision: string; basis: Record<string, string>; channel: string; open: string[] }
  >();
  /** What SCE does not answer the next time it is asked, by the part of the judgment it is about. */
  private readonly refusals: Array<{ part: "report" | "acceptance"; kind: string; message: string }> = [];
  private listSaves = 0;
  private acceptSaves = 0;
  private readonly requestsOf = new Map<string, FakeRequest[]>();
  /** Every model a work had, newest last, and each by its revision: what `model_history` and a review of an earlier one read. */
  private readonly modelHistoryOf = new Map<string, string[]>();
  private readonly modelsByRevision = new Map<
    string,
    { text: string; writtenFor: string | null; others: ReadonlyArray<{ name: string; text: string }> }
  >();
  /** Where each requirement of a work is carried, and the sentence the list quotes for it, when a test says. */
  private readonly carriedOf = new Map<string, Record<string, string[]>>();
  private readonly quotesOf = new Map<string, Record<string, string>>();

  /** The places of the design each requirement is carried by, as SCE would say them. */
  setCarried(id: string, carried: Record<string, string[]>): void {
    this.carriedOf.set(id, carried);
  }

  /** The sentences the work's requirement list quotes, by requirement: its sidecar. */
  setQuotes(id: string, quotes: Record<string, string>): void {
    this.quotesOf.set(id, quotes);
  }

  /** Every revision the owner's answers had, so that a request's own can be read back. */
  private readonly answersByRevision = new Map<
    string,
    { revision: string; entries: Record<string, { answer: string; answered_at: string }> }
  >();
  private readonly bundleOf = new Map<
    string,
    { revision: string; request: string; source: string; answers: string | null; model: string; requirements: string }
  >();
  private adapterList: Array<{ name: string; kind: string; capabilities: string[]; live: boolean }> = [];
  private requestSeq = 0;

  /** The AI adapters the core says are there. */
  setAdapters(list: Array<{ name: string; capabilities?: string[]; live?: boolean }>): void {
    this.adapterList = list.map((a) => ({
      name: a.name,
      kind: "claude-code",
      capabilities: a.capabilities ?? ["generate", "cancel"],
      live: a.live ?? true,
    }));
  }

  private hostList: Array<{
    name: string;
    hosting: boolean;
    reason: string | null;
    live: boolean;
    waiting: Array<{ work: string; request: string; connection: string; reason: string }>;
  }> = [];

  /**
   * What the shells say of the executor they host: that one runs, or why none does, and which
   * requests it left queued because it could not run them.
   */
  setHosts(
    list: Array<{
      name: string;
      hosting?: boolean;
      reason?: string | null;
      live?: boolean;
      waiting?: Array<{ work: string; request: string; connection: string; reason: string }>;
    }>,
  ): void {
    this.hostList = list.map((h) => ({
      name: h.name,
      hosting: h.hosting ?? false,
      reason: h.reason ?? null,
      live: h.live ?? true,
      waiting: h.waiting ?? [],
    }));
  }

  private connectionList: Array<{ id: string; model: string | null; revision: string }> = [];
  private defaultConnection: string | null = null;
  private claudeStatus: unknown = {
    claude: {
      client: { state: "installed", version: "2.1.291", path: "/home/me/.local/bin/claude" },
      account: { state: "signed-out" },
      sign_in: [
        { billing: "subscription", command: "claude auth login" },
        { billing: "usage", command: "claude auth login --console" },
      ],
    },
  };

  /** The connections the person keeps, and which is the default. */
  setConnections(list: Array<{ id: string; model: string | null; revision: string }>, defaultId: string | null): void {
    this.connectionList = list;
    this.defaultConnection = defaultId;
  }

  /** What Claude Code is, and who is signed in to it, as the core says it. */
  setClaudeStatus(status: unknown): void {
    this.claudeStatus = status;
  }

  /** The latest request of a work, as the core keeps it. */
  latestRequest(id: string): FakeRequest | undefined {
    return this.requestsOf.get(id)?.at(-1);
  }

  /** An executor takes the request: it is running, held by `holder`. */
  takeRequest(id: string, holder = "desktop"): void {
    const request = this.latestRequest(id);
    if (request === undefined) throw new Error(`${id} has no request`);
    request.state = "running";
    request.attempt += 1;
    request.holder = holder;
  }

  /** The executor stops answering: its lease ran out, and nothing was written. */
  letGoOfRequest(id: string): void {
    const request = this.latestRequest(id);
    if (request === undefined) throw new Error(`${id} has no request`);
    request.state = "interrupted";
  }

  /** The executor could not, and says why. */
  failRequest(id: string, note: string): void {
    const request = this.latestRequest(id);
    if (request === undefined) throw new Error(`${id} has no request`);
    request.state = "failed";
    request.note = note;
  }

  /** The executor is done and the core published what it wrote: the model and the list, for the text the request was about. */
  completeRequest(id: string, model = "<scxml><!-- generated --></scxml>"): void {
    const request = this.latestRequest(id);
    if (request === undefined) throw new Error(`${id} has no request`);
    request.state = "completed";
    this.setModel(id, model, request.source);
    this.setRequirements(id, request.source);
    // The bundle names what the request was about, as the core records it.
    this.bundleOf.set(id, {
      revision: this.revision(`bundle:${request.id}`),
      request: request.id,
      source: request.source,
      answers: request.answers,
      model: this.revision(`model:${model}`),
      requirements: this.lists.get(id)?.revision ?? this.revision("list"),
    });
  }

  /** A save of the text or the answers ends the request that was open, in the same step. */
  private supersedeOpen(id: string): void {
    const request = this.latestRequest(id);
    if (request !== undefined && ["queued", "running", "interrupted"].includes(request.state)) {
      request.state = "superseded";
    }
  }

  private requestJson(id: string, request: FakeRequest): unknown {
    return {
      id: request.id,
      work: id,
      seq: request.seq,
      key: request.key,
      origin: "gui",
      state: request.state,
      stored_state: request.state,
      attempt: request.attempt,
      inputs: { source: request.source, answers: request.answers },
      created_at: "2026-10-05T09:00:00Z",
      lease:
        request.holder === null
          ? null
          : { holder: request.holder, attempt: request.attempt, granted_at: "2026-10-05T09:00:01Z", expires_at: "2026-10-05T09:01:01Z" },
      candidate: null,
      outcome: null,
      pin:
        request.pin === null
          ? null
          : {
              connection: request.pin.connection,
              revision: request.pin.revision,
              adapter: "claude-code",
              model: request.pin.model,
              limits: { turns: null, seconds: null },
            },
      ended_at: null,
      note: request.note,
    };
  }

  /**
   * The requirement list an authoring client saved for a work, and the text revision it
   * says it read. A later call is a new revision of the list.
   */
  setRequirements(id: string, writtenFor: string | null, ids: string[] = ["R1", "R2"]): void {
    this.listSaves += 1;
    const list = { revision: this.revision(`list:${this.listSaves}`), writtenFor, ids };
    this.lists.set(id, list);
    this.listsByRevision.set(list.revision, list);
  }

  /** What the revision report says the revision stayed within the reach of, when it compares at all. */
  revisionVerdict: "within-reach" | "outside-reach" = "within-reach";
  /** Whether the report compared anything: a design that cites no requirement gives it nothing to compare. */
  revisionCompares = true;

  /** What the owner accepted, as the core keeps it: the work as it stands now, stated on `channel`. */
  setAcceptance(id: string, channel = "direct"): void {
    const basis = this.basisOf(id);
    if (basis === null) throw new Error(`${id} has no text, model and list to accept`);
    this.acceptSaves += 1;
    this.keepAcceptance(id, {
      revision: this.revision(`acceptance:${this.acceptSaves}`),
      basis,
      channel,
      open: ["1 question(s) the specification leaves open (open-guard)"],
    });
  }

  private keepAcceptance(
    id: string,
    record: { revision: string; basis: Record<string, string>; channel: string; open: string[] },
  ): void {
    this.acceptances.set(id, record);
    this.acceptancesByRevision.set(record.revision, record);
  }

  private commandSet: number = SUPPORTED_COMMAND_SET_VERSION;

  /** The command set this core says it speaks: another than the screen's, to see the screen refuse it. */
  describeAs(version: number): void {
    this.commandSet = version;
  }

  /** The same list kept again for another text: the revision is the same, and so is what SCE says of it. */
  keepRequirementsFor(id: string, writtenFor: string | null): void {
    const list = this.lists.get(id);
    if (list === undefined) throw new Error(`${id} has no requirement list`);
    list.writtenFor = writtenFor;
  }

  /** SCE does not answer the next judgment's `part` (`report`: the measure; `acceptance`: whether it holds). */
  sceRefuses(part: "report" | "acceptance", kind: string, message: string): void {
    this.refusals.push({ part, kind, message });
  }

  /** The revisions of everything an acceptance is about, or `null` while one of them is missing. */
  private basisOf(id: string): Record<string, string> | null {
    const source = this.works.get(id)?.revisions.at(-1)?.revision;
    const model = this.models.get(id);
    const list = this.lists.get(id);
    if (source === undefined || model === undefined || list === undefined) return null;
    const answers = this.answersOf.get(id)?.revision;
    return {
      source,
      model: this.revision(`model:${model.text}`),
      requirements: list.revision,
      ...(answers === undefined ? {} : { answers }),
    };
  }

  /** What an acceptance lapsed over, in the product's one sentence. */
  private lapseOf(then: Record<string, string>, now: Record<string, string>): string | null {
    const files: Array<[string, string]> = [
      ["design/model.scxml", "model"],
      ["spec/requirements.manifest.json", "requirements"],
      ["spec/source.txt", "source"],
      ["spec/answers.json", "answers"],
    ];
    const moved = files.filter(([, key]) => then[key] !== now[key]).map(([path]) => `${path} moved`);
    return moved.length === 0 ? null : moved.join("; ");
  }

  /** The owner's answers as saved, from another entrance or an earlier session. */
  setAnswers(id: string, entries: Record<string, string>): void {
    this.answerSaves += 1;
    const held = {
      revision: this.revision(`answers:${this.answerSaves}`),
      entries: Object.fromEntries(
        Object.entries(entries).map(([question, answer]) => [question, { answer, answered_at: "2026-10-03T09:00:00Z" }]),
      ),
    };
    this.answersOf.set(id, held);
    this.answersByRevision.set(held.revision, held);
  }

  /** What the core holds of the owner's answers, as words by question. */
  answersHeld(id: string): Record<string, string> {
    return Object.fromEntries(Object.entries(this.answersOf.get(id)?.entries ?? {}).map(([q, e]) => [q, e.answer]));
  }

  addWork(id: string, title: string, texts: string[]): void {
    this.works.set(id, { title, revisions: texts.map((text) => ({ revision: this.revision(text), text })) });
  }

  /** A text saved from another entrance: a new head, as `save_source` makes one. */
  saveElsewhere(id: string, text: string): void {
    this.works.get(id)?.revisions.push({ revision: this.revision(text), text });
  }

  /**
   * The work's model, and the text revision its writer says it read. `others` makes it
   * a model of several documents: `text` is then the entry `door.scxml`, and each of
   * `others` is a document it imports.
   */
  setModel(
    id: string,
    text: string,
    writtenFor: string | null,
    others: ReadonlyArray<{ name: string; text: string }> = [],
  ): void {
    this.models.set(id, { text, writtenFor, others });
    const revision = this.revision(`model:${text}`);
    this.modelsByRevision.set(revision, { text, writtenFor, others });
    this.modelHistoryOf.set(id, [...(this.modelHistoryOf.get(id) ?? []), revision]);
  }

  /** The SVG this core draws for a model, so a test can look for it on the screen. */
  figureSvg(text: string): string {
    return `<svg xmlns="http://www.w3.org/2000/svg" width="40pt" height="20pt"><text x="0" y="10">picture of ${text}</text></svg>`;
  }

  revision(text: string): string {
    if (!this.revisionOf.has(text)) this.revisionOf.set(text, hex(this.revisionOf.size + 1));
    return this.revisionOf.get(text) as string;
  }

  headText(id: string): string | undefined {
    return this.works.get(id)?.revisions.at(-1)?.text;
  }

  /** Hold the next call of `name` that `match`es until `release` is called. */
  hold(name: string, match: (args: Args) => boolean = () => true): { release: () => void } {
    let release = (): void => undefined;
    const opened = new Promise<void>((resolve) => {
      release = resolve;
    });
    this.gates.push({ name, match, opened, used: false });
    return { release: () => release() };
  }

  /** Make the next call of `name` fail. */
  failNext(name: string, error: CommandFailure): void {
    this.failures.push({ name, error });
  }

  callsOf(name: string): Array<Args> {
    return this.calls.filter((c) => c.name === name).map((c) => c.args);
  }

  async call(name: string, args: Args = {}): Promise<unknown> {
    this.calls.push({ name, args });
    const failure = this.failures.findIndex((f) => f.name === name);
    if (failure >= 0) throw this.failures.splice(failure, 1)[0]?.error;
    const gate = this.gates.find((g) => !g.used && g.name === name && g.match(args));
    if (gate !== undefined) {
      gate.used = true;
      await gate.opened;
    }
    return this.answer(name, args);
  }

  private answer(name: string, args: Args): unknown {
    const work = typeof args["id"] === "string" ? this.works.get(args["id"]) : undefined;
    switch (name) {
      case "describe":
        return {
          command_set_version: this.commandSet,
          commands: [],
          root: "/fake/works",
          entrance: "desktop",
          settings: true,
          writes_settings: true,
          starts_programs: true,
        };
      case "read_work_heads": {
        if (work === undefined) throw new CommandFailure("not-found", "work `absent`");
        const id = String(args["id"]);
        const model = this.models.get(id);
        const list = this.lists.get(id);
        return {
          source: work?.revisions.at(-1)?.revision ?? null,
          model: model === undefined ? null : { revision: this.revision(`model:${model.text}`), written_for: model.writtenFor },
          answers: this.answersOf.get(id)?.revision ?? null,
          requirements: list === undefined ? null : { revision: list.revision, written_for: list.writtenFor },
          acceptance: this.acceptances.get(id)?.revision ?? null,
          bundle: this.bundleOf.get(id)?.revision ?? null,
          request: ((r) => (r === undefined ? null : { id: r.id, state: r.state, attempt: r.attempt }))(this.latestRequest(id)),
        };
      }
      case "request_generation": {
        if (work === undefined) throw new CommandFailure("not-found", "work `absent`");
        const id = String(args["id"]);
        const expect = args["expect"] as { source: string; answers: string | null };
        const same = this.requestsOf.get(id)?.find((r) => r.key === args["key"]);
        if (same !== undefined) return { request: this.requestJson(id, same), created: false };
        const head = work.revisions.at(-1)?.revision ?? null;
        const answers = this.answersOf.get(id)?.revision ?? null;
        const moved = [
          ...(expect.source === head ? [] : ["source"]),
          ...((expect.answers ?? null) === answers ? [] : ["answers"]),
        ];
        if (moved.length > 0) {
          throw new CommandFailure("moved", `${moved.join(", ")} moved since you read them`, { moved });
        }
        const named = args["connection"] as { id: string; revision: string } | undefined;
        if (named !== undefined) {
          const kept = this.connectionList.find((c) => c.id === named.id);
          if (kept === undefined || kept.revision !== named.revision) {
            throw new CommandFailure("moved", "the connection was changed after it was read", {
              moved: ["connection"],
            });
          }
        }
        const open = this.latestRequest(id);
        if (open !== undefined && ["queued", "running", "interrupted"].includes(open.state)) {
          if (args["supersede"] !== true) {
            throw new CommandFailure("active-request", "the work already has an open request", { open: open.id });
          }
          open.state = "superseded";
        }
        this.requestSeq += 1;
        const request: FakeRequest = {
          id: `req-${this.requestSeq}`,
          seq: this.requestSeq,
          key: String(args["key"]),
          state: "queued",
          attempt: 0,
          source: expect.source,
          answers: expect.answers ?? null,
          holder: null,
          note: null,
          pin:
            named === undefined
              ? null
              : {
                  connection: named.id,
                  revision: named.revision,
                  model: this.connectionList.find((c) => c.id === named.id)?.model ?? null,
                },
        };
        this.requestsOf.set(id, [...(this.requestsOf.get(id) ?? []), request]);
        return { request: this.requestJson(id, request), created: true };
      }
      case "read_request": {
        const id = String(args["id"]);
        const found = this.requestsOf.get(id)?.find((r) => r.id === args["request"]);
        if (found === undefined) throw new CommandFailure("not-found", "no such request");
        return { request: this.requestJson(id, found) };
      }
      case "cancel_request": {
        const id = String(args["id"]);
        const found = this.requestsOf.get(id)?.find((r) => r.id === args["request"]);
        if (found === undefined) throw new CommandFailure("not-found", "no such request");
        if (!["queued", "running", "interrupted"].includes(found.state)) {
          throw new CommandFailure("request-ended", `the request is ${found.state}`, { state: found.state });
        }
        found.state = "cancelled";
        return { request: this.requestJson(id, found) };
      }
      case "list_connections":
        return {
          connections: this.connectionList.map((c) => ({
            connection: {
              id: c.id,
              adapter: "claude-code",
              display_name: null,
              executable: null,
              model: c.model,
              auth: "official-login",
              server_url: null,
              limits: { turns: null, seconds: null },
            },
            revision: c.revision,
          })),
          unreadable: [],
          default: this.defaultConnection,
        };
      case "read_claude_status":
        return this.claudeStatus;
      case "find_clients":
        return {
          claude: [{ path: "/home/me/.local/bin/claude", version: "2.1.291", found: "search-path" }],
          codex: [],
        };
      case "read_host_status":
        return {
          hosts: this.hostList.map((h) => ({
            name: h.name,
            hosting: h.hosting,
            reason: h.reason,
            client_version: h.hosting ? "2.1.289" : null,
            waiting: h.waiting,
            seen_at: "2026-10-05T09:00:00Z",
            live: h.live,
          })),
          unreadable: [],
        };
      case "read_adapter_status":
        return {
          adapters: this.adapterList.map((a) => ({
            name: a.name,
            kind: a.kind,
            capabilities: a.capabilities,
            version: "2.1",
            seen_at: "2026-10-05T09:00:00Z",
            live: a.live,
          })),
          unreadable: [],
        };
      case "read_work_snapshot": {
        // One synchronous step over the core's state: what a snapshot is.
        if (work === undefined) throw new CommandFailure("not-found", `work \`${String(args["id"])}\``);
        const id = String(args["id"]);
        const model = this.answer("read_model", { id }) as { model: unknown; standing: string | null };
        const head = work.revisions.at(-1);
        const list = this.lists.get(id);
        const held = this.acceptances.get(id);
        return {
          work: { id, title: work.title, created_at: "2026-10-03T09:00:00Z" },
          source: head === undefined ? null : { revision: head.revision, text: head.text },
          model: model.model,
          model_standing: model.standing,
          answers: this.answersOf.get(id) ?? null,
          requirements:
            list === undefined
              ? null
              : {
                  revision: list.revision,
                  written_for: list.writtenFor,
                  manifest: "{}",
                  sidecar:
                    this.quotesOf.get(id) === undefined
                      ? null
                      : JSON.stringify({ doc_id: "door", rev: "1", text: this.quotesOf.get(id) }),
                },
          requirements_standing: list === undefined ? null : standingOf(list.writtenFor, head?.revision ?? null),
          acceptance:
            held === undefined
              ? null
              : {
                  revision: held.revision,
                  accepted_at: "2026-10-03T09:00:10Z",
                  channel: held.channel,
                  basis: held.basis,
                  open: held.open,
                },
          bundle: this.bundleOf.get(id)?.revision ?? null,
        };
      }
      case "read_judgment": {
        // What SCE says of the revisions NAMED, wherever the work is now: a revision is never rewritten.
        if (work === undefined) throw new CommandFailure("not-found", `work \`${String(args["id"])}\``);
        const id = String(args["id"]);
        const asked = args["basis"] as Record<string, string | null | undefined>;
        const answersRevision = asked["answers"] ?? null;
        const source = work.revisions.find((r) => r.revision === asked["source"]);
        const model = this.modelsByRevision.get(String(asked["model"]));
        const list = this.listsByRevision.get(String(asked["requirements"]));
        if (
          source === undefined ||
          model === undefined ||
          list === undefined ||
          (answersRevision !== null && !this.answersByRevision.has(answersRevision))
        ) {
          throw new CommandFailure("not-found", "a revision of this work that was not kept");
        }
        const held = typeof args["acceptance"] === "string" ? this.acceptancesByRevision.get(args["acceptance"]) : undefined;
        if (typeof args["acceptance"] === "string" && held === undefined) {
          throw new CommandFailure("not-found", "an acceptance of this work that was not kept");
        }
        const basis: Record<string, string> = {
          source: source.revision,
          model: String(asked["model"]),
          requirements: list.revision,
          ...(answersRevision === null ? {} : { answers: answersRevision }),
        };
        const refusal = (part: "report" | "acceptance"): { kind: string; message: string; code: null } | null => {
          const at = this.refusals.findIndex((r) => r.part === part);
          const found = at < 0 ? undefined : this.refusals.splice(at, 1)[0];
          return found === undefined ? null : { kind: found.kind, message: found.message, code: null };
        };
        const lapse = held === undefined ? null : this.lapseOf(held.basis, basis);
        const refusedAcceptance = held === undefined ? null : refusal("acceptance");
        const refusedReport = refusal("report");
        return {
          basis,
          acceptance:
            held === undefined
              ? null
              : refusedAcceptance !== null
                ? { refused: refusedAcceptance }
                : { said: { standing: lapse === null ? "holds" : "lapsed", lapse } },
          report:
            refusedReport !== null
              ? { refused: refusedReport }
              : {
                  // SCE's words about the bytes: where the design stands to the text is the snapshot's to say.
                  said: {
                    generator: "fake-sce 0",
                    denominator: "synthesized",
                    outcomes: list.ids.map((requirement, i) => ({
                      id: requirement,
                      outcome:
                        model.text.includes("MISSING") && i === 0
                          ? "missing"
                          : model.text.includes("SCENARIO") && i === 1
                            ? "needs-scenario"
                            : model.text.includes("DANGLING") && i === 1
                              ? "dangling"
                              : model.text.includes("WAIVED") && i === 1
                                ? "waived"
                                : "implemented",
                      section: `S${i + 1}`,
                      node_paths:
                        this.carriedOf.get(id)?.[requirement] ??
                        (model.text.includes("MISSING") && i === 0 ? [] : [`states.s${i}`]),
                    })),
                    page: `ACCEPTANCE REPORT\n  ${list.ids.length} requirements\n`,
                    page_refusal: null,
                  },
                },
        };
      }
      case "accept": {
        const id = String(args["id"]);
        const now = this.basisOf(id);
        const model = this.models.get(id);
        const list = this.lists.get(id);
        if (now === null || model === undefined || list === undefined) {
          throw new CommandFailure("not-found", "a model or a requirement list of this work (none was saved)");
        }
        const expected = args["expect"] as Record<string, string>;
        // The core compares each revision, and the owner's answers are none (`null`, or absent) until given.
        const moved = Object.keys({ ...now, ...expected }).filter((key) => (now[key] ?? null) !== (expected[key] ?? null));
        if (moved.length > 0) {
          throw new CommandFailure(
            "moved",
            `${moved.join(", ")} changed after you were shown it, so nothing was accepted; read it again`,
            { moved, current: now },
          );
        }
        const head = work?.revisions.at(-1)?.revision ?? null;
        if (model.writtenFor !== head || list.writtenFor !== head) {
          throw new CommandFailure("not-current", "the model was not written for the text as it is now");
        }
        this.acceptSaves += 1;
        const revision = this.revision(`acceptance:${this.acceptSaves}`);
        this.keepAcceptance(id, { revision, basis: now, channel: "direct", open: [] });
        return { outcome: "saved", revision, parent: null };
      }
      case "read_answers": {
        const held =
          typeof args["revision"] === "string"
            ? this.answersByRevision.get(args["revision"])
            : typeof args["id"] === "string"
              ? this.answersOf.get(args["id"])
              : undefined;
        return { answers: held ?? null };
      }
      case "read_bundle": {
        const bundle = this.bundleOf.get(String(args["id"]));
        if (bundle === undefined) return { bundle: null };
        return {
          bundle: {
            revision: bundle.revision,
            bundle: {
              request: bundle.request,
              attempt: 1,
              executor: "desktop",
              source: bundle.source,
              answers: bundle.answers,
              model: bundle.model,
              requirements: bundle.requirements,
              previous: null,
              replaces: null,
              instructions: null,
              checks: [{ by: "core", name: "model", verdict: "accepted", generator: "fake-sce 0", digest: null, subject: bundle.model }],
              published_at: "2026-10-05T09:00:30Z",
            },
          },
        };
      }
      case "save_answers": {
        const id = String(args["id"]);
        const held = this.answersOf.get(id);
        if ((args["base"] ?? null) !== (held?.revision ?? null)) {
          throw new CommandFailure("conflict", "the answers moved", { base: args["base"] ?? null, current: held?.revision ?? null });
        }
        const wanted = args["answers"] as Record<string, string>;
        const entries: Record<string, { answer: string; answered_at: string }> = {};
        for (const [question, words] of Object.entries(wanted)) {
          const before = held?.entries[question];
          entries[question] =
            before !== undefined && before.answer === words
              ? before
              : { answer: words, answered_at: "2026-10-03T09:00:30Z" };
        }
        this.answerSaves += 1;
        const revision = this.revision(`answers:${this.answerSaves}`);
        this.answersOf.set(id, { revision, entries });
        this.answersByRevision.set(revision, { revision, entries });
        this.supersedeOpen(id);
        return { outcome: "saved", revision, parent: held?.revision ?? null };
      }
      case "model_history": {
        const revisions = this.modelHistoryOf.get(String(args["id"])) ?? [];
        return {
          entries: revisions.map((revision, i) => ({
            revision,
            parent: i === 0 ? null : (revisions[i - 1] ?? null),
            saved_at: "2026-10-05T09:00:00Z",
          })),
        };
      }
      case "review": {
        const model =
          typeof args["revision"] === "string"
            ? this.modelsByRevision.get(args["revision"])
            : typeof args["id"] === "string"
              ? this.models.get(args["id"])
              : undefined;
        if (model === undefined) throw new CommandFailure("not-found", "no model");
        const head = work?.revisions.at(-1)?.revision ?? null;
        const revision = this.revision(`model:${model.text}`);
        const standing =
          model.writtenFor === null ? "unstated" : model.writtenFor === head ? "current" : "behind";
        const base = { model: { revision, written_for: model.writtenFor }, source_head: head, standing, generator: "fake-sce 0" };
        if (model.text.includes("REFUSE")) {
          return {
            ...base,
            check: {
              verdict: "refused",
              kind: null,
              open: [],
              unresolved: [],
              records: [
                { code: "validation/invalid-reference", message: "no such state 'nowhere'", stage: "validation", line: 3 },
              ],
            },
            page: null,
            page_refusal: null,
          };
        }
        const refused = model.text.includes("NOPAGE");
        return {
          ...base,
          check: {
            verdict: "accepted",
            kind: "statechart",
            open: ["1 question(s) the specification leaves open (open-guard)"],
            // A model that applied the owner's answer to `open-guard` no longer asks it.
            unresolved: [
              { id: "open-guard", node_path: "states.closed.transitions[0]", line: 3, reason: "Which cards open the door?" },
              { id: "close-delay", node_path: "states.opened.transitions[0]", line: 4, reason: null },
            ].filter((q) => !model.text.includes(`APPLIED:${q.id}`)),
            records: [],
          },
          // Indented, with a run of spaces and a final newline: the screen keeps them.
          page: refused
            ? null
            : `machine door (lexicon: ${String(args["lexicon"] ?? "-")})\n  state closed:\n    on open   -> ${
                model.text.includes("LOCKED") ? "locked\n    on lock   -> locked" : "opened"
              }\n`,
          page_refusal: refused ? { code: "cli/pseudo-unsupported", message: "the page does not abbreviate this" } : null,
        };
      }
      case "remove_work": {
        if (work === undefined) throw new CommandFailure("not-found", "no such work");
        this.works.delete(String(args["id"]));
        return { removed: { id: String(args["id"]), title: work.title, created_at: "2026-10-03T09:00:00Z" } };
      }
      case "read_model":
      case "figures": {
        const model = typeof args["id"] === "string" ? this.models.get(args["id"]) : undefined;
        const head = work?.revisions.at(-1)?.revision ?? null;
        if (model === undefined) {
          if (name === "figures") throw new CommandFailure("not-found", "no model");
          return { model: null, source_head: head, standing: null };
        }
        const revision = this.revision(`model:${model.text}`);
        // The core's rule, in one place: for the text as it is, for an earlier one, or unsaid.
        const standing =
          model.writtenFor === null ? "unstated" : model.writtenFor === head ? "current" : "behind";
        if (name === "read_model") {
          const entry = model.others.length > 0 ? "door.scxml" : "model.scxml";
          return {
            model: {
              revision,
              written_for: model.writtenFor,
              text: model.text,
              entry,
              documents: [{ name: entry, text: model.text }, ...model.others],
            },
            source_head: head,
            standing,
          };
        }
        return {
          model: { revision, written_for: model.writtenFor },
          source_head: head,
          standing,
          generator: "fake-sce 0",
          sheets: [
            { name: "picture.svg", svg: this.figureSvg(model.text) },
            { name: "fields-1.svg", svg: this.figureSvg("the fields") },
          ],
        };
      }
      case "list_works":
        return {
          works: [...this.works].map(([id, w]) => ({ id, title: w.title, created_at: "2026-10-03T09:00:00Z" })),
          unreadable: [],
        };
      case "create_work": {
        const id = String(args["title"]).toLowerCase().replace(/[^a-z0-9]+/g, "-");
        this.works.set(id, { title: String(args["title"]), revisions: [] });
        return { id, title: String(args["title"]), created_at: "2026-10-03T09:00:00Z" };
      }
      case "read_source": {
        const revisions = work?.revisions ?? [];
        const wanted = args["revision"];
        const found = wanted === undefined ? revisions.at(-1) : revisions.find((r) => r.revision === wanted);
        return { source: found === undefined ? null : { revision: found.revision, text: found.text } };
      }
      case "history":
        return {
          entries: (work?.revisions ?? []).map((r, i, all) => ({
            revision: r.revision,
            parent: i === 0 ? null : (all[i - 1]?.revision ?? null),
            saved_at: "2026-10-03T09:00:00Z",
          })),
        };
      case "save_source": {
        if (work === undefined) throw new CommandFailure("not-found", "no such work");
        const head = work.revisions.at(-1)?.revision ?? null;
        if ((args["base"] ?? null) !== head) {
          throw new CommandFailure("conflict", "the base is not current", { base: args["base"] ?? null, current: head });
        }
        const text = String(args["text"]);
        const revision = this.revision(text);
        if (revision === head) return { outcome: "unchanged", revision };
        work.revisions.push({ revision, text });
        this.supersedeOpen(String(args["id"]));
        return { outcome: "saved", revision, parent: head };
      }
      case "read_revision_report": {
        // The core's rule: nothing accepted, nothing to report; a design or a list written for an
        // earlier text than the work has now is held back, in one sentence, and never compared.
        if (work === undefined) throw new CommandFailure("not-found", `work \`${String(args["id"])}\``);
        const id = String(args["id"]);
        const held = this.acceptances.get(id);
        if (held === undefined) return { acceptance: null, report: null };
        const head = work.revisions.at(-1)?.revision ?? null;
        const behind: string[] = [];
        if (standingOf(this.lists.get(id)?.writtenFor ?? null, head) === "behind") behind.push("requirement list");
        if (standingOf(this.models.get(id)?.writtenFor ?? null, head) === "behind") behind.push("model");
        if (behind.length > 0) {
          throw new CommandFailure(
            "revision-not-current",
            `the specification was changed after the ${behind.join(" and the ")} was written for it`,
            { source_head: head },
          );
        }
        const now = this.basisOf(id);
        if (now === null) throw new CommandFailure("not-found", "a model or a requirement list of this work");
        const ids = this.lists.get(id)?.ids ?? [];
        return {
          acceptance: {
            revision: held.revision,
            accepted_at: "2026-10-03T09:00:10Z",
            channel: held.channel,
            basis: held.basis,
            open: held.open,
          },
          report: {
            verdict: this.revisionVerdict,
            summary: {
              requirements: ids.length,
              // What the product actually compared: nothing, when no node of the design cites one.
              seen: this.revisionCompares ? ids.length : 0,
              violations: this.revisionVerdict === "outside-reach" ? 1 : 0,
              look: 0,
              ok: ids.length,
            },
            requirements: ids.map((requirement) => ({
              requirement,
              words: "carried",
              evidence: "unchanged",
              kind: "carries-over",
              severity: "ok",
            })),
            unclaimed: { added: [], gone: 0 },
            of: { accepted: held.basis, now },
            page: `REVISION REPORT\n  ${ids.length} requirements\n`,
          },
        };
      }
      default:
        throw new CommandFailure("unknown-command", name);
    }
  }
}

/** The core's rule, in one place: written for the text as it is, for an earlier one, or unsaid. */
function standingOf(writtenFor: string | null, head: string | null): "current" | "behind" | "unstated" {
  return writtenFor === null ? "unstated" : writtenFor === head ? "current" : "behind";
}

const settle = async (): Promise<void> => {
  for (let i = 0; i < 4; i += 1) await new Promise((resolve) => setTimeout(resolve, 0));
};

let root: HTMLElement;
let core: FakeCore;
let app: App;

async function start(): Promise<void> {
  document.body.innerHTML = '<div id="app"></div>';
  root = document.getElementById("app") as HTMLElement;
  core = new FakeCore();
  core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
  core.addWork("beta", "Beta", ["beta one"]);
  app = new App(root, { transport: core, storage: null, browserLanguage: "en" });
  await app.start();
  await settle();
}

const buttons = (label: string): HTMLButtonElement[] =>
  [...root.querySelectorAll("button")].filter((b) => b.textContent === label);

async function click(label: string, index = 0): Promise<void> {
  const button = buttons(label)[index];
  if (button === undefined) throw new Error(`no button "${label}" (${index}) in: ${root.textContent}`);
  button.click();
  await settle();
}

const editor = (): HTMLTextAreaElement => root.querySelector("#source") as HTMLTextAreaElement;
const heading = (): string => root.querySelector("main h2")?.textContent ?? "";
const status = (): string => root.querySelector("#status")?.textContent ?? "";

async function type(text: string): Promise<void> {
  editor().value = text;
  editor().dispatchEvent(new Event("input", { bubbles: true }));
  await settle();
}

beforeEach(start);

describe("an answer that arrives after the person has moved on", () => {
  it("does not put one work's older text into another work's editor", async () => {
    await click("Alpha");
    expect(editor().value).toBe("alpha two");
    // History is newest first, so the older revision is the second "Load into editor".
    const older = core.revision("alpha one");
    const held = core.hold("read_source", (a) => a["revision"] === older);
    await click("Load into editor", 1);

    await click("Beta");
    expect(heading()).toBe("Beta");
    await type("beta edited");
    held.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(editor().value).toBe("beta edited");
    await click("Save");
    expect(core.headText("beta")).toBe("beta edited");
    expect(core.headText("alpha")).toBe("alpha two");
  });

  it("does not load an older text over what was typed while it was being read", async () => {
    await click("Alpha");
    const older = core.revision("alpha one");
    const held = core.hold("read_source", (a) => a["revision"] === older);
    await click("Load into editor", 1);
    await type("typed meanwhile");
    held.release();
    await settle();

    expect(editor().value).toBe("typed meanwhile");
    expect(root.textContent).toContain("was not loaded");
  });

  it("shows the work last asked for when the first one answers last", async () => {
    const slow = core.hold("read_source", (a) => a["id"] === "alpha");
    await click("Alpha");
    await click("Beta");
    expect(heading()).toBe("Beta");
    slow.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(editor().value).toBe("beta one");
  });

  it("does not show an older revision of a work the person has left", async () => {
    await click("Alpha");
    const older = core.revision("alpha one");
    const held = core.hold("read_source", (a) => a["revision"] === older);
    await click("View", 1);
    await click("Beta");
    held.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(root.textContent).not.toContain("Viewing revision");
  });
});

describe("text that is not saved", () => {
  it("is not replaced by opening another work, and the person is asked", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Beta");

    expect(editor().value).toBe("alpha changed");
    expect(root.textContent).toContain("not saved");
    expect(core.callsOf("read_source").filter((a) => a["id"] === "beta")).toHaveLength(0);
    expect(app.hasUnsavedChanges()).toBe(true);
  });

  it("stays when the person chooses to stay", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Beta");
    await click("Stay here");

    expect(root.textContent).not.toContain("not saved");
    expect(editor().value).toBe("alpha changed");
  });

  it("is discarded only when the person says so", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Beta");
    await click("Discard my changes and open it");

    expect(heading()).toBe("Beta");
    expect(core.headText("alpha")).toBe("alpha two");
  });

  it("is saved first when the person asks, and the other work then opens", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Beta");
    await click("Save, then open it");

    expect(core.headText("alpha")).toBe("alpha changed");
    expect(heading()).toBe("Beta");
    expect(editor().value).toBe("beta one");
  });

  it("keeps the person waiting on the choice when the save before opening does not take", async () => {
    await click("Alpha");
    await type("alpha changed");
    // Another writer got there first, so this save is refused as a conflict.
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two", "alpha elsewhere"]);
    await click("Beta");
    await click("Save, then open it");

    expect(heading()).toBe("Alpha");
    expect(editor().value).toBe("alpha changed");
    expect(root.textContent).toContain("not saved");
  });

  it("is protected for the same work too: opening it again does not reload over it", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Alpha");

    expect(editor().value).toBe("alpha changed");
    expect(root.textContent).toContain("not saved");
  });

  it("is protected from what is typed while the other work is still being read", async () => {
    await click("Alpha");
    const slow = core.hold("read_source", (a) => a["id"] === "beta");
    await click("Beta");
    // Nothing was unsaved when Beta was asked for, so the read is on its way.
    await type("typed while waiting");
    slow.release();
    await settle();

    expect(heading()).toBe("Alpha");
    expect(editor().value).toBe("typed while waiting");
    expect(root.textContent).toContain("not saved");
    expect(app.hasUnsavedChanges()).toBe(true);

    // The choice is the same one as before the read: the text is kept unless the person lets it go.
    await click("Save, then open it");
    expect(core.headText("alpha")).toBe("typed while waiting");
    expect(heading()).toBe("Beta");
  });

  it("is not lost to the other work arriving, when the person then lets it go", async () => {
    await click("Alpha");
    const slow = core.hold("read_source", (a) => a["id"] === "beta");
    await click("Beta");
    await type("typed while waiting");
    slow.release();
    await settle();
    await click("Discard my changes and open it");

    expect(heading()).toBe("Beta");
    expect(editor().value).toBe("beta one");
    expect(core.headText("alpha")).toBe("alpha two");
  });

  it("does not hold the other work back when nothing was typed while it was read", async () => {
    await click("Alpha");
    const slow = core.hold("read_source", (a) => a["id"] === "beta");
    await click("Beta");
    slow.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(root.textContent).not.toContain("not saved");
  });

  it("is not unsaved once it is saved", async () => {
    await click("Alpha");
    expect(app.hasUnsavedChanges()).toBe(false);
    await type("alpha changed");
    expect(app.hasUnsavedChanges()).toBe(true);
    await click("Save");
    expect(app.hasUnsavedChanges()).toBe(false);
  });
});

describe("a save that took, and a read after it that did not", () => {
  it("is still a saved text, and the next save does not conflict with it", async () => {
    await click("Alpha");
    await type("alpha three");
    core.failNext("history", new CommandFailure("io", "the history could not be read"));
    await click("Save");

    expect(core.headText("alpha")).toBe("alpha three");
    expect(status()).toBe("Saved");
    expect(root.textContent).not.toContain("The save failed");
    expect(root.textContent).toContain("the history could not be read");

    await type("alpha four");
    await click("Save");
    expect(core.headText("alpha")).toBe("alpha four");
    expect(root.textContent).not.toContain("changed while you were editing");
  });

  it("is reported as failed when the save itself fails", async () => {
    await click("Alpha");
    await type("alpha three");
    core.failNext("save_source", new CommandFailure("io", "the disk is full"));
    await click("Save");

    expect(root.textContent).toContain("The save failed");
    expect(core.headText("alpha")).toBe("alpha two");
    expect(editor().value).toBe("alpha three");
  });
});

describe("the new-work field", () => {
  it("keeps what is typed in it across a redraw", async () => {
    const field = root.querySelector('input[name="title"]') as HTMLInputElement;
    field.value = "Gamma";
    field.dispatchEvent(new Event("input", { bubbles: true }));
    await click("Alpha");

    expect((root.querySelector('input[name="title"]') as HTMLInputElement).value).toBe("Gamma");
  });
});

// ---- the model, as SCE draws it -------------------------------------------

const images = (): HTMLImageElement[] => [...root.querySelectorAll<HTMLImageElement>(".sheet img")];
const modelText = (): string => root.querySelector(".model")?.textContent ?? "";
const decoded = (image: HTMLImageElement): string =>
  decodeURIComponent(image.src.replace("data:image/svg+xml;charset=utf-8,", ""));

describe("the model panel", () => {
  it("says there is no model when the work has none, and draws nothing", async () => {
    await click("Alpha");
    expect(modelText()).toContain("No model yet");
    expect(images()).toHaveLength(0);
    expect(core.callsOf("figures")).toHaveLength(0);
  });

  it("shows the sheets SCE drew, in its order, as images the model's text cannot script", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");

    expect(images()).toHaveLength(2);
    expect([...root.querySelectorAll(".sheet figcaption")].map((c) => c.textContent)).toEqual([
      "picture.svg",
      "fields-1.svg",
    ]);
    expect(decoded(images()[0] as HTMLImageElement)).toBe(core.figureSvg("<scxml/>"));
    expect(images()[0]?.getAttribute("alt")).toBe("picture");
    expect(modelText()).toContain("written for the text as it is now");
    expect(modelText()).toContain("Drawn by fake-sce 0");
    // The model's own text is there to read, folded.
    expect(root.querySelector(".scxml")?.textContent).toBe("<scxml/>");
  });

  it("shows a model of one document as one text, as before", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");

    const folded = root.querySelector(".model-scxml");
    expect(folded?.querySelector("summary")?.textContent).toBe("The model's SCXML");
    expect(folded?.querySelectorAll("pre.scxml")).toHaveLength(1);
    expect(folded?.querySelector("h5")).toBeNull();
  });

  it("shows a model of several documents under the file name each is imported by, the entry named", async () => {
    core.setModel("alpha", "<scxml>entry</scxml>", core.revision("alpha two"), [
      { name: "close.scxml", text: "<event-schema>close</event-schema>" },
      { name: "open.scxml", text: "<event-schema>open</event-schema>" },
    ]);
    await click("Alpha");

    const folded = root.querySelector(".model-scxml");
    expect(folded?.querySelector("summary")?.textContent).toBe("The model's SCXML: 3 documents");
    expect([...(folded?.querySelectorAll("h5.document-name code") ?? [])].map((c) => c.textContent)).toEqual([
      "door.scxml",
      "close.scxml",
      "open.scxml",
    ]);
    expect([...(folded?.querySelectorAll("pre.scxml") ?? [])].map((p) => p.textContent)).toEqual([
      "<scxml>entry</scxml>",
      "<event-schema>close</event-schema>",
      "<event-schema>open</event-schema>",
    ]);
    // Only the entry says so.
    expect(folded?.querySelectorAll("h5.document-name")[0]?.textContent).toContain("the document SCE is asked about");
    expect(folded?.querySelectorAll("h5.document-name")[1]?.textContent).not.toContain("asked about");
    // The review and the figures are of the same model.
    expect(core.callsOf("review")).toHaveLength(1);
    expect(images()).toHaveLength(2);
  });

  it("never turns the drawing into elements of the page", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    // Whatever the product wrote, even markup that would run in a page, is only an image here.
    const hostile = '<svg xmlns="http://www.w3.org/2000/svg"><script>window.__ran = true</script></svg>';
    core.figureSvg = () => hostile;
    await click("Alpha");

    expect(root.querySelector("script")).toBeNull();
    expect(root.querySelector(".sheet svg")).toBeNull();
    expect((window as unknown as Record<string, unknown>)["__ran"]).toBeUndefined();
    expect(images()).toHaveLength(2);
  });

  it("says a model written for an earlier text is behind, with both revisions", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha one"));
    await click("Alpha");

    const banner = root.querySelector(".model .banner-warn")?.textContent ?? "";
    expect(banner).toContain("earlier text");
    expect(banner).toContain(core.revision("alpha one").slice(0, 12));
    expect(banner).toContain(core.revision("alpha two").slice(0, 12));
    expect(images()).toHaveLength(2);
  });

  it("says when nothing records which text the model was written for", async () => {
    core.setModel("alpha", "<scxml/>", null);
    await click("Alpha");
    expect(modelText()).toContain("Nothing records which text");
  });

  it("shows SCE's refusal in the product's words, with the model still readable", async () => {
    core.setModel("alpha", "<scxml>big</scxml>", core.revision("alpha two"));
    core.failNext(
      "figures",
      new CommandFailure(
        "sce-refused",
        "SCE refused the model (cli/diagram-does-not-fit): the figure needs 925 x 125 pt",
        { code: "cli/diagram-does-not-fit" },
      ),
    );
    await click("Alpha");

    expect(images()).toHaveLength(0);
    const refusal = root.querySelector(".model .banner-error")?.textContent ?? "";
    expect(refusal).toContain("SCE did not draw this model");
    expect(refusal).toContain("the figure needs 925 x 125 pt");
    expect(refusal).toContain("cli/diagram-does-not-fit");
    expect(root.querySelector(".scxml")?.textContent).toBe("<scxml>big</scxml>");
    // The refusal is the model's, not the text's: the editor is untouched.
    expect(editor().value).toBe("alpha two");
    expect(root.querySelector("main > .banner-error")).toBeNull();
  });

  it("does not hold the editor back while SCE draws", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    const held = core.hold("figures");
    await click("Alpha");

    expect(editor().value).toBe("alpha two");
    expect(modelText()).toContain("SCE is drawing the model");
    expect(images()).toHaveLength(0);

    held.release();
    await settle();
    expect(images()).toHaveLength(2);
    expect(modelText()).not.toContain("SCE is drawing");
  });

  it("does not put one work's drawing under another work", async () => {
    core.setModel("alpha", "<scxml>alpha</scxml>", core.revision("alpha two"));
    const held = core.hold("figures");
    await click("Alpha");
    await click("Beta");
    held.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(images()).toHaveLength(0);
    expect(modelText()).toContain("No model yet");
  });

  it("moves where the model stands when the text is saved, without drawing it again", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    expect(core.callsOf("figures")).toHaveLength(1);
    expect(modelText()).toContain("written for the text as it is now");

    await type("alpha three");
    await click("Save");

    expect(modelText()).toContain("earlier text");
    expect(modelText()).toContain(core.revision("alpha two").slice(0, 12));
    expect(core.callsOf("figures")).toHaveLength(1);
    expect(images()).toHaveLength(2);
  });

  it("draws again when asked to read again", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    core.setModel("alpha", "<scxml>changed</scxml>", core.revision("alpha two"));
    await click("Read again");

    expect(core.callsOf("figures")).toHaveLength(2);
    expect(root.querySelector(".scxml")?.textContent).toBe("<scxml>changed</scxml>");
    expect(decoded(images()[0] as HTMLImageElement)).toBe(core.figureSvg("<scxml>changed</scxml>"));
  });

  it("asks SCE to draw in the language the screen is in, and again when the language changes", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    expect(core.callsOf("figures")[0]?.["lexicon"]).toBe("en");

    const picker = root.querySelector("header select") as HTMLSelectElement;
    picker.value = "ko";
    picker.dispatchEvent(new Event("change", { bubbles: true }));
    await settle();

    expect(core.callsOf("figures")).toHaveLength(2);
    expect(core.callsOf("figures")[1]?.["lexicon"]).toBe("ko");
    expect(images()).toHaveLength(2);
  });

  it("shows the drawings larger than SCE set them, in a size the person can change and that is kept", async () => {
    const kept = new Map<string, string>();
    const storage = {
      getItem: (key: string): string | null => kept.get(key) ?? null,
      setItem: (key: string, value: string): void => void kept.set(key, value),
    };
    const open = async (): Promise<void> => {
      document.body.innerHTML = '<div id="app"></div>';
      root = document.getElementById("app") as HTMLElement;
      core = new FakeCore();
      core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
      core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
      app = new App(root, { transport: core, storage, browserLanguage: "en" });
      await app.start();
      await settle();
      await click("Alpha");
    };
    const zoom = (): string => root.querySelector<HTMLElement>(".sheets")?.getAttribute("style") ?? "";

    await open();
    expect(zoom()).toContain("--sheet-zoom: 1.5");
    expect(buttons("150%")[0]?.getAttribute("aria-pressed")).toBe("true");

    await click("200%");
    expect(zoom()).toContain("--sheet-zoom: 2");
    expect(buttons("200%")[0]?.getAttribute("aria-pressed")).toBe("true");
    expect(buttons("150%")[0]?.getAttribute("aria-pressed")).toBe("false");
    expect(kept.get("sce.zoom")).toBe("2");
    expect(core.callsOf("figures")).toHaveLength(1);

    // A new screen starts at the size the person chose.
    await open();
    expect(zoom()).toContain("--sheet-zoom: 2");
  });

  it("reports a model that cannot be read as the panel's own message, not as an empty one", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    core.failNext("read_work_snapshot", new CommandFailure("corrupt", "the model file is damaged"));
    await click("Alpha");

    expect(modelText()).toContain("the model file is damaged");
    expect(modelText()).not.toContain("No model yet");
    expect(editor().value).toBe("alpha two");
  });
});

// ---- what SCE says of the model -------------------------------------------

const reviewText = (): string => root.querySelector(".review")?.textContent ?? "";
const pseudo = (): HTMLElement | null => root.querySelector(".review pre.pseudo");

describe("what SCE says of the model", () => {
  it("shows the verdict, what the model leaves open, and the page exactly as SCE wrote it", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");

    expect(reviewText()).toContain("SCE accepted the model as a statechart.");
    expect(reviewText()).toContain("1 question(s) the specification leaves open (open-guard)");
    expect(reviewText()).toContain("open-guard (line 3)");
    // Every space and line break of the page is the product's.
    expect(pseudo()?.textContent).toBe("machine door (lexicon: en)\n  state closed:\n    on open   -> opened\n");
    // The page is text, never markup.
    expect(root.querySelector(".review pre.pseudo *")).toBeNull();
  });

  it("says once that a passed check does not say the model agrees with the text", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");

    expect(reviewText()).toContain("does not say the model agrees with your text");
    expect(reviewText()).not.toContain("proved");
  });

  it("asks SCE for the page in the language the screen is in, and again when it changes", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    expect(core.callsOf("review")[0]?.["lexicon"]).toBe("en");

    const picker = root.querySelector("header select") as HTMLSelectElement;
    picker.value = "ko";
    picker.dispatchEvent(new Event("change", { bubbles: true }));
    await settle();

    expect(core.callsOf("review")).toHaveLength(2);
    expect(core.callsOf("review")[1]?.["lexicon"]).toBe("ko");
    expect(pseudo()?.textContent).toContain("(lexicon: ko)");
  });

  it("shows every record SCE wrote for a model it refuses, and no page", async () => {
    core.setModel("alpha", "<scxml>REFUSE</scxml>", core.revision("alpha two"));
    await click("Alpha");

    expect(reviewText()).toContain("SCE refused the model");
    expect(reviewText()).toContain("no such state 'nowhere'");
    expect(reviewText()).toContain("validation/invalid-reference");
    expect(reviewText()).toContain("line 3");
    expect(pseudo()).toBeNull();
    // A refused model is not told it passed.
    expect(reviewText()).not.toContain("does not say the model agrees");
  });

  it("keeps the verdict and says why when SCE accepts a model and will not write its page", async () => {
    core.setModel("alpha", "<scxml>NOPAGE</scxml>", core.revision("alpha two"));
    await click("Alpha");

    expect(reviewText()).toContain("SCE accepted the model");
    expect(reviewText()).toContain("did not write its pseudocode page");
    expect(reviewText()).toContain("cli/pseudo-unsupported");
    expect(pseudo()).toBeNull();
  });

  it("says SCE could not answer, in the product's words, and still draws the figures", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    core.failNext("review", new CommandFailure("sce-timeout", "the SCE generator did not finish in 30 s and was stopped"));
    await click("Alpha");

    expect(reviewText()).toContain("SCE could not read the model");
    expect(reviewText()).toContain("did not finish in 30 s");
    expect(pseudo()).toBeNull();
    expect(images()).toHaveLength(2);
  });

  it("does not hold the page back for a slow drawing, nor the drawing for a slow page", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    const drawing = core.hold("figures");
    await click("Alpha");
    expect(pseudo()?.textContent).toContain("machine door");
    expect(images()).toHaveLength(0);
    drawing.release();
    await settle();
    expect(images()).toHaveLength(2);

    await click("Beta");
    core.setModel("beta", "<scxml/>", core.revision("beta one"));
    const page = core.hold("review");
    await click("Alpha");
    expect(images()).toHaveLength(2);
    expect(reviewText()).toContain("Reading what SCE says");
    page.release();
    await settle();
    expect(pseudo()).not.toBeNull();
  });

  it("does not put one work's page under another work", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    const page = core.hold("review", (args) => args["id"] === "alpha");
    await click("Alpha");
    await click("Beta");
    page.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(root.querySelector(".review")).toBeNull();
    expect(pseudo()).toBeNull();
  });

  it("is not asked for a work that has no model, and is gone with a work that is removed", async () => {
    await click("Alpha");
    expect(core.callsOf("review")).toHaveLength(0);
    expect(root.querySelector(".review")).toBeNull();

    core.setModel("beta", "<scxml/>", core.revision("beta one"));
    await click("Beta");
    expect(pseudo()).not.toBeNull();
    await click("Remove this work");
    await click("Remove");
    expect(root.querySelector(".review")).toBeNull();
  });

  it("is not asked again when only the text is saved: the model is the same", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    expect(core.callsOf("review")).toHaveLength(1);

    await type("alpha changed");
    await click("Save");
    expect(core.callsOf("review")).toHaveLength(1);
    expect(core.callsOf("figures")).toHaveLength(1);
    // Where the model stands has moved, and the screen says so.
    expect(modelText()).toContain("earlier text");
    expect(pseudo()).not.toBeNull();
  });
});

// ---- the owner's answers --------------------------------------------------

const fields = (): HTMLTextAreaElement[] => [...root.querySelectorAll<HTMLTextAreaElement>("textarea[data-qid]")];
const fieldOf = (id: string): HTMLTextAreaElement => {
  const found = fields().find((f) => f.dataset["qid"] === id);
  if (found === undefined) throw new Error(`no answer field for ${id} in: ${root.textContent}`);
  return found;
};
const saveAnswersButton = (): HTMLButtonElement => root.querySelector("#save-answers") as HTMLButtonElement;
const answersStatus = (): string => root.querySelector("#answers-status")?.textContent ?? "";

async function answer(id: string, text: string): Promise<void> {
  const field = fieldOf(id);
  field.value = text;
  field.dispatchEvent(new Event("input", { bubbles: true }));
  await settle();
}

describe("the owner's answers", () => {
  beforeEach(() => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
  });

  it("are asked for, one field to each question the model marks, in the model's own words", async () => {
    await click("Alpha");

    expect(fields().map((f) => f.dataset["qid"])).toEqual(["open-guard", "close-delay"]);
    expect(root.querySelector(".answers")?.textContent).toContain("Which cards open the door?");
    expect(root.querySelector(".answers")?.textContent).toContain("The model gave no wording for this question.");
    expect(root.querySelector(".answers")?.textContent).toContain("Nothing here changes the model.");
    expect(saveAnswersButton().disabled).toBe(true);
  });

  it("are saved as typed, and the screen then shows what the core holds", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card on the list opens it.");
    expect(saveAnswersButton().disabled).toBe(false);
    expect(answersStatus()).toBe("Unsaved answers");
    expect(app.hasUnsavedChanges()).toBe(true);

    await click("Save answers");

    expect(core.callsOf("save_answers")).toEqual([
      { id: "alpha", answers: { "open-guard": "Any card on the list opens it." }, base: null },
    ]);
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card on the list opens it." });
    expect(answersStatus()).toBe("Answers saved");
    expect(fieldOf("open-guard").value).toBe("Any card on the list opens it.");
    expect(root.querySelector(".answered-at")?.textContent).toContain("Said");
    expect(saveAnswersButton().disabled).toBe(true);
    expect(app.hasUnsavedChanges()).toBe(false);
  });

  it("are shown when they were given before, and carried when another question is answered", async () => {
    core.setAnswers("alpha", { "open-guard": "Any card on the list opens it." });
    await click("Alpha");
    expect(fieldOf("open-guard").value).toBe("Any card on the list opens it.");

    await answer("close-delay", "Ten seconds.");
    await click("Save answers");

    const sent = core.callsOf("save_answers")[0];
    expect(sent?.["answers"]).toEqual({
      "close-delay": "Ten seconds.",
      "open-guard": "Any card on the list opens it.",
    });
    expect(sent?.["base"]).toMatch(/^[0-9a-f]{64}$/);
  });

  it("are taken back when their field is cleared", async () => {
    core.setAnswers("alpha", { "open-guard": "Any card.", "close-delay": "Ten seconds." });
    await click("Alpha");
    await answer("close-delay", "");
    await click("Save answers");

    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card." });
    expect(fieldOf("close-delay").value).toBe("");
  });

  it("are not offered for saving when nothing differs from what is saved", async () => {
    core.setAnswers("alpha", { "open-guard": "Any card." });
    await click("Alpha");
    await answer("open-guard", "Any card, always.");
    expect(saveAnswersButton().disabled).toBe(false);
    await answer("open-guard", "Any card.");
    expect(saveAnswersButton().disabled).toBe(true);
    expect(app.hasUnsavedChanges()).toBe(false);
  });

  it("keep what was typed, and say so, when they were saved elsewhere meanwhile", async () => {
    core.setAnswers("alpha", { "open-guard": "yes" });
    await click("Alpha");
    await answer("open-guard", "mine");
    // Another entrance saves on top while the person is typing.
    core.setAnswers("alpha", { "open-guard": "theirs" });
    await click("Save answers");

    expect(root.querySelector(".answers")?.textContent).toContain("saved elsewhere while you were typing");
    expect(fieldOf("open-guard").value).toBe("mine");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "theirs" });

    // Saved again, on top of the revision that turned up.
    await click("Save answers");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "mine" });
    expect(root.querySelector(".answers")?.textContent).not.toContain("saved elsewhere");
  });

  it("say why they were not saved, and keep what was typed", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card.");
    core.failNext("save_answers", new CommandFailure("io", "the disk is full"));
    await click("Save answers");

    expect(root.querySelector(".answers")?.textContent).toContain("The answers were not saved: the disk is full");
    expect(fieldOf("open-guard").value).toBe("Any card.");
    expect(app.hasUnsavedChanges()).toBe(true);
  });

  it("to questions the model no longer asks are kept apart, and can be taken back", async () => {
    core.setAnswers("alpha", { "old-question": "It stays the same." });
    await click("Alpha");

    expect(root.querySelector(".orphans")?.textContent).toContain("Answers to questions this model does not ask");
    expect(fieldOf("old-question").value).toBe("It stays the same.");
    await answer("old-question", "");
    await click("Save answers");
    expect(core.answersHeld("alpha")).toEqual({});
  });

  it("are asked about before another work replaces them, and saved when the person says so", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card.");
    await click("Beta");

    expect(root.textContent).toContain("not saved");
    expect(heading()).toBe("Alpha");
    expect(core.callsOf("read_source").filter((a) => a["id"] === "beta")).toHaveLength(0);

    await click("Save, then open it");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card." });
    expect(heading()).toBe("Beta");
  });

  it("are protected from what is typed while the other work is still being read", async () => {
    await click("Alpha");
    const slow = core.hold("read_source", (a) => a["id"] === "beta");
    await click("Beta");
    await answer("open-guard", "Any card.");
    slow.release();
    await settle();

    expect(heading()).toBe("Alpha");
    expect(fieldOf("open-guard").value).toBe("Any card.");
    expect(root.textContent).toContain("not saved");

    await click("Save, then open it");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card." });
    expect(heading()).toBe("Beta");
  });

  it("stay under the cursor when SCE's drawing arrives and the screen is redrawn", async () => {
    const drawing = core.hold("figures");
    await click("Alpha");
    const field = fieldOf("open-guard");
    field.focus();
    field.value = "Any car";
    field.dispatchEvent(new Event("input", { bubbles: true }));
    field.setSelectionRange(3, 5);

    drawing.release();
    await settle();

    const after = fieldOf("open-guard");
    expect(document.activeElement).toBe(after);
    expect(after.value).toBe("Any car");
    expect([after.selectionStart, after.selectionEnd]).toEqual([3, 5]);
  });

  it("that cannot be read are said so in words, and the model's review still shows", async () => {
    core.failNext("read_answers", new CommandFailure("corrupt", "the saved answers are not what the store wrote"));
    await click("Alpha");

    expect(reviewText()).toContain("Your answers could not be read");
    expect(reviewText()).toContain("not what the store wrote");
    expect(pseudo()).not.toBeNull();
    expect(fields()).toHaveLength(0);
  });

  it("of one work are not put under another when they arrive late", async () => {
    core.setAnswers("alpha", { "open-guard": "Any card." });
    const held = core.hold("read_answers", (args) => args["id"] === "alpha");
    await click("Alpha");
    await click("Beta");
    held.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(fields()).toHaveLength(0);
    expect(app.hasUnsavedChanges()).toBe(false);
  });
});

// ---- a desktop window being closed ----------------------------------------

/** What the shell of a desktop window hears from the screen, and is told to do. */
class FakeDesktop {
  readonly reports: boolean[] = [];
  closed = 0;
  unsaved(unsaved: boolean): void {
    this.reports.push(unsaved);
  }
  async close(): Promise<void> {
    this.closed += 1;
  }
}

describe("a desktop window asked to close", () => {
  let desktop: FakeDesktop;

  beforeEach(async () => {
    desktop = new FakeDesktop();
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
    core.addWork("beta", "Beta", ["beta one"]);
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    app = new App(root, { transport: core, storage: null, browserLanguage: "en", desktop });
    await app.start();
    await settle();
  });

  it("is told what is unsaved when it changes, and only then", async () => {
    await click("Alpha");
    expect(desktop.reports.at(-1)).toBe(false);
    const before = desktop.reports.length;

    await type("alpha changed");
    expect(desktop.reports.at(-1)).toBe(true);
    await type("alpha changed again");
    expect(desktop.reports.length).toBe(before + 1);

    await click("Save");
    expect(desktop.reports.at(-1)).toBe(false);
  });

  it("is told about typed answers the same way", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card.");
    expect(desktop.reports.at(-1)).toBe(true);
    await click("Save answers");
    expect(desktop.reports.at(-1)).toBe(false);
  });

  it("closes at once when nothing is unsaved after all", async () => {
    await click("Alpha");
    app.askToClose();
    await settle();

    expect(desktop.closed).toBe(1);
    expect(root.textContent).not.toContain("Closing the window would lose");
  });

  it("asks first when the editor holds text the core has not been given, and stays when the person says so", async () => {
    await click("Alpha");
    await type("alpha changed");
    app.askToClose();
    await settle();

    expect(root.textContent).toContain("Closing the window would lose");
    expect(desktop.closed).toBe(0);
    await click("Stay here");
    expect(root.textContent).not.toContain("Closing the window would lose");
    expect(desktop.closed).toBe(0);
    expect(editor().value).toBe("alpha changed");
  });

  it("says again that something is unsaved while the question is open: that is how the shell knows it is answered", async () => {
    await click("Alpha");
    await type("alpha changed");
    const before = desktop.reports.length;
    app.askToClose();
    await settle();

    expect(desktop.reports.length).toBeGreaterThan(before);
    expect(desktop.reports.at(-1)).toBe(true);
  });

  it("closes without saving only when the person says to discard", async () => {
    await click("Alpha");
    await type("alpha changed");
    app.askToClose();
    await settle();
    await click("Discard my changes and close");

    expect(desktop.closed).toBe(1);
    expect(core.headText("alpha")).toBe("alpha two");
  });

  it("saves the text and the answers first when the person says to, and then closes", async () => {
    await click("Alpha");
    await type("alpha changed");
    await answer("open-guard", "Any card.");
    app.askToClose();
    await settle();
    await click("Save, then close");

    expect(core.headText("alpha")).toBe("alpha changed");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card." });
    expect(desktop.closed).toBe(1);
  });

  it("stays open when the save before closing does not take", async () => {
    await click("Alpha");
    await type("alpha changed");
    core.failNext("save_source", new CommandFailure("io", "the disk is full"));
    app.askToClose();
    await settle();
    await click("Save, then close");

    expect(desktop.closed).toBe(0);
    expect(root.textContent).toContain("the disk is full");
    expect(app.hasUnsavedChanges()).toBe(true);
  });

  it("is not closed under a save that is still on its way", async () => {
    await click("Alpha");
    await type("alpha changed");
    const held = core.hold("save_source");
    app.askToClose();
    await settle();
    void click("Save, then close");
    await settle();
    expect(desktop.closed).toBe(0);
    expect(buttons("Discard my changes and close")[0]?.disabled).toBe(true);

    held.release();
    await settle();
    expect(desktop.closed).toBe(1);
  });
});

// ---- removing a work ------------------------------------------------------

const workLinks = (): string[] => [...root.querySelectorAll(".work-link")].map((b) => b.textContent ?? "");

describe("removing a work", () => {
  it("asks first, says what stays, and removes nothing until the person says yes", async () => {
    await click("Alpha");
    await click("Remove this work");

    expect(root.textContent).toContain('Remove "Alpha" from the list?');
    expect(root.textContent).toContain("removed.json");
    expect(core.callsOf("remove_work")).toHaveLength(0);

    await click("Keep the work");
    expect(root.textContent).not.toContain('Remove "Alpha" from the list?');
    expect(core.callsOf("remove_work")).toHaveLength(0);
    expect(workLinks()).toEqual(["Alpha", "Beta"]);
  });

  it("takes the work out of the list and the screen, and tells the person", async () => {
    await click("Alpha");
    await click("Remove this work");
    await click("Remove");

    expect(core.callsOf("remove_work")).toEqual([{ id: "alpha" }]);
    expect(workLinks()).toEqual(["Beta"]);
    expect(editor()).toBeNull();
    expect(root.textContent).toContain("Pick a work");
    expect(root.textContent).toContain('"Alpha" was removed from the list.');

    // The work beside it is as it was.
    await click("Beta");
    expect(editor().value).toBe("beta one");
    expect(root.textContent).not.toContain("was removed from the list");
  });

  it("says that text not yet saved goes with the work, and only then", async () => {
    await click("Alpha");
    await click("Remove this work");
    expect(root.textContent).not.toContain("not saved is lost");
    await click("Keep the work");

    await type("alpha changed");
    await click("Remove this work");
    expect(root.textContent).toContain("not saved is lost");
  });

  it("is not offered while a save is on its way", async () => {
    await click("Alpha");
    const held = core.hold("save_source");
    await type("alpha changed");
    await click("Save");

    const remove = buttons("Remove this work")[0];
    expect(remove?.disabled).toBe(true);
    held.release();
    await settle();
    expect(buttons("Remove this work")[0]?.disabled).toBe(false);
  });

  it("does not let a drawing that was still on its way land under the work that replaced it", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    const drawing = core.hold("figures");
    await click("Alpha");
    await click("Remove this work");
    await click("Remove");
    drawing.release();
    await settle();

    expect(images()).toHaveLength(0);
    expect(root.querySelector(".model")).toBeNull();
    expect(root.textContent).toContain("Pick a work");
  });

  it("keeps the work on screen, with the core's words, when the removal is refused", async () => {
    await click("Alpha");
    core.failNext("remove_work", new CommandFailure("busy", "another save held the work for 30000 ms"));
    await click("Remove this work");
    await click("Remove");

    expect(root.textContent).toContain("another save held the work");
    expect(heading()).toBe("Alpha");
    expect(workLinks()).toEqual(["Alpha", "Beta"]);
    expect(root.textContent).not.toContain("was removed from the list");
  });
});

// ---- accepting the design ---------------------------------------------------

const acceptanceText = (): string => root.querySelector(".acceptance")?.textContent ?? "";
const acceptButton = (): HTMLButtonElement => root.querySelector("#accept") as HTMLButtonElement;
const acceptNote = (): string => root.querySelector("#accept-note")?.textContent ?? "";
/** What SCE says of each requirement: its four cells. The last cell is the screen's own buttons (`goto`). */
const requirementRows = (): string[][] =>
  [...root.querySelectorAll(".acceptance tbody tr")].map((row) =>
    [...row.querySelectorAll("td:not(.goto)")].map((cell) => cell.textContent ?? ""),
  );

/** The revision of the text a work holds now: what a model or a list says it was written for. */
const headOf = (id: string): string => core.revision(core.headText(id) as string);

describe("accepting the design", () => {
  beforeEach(() => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
  });

  it("says there is no requirement list when the work has none, and offers nothing to accept", async () => {
    core.setModel("beta", "<scxml/>", headOf("beta"));
    await click("Beta");

    expect(acceptanceText()).toContain("No requirement list yet");
    expect(root.querySelector("#accept")).toBeNull();
    // There is no design to measure against anything, so SCE is not asked.
    expect(core.callsOf("read_judgment")).toHaveLength(0);
  });

  it("is not asked for a work that has no model", async () => {
    await click("Beta");

    expect(root.querySelector(".acceptance")).toBeNull();
    expect(core.callsOf("read_judgment")).toHaveLength(0);
  });

  it("shows SCE's count, each requirement in SCE's word, and the page exactly as SCE wrote it", async () => {
    core.setModel("alpha", "<scxml>MISSING SCENARIO</scxml>", headOf("alpha"));
    await click("Alpha");

    expect(acceptanceText()).toContain("SCE measured the design against 2 requirements (synthesized).");
    expect([...root.querySelectorAll(".acceptance .tally li")].map((li) => li.textContent)).toEqual([
      "1 missing: nothing in the design carries it",
      "1 needs-scenario: only a test can settle it",
    ]);
    expect(requirementRows()).toEqual([
      ["R1", "missing", "S1", "nowhere"],
      ["R2", "needs-scenario", "S2", "states.s1"],
    ]);
    expect(root.querySelector(".acceptance .page pre")?.textContent).toBe("ACCEPTANCE REPORT\n  2 requirements\n");
  });

  it("marks the requirements SCE finds unsettled, says what each of SCE's words means, and shows a new one as spelled", async () => {
    core.setModel("alpha", "<scxml>MISSING DANGLING</scxml>", headOf("alpha"));
    await click("Alpha");
    const lines = (selector: string): Array<string | null> =>
      [...root.querySelectorAll(selector)].map((li) => li.textContent);
    expect(lines(".acceptance .tally li")).toEqual([
      "1 missing: nothing in the design carries it",
      "1 dangling: the design cites it and the list has no such requirement",
    ]);
    expect(lines(".acceptance .tally li.unsettled")).toHaveLength(2);
    expect(acceptanceText()).toContain("The marked lines are requirements the design leaves unsettled.");

    // A word a later SCE writes is shown as it was spelled, and is not guessed to be a gap.
    core.setModel("alpha", "<scxml>WAIVED</scxml>", headOf("alpha"));
    await click("Read again");
    expect(lines(".acceptance .tally li")).toEqual(["1 implemented: a node of the design carries it", "1 waived"]);
    expect(lines(".acceptance .tally li.unsettled")).toEqual([]);
    expect(acceptanceText()).not.toContain("leaves unsettled");
  });

  it("tells the owner of the gaps before they accept, and still lets them accept", async () => {
    core.setModel("alpha", "<scxml>MISSING</scxml>", headOf("alpha"));
    await click("Alpha");
    expect([...root.querySelectorAll(".acceptance .tally li.unsettled")].map((li) => li.textContent)).toEqual([
      "1 missing: nothing in the design carries it",
    ]);
    expect([...root.querySelectorAll(".acceptance .gaps li")].map((li) => li.textContent)).toEqual([
      "Matters SCE lists as left open: 1",
    ]);
    expect(acceptanceText()).toContain("Nothing has been accepted yet.");
    expect(acceptButton().disabled).toBe(false);

    await click("Accept this design");

    const [sent] = core.callsOf("accept");
    expect(core.callsOf("accept")).toHaveLength(1);
    expect(sent?.["id"]).toBe("alpha");
    const expectation = sent?.["expect"] as Record<string, string | null>;
    // Exactly what the page showed, revision for revision; the owner had answered nothing.
    expect(Object.keys(expectation).sort()).toEqual(["answers", "model", "requirements", "source"]);
    expect(expectation["answers"]).toBeNull();
    expect(expectation["source"]).toBe(headOf("alpha"));
    expect(expectation["model"]).toBe(core.revision("model:<scxml>MISSING</scxml>"));
    expect(acceptanceText()).toContain("It holds");
    expect(acceptanceText()).toContain("Accepted here, in this application.");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toBe("This design is accepted as it is.");
  });

  it("accepts only what it showed: what moved meanwhile is not accepted, and what is there now is shown", async () => {
    await click("Alpha");
    // Another entrance saves a different design after this screen showed the page.
    core.setModel("alpha", "<scxml><!-- another --></scxml>", headOf("alpha"));
    await click("Accept this design");

    expect(core.callsOf("accept")).toHaveLength(1);
    expect(acceptanceText()).toContain("Nothing was accepted: model changed after you were shown it");
    expect(acceptanceText()).toContain("Nothing has been accepted yet.");
    // What is there now was read, so the next press is made knowing it: SCE was asked about the
    // design the page showed, and then about the one that is there now.
    const asked = core.callsOf("read_judgment").map((a) => (a["basis"] as Record<string, string>)["model"]);
    expect(asked).toEqual([core.revision("model:<scxml/>"), core.revision("model:<scxml><!-- another --></scxml>")]);
    await click("Accept this design");

    const second = core.callsOf("accept")[1]?.["expect"] as Record<string, string>;
    expect(second["model"]).toBe(core.revision("model:<scxml><!-- another --></scxml>"));
    expect(acceptanceText()).toContain("It holds");
    expect(acceptanceText()).not.toContain("Nothing was accepted");
  });

  it("is withheld while the saved answers are still being read, and offered when they are shown", async () => {
    core.setAnswers("alpha", { "open-guard": "Only listed cards." });
    const slow = core.hold("read_answers");
    await click("Alpha");

    // The page was measured with the answers, and the owner has not been shown them yet.
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("still being read");
    acceptButton().click();
    await settle();
    expect(core.callsOf("accept")).toHaveLength(0);

    slow.release();
    await settle();
    expect(fieldOf("open-guard").value).toBe("Only listed cards.");
    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");
  });

  it("stays withheld when the saved answers cannot be read, and says so", async () => {
    core.setAnswers("alpha", { "open-guard": "Only listed cards." });
    core.failNext("read_answers", new CommandFailure("corrupt", "the saved answers are not what the store wrote"));
    await click("Alpha");

    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("could not be read");
  });

  it("is withheld when the answers read are not the ones the page measured", async () => {
    core.setAnswers("alpha", { "open-guard": "Only listed cards." });
    await click("Alpha");
    expect(acceptButton().disabled).toBe(false);

    // Answers saved from another entrance after the page was measured; this screen read them before.
    core.setAnswers("alpha", { "open-guard": "Any card." });
    await click("Read again");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("not the one the design was measured against");
  });

  it("accepts nothing when the text moved on in another entrance after the page was shown", async () => {
    await click("Alpha");
    // The text moves on in another entrance; this screen has not read it yet.
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two", "alpha three"]);
    await click("Accept this design");

    expect(acceptanceText()).toContain("Nothing was accepted: source changed after you were shown it");
    expect(core.callsOf("accept")).toHaveLength(1);
  });

  it("is withheld while text or answers are typed and not saved, with the reason, and offered again when they are not", async () => {
    await click("Alpha");
    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");

    await type("alpha changed");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toBe("Save the text and your answers first: what is accepted is what is saved.");
    await type("alpha two");
    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");

    await answer("open-guard", "Any card on the list.");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("Save the text and your answers first");
    await answer("open-guard", "");
    expect(acceptButton().disabled).toBe(false);
    expect(core.callsOf("accept")).toHaveLength(0);
  });

  it("is withheld for a design or a list written for an earlier text, and says so", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha one"));
    await click("Alpha");

    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("written for an earlier text");
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", core.revision("alpha one"));
    await click("Read again");
    expect(acceptanceText()).toContain("The requirement list was written for an earlier text");
    expect(acceptButton().disabled).toBe(true);
  });

  it("is withheld while the text on screen is not the one the design was measured against", async () => {
    await click("Alpha");
    expect(acceptButton().disabled).toBe(false);

    // Another entrance saves a new text, and the authoring client writes the design and
    // the list for it. This screen reads the model again, and the text cannot be read just
    // now: what it shows is not the text the design is about.
    await core.call("save_source", { id: "alpha", text: "alpha three", base: headOf("alpha") });
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    core.failNext("read_source", new CommandFailure("transport", "temporary disconnect"));
    await click("Read again");

    expect(editor().value).toBe("alpha two");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("not the one the design was measured against");
    // The model is not called current beside a text it was not read for.
    expect(modelText()).not.toContain("written for the text as it is now");
    expect(modelText()).toContain("another text");
    acceptButton().click();
    await settle();
    expect(core.callsOf("accept")).toHaveLength(0);

    // Reading the work again shows the text the design is about, and the owner may accept.
    await click("Alpha");
    expect(editor().value).toBe("alpha three");
    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");
  });

  it("reads the text again with the model when the editor holds nothing of the person's", async () => {
    await click("Alpha");

    await core.call("save_source", { id: "alpha", text: "alpha three", base: headOf("alpha") });
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    await click("Read again");

    expect(editor().value).toBe("alpha three");
    expect(modelText()).toContain("written for the text as it is now");
    expect(acceptButton().disabled).toBe(false);
  });

  it("does not ask the owner to read the text again when the screen already shows it", async () => {
    await click("Alpha");
    await click("Read again");

    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");
  });

  it("says an acceptance lapsed, in SCE's sentence, and offers to accept the design as it is now", async () => {
    core.setAcceptance("alpha");
    core.setModel("alpha", "<scxml><!-- edited --></scxml>", headOf("alpha"));
    await click("Alpha");

    expect(acceptanceText()).toContain("no longer holds. SCE says: design/model.scxml moved");
    expect(acceptanceText()).toContain("What SCE listed as left open when it was accepted");
    expect(acceptButton().textContent).toBe("Accept the design as it is now");
    expect(acceptButton().disabled).toBe(false);
  });

  const revisionText = (): string => root.querySelector(".acceptance .revision")?.textContent ?? "";

  it("says what the revision did since the owner accepted, as the core's verdict, counts and page", async () => {
    core.setAcceptance("alpha");
    await click("Alpha");

    expect(revisionText()).toContain("What changed since you accepted");
    expect(revisionText()).toContain("The revision stayed within the reach of what changed");
    expect(revisionText()).toContain("2 requirements, 2 compared: 0 outside the reach, 0 to look at again.");
    // The page is the product's, exactly as it wrote it.
    expect(root.querySelector(".acceptance .revision .page pre")?.textContent).toBe("REVISION REPORT\n  2 requirements\n");
  });

  it("says when the design moved where the words did not, in a warning", async () => {
    core.revisionVerdict = "outside-reach";
    core.setAcceptance("alpha");
    await click("Alpha");

    expect(revisionText()).toContain("The design moved where the words did not.");
    expect(root.querySelector(".acceptance .revision .banner-warn")).not.toBeNull();
    expect(revisionText()).toContain("1 outside the reach");
  });

  it("does not call a revision within reach when the product compared nothing, as its own page says", async () => {
    // The product says "within-reach" with a warning that it saw no evidence for any requirement;
    // the screen must not say more than the product does.
    core.revisionCompares = false;
    core.setAcceptance("alpha");
    await click("Alpha");

    expect(revisionText()).toContain("Nothing was compared");
    expect(revisionText()).not.toContain("stayed within the reach");
    expect(revisionText()).toContain("2 requirements, 0 compared");
    expect(root.querySelector(".acceptance .revision .banner-warn")).not.toBeNull();
    // The product's page is still there to read.
    expect(root.querySelector(".acceptance .revision .page pre")).not.toBeNull();
  });

  it("has nothing to say of a revision for a work nobody accepted, and does not ask", async () => {
    await click("Alpha");

    expect(root.querySelector(".acceptance .revision")).toBeNull();
    expect(core.callsOf("read_revision_report")).toHaveLength(0);
  });

  it("holds the comparison back in the core's sentence while the text moved on and nothing was written for it", async () => {
    core.setAcceptance("alpha");
    core.saveElsewhere("alpha", "alpha from elsewhere");
    await click("Alpha");

    expect(revisionText()).toContain(
      "Not compared yet: the specification was changed after the requirement list and the model was written for it",
    );
    // No verdict is made up for a state that cannot be compared.
    expect(revisionText()).not.toContain("within the reach");
    expect(root.querySelector(".acceptance .revision .page")).toBeNull();
  });

  it("says in words when the revision report cannot be read, and the rest of the panel still shows", async () => {
    core.setAcceptance("alpha");
    core.failNext("read_revision_report", new CommandFailure("transport", "the server did not answer"));
    await click("Alpha");

    expect(revisionText()).toContain("could not be read");
    expect(acceptanceText()).toContain("SCE measured the design against 2 requirements");
  });

  it("says when an acceptance was relayed by an authoring client and not made in this application", async () => {
    core.setAcceptance("alpha", "relayed");
    await click("Alpha");

    expect(acceptanceText()).toContain("It holds");
    expect(acceptanceText()).toContain("Relayed by an authoring client: it was not accepted in this application.");
  });

  it("says SCE did not measure the design, keeps what was accepted, and offers nothing to accept", async () => {
    core.setAcceptance("alpha");
    core.sceRefuses("report", "sce-timeout", "SCE did not answer within 30 s");
    await click("Alpha");

    expect(acceptanceText()).toContain("SCE did not measure the design against the list: SCE did not answer within 30 s");
    expect(acceptanceText()).toContain("It holds");
    expect(root.querySelector(".acceptance table")).toBeNull();
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("SCE did not measure the design");
  });

  it("says in words when the acceptance cannot be read, and the review still shows", async () => {
    core.setAcceptance("alpha");
    core.sceRefuses("acceptance", "sce-failed", "the product crashed");
    await click("Alpha");

    // Said in words with SCE's reason; the measure it did give is shown, and the review still shows.
    expect(acceptanceText()).toContain("could not say whether it still holds: the product crashed");
    expect(acceptanceText()).toContain("SCE measured the design against 2 requirements");
    expect(pseudo()).not.toBeNull();
  });

  it("is read again when the answers are saved, which are part of what is accepted", async () => {
    await click("Alpha");
    await click("Accept this design");
    expect(acceptanceText()).toContain("It holds");
    const before = core.callsOf("read_judgment").length;

    await answer("open-guard", "Any card on the list.");
    await click("Save answers");

    expect(core.callsOf("read_judgment")).toHaveLength(before + 1);
    expect(acceptanceText()).toContain("no longer holds. SCE says: spec/answers.json moved");
    expect(acceptButton().textContent).toBe("Accept the design as it is now");
  });

  it("is not put under another work when it arrives late, and is gone with a work that is removed", async () => {
    core.setRequirements("alpha", headOf("alpha"), ["A1", "A2"]);
    core.setModel("beta", "<scxml/>", headOf("beta"));
    core.setRequirements("beta", headOf("beta"), ["B1"]);
    const slow = core.hold("read_judgment", (a) => a["id"] === "alpha");
    await click("Alpha");
    await click("Beta");
    slow.release();
    await settle();

    expect(requirementRows().map((row) => row[0])).toEqual(["B1"]);
    await click("Remove this work");
    await click("Remove");
    expect(root.querySelector(".acceptance")).toBeNull();
  });

  it("is not accepted twice while the first is on its way, and a work is not removed under it", async () => {
    await click("Alpha");
    const held = core.hold("accept");
    buttons("Accept this design")[0]?.click();
    await settle();

    expect(acceptButton().textContent).toBe("Accepting...");
    expect(acceptButton().disabled).toBe(true);
    expect(buttons("Remove this work")[0]?.disabled).toBe(true);
    held.release();
    await settle();

    expect(core.callsOf("accept")).toHaveLength(1);
    expect(acceptanceText()).toContain("It holds");
  });
});

/** A timer the test holds: a question waits until it is let go, so what follows it can be looked at. */
class ManualTicker implements Ticker {
  private readonly waiting: Array<{ run: () => void; cancelled: boolean; ms: number }> = [];

  after(ms: number, run: () => void): () => void {
    const entry = { run, cancelled: false, ms };
    this.waiting.push(entry);
    return () => {
      entry.cancelled = true;
    };
  }

  /** The waits asked for and neither let go nor cancelled, in the order they were asked. */
  get pending(): number[] {
    return this.waiting.filter((w) => !w.cancelled).map((w) => w.ms);
  }

  /** Let the next wait end, and let the question it starts and what follows from it finish. */
  async fire(): Promise<void> {
    const next = this.waiting.find((w) => !w.cancelled);
    if (next === undefined) throw new Error("no question is waiting to be asked");
    next.cancelled = true;
    next.run();
    for (let i = 0; i < 3; i += 1) await settle();
  }
}

describe("a token the server asks for in the middle of a read", () => {
  beforeEach(async () => {
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
    core.addWork("beta", "Beta", ["beta one"]);
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    app = new App(root, {
      transport: core,
      storage: null,
      browserLanguage: "en",
      credentials: { token: () => null, save: () => undefined },
    });
    await app.start();
    await settle();
  });

  // Each of these reads is started beside the others when a work opens, and each is
  // one the server can refuse first. Whichever it is, the person is shown the form that
  // asks for the token: a refusal that only sets a flag leaves the screen as it was.
  for (const name of ["read_answers", "review", "read_work_snapshot", "read_judgment", "figures"]) {
    it(`draws the sign-in form when \`${name}\` is the read that is refused`, async () => {
      core.failNext(name, new CommandFailure(UNAUTHORIZED, "no token"));

      await click("Alpha");
      await settle();

      expect(root.querySelector("form.token-form"), name).not.toBeNull();
    });
  }

  it("draws it though nothing else is left to draw, the refused read being the last to answer", async () => {
    // A work with no model has nothing to draw once the model panel says so, and the
    // answers are read beside it: the redraw that the model panel's answer brings comes
    // first, so the refusal that follows has to draw the form itself.
    core.failNext("read_answers", new CommandFailure(UNAUTHORIZED, "no token"));

    await click("Beta");
    await settle();

    expect(root.querySelector("form.token-form"), root.textContent ?? "").not.toBeNull();
  });
});

describe("a work that moves under the screen", () => {
  let ticker: ManualTicker;

  beforeEach(async () => {
    ticker = new ManualTicker();
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
    core.addWork("beta", "Beta", ["beta one"]);
    app = new App(root, { transport: core, storage: null, browserLanguage: "en", ticker });
    await app.start();
    await settle();
  });

  /** What the panel says of whether the owner's acceptance holds: the banner, not the note under the button. */
  const verdict = (): string => root.querySelector(".accepted .banner")?.textContent ?? "";

  /** The design on screen. */
  const design = (): string => root.querySelector(".scxml")?.textContent ?? "";
  /** The acceptance panel is waiting for SCE, and says nothing of whether anything holds. */
  const waitingForSce = (): boolean =>
    acceptanceText().includes("Reading the requirements and what was accepted") && verdict() === "";

  // What SCE says of the work is asked of the revisions the screen read the design in, so a verdict
  // is of the design beside it and of no other. The work can move while SCE answers, and the screen
  // can be behind the core for a moment; it is never made to say of one design what is true of
  // another. The screens before this one read the design, the list, the standing and the report with
  // a command each and compared them, and were handed, in turn, a "holds" beside a changed design and
  // a "lapsed" beside a restored one.

  it("shows no verdict beside a design SCE has not judged yet, and the verdict of that design once it has", async () => {
    core.setModel("alpha", "<scxml><!-- initial --></scxml>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    await click("Alpha");

    // An authoring client saves a design and the owner accepts it from another window.
    core.setModel("alpha", "<scxml><!-- accepted --></scxml>", headOf("alpha"));
    core.setAcceptance("alpha");
    const asked = core.hold("read_judgment");
    await ticker.fire();

    expect(design()).toBe("<scxml><!-- accepted --></scxml>");
    expect(waitingForSce()).toBe(true);
    asked.release();
    await settle();
    expect(verdict()).toContain("It holds");
  });

  it("shows the verdict of the design it shows when the work moved on while SCE was asked, and none beside the next", async () => {
    core.setModel("alpha", "<scxml><!-- initial --></scxml>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    await click("Alpha");
    core.setModel("alpha", "<scxml><!-- accepted --></scxml>", headOf("alpha"));
    core.setAcceptance("alpha");
    const asked = core.hold("read_judgment");
    await ticker.fire();

    // The work moves again while SCE is asked about the design the screen read.
    const changed = "<scxml><!-- changed after SCE was asked --></scxml>";
    core.setModel("alpha", changed, headOf("alpha"));
    asked.release();
    await settle();
    // SCE's word is of that design, which is the one shown: it holds of it.
    expect(design()).toBe("<scxml><!-- accepted --></scxml>");
    expect(verdict()).toContain("It holds");

    // The next question sees the work moved. The changed design is not shown beside a verdict of the
    // accepted one, and SCE's word for it is read.
    const again = core.hold("read_judgment");
    await ticker.fire();
    expect(design()).toBe(changed);
    expect(verdict()).not.toContain("It holds");
    expect(waitingForSce()).toBe(true);
    again.release();
    await settle();
    expect(verdict()).toContain("no longer holds");
    expect(verdict()).not.toContain("It holds");
  });

  it("shows no lapse beside an accepted design that was restored, and says it holds once SCE has been asked", async () => {
    const accepted = "<scxml><!-- accepted and later restored --></scxml>";
    core.setModel("alpha", accepted, headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    core.setAcceptance("alpha");
    await click("Alpha");
    expect(verdict()).toContain("It holds");

    // Changed for a moment, and SCE is slow to say what it thinks of that.
    const temporary = "<scxml><!-- temporary change --></scxml>";
    core.setModel("alpha", temporary, headOf("alpha"));
    const asked = core.hold("read_judgment");
    await ticker.fire();
    expect(design()).toBe(temporary);
    expect(waitingForSce()).toBe(true);
    asked.release();
    await settle();
    expect(verdict()).toContain("no longer holds");

    // Put back as it was accepted: the lapse of the design before is not left beside it.
    core.setModel("alpha", accepted, headOf("alpha"));
    const restored = core.hold("read_judgment");
    await ticker.fire();
    expect(design()).toBe(accepted);
    expect(verdict()).not.toContain("no longer holds");
    expect(waitingForSce()).toBe(true);
    restored.release();
    await settle();
    expect(verdict()).toContain("It holds");
    expect(verdict()).not.toContain("no longer holds");
  });

  // SCE not answering is an answer, and the screen asks again by itself for the reasons that pass:
  // a generator that timed out, crashed or could not be started is not a fact about the design.
  // One that refused the design is: asking again of the same revisions says the same, and only
  // costs a run, so it is shown and not asked again until the work has moved.
  describe("when SCE did not answer", () => {
    const parts = ["report", "acceptance"] as const;
    const passing = ["sce-timeout", "sce-failed", "sce-unavailable"] as const;

    beforeEach(() => {
      core.setModel("alpha", "<scxml/>", headOf("alpha"));
      core.setRequirements("alpha", headOf("alpha"));
      core.setAcceptance("alpha");
    });

    const refusalShown = (): string => acceptanceText();

    for (const part of parts) {
      for (const kind of passing) {
        it(`asks again by itself, and recovers, from a ${kind} on the ${part}`, async () => {
          core.sceRefuses(part, kind, "SCE failed this once");
          await click("Alpha");
          expect(refusalShown()).toContain("SCE failed this once");
          expect(acceptButton()?.disabled ?? true).toBe(true);

          // Nothing in the saved work changed and nothing was pressed.
          await ticker.fire();

          expect(refusalShown()).not.toContain("SCE failed this once");
          expect(acceptanceText()).toContain("SCE measured the design against 2 requirements");
          expect(verdict()).toContain("It holds");
        });
      }
    }

    it("waits twice as long after each time it fails again, and the usual time once it answers", async () => {
      core.sceRefuses("report", "sce-timeout", "still down");
      core.sceRefuses("report", "sce-timeout", "still down");
      await click("Alpha");
      expect(ticker.pending).toEqual([2000]);

      await ticker.fire();
      expect(refusalShown()).toContain("still down");
      expect(ticker.pending).toEqual([4000]);

      await ticker.fire();
      expect(refusalShown()).not.toContain("still down");
      expect(ticker.pending).toEqual([2000]);
    });

    for (const part of parts) {
      it(`shows why the design was refused on the ${part} and does not ask again of the same revisions`, async () => {
        core.sceRefuses(part, "sce-refused", "SCE refused the design: xml/parse-error");
        await click("Alpha");
        expect(refusalShown()).toContain("SCE refused the design: xml/parse-error");
        const asked = core.callsOf("read_judgment").length;

        await ticker.fire();
        await ticker.fire();

        expect(core.callsOf("read_judgment")).toHaveLength(asked);
        expect(refusalShown()).toContain("SCE refused the design: xml/parse-error");
        // It was the design SCE refused: another design is asked about.
        core.setModel("alpha", "<scxml><!-- fixed --></scxml>", headOf("alpha"));
        await ticker.fire();
        expect(core.callsOf("read_judgment")).toHaveLength(asked + 1);
        expect(refusalShown()).not.toContain("xml/parse-error");
      });
    }

    it("shows the measure while it says SCE could not say whether the acceptance holds", async () => {
      core.sceRefuses("acceptance", "sce-timeout", "the check took too long");
      await click("Alpha");

      expect(acceptanceText()).toContain("SCE measured the design against 2 requirements");
      expect(verdict()).toContain("could not say whether it still holds");
      expect(verdict()).toContain("the check took too long");
      expect(verdict()).not.toContain("It holds");
      expect(acceptButton().disabled).toBe(true);
    });
  });

  it("does not put what SCE said of an earlier read over what it said of a later one", async () => {
    // SCE is slow about the work as it was read when it opened. Meanwhile the owner saves answers,
    // which are part of what was accepted: the work is read again and SCE answers about it at once.
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    core.setAcceptance("alpha");
    const late = core.hold("read_judgment");
    await click("Alpha");
    expect(waitingForSce()).toBe(true);

    await answer("open-guard", "Any card on the list.");
    await click("Save answers");
    expect(verdict()).toContain("no longer holds");

    late.release();
    await settle();
    // The word about the work without the answers, arriving late, is not put over the word about it with them.
    expect(verdict()).toContain("no longer holds");
    expect(verdict()).not.toContain("It holds");
  });

  it.each(["model", "list"])(
    "says where the %s stands to the text as the snapshot did, not as a claim made after it was read",
    async (part) => {
      // The same bytes can be kept again for a text that came later: the revision is the same, and
      // so is what SCE says of it, but where the design stands to the text is a claim that moves.
      // That is the snapshot's to say, read in the same state as the revisions SCE is asked about.
      const earlier = core.revision("alpha one");
      core.setModel("alpha", "<scxml/>", part === "model" ? earlier : headOf("alpha"));
      core.setRequirements("alpha", part === "model" ? headOf("alpha") : earlier);
      const asked = core.hold("read_judgment");
      await click("Alpha");

      // After the screen read it, the same bytes are kept again for the text as it is now.
      if (part === "model") core.setModel("alpha", "<scxml/>", headOf("alpha"));
      else core.keepRequirementsFor("alpha", headOf("alpha"));
      asked.release();
      await settle();

      // What the screen read was behind the text, and it is still said so: SCE's word does not move it.
      expect(acceptButton().disabled).toBe(true);
      expect(acceptNote()).toContain("written for an earlier text");

      // The next question reads where it stands now, and the design may be accepted.
      await ticker.fire();
      expect(acceptButton().disabled).toBe(false);
      expect(acceptNote()).toBe("");
    },
  );

  it.each(["model", "list"])(
    "withholds the accept at once when only the claim of which text the %s was written for moved, before SCE answers",
    async (part) => {
      core.setModel("alpha", "<scxml/>", headOf("alpha"));
      core.setRequirements("alpha", headOf("alpha"));
      await click("Alpha");
      expect(acceptButton().disabled).toBe(false);

      // The same bytes are kept again for the earlier text: the revisions are the ones SCE judged,
      // and what moved is where the design stands to the text, which the new snapshot says.
      const earlier = core.revision("alpha one");
      if (part === "model") core.setModel("alpha", "<scxml/>", earlier);
      else core.keepRequirementsFor("alpha", earlier);
      const asked = core.hold("read_judgment");
      await ticker.fire();

      // SCE has not been heard from again, and does not need to be for this: the panel stays with
      // SCE's words about the same bytes, and the claim in it is the new snapshot's.
      expect(acceptanceText()).toContain("SCE measured the design against 2 requirements");
      expect(acceptButton().disabled).toBe(true);
      expect(acceptNote()).toContain("written for an earlier text");

      asked.release();
      await settle();
      expect(acceptButton().disabled).toBe(true);
      expect(acceptNote()).toContain("written for an earlier text");
    },
  );

  it("refuses a core of another command-set version before it reads anything of a work", async () => {
    // A screen and a core that read each other's answers differently must not look connected: the
    // check is the screen's one chance to say so before a check and an accept fail on a shape.
    const stale = new FakeCore();
    stale.addWork("alpha", "Alpha", ["alpha one"]);
    const older = SUPPORTED_COMMAND_SET_VERSION - 1;
    stale.describeAs(older);
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    app = new App(root, { transport: stale, storage: null, browserLanguage: "en", ticker });
    await app.start();
    await settle();

    expect(root.textContent).toContain(String(older));
    expect(root.textContent).toContain(String(SUPPORTED_COMMAND_SET_VERSION));
    expect(stale.callsOf("list_works")).toHaveLength(0);
    expect(stale.callsOf("read_judgment")).toHaveLength(0);
  });

  it("keeps the design and the verdict it shows together while the next read of the work is slow", async () => {
    const accepted = "<scxml><!-- accepted --></scxml>";
    core.setModel("alpha", accepted, headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    core.setAcceptance("alpha");
    await click("Alpha");

    core.setModel("alpha", "<scxml><!-- changed --></scxml>", headOf("alpha"));
    const slow = core.hold("read_work_snapshot");
    await ticker.fire();

    // Nothing of the new design has been read: the old design, and the verdict of it, stay as they are.
    expect(design()).toBe(accepted);
    expect(verdict()).toContain("It holds");
    slow.release();
    await settle();
    expect(design()).toBe("<scxml><!-- changed --></scxml>");
    expect(verdict()).toContain("no longer holds");
  });

  it("shows each design with its own verdict while the work keeps moving, and settles when it stops", async () => {
    core.setModel("alpha", "<scxml><!-- accepted --></scxml>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    core.setAcceptance("alpha");
    await click("Alpha");
    expect(verdict()).toContain("It holds");

    for (const i of [0, 1, 2]) {
      core.setModel("alpha", `<scxml><!-- moving ${i} --></scxml>`, headOf("alpha"));
      const asked = core.hold("read_judgment");
      await ticker.fire();
      // SCE has not been heard on this design: the design before is not spoken for.
      expect(design()).toBe(`<scxml><!-- moving ${i} --></scxml>`);
      expect(waitingForSce()).toBe(true);
      expect(root.querySelector("#accept")).toBeNull();
      asked.release();
      await settle();
      expect(verdict()).toContain("no longer holds");
    }

    // The work stops moving, and the screen has nothing more to read.
    const reads = core.callsOf("read_judgment").length;
    await ticker.fire();
    expect(core.callsOf("read_judgment")).toHaveLength(reads);
  });

  it("shows a model an authoring client saved after the work was opened, without being asked", async () => {
    await click("Alpha");
    expect(modelText()).toContain("No model yet");

    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await ticker.fire();

    expect(images()).toHaveLength(2);
    expect(modelText()).toContain("written for the text as it is now");
  });

  it("keeps what the person typed when the work moves under them", async () => {
    await click("Alpha");
    await type("alpha edited");

    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await ticker.fire();

    expect(images()).toHaveLength(2);
    expect(editor().value).toBe("alpha edited");
    expect(status()).toBe("Unsaved changes");
  });

  it("shows a text another entrance saved when the editor holds nothing of the person's", async () => {
    await click("Alpha");
    expect(editor().value).toBe("alpha two");

    core.saveElsewhere("alpha", "alpha three");
    await ticker.fire();

    expect(editor().value).toBe("alpha three");
  });

  it("does not replace typed text with a text saved elsewhere, and says where the model now stands", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    expect(modelText()).toContain("written for the text as it is now");

    await type("mine");
    core.saveElsewhere("alpha", "theirs");
    await ticker.fire();

    expect(editor().value).toBe("mine");
    expect(root.querySelector(".model .banner-warn")?.textContent ?? "").toContain("earlier text");
  });

  it("reads a model kept for a later text again, and does not draw it again", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha one"));
    await click("Alpha");
    expect(root.querySelector(".model .banner-warn")?.textContent ?? "").toContain("earlier text");

    // The authoring client read the new text and kept the model: the same revision, written for another text.
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await ticker.fire();

    expect(root.querySelector(".model .banner-warn")).toBeNull();
    expect(modelText()).toContain("written for the text as it is now");
    expect(core.callsOf("figures")).toHaveLength(1);
  });

  it("shows an acceptance made elsewhere", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    await click("Alpha");
    expect(acceptanceText()).toContain("Nothing has been accepted yet.");
    // The page always says what an acceptance holds for; only an acceptance says that it does.
    expect(acceptanceText()).not.toContain("It holds: the text");

    core.setAcceptance("alpha");
    await ticker.fire();

    expect(acceptanceText()).toContain("It holds: the text, the list, the design and your answers are as they were.");
  });

  it("shows answers saved from another window when none are typed here", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    expect(fieldOf("open-guard").value).toBe("");

    core.setAnswers("alpha", { "open-guard": "Any card on the list opens it." });
    await ticker.fire();

    expect(fieldOf("open-guard").value).toBe("Any card on the list opens it.");
  });

  it("does not replace answers being typed with answers saved from another window", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    await answer("open-guard", "Typed here.");

    core.setAnswers("alpha", { "open-guard": "Saved there." });
    await ticker.fire();

    expect(fieldOf("open-guard").value).toBe("Typed here.");
    expect(core.callsOf("read_answers")).toHaveLength(1);
  });

  it("reads nothing again while the work stays as it is shown", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    await click("Alpha");
    const read = (name: string): number => core.callsOf(name).length;
    const before = [read("read_model"), read("read_source"), read("read_answers"), read("requirements_report")];
    const asked = read("read_work_heads");

    await ticker.fire();
    await ticker.fire();

    expect(core.callsOf("read_work_heads")).toHaveLength(asked + 2);
    expect([read("read_model"), read("read_source"), read("read_answers"), read("requirements_report")]).toEqual(before);
  });

  it("does not take its own save for a change from elsewhere", async () => {
    await click("Alpha");
    await type("alpha edited");
    await click("Save");
    const reads = core.callsOf("read_source").length;

    await ticker.fire();

    expect(core.callsOf("read_source")).toHaveLength(reads);
    expect(editor().value).toBe("alpha edited");
  });

  it("asks again only when the read it started is done, and then reads nothing more", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    const slow = core.hold("read_work_snapshot");
    core.setModel("alpha", "<scxml>next</scxml>", headOf("alpha"));
    await ticker.fire();

    // The read is on its way: the next question is not scheduled, so it cannot start the read over.
    expect(core.callsOf("read_work_snapshot")).toHaveLength(2);
    expect(ticker.pending).toEqual([]);
    slow.release();
    await settle();
    await settle();
    expect(ticker.pending).toEqual([2000]);
    await ticker.fire();

    // The read answered and the screen shows it: nothing more is read.
    expect(core.callsOf("read_work_snapshot")).toHaveLength(2);
    expect(root.querySelector(".scxml")?.textContent).toBe("<scxml>next</scxml>");
  });

  it("waits twice as long after the core fails to answer, and the usual time again once it does", async () => {
    await click("Alpha");
    expect(ticker.pending).toEqual([2000]);

    core.failNext("read_work_heads", new CommandFailure("io", "the works folder could not be read"));
    await ticker.fire();
    expect(ticker.pending).toEqual([4000]);

    await ticker.fire();
    expect(ticker.pending).toEqual([2000]);
  });

  it("stops asking about a work the person left", async () => {
    await click("Alpha");
    await click("Beta");
    expect(ticker.pending).toEqual([2000]);
    const before = core.callsOf("read_work_heads").length;

    await ticker.fire();

    expect(
      core
        .callsOf("read_work_heads")
        .slice(before)
        .map((a) => a["id"]),
    ).toEqual(["beta"]);
  });

  it("stops asking when the server wants a token, and asks again once the person has signed in", async () => {
    const held: { token: string | null } = { token: null };
    app = new App(root, {
      transport: core,
      storage: null,
      browserLanguage: "en",
      ticker,
      credentials: { token: () => held.token, save: (token) => (held.token = token) },
    });
    await app.start();
    await settle();
    await click("Alpha");
    core.failNext("read_work_heads", new CommandFailure(UNAUTHORIZED, "no token"));

    await ticker.fire();

    expect(root.querySelector("form.token-form")).not.toBeNull();
    expect(ticker.pending).toEqual([]);

    const field = root.querySelector('input[name="token"]') as HTMLInputElement;
    field.value = "0123456789abcdef";
    field.form?.dispatchEvent(new Event("submit", { cancelable: true, bubbles: true }));
    await settle();
    await settle();

    expect(held.token).toBe("0123456789abcdef");
    expect(root.querySelector("form.token-form")).toBeNull();
    expect(ticker.pending).toEqual([2000]);
  });

  it("review regression: recovers source refresh after one temporary failure", async () => {
    await click("Alpha");
    core.saveElsewhere("alpha", "alpha three");
    core.failNext("read_source", new CommandFailure("transport", "temporary disconnect"));
    await ticker.fire();
    expect(editor().value).toBe("alpha two");

    await ticker.fire();
    expect(editor().value).toBe("alpha three");
  });

  it("review regression: recovers model refresh after one temporary failure", async () => {
    await click("Alpha");
    core.setModel("alpha", "<scxml>new result</scxml>", headOf("alpha"));
    core.failNext("read_work_snapshot", new CommandFailure("transport", "temporary disconnect"));
    await ticker.fire();
    expect(modelText()).toContain("temporary disconnect");

    await ticker.fire();
    expect(root.querySelector(".scxml")?.textContent).toBe("<scxml>new result</scxml>");
  });

  it("review regression: resumes source refresh after typed changes are undone", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    await type("draft being typed here");
    core.saveElsewhere("alpha", "alpha three");
    await ticker.fire();
    expect(editor().value).toBe("draft being typed here");

    await type("alpha two");
    await ticker.fire();
    expect(editor().value).toBe("alpha three");
  });

  it("review regression: recovers requirements refresh after one temporary failure", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    core.setRequirements("alpha", headOf("alpha"));
    core.failNext("read_work_snapshot", new CommandFailure("transport", "temporary disconnect"));
    await ticker.fire();
    // The work is one read, so the design and the list beside it fail together and say so.
    expect(modelText()).toContain("temporary disconnect");

    await ticker.fire();
    expect(acceptanceText()).toContain("SCE measured the design against 2 requirements");
  });

  it("recovers what SCE said when it could not be read once, at the next question", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    await click("Alpha");
    core.setAcceptance("alpha");
    core.failNext("read_judgment", new CommandFailure("transport", "temporary disconnect"));
    await ticker.fire();
    // The design was read and is shown; what SCE says of it was not, and the panel says so.
    expect(acceptanceText()).toContain("temporary disconnect");
    expect(design()).toBe("<scxml/>");

    await ticker.fire();
    expect(verdict()).toContain("It holds");
    expect(acceptanceText()).toContain("SCE measured the design against 2 requirements");
  });

  it("asks SCE again when answers saved elsewhere moved while the person was typing over them, and says nothing until they are shown", async () => {
    // The answers are part of what the owner accepts. Another window saves some while this one
    // holds answers being typed: the screen cannot read them over the person's head. What SCE says
    // is asked of the revisions the core holds now, and it is not said of the answers on screen,
    // which are not those; it is said once the answers on screen are the ones judged.
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    core.setAcceptance("alpha");
    await click("Alpha");
    expect(verdict()).toContain("It holds");
    await answer("open-guard", "Typed here.");

    core.setAnswers("alpha", { "open-guard": "Saved there." });
    const before = core.callsOf("read_judgment").length;
    await ticker.fire();

    expect(fieldOf("open-guard").value).toBe("Typed here.");
    const asked = core.callsOf("read_judgment").slice(before);
    expect(asked).toHaveLength(1);
    expect((asked[0]?.["basis"] as Record<string, string>)["answers"]).toBe(core.revision("answers:1"));
    expect(verdict()).toContain("not known yet");
    expect(verdict()).not.toContain("It holds");

    // The typing is undone: the answers saved elsewhere are shown, and SCE's word for them is said.
    await answer("open-guard", "");
    await ticker.fire();
    expect(fieldOf("open-guard").value).toBe("Saved there.");
    expect(verdict()).toContain("no longer holds");
  });

  it("recovers answers that could not be read when the work opened, at the next question", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setAnswers("alpha", { "open-guard": "Any car" });
    core.failNext("read_answers", new CommandFailure("io", "temporary disconnect"));
    await click("Alpha");
    expect(reviewText()).toContain("Your answers could not be read");
    expect(fields()).toHaveLength(0);

    await ticker.fire();

    expect(reviewText()).not.toContain("Your answers could not be read");
    expect(fieldOf("open-guard").value).toBe("Any car");
  });

  it("recovers answers saved from another entrance and read badly once, at the next question", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    core.setAnswers("alpha", { "open-guard": "Any car" });
    core.failNext("read_answers", new CommandFailure("io", "temporary disconnect"));
    await ticker.fire();
    expect(fieldOf("open-guard").value).toBe("");

    await ticker.fire();

    expect(fieldOf("open-guard").value).toBe("Any car");
  });

  it("review regression: keeps the source and model coherent when they move while a work opens", async () => {
    const oldModel = "<scxml><!-- model of alpha two --></scxml>";
    const newModel = "<scxml><!-- model of alpha three --></scxml>";
    core.setModel("alpha", oldModel, headOf("alpha"));
    const slow = core.hold("read_model");
    await click("Alpha");
    expect(editor().value).toBe("alpha two");

    core.saveElsewhere("alpha", "alpha three");
    core.setModel("alpha", newModel, headOf("alpha"));
    slow.release();
    await settle();
    await settle();

    expect([["alpha two", oldModel], ["alpha three", newModel]]).toContainEqual([
      editor().value,
      root.querySelector(".scxml")?.textContent,
    ]);
  });

  it("says so when the work was taken away from another window, and stops asking", async () => {
    await click("Alpha");
    await core.call("remove_work", { id: "alpha" });

    await ticker.fire();

    expect(root.textContent).toContain("work `absent`");
    expect(ticker.pending).toEqual([]);
  });
});

// ---- asking for a model ---------------------------------------------------

describe("asking for a model", () => {
  let ticker: ManualTicker;

  beforeEach(async () => {
    ticker = new ManualTicker();
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
    core.addWork("beta", "Beta", ["beta one"]);
    // The person has chosen a connection, so a request is made for it and an executor can run it.
    // What a request made without one is read as is the business of the cases that clear this.
    core.setConnections([{ id: "claude", model: "opus", revision: hex(7) }], "claude");
    app = new App(root, { transport: core, storage: null, browserLanguage: "en", ticker });
    await app.start();
    await settle();
  });

  const generationStatus = (): string => root.querySelector("#generation-status")?.textContent ?? "";
  const generationAi = (): string => root.querySelector("#generation-ai")?.textContent ?? "";
  const button = (id: string): HTMLButtonElement | null => root.querySelector<HTMLButtonElement>(`#${id}`);

  async function press(id: string): Promise<void> {
    const found = button(id);
    if (found === null) throw new Error(`no #${id} in: ${root.textContent}`);
    found.click();
    await settle();
  }

  it("offers the button for a work that has text, and says that no AI is connected", async () => {
    await click("Alpha");

    expect(button("generate")?.textContent).toBe("Generate pseudocode");
    expect(generationStatus()).toContain("no pseudocode yet");
    expect(generationAi()).toContain("No AI is connected");
  });

  it("says which AI is there when one is", async () => {
    core.setAdapters([{ name: "desktop" }]);

    await click("Alpha");

    expect(generationAi()).toBe("Connected: desktop");
  });

  it("says why no AI is connected, in the words the shell gave, where the owner looks", async () => {
    core.setHosts([
      { name: "desktop", reason: "no Claude Code to write models with: install it, or set SCE_CLAUDE to its path" },
    ]);

    await click("Alpha");

    expect(generationAi()).toBe(
      "No AI is connected. desktop: no Claude Code to write models with: install it, or set SCE_CLAUDE to its path",
    );
  });

  it("does not repeat a reason once an AI is connected, or for a shell that stopped saying it", async () => {
    core.setHosts([
      { name: "desktop", reason: "no Claude Code" },
      { name: "web-shell", reason: "the executor is off", live: false },
    ]);
    core.setAdapters([{ name: "someone" }]);

    await click("Alpha");

    expect(generationAi()).toBe("Connected: someone");
    // The AI goes away; the screen hears of it at its next question, and then says why.
    core.setAdapters([]);
    await ticker.fire();
    expect(generationAi()).toBe("No AI is connected. desktop: no Claude Code");
    await press("generate");
    expect(generationStatus()).toContain("no AI is connected to take it");
    expect(generationAi()).toBe("No AI is connected. desktop: no Claude Code");
  });

  it("follows the shell coming up with its executor, without a press", async () => {
    core.setHosts([{ name: "desktop", reason: "no Claude Code" }]);
    await click("Alpha");
    expect(generationAi()).toContain("no Claude Code");

    core.setHosts([{ name: "desktop", hosting: true }]);
    core.setAdapters([{ name: "desktop" }]);
    await ticker.fire();

    expect(generationAi()).toBe("Connected: desktop");
  });

  it("does not offer a request for a work that has no text yet", async () => {
    core.addWork("empty", "Empty", []);
    await core.call("list_works");
    await app.start();
    await settle();

    await click("Empty");

    expect(button("generate")?.disabled).toBe(true);
  });

  it("makes the request for the connection that is the default, at the revision it read", async () => {
    core.setConnections([{ id: "claude", model: "opus", revision: hex(7) }], "claude");
    await app.start();
    await settle();
    await click("Alpha");

    await press("generate");

    expect(core.callsOf("request_generation")[0]).toMatchObject({
      connection: { id: "claude", revision: hex(7) },
    });
    expect(root.textContent).toContain("Will ask: Claude Code (opus)");
  });

  it("makes a request for no connection when none is chosen, and says none is", async () => {
    core.setConnections([], null);
    await app.start();
    await settle();
    await click("Alpha");

    await press("generate");

    expect(core.callsOf("request_generation")[0]).not.toHaveProperty("connection");
    expect(root.textContent).toContain("No AI connection is chosen yet");
  });

  it("says the connection moved, reads it again, and asks nothing, when it changed after it was read", async () => {
    core.setConnections([{ id: "claude", model: "opus", revision: hex(7) }], "claude");
    await app.start();
    await settle();
    await click("Alpha");
    core.setConnections([{ id: "claude", model: "sonnet", revision: hex(8) }], "claude");

    await press("generate");

    expect(root.textContent).toContain("The AI connection was changed after this screen read it");
    expect(root.textContent).toContain("Will ask: Claude Code (sonnet)");
    expect(core.callsOf("list_connections").length).toBeGreaterThanOrEqual(2);
    // The press that was refused made no request; the next press asks for the revision now kept.
    await press("generate");
    expect(core.callsOf("request_generation").at(-1)).toMatchObject({
      connection: { id: "claude", revision: hex(8) },
    });
  });

  it("shows the AI connection, with the commands that sign in when nobody is", async () => {
    const panel = root.querySelector("#ai-settings");
    expect(panel).not.toBeNull();
    expect(core.callsOf("read_claude_status")).toHaveLength(1);
    (panel as HTMLDetailsElement).open = true;
    (panel as HTMLDetailsElement).dispatchEvent(new Event("toggle"));
    expect([...(panel?.querySelectorAll("code") ?? [])].map((c) => c.textContent)).toEqual([
      "claude auth login",
      "claude auth login --console",
    ]);
  });

  it("keeps the panel open through a redraw that is not the person's", async () => {
    core.setAdapters([{ name: "desktop" }]);
    const panel = root.querySelector<HTMLDetailsElement>("#ai-settings")!;
    panel.open = true;
    panel.dispatchEvent(new Event("toggle"));

    await click("Alpha");

    expect(root.querySelector<HTMLDetailsElement>("#ai-settings")?.open).toBe(true);
  });

  it("keeps the program list under the person's keys while the program they chose is asked of", async () => {
    const panel = root.querySelector<HTMLDetailsElement>("#ai-settings")!;
    panel.open = true;
    panel.dispatchEvent(new Event("toggle"));
    const list = root.querySelector<HTMLSelectElement>("#ai-program")!;
    list.focus();

    list.value = "/home/me/.local/bin/claude";
    list.dispatchEvent(new Event("change"));
    await settle();

    // The settings were drawn again, for the answer about the program that was chosen...
    const after = root.querySelector<HTMLSelectElement>("#ai-program");
    expect(core.callsOf("read_claude_status").at(-1)).toMatchObject({ executable: "/home/me/.local/bin/claude" });
    expect(after).not.toBe(list);
    // ...and the list the person was in is still the one in hand.
    expect(document.activeElement).toBe(after);
  });

  it("asks about the text the screen shows, and says the request waits for the AI", async () => {
    core.setAdapters([{ name: "desktop" }]);
    await click("Alpha");

    await press("generate");

    expect(core.callsOf("request_generation")).toHaveLength(1);
    const asked = core.callsOf("request_generation")[0];
    expect(asked).toMatchObject({
      id: "alpha",
      origin: "gui",
      supersede: false,
      expect: { source: headOf("alpha"), answers: null },
    });
    expect(String(asked?.["key"])).toMatch(/^gui-/);
    expect(generationStatus()).toBe("The request is registered. Waiting for the AI to take it.");
    expect(button("generate")).toBeNull();
    expect(button("cancel-request")).not.toBeNull();
  });

  it("says that nobody is there to take a request that waits, when no AI is connected", async () => {
    await click("Alpha");

    await press("generate");

    expect(generationStatus()).toContain("no AI is connected to take it");
  });

  it("saves what is typed first, and asks about what was saved", async () => {
    await click("Alpha");
    await type("alpha typed");

    await press("generate");

    expect(core.headText("alpha")).toBe("alpha typed");
    const calls = core.calls.map((c) => c.name).filter((n) => n === "save_source" || n === "request_generation");
    expect(calls).toEqual(["save_source", "request_generation"]);
    expect(core.callsOf("request_generation")[0]).toMatchObject({
      expect: { source: core.revision("alpha typed") },
    });
    expect(app.hasUnsavedChanges()).toBe(false);
  });

  it("asks nothing when what was typed could not be saved, and leaves the conflict to the person", async () => {
    await click("Alpha");
    await type("alpha typed");
    core.saveElsewhere("alpha", "alpha from elsewhere");

    await press("generate");

    expect(core.callsOf("request_generation")).toHaveLength(0);
    expect(root.textContent).toContain("The text changed while you were editing");
    expect(editor().value).toBe("alpha typed");
  });

  it("is refused when the text moved under the screen, says so, and asks nothing of the AI", async () => {
    await click("Alpha");
    core.saveElsewhere("alpha", "alpha from elsewhere");

    await press("generate");

    expect(core.latestRequest("alpha")).toBeUndefined();
    expect(root.textContent).toContain("changed while the request was being made");
  });

  it("shows a request an executor took, and who holds it, without being asked", async () => {
    core.setAdapters([{ name: "desktop" }]);
    await click("Alpha");
    await press("generate");

    core.takeRequest("alpha", "desktop");
    await ticker.fire();

    expect(generationStatus()).toBe("The AI is writing the model (attempt 1, desktop).");
    expect(button("cancel-request")).not.toBeNull();
    expect(button("generate")).toBeNull();
  });

  it("shows the reason a request failed", async () => {
    await click("Alpha");
    await press("generate");
    core.takeRequest("alpha");
    await ticker.fire();

    core.failRequest("alpha", "The text never says which cards open the door.");
    await ticker.fire();

    expect(generationStatus()).toBe("The AI could not write the model: The text never says which cards open the door.");
    expect(button("generate")?.textContent).toBe("Generate again");
  });

  it("offers to replace a request that was let go of, and asks for the replacement as one", async () => {
    core.setAdapters([{ name: "desktop" }]);
    await click("Alpha");
    await press("generate");
    core.takeRequest("alpha");
    core.letGoOfRequest("alpha");
    await ticker.fire();

    expect(generationStatus()).toContain("stopped answering");
    expect(button("generate")?.textContent).toBe("Replace the request and generate again");

    await press("generate");

    expect(core.callsOf("request_generation")[1]).toMatchObject({ supersede: true });
    expect(core.latestRequest("alpha")?.state).toBe("queued");
    expect(generationStatus()).toContain("registered");
  });

  it("shows the model when the request completes, without a press, and says the request finished", async () => {
    core.setAdapters([{ name: "desktop" }]);
    await click("Alpha");
    expect(modelText()).toContain("No model yet");
    await press("generate");
    core.takeRequest("alpha");
    await ticker.fire();

    core.completeRequest("alpha");
    await ticker.fire();

    expect(images()).toHaveLength(2);
    expect(generationStatus()).toBe("The last request finished.");
    expect(button("generate")?.textContent).toBe("Generate again");
  });

  it("calls the open request off, and says the owner did", async () => {
    await click("Alpha");
    await press("generate");

    await press("cancel-request");

    expect(core.callsOf("cancel_request")).toEqual([{ id: "alpha", request: "req-1" }]);
    expect(core.latestRequest("alpha")?.state).toBe("cancelled");
    expect(generationStatus()).toBe("You cancelled the last request.");
    expect(button("cancel-request")).toBeNull();
  });

  it("does not make a second request while the first is on its way", async () => {
    await click("Alpha");
    const held = core.hold("request_generation");

    const first = press("generate");
    await settle();
    expect(button("generate")).toBeNull();
    expect(generationStatus()).toBe("Registering the request...");
    held.release();
    await first;

    expect(core.callsOf("request_generation")).toHaveLength(1);
  });

  it("says why a request waits for its connection, and offers to ask again with the one chosen now", async () => {
    core.setAdapters([{ name: "desktop" }]);
    core.setConnections([{ id: "claude", model: "opus", revision: hex(7) }], "claude");
    await app.start();
    await settle();
    await click("Alpha");
    await press("generate");
    const waiting = core.latestRequest("alpha")!;
    core.setHosts([
      {
        name: "desktop",
        hosting: true,
        waiting: [
          { work: "alpha", request: waiting.id, connection: "claude", reason: "nobody is signed in to Claude Code" },
        ],
      },
    ]);

    await ticker.fire();

    expect(generationStatus()).toContain("nobody is signed in to Claude Code");
    expect(button("generate")?.textContent).toBe("Generate again with the chosen connection");
    // Asking again replaces the one that waits, and is made for the connection kept now.
    core.setConnections([{ id: "claude", model: "sonnet", revision: hex(8) }], "claude");
    await app.start();
    await settle();
    await click("Alpha");
    await press("generate");
    expect(core.callsOf("request_generation").at(-1)).toMatchObject({
      supersede: true,
      connection: { id: "claude", revision: hex(8) },
    });
  });

  it("says a request nobody chose a connection for is not run by the application, and asks again with the one chosen", async () => {
    core.setAdapters([{ name: "desktop" }]);
    core.setConnections([{ id: "claude", model: "opus", revision: hex(7) }], "claude");
    await core.call("request_generation", {
      id: "alpha",
      key: "before-connections",
      origin: "gui",
      expect: { source: headOf("alpha"), answers: null },
      supersede: false,
    });
    await app.start();
    await settle();

    await click("Alpha");

    expect(generationStatus()).toContain("made without choosing an AI connection");
    expect(button("generate")?.textContent).toBe("Generate again with the chosen connection");
    await press("generate");
    expect(core.callsOf("request_generation").at(-1)).toMatchObject({
      supersede: true,
      connection: { id: "claude", revision: hex(7) },
    });
    expect(generationStatus()).not.toContain("made without choosing");
  });

  it("opens the AI connection instead of asking, when nothing is chosen for a request that was made without one", async () => {
    core.setConnections([], null);
    core.setAdapters([{ name: "desktop" }]);
    await core.call("request_generation", {
      id: "alpha",
      key: "before-connections",
      origin: "gui",
      expect: { source: headOf("alpha"), answers: null },
      supersede: false,
    });
    await app.start();
    await settle();
    await click("Alpha");
    const asked = core.callsOf("request_generation").length;

    await press("generate");

    expect(core.callsOf("request_generation")).toHaveLength(asked);
    expect(root.querySelector<HTMLDetailsElement>("#ai-settings")?.open).toBe(true);
    expect(root.textContent).toContain("Choose and save an AI connection first");
  });

  it("offers to replace an open request that was made from another window", async () => {
    await click("Alpha");
    // Another window asked first: this screen has not heard of it yet.
    await core.call("request_generation", {
      id: "alpha",
      key: "other-window",
      origin: "gui",
      expect: { source: headOf("alpha"), answers: null },
      supersede: false,
    });

    await press("generate");

    expect(root.textContent).toContain("already open");
    await press("replace-request");

    expect(core.callsOf("request_generation").at(-1)).toMatchObject({ supersede: true });
    expect(core.latestRequest("alpha")?.id).toBe("req-2");
  });

  it("asks before a save that would end the open request, and saves when told to", async () => {
    await click("Alpha");
    await press("generate");
    await type("alpha changed meanwhile");

    await click("Save");

    expect(root.textContent).toContain("A model is being written for this text");
    expect(core.callsOf("save_source")).toHaveLength(0);
    expect(app.hasUnsavedChanges()).toBe(true);

    await press("guard-save");

    expect(core.callsOf("save_source")).toHaveLength(1);
    expect(core.headText("alpha")).toBe("alpha changed meanwhile");
    expect(root.textContent).not.toContain("A model is being written for this text");
    expect(generationStatus()).toContain("ended because the text or the answers");
    expect(core.latestRequest("alpha")?.state).toBe("superseded");
  });

  it("leaves what was typed unsaved, and the request open, when the person says not to save", async () => {
    await click("Alpha");
    await press("generate");
    await type("alpha changed meanwhile");
    await click("Save");

    await press("guard-leave");

    expect(core.callsOf("save_source")).toHaveLength(0);
    expect(editor().value).toBe("alpha changed meanwhile");
    expect(core.latestRequest("alpha")?.state).toBe("queued");
    expect(root.textContent).not.toContain("A model is being written for this text");
  });

  it("asks before saving answers while a request is open, as it does for the text", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    await press("generate");
    await answer("open-guard", "Any card on the list opens it.");

    await click("Save answers");

    expect(root.textContent).toContain("A model is being written for this text");
    expect(core.callsOf("save_answers")).toHaveLength(0);

    await press("guard-save");

    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card on the list opens it." });
    expect(core.latestRequest("alpha")?.state).toBe("superseded");
  });

  it("does not ask when the request is over, or when nothing was asked for", async () => {
    await click("Alpha");
    await type("alpha typed");
    await click("Save");
    expect(root.textContent).not.toContain("A model is being written for this text");

    await press("generate");
    core.takeRequest("alpha");
    core.completeRequest("alpha");
    await ticker.fire();
    await type("alpha typed again");
    await click("Save");

    expect(root.textContent).not.toContain("A model is being written for this text");
    expect(core.headText("alpha")).toBe("alpha typed again");
  });

  it("asks nothing about a request of the work the person has left", async () => {
    core.setAdapters([{ name: "desktop" }]);
    await click("Alpha");
    await press("generate");
    await click("Beta");

    expect(generationStatus()).toContain("no pseudocode yet");
    expect(button("cancel-request")).toBeNull();
  });
});

// ---- where an answer stands -----------------------------------------------

describe("where an answer stands", () => {
  let ticker: ManualTicker;

  beforeEach(async () => {
    ticker = new ManualTicker();
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    core.setAdapters([{ name: "desktop" }]);
    core.setConnections([{ id: "claude", model: "opus", revision: hex(7) }], "claude");
    app = new App(root, { transport: core, storage: null, browserLanguage: "en", ticker });
    await app.start();
    await settle();
  });

  const stateOf = (id: string): string => root.querySelector(`[data-answer-state="${id}"]`)?.textContent ?? "(none)";
  const regenerate = (): HTMLButtonElement | null => root.querySelector<HTMLButtonElement>("#regenerate");

  async function pressRegenerate(): Promise<void> {
    const found = regenerate();
    if (found === null) throw new Error(`no regenerate button in: ${root.textContent}`);
    found.click();
    await settle();
  }

  it("says nothing of a question nobody answered, and that an answer is typed and not saved", async () => {
    await click("Alpha");
    expect(stateOf("open-guard")).toBe("");

    await answer("open-guard", "Any card on the list opens it.");

    expect(stateOf("open-guard")).toBe("Typed, not saved yet.");
    expect(stateOf("close-delay")).toBe("");
  });

  it("says it is saved, and not in the model shown, once it is saved", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card on the list opens it.");

    await click("Save answers");

    expect(stateOf("open-guard")).toBe("Saved. It is not in the model shown yet.");
  });

  it("saves the answers typed, asks about what is then saved, and says the AI is writing it in", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card on the list opens it.");

    await pressRegenerate();

    expect(core.callsOf("save_answers")).toHaveLength(1);
    const asked = core.callsOf("request_generation")[0];
    expect(asked?.["expect"]).toMatchObject({ source: headOf("alpha") });
    expect((asked?.["expect"] as { answers: string }).answers).toMatch(/^[0-9a-f]{64}$/);
    expect(stateOf("open-guard")).toBe("Saved. The AI is writing it into a new model.");
    expect(regenerate()).toBeNull();
  });

  it("says a model made after the answer that still asks the question did not take it in", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card on the list opens it.");
    await pressRegenerate();
    core.takeRequest("alpha");
    await ticker.fire();

    core.completeRequest("alpha");
    await ticker.fire();

    expect(stateOf("open-guard")).toContain("still asks this question");
    expect(regenerate()).not.toBeNull();
  });

  it("says a model made after the answer that no longer asks the question has it in", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card on the list opens it.");
    await pressRegenerate();
    core.takeRequest("alpha");
    await ticker.fire();

    core.completeRequest("alpha", "<scxml><!-- APPLIED:open-guard --></scxml>");
    await ticker.fire();

    // The question is no longer asked, so its answer is listed with the ones the model does not ask.
    expect(fields().map((f) => f.dataset["qid"])).toEqual(["close-delay", "open-guard"]);
    expect(stateOf("open-guard")).toContain("In the model shown");
  });

  it("goes back to saved when the answer is changed after the model was made", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card on the list opens it.");
    await pressRegenerate();
    core.takeRequest("alpha");
    await ticker.fire();
    core.completeRequest("alpha", "<scxml><!-- APPLIED:open-guard --></scxml>");
    await ticker.fire();

    await answer("open-guard", "Only cards of today.");
    expect(stateOf("open-guard")).toBe("Typed, not saved yet.");
    await click("Save answers");

    expect(stateOf("open-guard")).toBe("Saved. It is not in the model shown yet.");
  });

  it("is not claimed for a model that no request made", async () => {
    core.setAnswers("alpha", { "open-guard": "Any card on the list opens it." });
    await click("Alpha");

    expect(stateOf("open-guard")).toBe("Saved. It is not in the model shown yet.");
  });
});

// ---- what a question and a requirement are about --------------------------

describe("the sentence a question or a requirement is about", () => {
  const FIRST = "The door is closed until a card is shown.";
  const SECOND = "A listed card opens it.";
  let ticker: ManualTicker;

  beforeEach(async () => {
    ticker = new ManualTicker();
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["The door.", `A door controller.\n${FIRST}\n${SECOND}\n`]);
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    core.setCarried("alpha", { R1: ["states.closed"], R2: ["states.opened"] });
    core.setQuotes("alpha", { R1: FIRST, R2: SECOND });
    app = new App(root, { transport: core, storage: null, browserLanguage: "en", ticker });
    await app.start();
    await settle();
  });

  const grounds = (): HTMLElement[] => [...root.querySelectorAll<HTMLElement>("[data-ground]")];
  const groundOfQuestion = (id: string): HTMLElement | null =>
    fieldOf(id).closest(".question")?.querySelector<HTMLElement>("[data-ground]") ?? null;
  const selected = (): string => editor().value.slice(editor().selectionStart, editor().selectionEnd);
  const rowButton = (requirement: string, cls: string): HTMLButtonElement =>
    root.querySelector(`tr[data-requirement="${requirement}"] button.${cls}`) as HTMLButtonElement;

  /** The revision of the requirement list the core holds now: the one an accept would be matched against. */
  const listRevisionNow = async (): Promise<string> =>
    ((await core.call("read_work_snapshot", { id: "alpha" })) as { requirements: { revision: string } }).requirements
      .revision;

  // The sentences are the list's and the outcomes are SCE's measure of that same list, which SCE
  // is asked about by its revision: what the owner reads each requirement by is what SCE measured.
  // The screens before this one read the list and the measure with a command each, and could show
  // the sentences of one list beside the outcomes of another.

  it("accepts nothing of a list that moved while SCE measured the one the owner is reading", async () => {
    const slow = core.hold("read_judgment");
    await click("Alpha");

    // An authoring client saves another list, with other sentences, while SCE is asked.
    core.setRequirements("alpha", headOf("alpha"));
    core.setQuotes("alpha", { R1: SECOND, R2: FIRST });
    slow.release();
    await settle();

    // The sentence and the outcomes are of the list the screen read, which is the list SCE measured.
    expect(groundOfQuestion("open-guard")?.querySelector("blockquote")?.textContent).toBe(FIRST);
    await click("Accept this design");

    // The core holds another list and accepts nothing of it: the owner never accepts a list beside
    // the sentence of another. What is there now is read, with its own sentences.
    expect(core.callsOf("accept")).toHaveLength(1);
    expect(acceptanceText()).toContain("Nothing was accepted: requirements changed after you were shown it");
    expect(acceptanceText()).toContain("Nothing has been accepted yet.");
    expect(groundOfQuestion("open-guard")?.querySelector("blockquote")?.textContent).toBe(SECOND);
  });

  it("reads the list that moved at the next question, so a sentence is of the list measured and an accept is of it", async () => {
    // The text, the model and the answers stay the same; only the requirement list and the
    // sentences it quotes change, after the old list was read.
    await click("Alpha");
    core.setRequirements("alpha", headOf("alpha"));
    core.setQuotes("alpha", { R1: SECOND, R2: FIRST });
    await ticker.fire();

    // The owner reads R1 by the sentence of the list SCE measured, and accepts that list.
    expect(groundOfQuestion("open-guard")?.querySelector("blockquote")?.textContent).toBe(SECOND);
    const asked = core.callsOf("read_judgment").at(-1)?.["basis"] as { requirements: string };
    expect(asked.requirements).toBe(await listRevisionNow());
    await click("Accept this design");
    const accepted = core.callsOf("accept");
    expect(accepted).toHaveLength(1);
    expect((accepted[0]?.["expect"] as { requirements: string }).requirements).toBe(await listRevisionNow());
    expect(acceptanceText()).toContain("It holds");
  });

  it("shows the sentence of each list SCE measured while the list keeps moving, and none while SCE has not", async () => {
    await click("Alpha");

    for (const i of [0, 1, 2]) {
      core.setRequirements("alpha", headOf("alpha"));
      core.setQuotes("alpha", { R1: i % 2 === 0 ? SECOND : FIRST, R2: i % 2 === 0 ? FIRST : SECOND });
      const asked = core.hold("read_judgment");
      await ticker.fire();

      // The list moved and SCE has not measured it: the sentence of the list before is not shown
      // beside it, and nothing is offered to accept.
      expect(groundOfQuestion("open-guard")).toBeNull();
      expect(root.querySelector("#accept")).toBeNull();
      asked.release();
      await settle();
      expect(groundOfQuestion("open-guard")?.querySelector("blockquote")?.textContent).toBe(
        i % 2 === 0 ? SECOND : FIRST,
      );
      expect(acceptButton().disabled).toBe(false);
    }
  });

  it("is shown with each question the model asks inside a part a requirement is carried by", async () => {
    await click("Alpha");

    expect(grounds().map((g) => g.dataset["ground"])).toEqual(["R1", "R2"]);
    expect(groundOfQuestion("open-guard")?.textContent).toContain(FIRST);
    expect(groundOfQuestion("open-guard")?.textContent).toContain("requirement R1");
    expect(groundOfQuestion("close-delay")?.textContent).toContain(SECOND);
  });

  it("takes the person to the sentence in the text", async () => {
    await click("Alpha");

    groundOfQuestion("open-guard")?.querySelector("button")?.click();
    await settle();

    expect(selected()).toBe(FIRST);
  });

  it("says so when the text no longer holds the sentence, and leaves the text alone", async () => {
    await click("Alpha");
    await type("Something else entirely.");

    groundOfQuestion("open-guard")?.querySelector("button")?.click();
    await settle();

    expect(root.textContent).toContain("not in the text as it is now");
    expect(editor().value).toBe("Something else entirely.");
  });

  it("is not shown for a work with no requirement list", async () => {
    core.addWork("beta", "Beta", ["Another door."]);
    core.setModel("beta", "<scxml/>", headOf("beta"));
    await core.call("list_works");
    await app.start();
    await settle();

    await click("Beta");

    expect(grounds()).toEqual([]);
  });

  it("takes the person from a requirement to its sentence", async () => {
    await click("Alpha");

    rowButton("R2", "show-text").click();
    await settle();

    expect(selected()).toBe(SECOND);
  });

  it("lights the lines of the pseudocode that name the states a requirement is carried by", async () => {
    await click("Alpha");
    expect(root.querySelectorAll(".pseudo .lit")).toHaveLength(0);

    rowButton("R1", "mark-lines").click();
    await settle();

    expect([...root.querySelectorAll(".review .pseudo .lit")].map((l) => l.textContent?.trim())).toEqual([
      "state closed:",
    ]);
    expect(root.querySelector("[data-marked]")?.textContent).toContain("closed");
    expect(rowButton("R1", "mark-lines").textContent).toBe("Clear the mark");
  });

  it("marks one requirement at a time, and clears the mark with the same press", async () => {
    await click("Alpha");
    rowButton("R1", "mark-lines").click();
    await settle();

    rowButton("R2", "mark-lines").click();
    await settle();
    expect([...root.querySelectorAll(".review .pseudo .lit")].map((l) => l.textContent?.trim())).toEqual([
      "on open   -> opened",
    ]);

    rowButton("R2", "mark-lines").click();
    await settle();
    expect(root.querySelectorAll(".pseudo .lit")).toHaveLength(0);
    expect(root.querySelector("[data-marked]")).toBeNull();
  });

  it("says when no line of the page names the states", async () => {
    core.setCarried("alpha", { R1: ["states.nowhere"], R2: ["states.opened"] });
    await click("Alpha");

    rowButton("R1", "mark-lines").click();
    await settle();

    expect(root.querySelectorAll(".pseudo .lit")).toHaveLength(0);
    expect(root.querySelector("[data-marked]")?.textContent).toContain("No line of the page names nowhere");
  });
});

// ---- what changed from the model before -----------------------------------

describe("what a new model changed from the one before", () => {
  let ticker: ManualTicker;

  beforeEach(async () => {
    ticker = new ManualTicker();
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
    app = new App(root, { transport: core, storage: null, browserLanguage: "en", ticker });
    await app.start();
    await settle();
  });

  const change = (): HTMLElement | null => root.querySelector<HTMLElement>("[data-change]");
  const diffLines = (): string[] =>
    [...root.querySelectorAll(".diff span")].map((s) => (s.textContent ?? "").replace(/\n$/, ""));

  it("says nothing of the first model: there is nothing before it", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));

    await click("Alpha");

    expect(change()).toBeNull();
  });

  it("shows the lines of the pseudocode that were added and removed, with the lines around them", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setModel("alpha", "<scxml>LOCKED</scxml>", headOf("alpha"));

    await click("Alpha");

    expect(change()?.dataset["change"]).toBe("changed");
    expect(change()?.querySelector("summary")?.textContent).toBe(
      "What changed from the model before (2 added, 1 removed)",
    );
    // Each line is the page's own, after a two-column mark: space, `+` or `-`.
    expect(diffLines()).toEqual([
      "  machine door (lexicon: en)",
      "    state closed:",
      "-     on open   -> opened",
      "+     on open   -> locked",
      "+     on lock   -> locked",
    ]);
    expect(root.querySelectorAll(".diff .added")).toHaveLength(2);
    expect(root.querySelectorAll(".diff .removed")).toHaveLength(1);
  });

  it("says the pseudocode is the same when only the model's text differs", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setModel("alpha", "<scxml><!-- another comment --></scxml>", headOf("alpha"));

    await click("Alpha");

    expect(change()?.dataset["change"]).toBe("unchanged");
    expect(change()?.textContent).toBe("The pseudocode is the same as in the model before.");
  });

  it("says when SCE wrote no page for the model before, and does not compare", async () => {
    core.setModel("alpha", "<scxml>NOPAGE</scxml>", headOf("alpha"));
    core.setModel("alpha", "<scxml/>", headOf("alpha"));

    await click("Alpha");

    expect(change()?.dataset["change"]).toBe("unavailable");
    expect(change()?.textContent).toContain("did not write a page for the model before");
  });

  it("appears when a new model is saved from another entrance, without a press", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    expect(change()).toBeNull();

    core.setModel("alpha", "<scxml>LOCKED</scxml>", headOf("alpha"));
    await ticker.fire();
    await ticker.fire();

    expect(change()?.dataset["change"]).toBe("changed");
  });

  it("is not asked for again while the model stays the same", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setModel("alpha", "<scxml>LOCKED</scxml>", headOf("alpha"));
    await click("Alpha");
    const asked = core.callsOf("model_history").length;

    await ticker.fire();
    await ticker.fire();

    expect(core.callsOf("model_history")).toHaveLength(asked);
  });
});

// ---- putting an answer into the text ---------------------------------------

describe("an answer the owner chooses to put into the text", () => {
  beforeEach(async () => {
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "The door opens for a card."]);
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setAnswers("alpha", { "open-guard": "Any card on the list opens it." });
    app = new App(root, { transport: core, storage: null, browserLanguage: "en" });
    await app.start();
    await settle();
  });

  const addButton = (id: string): HTMLButtonElement | null =>
    fieldOf(id).closest(".question")?.querySelector<HTMLButtonElement>("button.add-to-text") ?? null;

  it("is offered for an answer that is saved, and not for a question nobody answered", async () => {
    await click("Alpha");

    expect(addButton("open-guard")).not.toBeNull();
    expect(addButton("close-delay")).toBeNull();
  });

  it("is added to the end of the text and not saved, so the owner puts it where it belongs", async () => {
    await click("Alpha");

    addButton("open-guard")?.click();
    await settle();

    expect(editor().value).toBe("The door opens for a card.\nAny card on the list opens it.\n");
    expect(app.hasUnsavedChanges()).toBe(true);
    expect(core.callsOf("save_source")).toHaveLength(0);
    expect(core.headText("alpha")).toBe("The door opens for a card.");
  });

  it("is not offered for an answer that is typed and not yet saved: only what is held can be added", async () => {
    await click("Alpha");

    await answer("open-guard", "Only cards of today.");

    expect(addButton("open-guard")?.hidden).toBe(true);
  });
});
