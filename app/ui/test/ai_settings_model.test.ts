// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import { SUPPORTED_COMMAND_SET_VERSION } from "../src/contract";
import type { ClaudeAccount, ClaudeClient, ClaudeStatus, Connection, ConnectionListing, Described } from "../src/contract";
import {
  CLAUDE_CONNECTION_ID,
  claudeConnection,
  connectionForRequest,
  modelChoices,
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
