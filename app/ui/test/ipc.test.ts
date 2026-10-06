// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import { apiOver } from "../src/api";
import { CommandFailure, failureFromInvoke, httpTransport, insideTauri, TRANSPORT } from "../src/ipc";
import { credentials, takeToken, tokenFromPaste } from "../src/token";

interface Seen {
  url: string;
  init: RequestInit;
}

/** A `fetch` that answers with `status` and `body`, remembering what it was asked. */
function fetching(status: number, body: unknown, seen: Seen[] = []): typeof fetch {
  return (async (url: string, init: RequestInit) => {
    seen.push({ url, init });
    return new Response(typeof body === "string" ? body : JSON.stringify(body), { status });
  }) as unknown as typeof fetch;
}

describe("the browser transport", () => {
  it("posts the command with the bearer token and returns the answer", async () => {
    const seen: Seen[] = [];
    const transport = httpTransport({ fetch: fetching(200, { works: [] }, seen), token: () => "tok" });
    expect(await transport.call("list_works", {})).toEqual({ works: [] });
    expect(seen[0]?.url).toBe("/api/call");
    expect(seen[0]?.init.method).toBe("POST");
    expect(seen[0]?.init.headers).toMatchObject({ Authorization: "Bearer tok" });
    expect(JSON.parse(String(seen[0]?.init.body))).toEqual({ name: "list_works", args: {} });
  });

  it("sends no Authorization header when it has no token", async () => {
    const seen: Seen[] = [];
    await httpTransport({ fetch: fetching(200, {}, seen), token: () => null }).call("describe");
    expect(seen[0]?.init.headers).not.toHaveProperty("Authorization");
  });

  it("turns a refusal into a CommandFailure that keeps its kind and detail", async () => {
    const refusal = { kind: "conflict", message: "stale", detail: { base: "a", current: "b" } };
    const transport = httpTransport({ fetch: fetching(409, refusal), token: () => "t" });
    const error = await transport.call("save_source", {}).catch((e: unknown) => e);
    expect(error).toBeInstanceOf(CommandFailure);
    expect(error).toMatchObject({ kind: "conflict", message: "stale", detail: { base: "a", current: "b" } });
  });

  it("calls an answer that is not a refusal a transport failure", async () => {
    const transport = httpTransport({ fetch: fetching(502, "<html>bad gateway</html>"), token: () => "t" });
    await expect(transport.call("describe")).rejects.toMatchObject({ kind: TRANSPORT });
    const odd = httpTransport({ fetch: fetching(500, { oops: true }), token: () => "t" });
    await expect(odd.call("describe")).rejects.toMatchObject({ kind: TRANSPORT });
  });

  it("calls a server that cannot be reached a transport failure", async () => {
    const down = (async () => {
      throw new TypeError("connection refused");
    }) as unknown as typeof fetch;
    await expect(httpTransport({ fetch: down, token: () => "t" }).call("describe")).rejects.toMatchObject({
      kind: TRANSPORT,
    });
  });

  it("prefixes the address when the server is somewhere else", async () => {
    const seen: Seen[] = [];
    await httpTransport({ fetch: fetching(200, {}, seen), token: () => "t", base: "http://100.64.0.2:5174" }).call(
      "describe",
    );
    expect(seen[0]?.url).toBe("http://100.64.0.2:5174/api/call");
  });
});

describe("the window transport", () => {
  it("recognises a window by Tauri's marker", () => {
    expect(insideTauri({ __TAURI_INTERNALS__: {} })).toBe(true);
    expect(insideTauri({})).toBe(false);
  });

  it("reads what a rejected invoke carries as the core's refusal", () => {
    expect(failureFromInvoke({ kind: "busy", message: "wait" })).toMatchObject({ kind: "busy" });
    expect(failureFromInvoke("a string")).toMatchObject({ kind: TRANSPORT });
  });
});

