// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import { SUPPORTED_COMMAND_SET_VERSION } from "../src/contract";
import type {
  ClaudeAccount,
  ClaudeClient,
  ClaudeStatus,
  CodexStatus,
  Connection,
  ConnectionListing,
  Described,
} from "../src/contract";
import {
  CLAUDE_CONNECTION_ID,
  CODEX_CONNECTION_ID,
  accountOf,
  chosenSource,
  claudeConnection,
  codexConnection,
  codexReadinessOf,
  connectionForRequest,
  defaultKind,
  modelChoices,
  outlookOf,
  readinessOf,
  type Asked,
} from "../src/ai_settings_model";

const SIGN_IN = [
  { billing: "subscription", command: "claude auth login" },
  { billing: "usage", command: "claude auth login --console" },
] as const;

const REVISION = "a".repeat(64);
const OTHER = "b".repeat(64);

const described = (over: Partial<Described> = {}): Described => ({
  command_set_version: SUPPORTED_COMMAND_SET_VERSION,
  commands: [],
  root: "/works",
  entrance: "desktop",
  settings: true,
  writes_settings: true,
  starts_programs: true,
  ...over,
});

const status = (
  account: ClaudeAccount,
  client: ClaudeClient = { state: "installed", version: "2.1.291", path: "/home/me/.local/bin/claude" },
): ClaudeStatus => ({
  client,
  account,
  sign_in: SIGN_IN,
});

const answered = (account: ClaudeAccount, client?: ClaudeClient): Asked => ({
  phase: "answered",
  status: status(account, client),
});

const signedIn = (over: Partial<Extract<ClaudeAccount, { state: "signed-in" }>> = {}): ClaudeAccount => ({
  state: "signed-in",
  route: "claude-official-login",
  billing: "subscription",
  environment: null,
  usable: true,
  decision: { decision: "use", status: "conditional" },
  ...over,
});

const connection = (over: Partial<Connection> = {}): Connection => ({
  id: CLAUDE_CONNECTION_ID,
  adapter: "claude-code",
  display_name: null,
  executable: null,
  model: "opus",
  auth: "official-login",
  server_url: null,
  limits: { turns: null, seconds: null },
  ...over,
});

const listing = (over: Partial<ConnectionListing> = {}): ConnectionListing => ({
  connections: [{ connection: connection(), revision: REVISION }],
  unreadable: [],
  default: CLAUDE_CONNECTION_ID,
  ...over,
});

