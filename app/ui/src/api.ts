// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The commands, typed. Every answer is checked on the way in (see `contract.ts`),
// so a view receives a `Listing`, not `unknown`.

import {
  ContractError,
  parseAdapterListing,
  parseAuthPolicy,
  parseBundleRead,
  parseClaudeStatus,
  parseCodexStatus,
  parseConnectionListing,
  parseDefaultConnection,
  parseDeletedConnection,
  parseDescribed,
  parseFigures,
  parseFindClients,
  parseHistory,
  parseHostListing,
  parseJudgment,
  parseListing,
  parseReadConnection,
  parseRegisteredRequest,
  parseRequestList,
  parseRequestReply,
  parseReadAnswers,
  parseReadSource,
  parseRemoved,
  parseReview,
  parseSaved,
  parseServerStatus,
  parseWork,
  parseWorkAndHead,
  parseWorkHeads,
  parseWorkSnapshot,
  sameBasis,
  type AdapterListing,
  type Answers,
  type AuthPolicy,
  type Basis,
  type BundleRead,
  type ClaudeStatus,
  type CodexStatus,
  type Connection,
  type ConnectionListing,
  type Described,
  type Figures,
  type FoundClients,
  type GenerationRequest,
  type HistoryEntry,
  type HostListing,
  type Judgment,
  type Listing,
  type RegisteredRequest,
  type Review,
  type Revision,
  type Saved,
  type ServerStatus,
  type SourceText,
  type StoredConnection,
  type Work,
  type WorkAndHead,
  type WorkHeads,
  type WorkSnapshot,
} from "./contract";
import type { Transport } from "./ipc";

