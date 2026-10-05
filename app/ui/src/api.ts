// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The commands, typed. Every answer is checked on the way in (see `contract.ts`),
// so a view receives a `Listing`, not `unknown`.

import {
  parseDescribed,
  parseFigures,
  parseHistory,
  parseListing,
  parseReadAcceptance,
  parseReadAnswers,
  parseReadModel,
  parseReadRequirements,
  parseReadSource,
  parseRemoved,
  parseRequirementsReport,
  parseReview,
  parseSaved,
  parseWork,
  parseWorkAndHead,
  parseWorkSnapshot,
  type Answers,
  type Basis,
  type Described,
  type Figures,
  type HistoryEntry,
  type Listing,
  type ReadAcceptance,
  type ReadModel,
  type ReadRequirements,
  type RequirementsReport,
  type Review,
  type Revision,
  type Saved,
  type SourceText,
  type Work,
  type WorkAndHead,
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
  /** The current model, or the model of `revision`; `model` is `null` for a work with none yet. */
  readModel(id: string, revision?: Revision): Promise<ReadModel>;
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
  /** The owner's answers to the model's open questions; `null` when they have answered nothing. */
  readAnswers(id: string): Promise<Answers | null>;
  /**
   * Save the answers as the owner now has them (question id to words; a question
   * left out is not answered) on top of `base` (`null` for a work's first answers).
   * Refused with `conflict` if `base` is no longer current.
   */
  saveAnswers(id: string, answers: Readonly<Record<string, string>>, base: Revision | null): Promise<Saved>;
  /**
   * The requirement list the work's text was read into, and where it stands to the
   * text. The list is written by an authoring client; this screen only reads it.
   */
  readRequirements(id: string): Promise<ReadRequirements>;
  /**
   * SCE's measure of the work's design against its requirement list, and the page
   * the owner reads before accepting. Refused with `not-found` when the work has no
   * model or no list yet, and with a `sce-*` kind when SCE does not answer.
   */
  requirementsReport(id: string): Promise<RequirementsReport>;
  /** Whether the owner's acceptance still holds, and what SCE says moved when it does not. */
  readAcceptance(id: string): Promise<ReadAcceptance>;
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
   * still holds; that is SCE's answer (`readAcceptance`).
   */
  readWorkSnapshot(id: string): Promise<WorkSnapshot>;
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
    async readModel(id, revision) {
      const args = revision === undefined ? { id } : { id, revision };
      return parseReadModel(await transport.call("read_model", args));
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
    async readAnswers(id) {
      return parseReadAnswers(await transport.call("read_answers", { id }));
    },
    async saveAnswers(id, answers, base) {
      return parseSaved(await transport.call("save_answers", { id, answers, base }));
    },
    async readRequirements(id) {
      return parseReadRequirements(await transport.call("read_requirements", { id }));
    },
    async requirementsReport(id) {
      return parseRequirementsReport(await transport.call("requirements_report", { id }));
    },
    async readAcceptance(id) {
      return parseReadAcceptance(await transport.call("read_acceptance", { id }));
    },
    async accept(id, expect) {
      return parseSaved(await transport.call("accept", { id, expect }));
    },
    async readWorkSnapshot(id) {
      return parseWorkSnapshot(await transport.call("read_work_snapshot", { id }));
    },
    async removeWork(id) {
      return parseRemoved(await transport.call("remove_work", { id }));
    },
  };
}
