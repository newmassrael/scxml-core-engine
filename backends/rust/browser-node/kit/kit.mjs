// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The kit a browser node is held to (docs/adr/0117, D2): its cases, run by a host against the
// binding's own server (`sce-rust-mesh`'s example `https_echo`, which answers every envelope a
// client sends with the same envelope) at the page's origin. Node runs it (`run_node.mjs`), and
// so do headless Chrome and Firefox (`run_browser.mjs`, `page.html`), each on its own parts:
// Node on its fetch, WebCrypto and timers with a store and a lock in memory, a browser on
// IndexedDB and Web Locks too.
//
// `env` is what a host gives the kit:
//   host       the name of the host and its version, which the report carries
//   Client     the module's class
//   platform() the platform a page of the origin is made of (`pagePlatform` in a browser)
//   origin     the origin that serves the page and the binding
//   server     what the server printed when it started: { key_id, public_key, fresh_ms }
//   control    { proxy(open) }: let the page's exchanges reach the server, or fail them at the
//              transport, as a network that is gone does
//
// Each case answers nothing or throws why; `runKit` answers one row per case.

import { startNode, NodeRefused } from "../js/node.mjs";

const WAIT_MS = 20_000;
const SERVER = "kit";

let names = 0;

/** A machine name no other case of this run posts under, so each starts with no pin. */
function unique(env) {
  names += 1;
  return `kit-${Date.now().toString(36)}-${Math.floor(Math.random() * 0x10000).toString(36)}-${names}`;
}

/** The name of a node's store, as the shell names it. */
function storeName(own) {
  return `sce-node/${SERVER}/${own}`;
}

/** An envelope the server sends back as it came: one CBOR byte string, told apart by `tag`. */
function envelopeOf(tag) {
  return Uint8Array.from([0x43, 0x6b, 0x69, tag & 0xff]);
}

function same(a, b) {
  return a.length === b.length && a.every((byte, at) => byte === b[at]);
}

function check(holds, what) {
  if (!holds) throw new Error(what);
}

function settingsOf(env, own, changes = {}) {
  return {
    base: env.origin,
    own,
    server: SERVER,
    hold_ms: 1000,
    exchange_timeout_ms: 5000,
    backoff: { initial_ms: 100, max_ms: 500, factor: 2, jitter: true },
    verify: [{ id: env.server.key_id, public_key: env.server.public_key }],
    ...changes,
  };
}

/** What a node published, and a wait for the first event from `from` on that passes `test`. */
function recorder() {
  const events = [];
  const waiters = new Set();
  const onEvent = (event) => {
    events.push(event);
    for (const waiter of [...waiters]) {
      if (events.length - 1 >= waiter.from && waiter.test(event)) {
        waiters.delete(waiter);
        waiter.resolve(event);
      }
    }
  };
  const until = (test, what, from = 0, ms = WAIT_MS) => {
    const seen = events.slice(from).find(test);
    if (seen) return Promise.resolve(seen);
    return new Promise((resolve, reject) => {
      const waiter = { test, resolve, from };
      waiters.add(waiter);
      setTimeout(() => {
        if (waiters.delete(waiter)) reject(new Error(`no ${what} within ${ms} ms`));
      }, ms);
    });
  };
  return { events, onEvent, until, mark: () => events.length };
}

const is = (tell) => (event) => event.tell === tell;
const arrivedAs = (envelope) => (event) => event.tell === "arrived" && same(event.envelope, envelope);

/**
 * A node of `own`, whose page keeps what arrives and reports it held, as a page that keeps what
 * its node hands it does: the server then deletes it from the mailbox (ADR 0109, D10).
 */
async function start(env, own, changes = {}, platform = env.platform()) {
  const seen = recorder();
  let node = null;
  node = await startNode({
    settings: settingsOf(env, own, changes),
    platform,
    Client: env.Client,
    onEvent: (event) => {
      seen.onEvent(event);
      if (event.tell === "arrived") node?.held(event.mark);
    },
  });
  return { node, seen };
}

/** A node that speaks and whose server took it: it holds the lock, and a request of it was taken. */
async function speaking(env, own) {
  const started = await start(env, own);
  await started.seen.until(is("leading"), "lock");
  await started.seen.until(is("ready"), "ready server");
  return started;
}