describe("what the AI settings can say, by where the screen is", () => {
  it("is nothing to ask in a window that may not start a program", () => {
    for (const entrance of ["browser", "tool"] as const) {
      const says = readinessOf(described({ entrance, starts_programs: false, writes_settings: false }), { phase: "idle" });
      expect(says.kind).toBe("not-here");
    }
  });

  it("is not asked until it is asked, and is waiting while it is", () => {
    expect(readinessOf(described(), { phase: "idle" }).kind).toBe("unasked");
    expect(readinessOf(described(), { phase: "asking" }).kind).toBe("asking");
  });

  it("is refused to the screen as a thing it cannot ask here, and not as a failure", () => {
    expect(readinessOf(described(), { phase: "refused", kind: "not-allowed-here" }).kind).toBe("not-here");
    const failed = readinessOf(described(), { phase: "refused", kind: "internal" });
    expect(failed).toMatchObject({ kind: "unknown" });
  });

  it("is a program to install when there is none, and a program to check when it does not answer", () => {
    expect(readinessOf(described(), answered({ state: "unknown", reason: "not found" }, { state: "missing" })).kind).toBe(
      "no-client",
    );
    expect(readinessOf(described(), answered({ state: "unknown", reason: "mute" }, { state: "unverified" })).kind).toBe(
      "client-unverified",
    );
  });

  it("is a client that could not be asked, which is not nobody signed in", () => {
    const says = readinessOf(described(), answered({ state: "unknown", reason: "it did not answer within 20 seconds" }));
    expect(says).toEqual({ kind: "unknown", reason: "it did not answer within 20 seconds" });
  });

  it("is the two commands that sign in when nobody is, and no button that signs in", () => {
    const says = readinessOf(described(), answered({ state: "signed-out" }));
    expect(says).toEqual({ kind: "signed-out", commands: SIGN_IN });
  });

  it("is a login the build does not use, said as the login it is", () => {
    const says = readinessOf(
      described(),
      answered(
        signedIn({
          route: "unlisted",
          billing: null,
          usable: false,
          decision: { decision: "refuse", status: "unconfirmed", reason: "unconfirmed" },
        }),
      ),
    );
    expect(says).toMatchObject({ kind: "not-used", reason: "unconfirmed" });
  });

  it("is a login that is ready, with how it is billed and what decided it", () => {
    expect(readinessOf(described(), answered(signedIn()))).toEqual({
      kind: "ready",
      version: "2.1.291",
      path: "/home/me/.local/bin/claude",
      billing: "subscription",
      environment: null,
    });
    // A key in the environment decides it, by name, and is billed by use.
    expect(
      readinessOf(
        described(),
        answered(signedIn({ route: "claude-api-key", billing: "usage", environment: "ANTHROPIC_API_KEY" })),
      ),
    ).toEqual({
      kind: "ready",
      version: "2.1.291",
      path: "/home/me/.local/bin/claude",
      billing: "usage",
      environment: "ANTHROPIC_API_KEY",
    });
    // A key the client holds itself is billed by use too, and nothing in the environment decided it.
    expect(
      readinessOf(described(), answered(signedIn({ route: "claude-api-key", billing: "usage", environment: null }))),
    ).toMatchObject({ billing: "usage", environment: null });
  });

  it("does not say ready for a way the build does not use, whatever else it says", () => {
    const says = readinessOf(
      described(),
      answered(signedIn({ usable: false, decision: { decision: "refuse", status: "conditional", reason: "switched-off" } })),
    );
    expect(says.kind).toBe("not-used");
    expect(says).toMatchObject({ reason: "switched-off" });
  });
});

describe("the one connection to Claude Code that the settings keep", () => {
  it("is the saved one when there is one, and nothing when there is none", () => {
    expect(claudeConnection(null)).toBeNull();
    expect(claudeConnection(listing({ connections: [] }))).toBeNull();
    expect(claudeConnection(listing())?.connection.model).toBe("opus");
    // Another connection is not this one.
    expect(
      claudeConnection(
        listing({ connections: [{ connection: connection({ id: "pc2", adapter: "local" }), revision: OTHER }] }),
      ),
    ).toBeNull();
  });

  it("is chosen among a few names the client documents, the client's own default, and a name the person types", () => {
    const choices = modelChoices(null);
    expect(choices.map((c) => c.model)).toEqual([null, "opus", "sonnet", "haiku"]);
    // A model the person typed is a choice too, so that saving does not lose it.
    expect(modelChoices("claude-opus-4-1").map((c) => c.model)).toEqual([null, "opus", "sonnet", "haiku", "claude-opus-4-1"]);
    expect(modelChoices("sonnet").map((c) => c.model)).toEqual([null, "opus", "sonnet", "haiku"]);
  });
});

describe("the connection a request is made for", () => {
  it("is the default one, at the revision the screen read", () => {
    expect(connectionForRequest(listing())).toEqual({ id: CLAUDE_CONNECTION_ID, revision: REVISION });
  });

  it("is none when nothing is the default, or the default is not there to read", () => {
    expect(connectionForRequest(null)).toBeNull();
    expect(connectionForRequest(listing({ default: null }))).toBeNull();
    expect(connectionForRequest(listing({ default: "gone" }))).toBeNull();
  });

  it("is the default whichever kind of connection it is", () => {
    const local = connection({ id: "pc2", adapter: "local", auth: "none", display_name: "pc2", server_url: "http://127.0.0.1:1/v1" });
    const both = listing({
      connections: [
        { connection: connection(), revision: REVISION },
        { connection: local, revision: OTHER },
      ],
      default: "pc2",
    });
    expect(connectionForRequest(both)).toEqual({ id: "pc2", revision: OTHER });
  });
});

// ---- Codex ---------------------------------------------------------------------------------

