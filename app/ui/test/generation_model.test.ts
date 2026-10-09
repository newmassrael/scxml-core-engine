// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import type {
  AdapterListing,
  AdapterStatus,
  GenerationRequest,
  HostListing,
  HostStatus,
  RequestHead,
  RequestState,
} from "../src/contract";
import {
  connectedNames,
  controlsOf,
  hostsKey,
  isConnected,
  pressKey,
  savingEndsARequest,
  statusOf,
  whyNoAi,
} from "../src/generation_model";

const head = (state: RequestState, attempt = 1, id = "req-1"): RequestHead => ({ id, state, attempt });

const adapter = (over: Partial<AdapterStatus> = {}): AdapterStatus => ({
  name: "desktop",
  kind: "claude-code",
  capabilities: ["generate", "cancel"],
  version: "2.1",
  seen_at: "2026-10-05T09:00:00Z",
  live: true,
  ...over,
});

const listing = (...adapters: AdapterStatus[]): AdapterListing => ({ adapters, unreadable: [] });

const detail = (over: Partial<GenerationRequest> = {}): GenerationRequest => ({
  id: "req-1",
  work: "door",
  seq: 1,
  key: "k",
  origin: "gui",
  state: "running",
  stored_state: "running",
  attempt: 1,
  inputs: { source: "a".repeat(64), answers: null },
  created_at: "2026-10-05T09:00:00Z",
  lease: { holder: "desktop", attempt: 1, granted_at: "2026-10-05T09:00:01Z", expires_at: "2026-10-05T09:01:01Z" },
  candidate: null,
  outcome: null,
  pin: null,
  fresh_ids: false,
  ended_at: null,
  note: null,
  ...over,
});

describe("whether an AI is there", () => {
  it("is so when an adapter that can generate is live", () => {
    expect(isConnected(listing(adapter()))).toBe(true);
    expect(connectedNames(listing(adapter(), adapter({ name: "web-shell" })))).toEqual(["desktop", "web-shell"]);
  });

  it("is not so for no adapter, one that stopped reporting, or one that cannot generate", () => {
    expect(isConnected(null)).toBe(false);
    expect(isConnected(listing())).toBe(false);
    expect(isConnected(listing(adapter({ live: false })))).toBe(false);
    expect(isConnected(listing(adapter({ capabilities: ["cancel"] })))).toBe(false);
    expect(connectedNames(listing(adapter({ live: false })))).toEqual([]);
  });
});

describe("why no AI is hosted", () => {
  const host = (over: Partial<HostStatus> = {}): HostStatus => ({
    name: "desktop",
    hosting: false,
    reason: "no Claude Code to write models with: install it, or set SCE_CLAUDE",
    client_version: null,
    waiting: [],
    seen_at: "2026-10-05T09:00:00Z",
    live: true,
    ...over,
  });
  const hosts = (...list: HostStatus[]): HostListing => ({ hosts: list, unreadable: [] });

  it("is what each shell that is there said, with its name", () => {
    expect(whyNoAi(hosts(host(), host({ name: "web-shell", reason: "the executor is off" })))).toEqual([
      "desktop: no Claude Code to write models with: install it, or set SCE_CLAUDE",
      "web-shell: the executor is off",
    ]);
  });

  it("is nothing for a shell that hosts, one that stopped saying so, one that gave no reason, or nobody", () => {
    expect(whyNoAi(hosts(host({ hosting: true, reason: null })))).toEqual([]);
    expect(whyNoAi(hosts(host({ live: false })))).toEqual([]);
    expect(whyNoAi(hosts(host({ reason: null })))).toEqual([]);
    expect(whyNoAi(hosts())).toEqual([]);
    expect(whyNoAi(null)).toEqual([]);
  });
});

