// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the AI settings say, apart from how they are drawn.
//
// The core answers three things: where the screen is (which entrance, so what it may ask and
// change), what the person keeps (the connections), and what Claude Code is and who is signed in
// to it. This turns them into one state a sentence can be chosen from. It decides nothing about
// what is allowed: whether a way of signing in is used is the build's table, and the core says
// it (`usable`); this only does not offer what it knows would be refused.
//
// Three things are kept apart because the screen acts on each differently: a program that is not
// there (install it), a program that could not be asked (ask again: not "nobody is signed in",
// or a person would be sent to sign in for nothing), and somebody signed in by a way the build
// does not use (said as the login it is, and not hidden).

import type {
  Billing,
  ClaudeStatus,
  ConnectionListing,
  Described,
  SignInCommand,
  StoredConnection,
} from "./contract";
import type { ConnectionRef } from "./api";

/** The one connection to Claude Code that the settings keep. */
export const CLAUDE_CONNECTION_ID = "claude";

/**
 * Names Claude Code documents for `--model`. The client offers no list to read, so these are
 * what is offered, and a name the person types is kept as a choice too. An alias is not a model:
 * what it names is the client's to decide and can change, and an account may not be able to use it.
 */
const CLAUDE_MODEL_ALIASES = ["opus", "sonnet", "haiku"] as const;

/** Where the asking of Claude Code stands. */
export type Asked =
  | { readonly phase: "idle" }
  | { readonly phase: "asking" }
  | { readonly phase: "answered"; readonly status: ClaudeStatus }
  /** The core refused to ask, with its word for why, and its own words when it gave any. */
  | { readonly phase: "refused"; readonly kind: string; readonly message?: string };

/** What the AI settings say of Claude Code. */
export type Readiness =
  /** This window cannot ask and cannot change anything: the settings are the desktop window's. */
  | { readonly kind: "not-here" }
  | { readonly kind: "unasked" }
  | { readonly kind: "asking" }
  | { readonly kind: "no-client" }
  | { readonly kind: "client-unverified" }
  /** It could not be asked. Says why, as the client or the core said it. */
  | { readonly kind: "unknown"; readonly reason: string }
  | { readonly kind: "signed-out"; readonly commands: readonly SignInCommand[] }
  /** Signed in by a way the build does not use. `reason` is the core's. */
  | { readonly kind: "not-used"; readonly reason: "forbidden" | "unconfirmed" | "switched-off" }
  | {
      readonly kind: "ready";
      readonly version: string;
      readonly billing: Billing;
      /** The variable that decided the login, when the client said one did. */
      readonly environment: string | null;
    };

/** What to say of Claude Code, from what the window may do and what was asked of it. */
export function readinessOf(described: Described, asked: Asked): Readiness {
  if (asked.phase === "refused" && asked.kind === "not-allowed-here") return { kind: "not-here" };
  if (!described.starts_programs) return { kind: "not-here" };
  switch (asked.phase) {
    case "idle":
      return { kind: "unasked" };
    case "asking":
      return { kind: "asking" };
    case "refused":
      return { kind: "unknown", reason: asked.message ?? `the core refused it (${asked.kind})` };
    case "answered":
      return readinessOfStatus(asked.status);
  }
}

function readinessOfStatus(status: ClaudeStatus): Readiness {
  if (status.client.state === "missing") return { kind: "no-client" };
  if (status.client.state === "unverified") return { kind: "client-unverified" };
  const account = status.account;
  switch (account.state) {
    case "unknown":
      return { kind: "unknown", reason: account.reason };
    case "signed-out":
      return { kind: "signed-out", commands: status.sign_in };
    case "signed-in": {
      if (!account.usable) {
        // A refusal says why; a way that is usable says nothing, so it is not here.
        const reason = account.decision.decision === "refuse" ? account.decision.reason : "unconfirmed";
        return { kind: "not-used", reason };
      }
      if (account.billing === null) {
        return { kind: "unknown", reason: "this build uses a way of signing in that the screen cannot describe" };
      }
      return {
        kind: "ready",
        version: status.client.version,
        billing: account.billing,
        environment: account.environment,
      };
    }
  }
}

/** The connection to Claude Code that is kept, or `null` when none is (or the settings were not read). */
export function claudeConnection(listing: ConnectionListing | null): StoredConnection | null {
  return listing?.connections.find((c) => c.connection.id === CLAUDE_CONNECTION_ID) ?? null;
}

/** A model the person can choose for the connection. `null` is the client's own default. */
export interface ModelChoice {
  readonly model: string | null;
  /** Whether the name is one the client documents, and so is shown as an alias and not as an id. */
  readonly documented: boolean;
}

/**
 * The models offered: the client's default, the names it documents, and the one the connection
 * has now when it is none of those (so that saving does not lose a model the person typed).
 */
export function modelChoices(current: string | null): readonly ModelChoice[] {
  const choices: ModelChoice[] = [
    { model: null, documented: true },
    ...CLAUDE_MODEL_ALIASES.map((model) => ({ model: model as string | null, documented: true })),
  ];
  if (current !== null && !CLAUDE_MODEL_ALIASES.some((alias) => alias === current)) {
    choices.push({ model: current, documented: false });
  }
  return choices;
}

/**
 * The connection a request is made for: the default one, at the revision the screen read. `null`
 * when nothing is the default or the default is not among what was read (a request is made for
 * a connection the screen has seen, not one it assumes).
 */
export function connectionForRequest(listing: ConnectionListing | null): ConnectionRef | null {
  if (listing === null || listing.default === null) return null;
  const found = listing.connections.find((c) => c.connection.id === listing.default);
  return found === undefined ? null : { id: found.connection.id, revision: found.revision };
}