const CODEX_SIGN_IN = [
  { source: "official-login", billing: "subscription", command: "codex login -c cli_auth_credentials_store=file", home: null },
  { source: "official-login", billing: "usage", command: "codex login --with-api-key -c cli_auth_credentials_store=file", home: null },
  { source: "app-store", billing: "subscription", command: "codex login -c cli_auth_credentials_store=file", home: "/s/codex-home" },
  { source: "app-store", billing: "usage", command: "codex login --with-api-key -c cli_auth_credentials_store=file", home: "/s/codex-home" },
] as const;

const codexSignedIn = (over: Partial<Extract<ClaudeAccount, { state: "signed-in" }>> = {}): ClaudeAccount =>
  signedIn({ route: "codex-cli-chat-gpt-login", ...over });

const codexAccounts = (
  official: ClaudeAccount,
  store: ClaudeAccount = { state: "signed-out" },
  key: ClaudeAccount = { state: "signed-out" },
): CodexStatus["accounts"] => [
  { ...official, source: "official-login" },
  { ...store, source: "app-store" },
  { ...key, source: "env-api-key" },
];

const codexStatus = (over: Partial<CodexStatus> = {}): CodexStatus => ({
  client: { state: "installed", version: "0.159.0", path: "/home/me/.local/bin/codex" },
  support: { state: "verified" },
  accounts: codexAccounts(codexSignedIn()),
  sign_in: CODEX_SIGN_IN,
  key_variable: "CODEX_API_KEY",
  ...over,
});

const codexConnectionOf = (over: Partial<Connection> = {}): Connection =>
  connection({ id: CODEX_CONNECTION_ID, adapter: "codex", model: null, auth: "official-login", ...over });

describe("which client the settings show first", () => {
  it("is the one the default connection is for, and Claude Code when nothing says", () => {
    const codexDefault = listing({
      connections: [
        { connection: connection(), revision: REVISION },
        { connection: codexConnectionOf(), revision: OTHER },
      ],
      default: CODEX_CONNECTION_ID,
    });
    expect(defaultKind(codexDefault)).toBe("codex");
    expect(defaultKind(listing())).toBe("claude-code");
    expect(defaultKind(listing({ default: null }))).toBe("claude-code");
    expect(defaultKind(null)).toBe("claude-code");
  });

  it("is Claude Code for a default that is neither client, which these settings do not edit", () => {
    const local = connection({ id: "pc2", adapter: "local", auth: "none", display_name: "pc2", server_url: "http://127.0.0.1:1/v1" });
    expect(defaultKind(listing({ connections: [{ connection: local, revision: OTHER }], default: "pc2" }))).toBe("claude-code");
  });

  it("finds the connection to Codex that is kept, apart from the one to Claude Code", () => {
    const both = listing({
      connections: [
        { connection: connection(), revision: REVISION },
        { connection: codexConnectionOf({ model: "gpt-x" }), revision: OTHER },
      ],
    });
    expect(codexConnection(both)?.connection.model).toBe("gpt-x");
    expect(claudeConnection(both)?.connection.model).toBe("opus");
    expect(codexConnection(listing())).toBeNull();
    expect(codexConnection(null)).toBeNull();
  });
});

describe("what the settings can say of Codex, by where the screen is", () => {
  const asked = (status: CodexStatus): Asked<CodexStatus> => ({ phase: "answered", status });

  it("is nothing to ask in a window that may not start a program", () => {
    expect(codexReadinessOf(described({ starts_programs: false }), { phase: "idle" }).kind).toBe("not-here");
    expect(codexReadinessOf(described(), { phase: "refused", kind: "not-allowed-here" }).kind).toBe("not-here");
  });

  it("is not asked until it is asked, and waiting while it is", () => {
    expect(codexReadinessOf(described(), { phase: "idle" }).kind).toBe("unasked");
    expect(codexReadinessOf(described(), { phase: "asking" }).kind).toBe("asking");
  });

  it("is a failure to ask, in the core's words, when the core refused for another reason", () => {
    expect(codexReadinessOf(described(), { phase: "refused", kind: "no-settings", message: "needs a settings folder" })).toEqual({
      kind: "unknown",
      reason: "needs a settings folder",
    });
  });

  it("is the whole answer once it came, for the screen to show a source of", () => {
    const status = codexStatus();
    expect(codexReadinessOf(described(), asked(status))).toEqual({ kind: "answered", status });
  });
});

