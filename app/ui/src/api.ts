// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The commands, typed. Every answer is checked on the way in (see `contract.ts`),
// so a view receives a `Listing`, not `unknown`.

import {
  parseDescribed,
  parseFigures,
  parseHistory,
  parseListing,
  parseReadModel,
  parseReadSource,
  parseRemoved,
  parseReview,
  parseSaved,
  parseWork,
  parseWorkAndHead,
  type Described,
  type Figures,
  type HistoryEntry,
  type Listing,
  type ReadModel,
  type Review,
  type Revision,
  type Saved,
  type SourceText,
  type Work,
  type WorkAndHead,
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
    async removeWork(id) {
      return parseRemoved(await transport.call("remove_work", { id }));
    },
  };
}
