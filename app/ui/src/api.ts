// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The commands, typed. Every answer is checked on the way in (see `contract.ts`),
// so a view receives a `Listing`, not `unknown`.

import {
  parseDescribed,
  parseHistory,
  parseListing,
  parseReadSource,
  parseSaved,
  parseWork,
  parseWorkAndHead,
  type Described,
  type HistoryEntry,
  type Listing,
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
  };
}