describe("the source of the credential the settings show", () => {
  const kept = (auth: Connection["auth"]) => ({ connection: codexConnectionOf({ auth }), revision: REVISION });

  it("is the one the person chose in the list, over what is kept", () => {
    expect(chosenSource(codexStatus(), kept("app-store"), "env-api-key")).toBe("env-api-key");
  });

  it("is the one the connection that is kept names, when nothing was chosen", () => {
    expect(chosenSource(codexStatus(), kept("app-store"), undefined)).toBe("app-store");
    expect(chosenSource(codexStatus(), kept("env-api-key"), undefined)).toBe("env-api-key");
  });

  it("is the first source somebody is usably signed in by, when no connection is kept", () => {
    const status = codexStatus({ accounts: codexAccounts({ state: "signed-out" }, codexSignedIn()) });
    expect(chosenSource(status, null, undefined)).toBe("app-store");
  });

  it("is the official client's login when nothing is signed in anywhere and nothing is kept", () => {
    const status = codexStatus({ accounts: codexAccounts({ state: "signed-out" }) });
    expect(chosenSource(status, null, undefined)).toBe("official-login");
    expect(chosenSource(null, null, undefined)).toBe("official-login");
  });

  it("is not a source a connection to Codex cannot take, even if the stored one is hand-edited to it", () => {
    expect(chosenSource(codexStatus(), kept("none"), undefined)).toBe("official-login");
    expect(chosenSource(codexStatus(), kept("server-key"), undefined)).toBe("official-login");
  });
});

describe("whether a request made for Codex would run, and if not why it waits", () => {
  it("runs when the program is there, this build verified it, and somebody is signed in by a way it uses", () => {
    expect(outlookOf(codexStatus(), "official-login")).toEqual({ runs: true });
  });

  it("waits, first, for a program that is not there or is not Codex", () => {
    expect(outlookOf(codexStatus({ client: { state: "missing" } }), "official-login")).toEqual({
      runs: false,
      because: "no-client",
    });
    expect(outlookOf(codexStatus({ client: { state: "unverified" } }), "official-login")).toEqual({
      runs: false,
      because: "client-unverified",
    });
  });

  it("waits for a version this build did not verify, whoever is signed in", () => {
    const unverified = codexStatus({ support: { state: "unverified", reason: "has not been verified" } });
    expect(outlookOf(unverified, "official-login")).toEqual({ runs: false, because: "unsupported" });
    const unknown = codexStatus({ support: { state: "unknown", reason: "it did not list its features" } });
    expect(outlookOf(unknown, "official-login")).toEqual({ runs: false, because: "support-unknown" });
  });

  it("waits for a login, said apart from one that could not be asked and from one the build does not use", () => {
    const out = codexStatus({ accounts: codexAccounts({ state: "signed-out" }) });
    expect(outlookOf(out, "official-login")).toEqual({ runs: false, because: "signed-out" });
    const unasked = codexStatus({ accounts: codexAccounts({ state: "unknown", reason: "it did not answer" }) });
    expect(outlookOf(unasked, "official-login")).toEqual({ runs: false, because: "account-unknown" });
    const unused = codexStatus({
      accounts: codexAccounts(
        codexSignedIn({ billing: null, usable: false, decision: { decision: "refuse", status: "unconfirmed", reason: "unconfirmed" } }),
      ),
    });
    expect(outlookOf(unused, "official-login")).toEqual({ runs: false, because: "not-used" });
  });

  it("is of the source that is shown, and not of another that happens to be signed in", () => {
    const status = codexStatus({ accounts: codexAccounts(codexSignedIn(), { state: "signed-out" }) });
    expect(outlookOf(status, "official-login")).toEqual({ runs: true });
    expect(outlookOf(status, "app-store")).toEqual({ runs: false, because: "signed-out" });
  });

  it("is not knowing, and not running, for a source the core did not answer", () => {
    const status = codexStatus({ accounts: [] });
    expect(outlookOf(status, "official-login")).toEqual({ runs: false, because: "account-unknown" });
    expect(accountOf(status, "official-login")).toBeNull();
  });
});