export interface Api {
  describe(): Promise<Described>;
  listWorks(): Promise<Listing>;
  createWork(title: string): Promise<Work>;
  readWork(id: string): Promise<WorkAndHead>;
  /** The current text, or the text of `revision`; `null` for a work with none yet. */
  readSource(id: string, revision?: Revision): Promise<SourceText | null>;
  /** Save `text` on top of `base` (`null` for a work's first text). Refused with `conflict` if `base` is no longer current. */
  saveSource(id: string, text: string, base: Revision | null): Promise<Saved>;
  history(id: string): Promise<HistoryEntry[]>;
  /**
   * What SCE draws of the work's model (the current one, or `revision`'s): its own
   * pictures, then the table of every value it states. Refused with `sce-refused`
   * when SCE will not draw it, in the product's own words.
   */
  figures(id: string, revision?: Revision, lexicon?: string): Promise<Figures>;
  /**
   * What SCE says of the work's model (the current one, or `revision`'s): its check
   * and its pseudocode page, in the vocabulary `lexicon` names. A model SCE refuses
   * is an answer (`check.verdict` is `refused`); SCE not answering at all is a
   * refusal with a `sce-*` kind.
   */
  review(id: string, revision?: Revision, lexicon?: string): Promise<Review>;
  /**
   * The owner's answers to the model's open questions (the current ones, or `revision`'s);
   * `null` when they have answered nothing.
   */
  readAnswers(id: string, revision?: Revision): Promise<Answers | null>;
  /**
   * The bundle that is the work's model and requirement list now, with the text and answers
   * the request that made it was about; `null` for a work whose model was not made by a request.
   */
  readBundle(id: string): Promise<BundleRead | null>;
  /** The revisions of the work's model, oldest first, each with the one it followed. */
  modelHistory(id: string): Promise<HistoryEntry[]>;
  /**
   * Save the answers as the owner now has them (question id to words; a question
   * left out is not answered) on top of `base` (`null` for a work's first answers).
   * Refused with `conflict` if `base` is no longer current.
   */
  saveAnswers(id: string, answers: Readonly<Record<string, string>>, base: Revision | null): Promise<Saved>;
  /**
   * What SCE says of the revisions `basis` names: SCE's measure of the design against the
   * list and the page the owner reads before accepting, and whether the acceptance
   * `acceptance` names (none when `null`) holds for them. It is of those revisions and of no
   * other, whatever has been saved since, so a screen that shows a work and asks about the
   * revisions it shows has a verdict that is of what it shows. SCE not answering is an answer
   * (`refusal` on the part it did not answer); a revision the work does not keep is refused
   * with `not-found`, and an answer that names other revisions than were asked is refused as
   * a broken contract rather than shown.
   */
  readJudgment(id: string, basis: Basis, acceptance: Revision | null): Promise<Judgment>;
  /**
   * Accept the design as the owner was shown it: `expect` is the `basis` of the report
   * they read. Refused with `moved` when any of it has changed since, and with
   * `not-current` when the design or the list was written for an earlier text; in
   * both nothing is accepted.
   */
  accept(id: string, expect: Basis): Promise<Saved>;
  /**
   * The work and its text, model, answers, requirement list and acceptance as they
   * stood together. A screen that is told something changed reads this, not one
   * command for each chain: a save landing between those reads would show a text of
   * one moment beside a model of another. It does not say whether the acceptance
   * still holds; that is SCE's answer, asked of the revisions this names (`readJudgment`).
   */
  readWorkSnapshot(id: string): Promise<WorkSnapshot>;
  /**
   * Where each chain of the work stands, and nothing it holds, read as one state. Cheap
   * enough to ask every few seconds; what it says is compared with what the screen shows.
   */
  readWorkHeads(id: string): Promise<WorkHeads>;
  /**
   * Ask for a model of the work's text. `expect` is what the screen read (the text, and
   * the owner's answers when they had given some); the request is refused as `moved` when
   * the work is no longer at them. `key` makes the same press, sent again, the request it
   * already made. An open request is a refusal (`active-request`) unless `supersede` says
   * the new one replaces it.
   */
  requestGeneration(
    id: string,
    key: string,
    expect: { readonly source: Revision; readonly answers: Revision | null },
    supersede?: boolean,
    connection?: ConnectionRef,
  ): Promise<RegisteredRequest>;
  /** One request of the work, as the clock reads it now. */
  readRequest(id: string, request: string): Promise<GenerationRequest>;
  /** The work's requests, the newest first. */
  listRequests(id: string): Promise<GenerationRequest[]>;
  /** Call the request off. Whoever holds it is told at its next word. */
  cancelRequest(id: string, request: string): Promise<GenerationRequest>;
  /** Which AI adapters are there, and what each can do. */
  readAdapterStatus(): Promise<AdapterListing>;
  /**
   * Whether each shell hosts an executor and, when it does not, why in words the owner can act on
   * (what to install, what to set). Said by the shells themselves, and counted while recent.
   */
  readHostStatus(): Promise<HostListing>;
  /**
   * Take a work out of the list. Its files stay in the works folder, so this can be
   * undone by hand; every later read or save of it is refused as `not-found`.
   */
  removeWork(id: string): Promise<Work>;
  /** The connections the person keeps, each with the revision it is kept under, and the default. */
  listConnections(): Promise<ConnectionListing>;
  /** One connection as it is now, or as it was at `revision`; `null` when there is none. */
  readConnection(id: string, revision?: Revision): Promise<StoredConnection | null>;
  /** Which ways of signing in the build uses, and the decision for each. */
  readAuthPolicy(): Promise<AuthPolicy>;
  /**
   * Save `connection` on top of `base` (`null` for a connection's first save). Refused with
   * `conflict` when `base` is no longer current, and with `not-allowed-here` by any entrance
   * but the desktop window.
   */
  saveConnection(connection: Connection, base: Revision | null): Promise<Saved>;
  /** Forget a connection. Requests made for it keep the revision they were made with. */
  deleteConnection(id: string, base: Revision): Promise<string>;
  /**
   * Make `id` the connection a new request uses (`null` for none). `expect` is the default the
   * screen read, so that two windows do not overwrite each other's choice unseen.
   */
  setDefaultConnection(id: string | null, expect: string | null): Promise<string | null>;
  /**
   * Whether Claude Code is installed, who is signed in to it, and how that is billed. Starts the
   * program to ask it, so only the desktop window may; any other entrance is refused with
   * `not-allowed-here`. `connection` is the connection the screen is about: the program that
   * answers is the one it names, whether or not it is the default (the default's program answers
   * when none is given, or when that connection is not kept yet). `executable` is the program the
   * screen is about to keep for it, which is not kept yet: that program answers, or the
   * application's own choice when it is `null`; a program the application did not find is
   * refused with `bad-connection`.
   */
  readClaudeStatus(connection?: string, executable?: string | null): Promise<ClaudeStatus>;
  /**
   * Whether Codex is installed, whether this build verified that version, and who is signed in by
   * each of the three sources a connection can take its credential from. Starts the program to
   * ask it, so only the desktop window may; any other entrance is refused with `not-allowed-here`.
   * `connection` and `executable` are the connection the screen is about and the program it is
   * about to keep for it, as for Claude Code.
   */
  readCodexStatus(connection?: string, executable?: string | null): Promise<CodexStatus>;
  /**
   * The programs of each client the application finds (on the search path, then in the folders
   * the official installers use), each saying it is that client. A connection names a program
   * only among those of its own client. Starts the programs to ask them, so only the desktop
   * window may; any other entrance is refused with `not-allowed-here`.
   */
  findClients(): Promise<FoundClients>;
  /**
   * What a model server at `serverUrl` is: whether it is there, whether its certificate is accepted,
   * whether it wants a key, and which models it lists. Calls the address, so only the desktop window
   * may; any other entrance is refused with `not-allowed-here`. An address a connection could not
   * keep is refused with `bad-connection`; a server that does not answer is a state of the answer.
   */
  readServerStatus(serverUrl: string): Promise<ServerStatus>;
}

/** A connection and the revision of it the person read: what a request is made for. */
export interface ConnectionRef {
  readonly id: string;
  readonly revision: Revision;
}

/**
 * What a status of a client is asked about. Each is sent only when it was given, and `executable`
 * is sent when it is `null` too: `null` is the application's own choice of program, which is not
 * the same as not saying (then it is the connection's program that answers).
 */