describe("what the screen reads of the shells, as one thing to compare", () => {
  const shell = (waiting: HostStatus["waiting"], over: Partial<HostStatus> = {}): HostListing => ({
    hosts: [
      {
        name: "desktop",
        hosting: true,
        reason: null,
        client_version: "2.1.291",
        waiting,
        seen_at: "2026-10-05T09:00:00Z",
        live: true,
        ...over,
      },
    ],
    unreadable: [],
  });
  const left = (reason: string) => [{ work: "door", request: "req-1", connection: "claude", reason }];

  it("moves when a shell says why a request waits, or says something else", () => {
    expect(hostsKey(shell(left("nobody is signed in")))).not.toBe(hostsKey(shell([])));
    expect(hostsKey(shell(left("nobody is signed in")))).not.toBe(hostsKey(shell(left("no adapter"))));
  });

  it("does not move when a shell says the same again a moment later", () => {
    const again = shell(left("nobody is signed in"), { seen_at: "2026-10-05T09:00:30Z" });
    expect(hostsKey(again)).toBe(hostsKey(shell(left("nobody is signed in"))));
  });

  it("does not count what a shell that stopped saying it last said", () => {
    expect(hostsKey(shell(left("nobody is signed in"), { live: false }))).toBe(hostsKey(shell([])));
    expect(hostsKey(null)).toBe("");
  });
});

describe("where the latest request stands", () => {
  it("is idle for a work nobody asked a model for", () => {
    expect(statusOf(null, null, null)).toEqual({ kind: "idle" });
  });

  it("says whether an AI is there to take a request that waits", () => {
    const queued = (connected: boolean) => ({ kind: "queued", connected, waiting: null, unchosen: false });
    expect(statusOf(head("queued", 0), null, listing(adapter()))).toEqual(queued(true));
    expect(statusOf(head("queued", 0), null, listing())).toEqual(queued(false));
    expect(statusOf(head("queued", 0), null, null)).toEqual(queued(false));
  });

  describe("a request that waits", () => {
    const waitingHost = (over: Partial<HostStatus> = {}): HostStatus => ({
      name: "desktop",
      hosting: true,
      reason: null,
      client_version: "2.1.291",
      waiting: [
        { work: "door", request: "req-1", connection: "claude", reason: "nobody is signed in to Claude Code" },
        { work: "door", request: "req-9", connection: "claude", reason: "another request's reason" },
      ],
      seen_at: "2026-10-05T09:00:00Z",
      live: true,
      ...over,
    });
    const seen = (...list: HostStatus[]): HostListing => ({ hosts: list, unreadable: [] });

    it("says why the executor could not run it, in the words the executor gave", () => {
      expect(statusOf(head("queued", 0), detail({ state: "queued" }), listing(adapter()), seen(waitingHost()))).toMatchObject({
        kind: "queued",
        waiting: "nobody is signed in to Claude Code",
      });
    });

    it("does not take the reason of another request, of a shell that stopped saying it, or of nobody", () => {
      const asked = (hosts: HostListing | null) =>
        statusOf(head("queued", 0, "req-2"), detail({ id: "req-2", state: "queued" }), listing(adapter()), hosts);
      expect(asked(seen(waitingHost()))).toMatchObject({ waiting: null });
      expect(
        statusOf(head("queued", 0), detail({ state: "queued" }), listing(adapter()), seen(waitingHost({ live: false }))),
      ).toMatchObject({ waiting: null });
      expect(asked(null)).toMatchObject({ waiting: null });
    });

    it("is one nobody chose a connection for, when the application made it", () => {
      const made = (over: Partial<GenerationRequest>) =>
        statusOf(head("queued", 0), detail({ state: "queued", lease: null, ...over }), listing(adapter()));
      expect(made({ origin: "gui", pin: null })).toMatchObject({ unchosen: true });
      // An authoring client made it for itself: it waits for that client, and is not unchosen.
      expect(made({ origin: "mcp", pin: null })).toMatchObject({ unchosen: false });
      // One made for a connection is chosen, whoever made it.
      expect(
        made({
          origin: "gui",
          pin: { connection: "claude", revision: "a".repeat(64), adapter: "claude-code", model: null, limits: { turns: null, seconds: null } },
        }),
      ).toMatchObject({ unchosen: false });
      // What was not read of the request is not guessed at.
      expect(statusOf(head("queued", 0), null, listing(adapter()))).toMatchObject({ unchosen: false });
    });
  });

  it("says who holds a running request when the request was read, and not when it was not", () => {
    expect(statusOf(head("running"), detail(), null)).toEqual({ kind: "running", attempt: 1, holder: "desktop" });
    expect(statusOf(head("running", 2), null, null)).toEqual({ kind: "running", attempt: 2, holder: null });
  });

  it("gives the reason a request failed, from what was read of it", () => {
    expect(statusOf(head("failed"), detail({ state: "failed", note: "no cards named" }), null)).toEqual({
      kind: "failed",
      reason: "no cards named",
    });
    expect(statusOf(head("failed"), null, null)).toEqual({ kind: "failed", reason: null });
  });

  it("does not use what was read of another request", () => {
    const other = detail({ id: "req-0", note: "an old failure", state: "failed" });
    expect(statusOf(head("failed", 1, "req-1"), other, null)).toEqual({ kind: "failed", reason: null });
  });

  it("names every other state as it is", () => {
    for (const state of ["interrupted", "cancelled", "superseded", "completed"] as const) {
      expect(statusOf(head(state), null, null)).toEqual({ kind: state });
    }
  });
});

