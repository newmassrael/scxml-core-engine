// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the screen says of asking for a model, and what it lets the person do about it.
//
// The core says where the work's latest request stands (`RequestHead`, by the clock: a lease
// that ran out is `interrupted` with nothing written) and which AI adapters are there. This
// turns that into one state a sentence can be chosen from, and the buttons that state allows.
// It decides nothing about a request: whether one may be made, replaced or cancelled is the
// core's, which refuses; the screen only does not offer what it knows would be refused.

import {
  isOpenRequest,
  type AdapterListing,
  type GenerationRequest,
  type RequestHead,
} from "./contract";

/** Where the work's latest request stands, in the terms the person acts on. */
export type Status =
  /** Never asked, or the last request is not something to say anything about. */
  | { readonly kind: "idle" }
  /** Asked, and nobody has taken it. `connected` says whether an AI is there to. */
  | { readonly kind: "queued"; readonly connected: boolean }
  | { readonly kind: "running"; readonly attempt: number; readonly holder: string | null }
  /** The one that held it stopped answering. Nothing was published. */
  | { readonly kind: "interrupted" }
  | { readonly kind: "failed"; readonly reason: string | null }
  | { readonly kind: "cancelled" }
  /** The text or the answers it was about were saved. */
  | { readonly kind: "superseded" }
  | { readonly kind: "completed" };

/** Whether an AI that can write a model is there now. */
export function isConnected(adapters: AdapterListing | null): boolean {
  return adapters !== null && adapters.adapters.some((a) => a.live && a.capabilities.includes("generate"));
}

/** The names of the adapters that are there now. */
export function connectedNames(adapters: AdapterListing | null): string[] {
  return adapters === null ? [] : adapters.adapters.filter((a) => a.live).map((a) => a.name);
}

/**
 * The state of the latest request. `detail` is what was read of it (the reason it failed,
 * who holds it); one that is of another request than `head` is not about this one and is
 * not used.
 */
export function statusOf(
  head: RequestHead | null,
  detail: GenerationRequest | null,
  adapters: AdapterListing | null,
): Status {
  if (head === null) return { kind: "idle" };
  const about = detail !== null && detail.id === head.id ? detail : null;
  switch (head.state) {
    case "queued":
      return { kind: "queued", connected: isConnected(adapters) };
    case "running":
      return { kind: "running", attempt: head.attempt, holder: about?.lease?.holder ?? null };
    case "interrupted":
      return { kind: "interrupted" };
    case "failed":
      return { kind: "failed", reason: about?.note ?? null };
    case "cancelled":
      return { kind: "cancelled" };
    case "superseded":
      return { kind: "superseded" };
    case "completed":
      return { kind: "completed" };
  }
}

/** What the person may do about the request, as far as the screen knows. */
export interface Controls {
  /** A request can be made now. */
  readonly canGenerate: boolean;
  /** Making one replaces the open one: the request is let go of, and asking again is the person's choice. */
  readonly replaces: boolean;
  /** The open request can be called off. */
  readonly canCancel: boolean;
}

/**
 * The buttons a state allows. `busy` is a request being made or called off right now: a
 * second press while the first is on its way would be a second request.
 */
export function controlsOf(status: Status, busy: boolean): Controls {
  const open = status.kind === "queued" || status.kind === "running" || status.kind === "interrupted";
  return {
    // A request that is queued or running is being served; one that was let go of is not,
    // and asking again replaces it. Everything else has ended.
    canGenerate: !busy && (!open || status.kind === "interrupted"),
    replaces: status.kind === "interrupted",
    canCancel: !busy && open,
  };
}

/**
 * Whether saving the text or the answers would end the latest request. The core does that, in
 * the same step as the save, so the screen says so before: what the request writes would not be
 * published, and that is the person's to decide.
 */
export function savingEndsARequest(head: RequestHead | null): boolean {
  return head !== null && isOpenRequest(head.state);
}

/** A key that makes one press of the button one request, however many times it is sent. */
export function pressKey(random: () => number = Math.random, now: () => number = Date.now): string {
  return `gui-${now().toString(36)}-${Math.floor(random() * 0x7fffffff).toString(36)}`;
}