describe("the typed commands", () => {
  const A = "a".repeat(64);

  it("check the answer before a view sees it", async () => {
    const api = apiOver({ call: async () => ({ works: "not a list", unreadable: [] }) });
    await expect(api.listWorks()).rejects.toThrow(/listing\.works: expected a list/);
  });

  it("send a read of an old revision with its revision, and a current read without", async () => {
    const calls: Array<[string, unknown]> = [];
    const api = apiOver({
      call: async (name, args) => {
        calls.push([name, args]);
        return { source: { revision: A, text: "t" } };
      },
    });
    await api.readSource("w");
    await api.readSource("w", A);
    expect(calls).toEqual([
      ["read_source", { id: "w" }],
      ["read_source", { id: "w", revision: A }],
    ]);
  });

  describe("asking what SCE says of the revisions that were read", () => {
    const B = "b".repeat(64);
    const C = "c".repeat(64);
    const basis = { source: A, model: B, requirements: C, answers: null };
    // The core leaves `answers` out of a basis when the owner had answered nothing.
    const named = { source: A, model: B, requirements: C };
    const answered = (over: Record<string, unknown> = {}): unknown => ({
      basis: named,
      acceptance: null,
      report: { said: { generator: null, denominator: null, outcomes: [], page: null, page_refusal: null } },
      ...over,
    });

    it("names the revisions, and the acceptance only when there is one", async () => {
      const calls: Array<[string, unknown]> = [];
      const api = apiOver({
        call: async (name, args) => {
          calls.push([name, args]);
          return answered();
        },
      });
      await api.readJudgment("w", basis, null);
      await api.readJudgment("w", basis, "d".repeat(64));
      expect(calls).toEqual([
        ["read_judgment", { id: "w", basis }],
        ["read_judgment", { id: "w", basis, acceptance: "d".repeat(64) }],
      ]);
    });

    it("refuses an answer that is of other revisions than the ones that were asked about", async () => {
      // The caller is the only one who knows what it asked: a core that answered about another design
      // would have the screen show that verdict beside the design it read.
      const other = { ...named, model: "e".repeat(64) };
      const api = apiOver({ call: async () => answered({ basis: other }) });
      await expect(api.readJudgment("w", basis, null)).rejects.toThrow(/read_judgment\.basis/);
    });
  });

  it("send a first save with a null base", async () => {
    const calls: Array<[string, unknown]> = [];
    const api = apiOver({
      call: async (name, args) => {
        calls.push([name, args]);
        return { outcome: "saved", revision: A, parent: null };
      },
    });
    await api.saveSource("w", "text", null);
    expect(calls).toEqual([["save_source", { id: "w", text: "text", base: null }]]);
  });
});

describe("the bearer token", () => {
  const store = () => {
    const kept = new Map<string, string>();
    return {
      getItem: (k: string) => kept.get(k) ?? null,
      setItem: (k: string, v: string) => void kept.set(k, v),
    };
  };

  it("is taken from the fragment, kept for the tab, and removed from the address", () => {
    const storage = store();
    expect(takeToken("#token=abc123", storage)).toEqual({ token: "abc123", cleanHash: "" });
    expect(takeToken("", storage)).toEqual({ token: "abc123", cleanHash: "" });
  });

  it("leaves a fragment that is not a token alone", () => {
    expect(takeToken("#section", store())).toEqual({ token: null, cleanHash: "#section" });
  });

  it("is read from what a person pastes, which may be the token or the address that carries it", () => {
    expect(tokenFromPaste("  abc123  ")).toBe("abc123");
    expect(tokenFromPaste("http://100.64.0.2:5174/#token=abc123")).toBe("abc123");
    expect(tokenFromPaste("http://localhost:5174/?x=1#token=abc%2B123&y=2")).toBe("abc+123");
    expect(tokenFromPaste("")).toBe("");
  });

  it("is held for the transport, replaced when a person supplies one, and kept for the tab", () => {
    const storage = store();
    const held = credentials(null, storage);
    expect(held.token()).toBeNull();
    held.save("fresh");
    expect(held.token()).toBe("fresh");
    expect(takeToken("", storage).token).toBe("fresh");
  });

  it("is still used for this load when the browser refuses storage", () => {
    const refusing = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    };
    expect(takeToken("#token=abc123", refusing).token).toBe("abc123");
    expect(takeToken("", refusing).token).toBeNull();
    expect(takeToken("#token=abc123", null).token).toBe("abc123");
  });
});