function statusArguments(connection: string | undefined, executable: string | null | undefined): Record<string, unknown> {
  return {
    ...(connection === undefined ? {} : { connection }),
    ...(executable === undefined ? {} : { executable }),
  };
}

export function apiOver(transport: Transport): Api {
  return {
    async describe() {
      return parseDescribed(await transport.call("describe"));
    },
    async listWorks() {
      return parseListing(await transport.call("list_works"));
    },
    async createWork(title) {
      return parseWork(await transport.call("create_work", { title }));
    },
    async readWork(id) {
      return parseWorkAndHead(await transport.call("read_work", { id }));
    },
    async readSource(id, revision) {
      const args = revision === undefined ? { id } : { id, revision };
      return parseReadSource(await transport.call("read_source", args));
    },
    async saveSource(id, text, base) {
      return parseSaved(await transport.call("save_source", { id, text, base }));
    },
    async history(id) {
      return parseHistory(await transport.call("history", { id }));
    },
    async figures(id, revision, lexicon) {
      const args = {
        id,
        ...(revision === undefined ? {} : { revision }),
        ...(lexicon === undefined ? {} : { lexicon }),
      };
      return parseFigures(await transport.call("figures", args));
    },
    async review(id, revision, lexicon) {
      const args = {
        id,
        ...(revision === undefined ? {} : { revision }),
        ...(lexicon === undefined ? {} : { lexicon }),
      };
      return parseReview(await transport.call("review", args));
    },
    async readAnswers(id, revision) {
      const args = revision === undefined ? { id } : { id, revision };
      return parseReadAnswers(await transport.call("read_answers", args));
    },
    async readBundle(id) {
      return parseBundleRead(await transport.call("read_bundle", { id }));
    },
    async modelHistory(id) {
      return parseHistory(await transport.call("model_history", { id }));
    },
    async saveAnswers(id, answers, base) {
      return parseSaved(await transport.call("save_answers", { id, answers, base }));
    },
    async readJudgment(id, basis, acceptance) {
      const args = { id, basis, ...(acceptance === null ? {} : { acceptance }) };
      const judgment = parseJudgment(await transport.call("read_judgment", args));
      // Only this caller knows what it asked, so only here can the answer be held to it.
      if (!sameBasis(judgment.basis, basis)) {
        throw new ContractError("read_judgment.basis", "the revisions that were asked about");
      }
      return judgment;
    },
    async accept(id, expect) {
      return parseSaved(await transport.call("accept", { id, expect }));
    },
    async readWorkSnapshot(id) {
      return parseWorkSnapshot(await transport.call("read_work_snapshot", { id }));
    },
    async readWorkHeads(id) {
      return parseWorkHeads(await transport.call("read_work_heads", { id }));
    },
    async requestGeneration(id, key, expect, supersede = false, connection) {
      const args = {
        id,
        key,
        origin: "gui",
        expect,
        supersede,
        ...(connection === undefined ? {} : { connection }),
      };
      return parseRegisteredRequest(await transport.call("request_generation", args));
    },
    async readRequest(id, request) {
      return parseRequestReply(await transport.call("read_request", { id, request }));
    },
    async listRequests(id) {
      return parseRequestList(await transport.call("list_requests", { id }));
    },
    async cancelRequest(id, request) {
      return parseRequestReply(await transport.call("cancel_request", { id, request }));
    },
    async readAdapterStatus() {
      return parseAdapterListing(await transport.call("read_adapter_status"));
    },
    async readHostStatus() {
      return parseHostListing(await transport.call("read_host_status"));
    },
    async removeWork(id) {
      return parseRemoved(await transport.call("remove_work", { id }));
    },
    async listConnections() {
      return parseConnectionListing(await transport.call("list_connections"));
    },
    async readConnection(id, revision) {
      const args = revision === undefined ? { id } : { id, revision };
      return parseReadConnection(await transport.call("read_connection", args));
    },
    async readAuthPolicy() {
      return parseAuthPolicy(await transport.call("read_auth_policy"));
    },
    async saveConnection(connection, base) {
      return parseSaved(await transport.call("save_connection", { connection, base }));
    },
    async deleteConnection(id, base) {
      return parseDeletedConnection(await transport.call("delete_connection", { id, base }));
    },
    async setDefaultConnection(id, expect) {
      return parseDefaultConnection(await transport.call("set_default_connection", { id, expect }));
    },
    async readClaudeStatus(connection, executable) {
      return parseClaudeStatus(await transport.call("read_claude_status", statusArguments(connection, executable)));
    },
    async readCodexStatus(connection, executable) {
      return parseCodexStatus(await transport.call("read_codex_status", statusArguments(connection, executable)));
    },
    async findClients() {
      return parseFindClients(await transport.call("find_clients"));
    },
    async readServerStatus(serverUrl) {
      return parseServerStatus(await transport.call("read_server_status", { server_url: serverUrl }));
    },
  };
}
