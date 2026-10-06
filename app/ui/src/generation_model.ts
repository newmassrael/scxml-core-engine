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
  type HostListing,
  type RequestHead,
} from "./contract";

/** Where the work's latest request stands, in the terms the person acts on. */
export type Status =
  /** Never asked, or the last request is not something to say anything about. */
  | { readonly kind: "idle" }
  /**
   * Asked, and nobody has taken it. `connected` says whether an AI is there to. `waiting` is why
   * the executor of the connection it was made for left it, in that executor's words; `unchosen`
   * is a request the application made for no connection, which the application does not run.
   */
  | {
      readonly kind: "queued";
      readonly connected: boolean;
      readonly waiting: string | null;
      readonly unchosen: boolean;
    }
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

/**
 * Why the shells that are there host no executor, in the words they gave, each with the shell's
 * name. A shell that has stopped saying so is not counted (it may be gone, and what it last said
 * would then be about nothing), and neither is one that hosts.
 */
export function whyNoAi(hosts: HostListing | null): string[] {
  if (hosts === null) return [];
  return hosts.hosts
    .filter((h) => h.live && !h.hosting && h.reason !== null)
    .map((h) => `${h.name}: ${h.reason}`);
}

/**
 * What the screen reads of the shells, as one string, so that two readings are compared by what
 * they would make it say: why no AI is hosted, and which requests wait and why. A shell that
 * only said it again, a moment later, is not a change.
 */
export function hostsKey(hosts: HostListing | null): string {
  if (hosts === null) return "";
  const waiting = hosts.hosts
    .filter((h) => h.live)
    .flatMap((h) => h.waiting.map((w) => `${w.request}=${w.reason}`));
  return [...whyNoAi(hosts), ...waiting].join("\n");
}

/** The names of the adapters that are there now. */
export function connectedNames(adapters: AdapterListing | null): string[] {
  return adapters === null ? [] : adapters.adapters.filter((a) => a.live).map((a) => a.name);
}

/**
 * Why a shell's executor left the request `request` queued, in its words; `null` when none says.
 * A shell that stopped saying so is not counted, as for `whyNoAi`.
 */
export function whyWaiting(hosts: HostListing | null, request: string): string | null {
  if (hosts === null) return null;
  for (const host of hosts.hosts) {
    if (!host.live) continue;
    const found = host.waiting.find((w) => w.request === request);
    if (found !== undefined) return found.reason;
  }
  return null;
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
  hosts: HostListing | null = null,
): Status {
  if (head === null) return { kind: "idle" };
  const about = detail !== null && detail.id === head.id ? detail : null;
  switch (head.state) {
    case "queued":
      return {
        kind: "queued",
        connected: isConnected(adapters),
        waiting: whyWaiting(hosts, head.id),
        // Made in the application, for no connection: the application does not choose one for it.
        // What was not read of the request is not guessed at.
        unchosen: about !== null && about.pin === null && about.origin === "gui",
      };
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
  // A request nobody will take as it is: one let go of, one that waits for a connection that
  // cannot be run, one the application does not run because nobody chose a connection for it.
  // Asking again replaces it; the person chooses whether to.
  const left =
    status.kind === "interrupted" ||
    (status.kind === "queued" && (status.waiting !== null || status.unchosen));
  return {
    // A request that is queued or running is being served; one that is left is not. Everything
    // else has ended.
    canGenerate: !busy && (!open || left),
    replaces: left,
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