async function outboxOf(env, own) {
  const store = await env.platform().openStore(storeName(own));
  try {
    return await store.outbox();
  } finally {
    store.close();
  }
}

const CASES = [
  {
    // D2, D4, D6: the first request is refused for its freshness and signed again with the value
    // the refusal gives, every answer is verified under the server's key, what the page accepted
    // reaches the server and comes back, and the store releases it once the server holds it.
    name: "a node speaks to its server and hears it",
    async run(env) {
      const own = unique(env);
      const { node, seen } = await speaking(env, own);
      try {
        const persisted = await seen.until(is("persisted"), "published persistence");
        const manager = env.platform().storage !== undefined;
        check(
          manager ? typeof persisted.value === "boolean" : persisted.value === null,
          `the persistence published is ${persisted.value}`,
        );
        const envelope = envelopeOf(1);
        const number = await node.accept(envelope);
        check(Number.isInteger(number) && number > 0, `the store numbered the envelope ${number}`);
        const arrived = await seen.until(arrivedAs(envelope), "echo of what the page accepted");
        check(arrived.mark && arrived.mark.number >= 1, "an arrival carries no mark to report");
        await seen.until((event) => event.tell === "carried" && event.count === 1, "release");
        check((await outboxOf(env, own)).length === 0, "the store still holds what its server holds");
      } finally {
        await node.close();
      }
    },
  },
  {
    // D3: the key is the platform's, not extractable, kept by the store, and the same key speaks
    // for the node when the page starts again, which the server, pinned to it, takes.
    name: "the key cannot leave and is the one the node keeps",
    async run(env) {
      const own = unique(env);
      const first = await speaking(env, own);
      const keyId = first.node.keyId();
      await first.node.close();
      const platform = env.platform();
      const store = await platform.openStore(storeName(own));
      const pair = await store.key();
      store.close();
      check(pair !== undefined, "the store keeps no key");
      check(pair.privateKey.extractable === false, "the private key is extractable");
      check(pair.privateKey.algorithm.name === "Ed25519", "the key is not Ed25519");
      let exported = false;
      try {
        await platform.crypto.subtle.exportKey("pkcs8", pair.privateKey);
        exported = true;
      } catch {
        // What the platform answers a key that cannot leave.
      }
      check(!exported, "the private key was exported");
      const second = await speaking(env, own);
      try {
        check(second.node.keyId() === keyId, "the node speaks with another key after a restart");
      } finally {
        await second.node.close();
      }
    },
  },
  {
    // D6: what the page accepted is in the store before it is told so, stays there while its
    // server cannot be reached and after the page ends, and the page that speaks next carries it.
    name: "what was accepted is kept until its server holds it",
    async run(env) {
      const own = unique(env);
      const first = await speaking(env, own);
      const envelope = envelopeOf(3);
      try {
        await env.control.proxy(false);
        await first.node.accept(envelope);
        await first.seen.until(is("interrupted"), "interruption");
      } finally {
        await first.node.close();
        await env.control.proxy(true);
      }
      const kept = await outboxOf(env, own);
      check(kept.length === 1 && same(kept[0].bytes, envelope), "the store lost what was accepted");
      const next = await speaking(env, own);
      try {
        await next.seen.until(arrivedAs(envelope), "echo of what the page before accepted");
        await next.seen.until((event) => event.tell === "carried" && event.count === 1, "release");
        check((await outboxOf(env, own)).length === 0, "the store still holds what its server holds");
      } finally {
        await next.node.close();
      }
    },
  },
  {
    // D5: of two pages of the origin, the one that holds the lock speaks; the other hands it what
    // it accepts through the store and hears what it publishes; when the first ends, the second
    // takes the lock and speaks.
    name: "one page speaks for the origin",
    async run(env) {
      const own = unique(env);
      const first = await speaking(env, own);
      let second = null;
      try {
        second = await start(env, own);
        check(!second.node.leading, "a second page speaks while the first holds the lock");
        const envelope = envelopeOf(4);
        await second.node.accept(envelope);
        await second.seen.until(arrivedAs(envelope), "echo heard by the page that does not speak");
        await first.seen.until(arrivedAs(envelope), "echo at the page that speaks");
        check(!second.seen.events.some(is("leading")), "the second page spoke");
        const before = second.seen.mark();
        await first.node.close();
        await second.seen.until(is("leading"), "lock taken from the page that ended", before);
        await second.seen.until(is("ready"), "ready server of the page that took the lock", before);
        const after = envelopeOf(5);
        await second.node.accept(after);
        await second.seen.until(arrivedAs(after), "echo at the page that took the lock");
      } finally {
        await first.node.close();
        await second?.node.close();
      }
    },
  },
  {
    // SCE_MESH.md section 22.9 and D4: an answer the server's key did not sign, a refusal for freshness among
    // them, is taken nothing of, and the node is never ready.
    name: "an answer the trusted key did not sign is not taken",
    async run(env) {
      const own = unique(env);
      const stranger = "A".repeat(42) + "E";
      const { node, seen } = await start(env, own, {
        verify: [{ id: env.server.key_id, public_key: stranger }],
      });
      try {
        await seen.until(is("unauthenticated"), "refusal of an answer by another key");
        check(!seen.events.some(is("ready")), "a server whose answers do not verify was ready");
      } finally {
        await node.close();
      }
    },
  },
  {
    // D9: the binding is to the origin that served the page.
    name: "a server of another origin is refused",
    async run(env) {
      let refused = null;
      try {
        const { node } = await start(env, unique(env), { base: "http://127.0.0.1:1" });
        await node.close();
      } catch (error) {
        refused = error;
      }
      check(refused instanceof NodeRefused && refused.code === "settings", `refused as ${refused}`);
      check(/origin/.test(refused.message), `refused for another reason: ${refused.message}`);
    },
  },
  {
    // D3: a platform whose Web Cryptography API makes no Ed25519 key is refused by name, and no
    // key of another kind is made in its place.
    name: "a platform without Ed25519 is refused by name",
    async run(env) {
      const platform = env.platform();
      const subtle = platform.crypto.subtle;
      const without = {
        ...platform,
        crypto: {
          getRandomValues: (bytes) => platform.crypto.getRandomValues(bytes),
          subtle: {
            generateKey: () =>
              Promise.reject(new DOMException("Unrecognized name.", "NotSupportedError")),
            exportKey: (...args) => subtle.exportKey(...args),
            sign: (...args) => subtle.sign(...args),
          },
        },
      };
      const own = unique(env);
      let refused = null;
      try {
        const { node } = await start(env, own, {}, without);
        await node.close();
      } catch (error) {
        refused = error;
      }
      check(refused instanceof NodeRefused && refused.code === "ed25519", `refused as ${refused}`);
      const store = await platform.openStore(storeName(own));
      const pair = await store.key();
      store.close();
      check(pair === undefined, "a key was kept in place of the Ed25519 one");
    },
  },
];

/** Run every case on `env`, one after another, and answer a row of each. */
export async function runKit(env) {
  const rows = [];
  for (const kitCase of CASES) {
    const started = Date.now();
    try {
      await kitCase.run(env);
      rows.push({ name: kitCase.name, pass: true, seconds: (Date.now() - started) / 1000 });
    } catch (error) {
      rows.push({
        name: kitCase.name,
        pass: false,
        error: String(error?.stack ?? error),
        seconds: (Date.now() - started) / 1000,
      });
    }
  }
  return { host: env.host, rows };
}

/** Print `report` and answer the exit status: 0 when every case passed, 1 otherwise. */
export function printReport(report, log = console.log) {
  let failed = 0;
  for (const row of report.rows) {
    log(`${row.pass ? "pass" : "FAIL"} ${row.name} (${row.seconds.toFixed(1)}s)`);
    if (!row.pass) {
      failed += 1;
      log(`     ${row.error.split("\n").join("\n     ")}`);
    }
  }
  log(`${report.host}: ${report.rows.length - failed} of ${report.rows.length} cases passed`);
  return failed === 0 && report.rows.length > 0 ? 0 : 1;
}
