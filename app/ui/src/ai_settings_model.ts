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
  CodexAccount,
  CodexSource,
  CodexStatus,
  ConnectionListing,
  Described,
  SignInCommand,
  StoredConnection,
} from "./contract";
import type { ConnectionRef } from "./api";

/** The one connection to Claude Code that the settings keep. */
export const CLAUDE_CONNECTION_ID = "claude";

/** The one connection to Codex that the settings keep. */
export const CODEX_CONNECTION_ID = "codex";

/** The one connection to a model server of the person's that the settings keep. */
export const LOCAL_CONNECTION_ID = "server";

/** The ways to reach a model that these settings edit a connection to. */
export type ClientKind = "claude-code" | "codex" | "local";

/**
 * Names Claude Code documents for `--model`. The client offers no list to read, so these are
 * what is offered, and a name the person types is kept as a choice too. An alias is not a model:
 * what it names is the client's to decide and can change, and an account may not be able to use it.
 */
const CLAUDE_MODEL_ALIASES = ["opus", "sonnet", "haiku"] as const;

/** Where the asking of a client stands; `S` is what the core answered. */
export type Asked<S = ClaudeStatus> =
  | { readonly phase: "idle" }
  | { readonly phase: "asking" }
  | { readonly phase: "answered"; readonly status: S }
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
      /** The program that answered, as this computer names it. */
      readonly path: string;
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
        path: status.client.path,
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

/** The connection to Codex that is kept, or `null` when none is (or the settings were not read). */
export function codexConnection(listing: ConnectionListing | null): StoredConnection | null {
  return listing?.connections.find((c) => c.connection.id === CODEX_CONNECTION_ID) ?? null;
}

/**
 * The connection to a model server that is kept: the default one when it is to a server (whatever
 * it was named, because it may have been saved by something else), else the one these settings
 * name, else the first connection to a server there is. `null` when none is (or the settings were
 * not read).
 */
export function localConnection(listing: ConnectionListing | null): StoredConnection | null {
  if (listing === null) return null;
  const servers = listing.connections.filter((c) => c.connection.adapter === "local");
  return (
    servers.find((c) => c.connection.id === listing.default) ??
    servers.find((c) => c.connection.id === LOCAL_CONNECTION_ID) ??
    servers[0] ??
    null
  );
}

/**
 * The client the settings show when the person has not chosen one: the one the default connection
 * is for. Claude Code when there is no default.
 */
export function defaultKind(listing: ConnectionListing | null): ClientKind {
  const stored = listing?.connections.find((c) => c.connection.id === listing.default);
  switch (stored?.connection.adapter) {
    case "codex":
      return "codex";
    case "local":
      return "local";
    default:
      return "claude-code";
  }
}

// ---- Codex -----------------------------------------------------------------------------------

/** What the AI settings say of something they ask the core about, before the answer is looked into. */
export type Answering<S> =
  /** This window cannot ask: the settings are the desktop window's. */
  | { readonly kind: "not-here" }
  | { readonly kind: "unasked" }
  | { readonly kind: "asking" }
  /** It could not be asked. Says why, as the core said it. */
  | { readonly kind: "unknown"; readonly reason: string }
  /** The core answered: the screen shows what it said. */
  | { readonly kind: "answered"; readonly status: S };

/** What to say of something asked of the core, from what the window may do and what was asked. */
export function answeringOf<S>(described: Described, asked: Asked<S>): Answering<S> {
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
      return { kind: "answered", status: asked.status };
  }
}

/** What the AI settings say of Codex, before the core's answer is looked into. */
export type CodexReadiness = Answering<CodexStatus>;

/** What to say of Codex, from what the window may do and what was asked of it. */
export function codexReadinessOf(described: Described, asked: Asked<CodexStatus>): CodexReadiness {
  return answeringOf(described, asked);
}

/** The sources a connection to Codex takes its credential from, in the order they are offered. */
export const CODEX_SOURCES: readonly CodexSource[] = ["official-login", "app-store", "env-api-key"];

/** What the core said of who is signed in by `source`; `null` when it did not say. */
export function accountOf(status: CodexStatus, source: CodexSource): CodexAccount | null {
  return status.accounts.find((account) => account.source === source) ?? null;
}

/**
 * The source the settings show: the one the person chose in the list, else the one the kept
 * connection names, else the first one somebody is signed in by in a way the build uses, else the
 * official client's own login. A source the connection cannot take (a stored connection edited by
 * hand to one) is not shown as the choice.
 */
export function chosenSource(
  status: CodexStatus | null,
  kept: StoredConnection | null,
  draft: CodexSource | undefined,
): CodexSource {
  if (draft !== undefined) return draft;
  const named = CODEX_SOURCES.find((source) => source === kept?.connection.auth);
  if (named !== undefined) return named;
  if (status !== null) {
    const usable = CODEX_SOURCES.find((source) => {
      const account = accountOf(status, source);
      return account?.state === "signed-in" && account.usable;
    });
    if (usable !== undefined) return usable;
  }
  return "official-login";
}

/** Why a request made for Codex would wait. */
export type Because =
  | "no-client"
  | "client-unverified"
  | "unsupported"
  | "support-unknown"
  | "signed-out"
  | "not-used"
  | "account-unknown";

/** Whether a request made for Codex by `source` would run, and if not, the first thing it waits for. */
export type Outlook = { readonly runs: true } | { readonly runs: false; readonly because: Because };

/**
 * What the core's answer says would become of a request. The order is the order the things are
 * true in: a program that is not there is not asked who is signed in, and a version this build
 * did not verify is not run whoever is signed in, so the screen says that and not "sign in".
 */
export function outlookOf(status: CodexStatus, source: CodexSource): Outlook {
  if (status.client.state === "missing") return { runs: false, because: "no-client" };
  if (status.client.state === "unverified") return { runs: false, because: "client-unverified" };
  if (status.support.state === "unverified") return { runs: false, because: "unsupported" };
  if (status.support.state === "unknown") return { runs: false, because: "support-unknown" };
  const account = accountOf(status, source);
  if (account === null || account.state === "unknown") return { runs: false, because: "account-unknown" };
  if (account.state === "signed-out") return { runs: false, because: "signed-out" };
  return account.usable ? { runs: true } : { runs: false, because: "not-used" };
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
