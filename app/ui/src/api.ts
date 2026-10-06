// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The commands, typed. Every answer is checked on the way in (see `contract.ts`),
// so a view receives a `Listing`, not `unknown`.

import {
  ContractError,
  parseAdapterListing,
  parseBundleRead,
  parseDescribed,
  parseFigures,
  parseHistory,
  parseHostListing,
  parseJudgment,
  parseListing,
  parseRegisteredRequest,
  parseRequestList,
  parseRequestReply,
  parseReadAnswers,
  parseReadSource,
  parseRemoved,
  parseReview,
  parseSaved,
  parseWork,
  parseWorkAndHead,
  parseWorkHeads,
  parseWorkSnapshot,
  sameBasis,
  type AdapterListing,
  type Answers,
  type Basis,
  type BundleRead,
  type Described,
  type Figures,
  type GenerationRequest,
  type HistoryEntry,
  type HostListing,
  type Judgment,
  type Listing,
  type RegisteredRequest,
  type Review,
  type Revision,
  type Saved,
  type SourceText,
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
    async requestGeneration(id, key, expect, supersede = false) {
      const args = { id, key, origin: "gui", expect, supersede };
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
  };
}