describe("what the person may do", () => {
  it("is to ask, when nothing is open and nothing is on its way", () => {
    for (const status of [
      statusOf(null, null, null),
      statusOf(head("completed"), null, null),
      statusOf(head("failed"), null, null),
      statusOf(head("cancelled"), null, null),
      statusOf(head("superseded"), null, null),
    ]) {
      expect(controlsOf(status, false)).toEqual({ canGenerate: true, replaces: false, canCancel: false });
    }
  });

  it("is to wait or call it off, while a request is queued or running", () => {
    for (const status of [statusOf(head("queued", 0), null, null), statusOf(head("running"), null, null)]) {
      expect(controlsOf(status, false)).toEqual({ canGenerate: false, replaces: false, canCancel: true });
    }
  });

  it("is to ask again over a request that waits for a connection, or to call it off", () => {
    const unchosen = statusOf(head("queued", 0), detail({ state: "queued", lease: null, origin: "gui" }), null);
    const pinned = {
      connection: "claude",
      revision: "a".repeat(64),
      adapter: "claude-code",
      model: null,
      limits: { turns: null, seconds: null },
    } as const;
    const stuck = statusOf(head("queued", 0), detail({ state: "queued", lease: null, pin: pinned }), null, {
      hosts: [
        {
          name: "desktop",
          hosting: true,
          reason: null,
          client_version: null,
          waiting: [{ work: "door", request: "req-1", connection: "claude", reason: "nobody is signed in" }],
          seen_at: "2026-10-05T09:00:00Z",
          live: true,
        },
      ],
      unreadable: [],
    });
    for (const status of [unchosen, stuck]) {
      expect(controlsOf(status, false)).toEqual({ canGenerate: true, replaces: true, canCancel: true });
    }
    expect(controlsOf(unchosen, true)).toEqual({ canGenerate: false, replaces: true, canCancel: false });
  });

  it("is to ask again over a request that was let go of, or to call it off", () => {
    expect(controlsOf(statusOf(head("interrupted"), null, null), false)).toEqual({
      canGenerate: true,
      replaces: true,
      canCancel: true,
    });
  });

  it("is nothing while a press is on its way: a second one would be a second request", () => {
    for (const status of [statusOf(null, null, null), statusOf(head("interrupted"), null, null)]) {
      expect(controlsOf(status, true)).toEqual({
        canGenerate: false,
        replaces: status.kind === "interrupted",
        canCancel: false,
      });
    }
  });
});

describe("what a save does to a request", () => {
  it("ends one that is open, by the core's rule, and the person is told first", () => {
    for (const state of ["queued", "running", "interrupted"] as const) {
      expect(savingEndsARequest(head(state)), state).toBe(true);
    }
    for (const state of ["completed", "failed", "cancelled", "superseded"] as const) {
      expect(savingEndsARequest(head(state)), state).toBe(false);
    }
    expect(savingEndsARequest(null)).toBe(false);
  });
});

describe("the key of a press", () => {
  it("is one name for one press and another for the next", () => {
    const a = pressKey(() => 0.25, () => 1000);
    const b = pressKey(() => 0.5, () => 1000);
    expect(a).not.toBe(b);
    expect(a).toMatch(/^gui-[0-9a-z]+-[0-9a-z]+$/);
    expect(a.length).toBeLessThanOrEqual(64);
  });
});
