// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The page's half of a browser node (docs/adr/0117, D2): the shell of the platform's own parts
// around the module's client core. It carries an exchange with `fetch`, signs with an Ed25519
// key of the Web Cryptography API that cannot leave (D3), keeps what the node accepted in the
// page's store before the page is told so (D6), waits with the page's timers, and lets one page of
// the origin speak for all of them, by the node's Web Lock (D5). It decides nothing of the
// binding: every decision is the core's, which answers each input with the effects this module
// carries out (`sce_rust_mesh::https::browser`).
//
//   const node = await startNode({
//     settings: { base, own, server, verify, ... },   // as BrowserClient::start reads them
//     platform: pagePlatform(),                        // or the platform a host builds
//     Client,                                          // the module's class
//     onEvent: (event) => { ... },
//   });
//   await node.accept(envelope);   // resolves once the store holds it
//
// What a node publishes reaches every page of the origin that opened it:
//   { tell: "leading" }                       this page holds the lock and speaks (this page only)
//   { tell: "persisted", value }              whether the store is persistent (null: no manager)
//   { tell: "ready" | "restarted", peer }
//   { tell: "lost" | "interrupted" | "unauthenticated", peer, words }
//   { tell: "arrived", peer, envelope, mark } envelope a Uint8Array; report what was kept with
//                                             node.held(mark)
//   { tell: "carried", count }                the server holds that many more accepted envelopes
//
// The lock is a convenience of one storage bucket and no fence (D5): a page may go on as the node
// after another took the lock, and what must be one is therefore the store's. In this module that
// is the outbox, numbered by the store, from which the page that speaks takes what any page
// accepted. Two pages that both speak carry an accepted envelope twice, which the receiver's
// window drops (SCE_MESH.md section 10.5).

/** Why a node was not started or an input was not taken: `code` names the refusal. */
export class NodeRefused extends Error {
  constructor(code, message) {
    super(message);
    this.name = "NodeRefused";
    this.code = code;
  }
}

/** The algorithm of the node's key (D3): WebCrypto's Ed25519, and no other. */
const KEY_ALGORITHM = { name: "Ed25519" };

/**
 * The platform of the page this module runs in, from its own globals: a secure context, its
 * origin, and the parts the node is made of. A page without one of them is refused by name.
 */
export function pagePlatform(scope = globalThis) {
  if (!scope.isSecureContext) {
    throw new NodeRefused("secure-context", "a browser node runs only in a secure context");
  }
  for (const [part, present] of [
    ["web-locks", scope.navigator?.locks],
    ["indexeddb", scope.indexedDB],
    ["broadcast-channel", scope.BroadcastChannel],
    ["webcrypto", scope.crypto?.subtle],
  ]) {
    if (!present) {
      throw new NodeRefused(part, `this page has no ${part}, which a browser node is made of`);
    }
  }
  return {
    origin: scope.location.origin,
    fetch: scope.fetch.bind(scope),
    // D9: the binding is to the origin that served the page, and the platform holds it there:
    // a request to another origin is refused by `fetch` itself, and no credential of the page's
    // and no cached answer goes with an exchange.
    fetchInit: { mode: "same-origin", credentials: "same-origin", cache: "no-store" },
    crypto: scope.crypto,
    locks: scope.navigator.locks,
    storage: scope.navigator.storage,
    openStore: (name) => openIndexedDbStore(scope.indexedDB, name),
    BroadcastChannel: scope.BroadcastChannel,
    setTimeout: scope.setTimeout.bind(scope),
    clearTimeout: scope.clearTimeout.bind(scope),
    now: () => Date.now(),
  };
}

/** Start a node of `settings` on `platform`; it speaks once it holds the node's lock. */
export async function startNode({ settings, platform, Client, onEvent }) {
  const node = new BrowserNode(settings, platform, Client, onEvent);
  await node.open();
  return node;
}

// ── The store (D6): IndexedDB, each write one transaction of durability `strict` ──────────────

const STORE_VERSION = 1;
const STRICT = { durability: "strict" };

function requested(request) {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

function finished(transaction) {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error);
    transaction.onabort = () =>
      reject(transaction.error ?? new DOMException("the transaction was aborted", "AbortError"));
  });
}

/**
 * The node's store in the IndexedDB database `name`: its key pair, kept as the platform's
 * `CryptoKey`s, and the outbox of what the node accepted, numbered by the store. Every write is
 * one transaction, completed with durability `strict` before it resolves.
 */
export async function openIndexedDbStore(factory, name) {
  const opening = factory.open(name, STORE_VERSION);
  opening.onupgradeneeded = () => {
    const db = opening.result;
    if (!db.objectStoreNames.contains("key")) db.createObjectStore("key");
    if (!db.objectStoreNames.contains("outbox")) db.createObjectStore("outbox", { autoIncrement: true });
  };
  const db = await requested(opening);
  return {
    async key() {
      const transaction = db.transaction("key", "readonly");
      const done = finished(transaction);
      const pair = await requested(transaction.objectStore("key").get("pair"));
      await done;
      return pair;
    },
    // The pair stored first is the node's: one page's write, or another's that came before it.
    async keepKey(pair) {
      const transaction = db.transaction("key", "readwrite", STRICT);
      const done = finished(transaction);
      const keys = transaction.objectStore("key");
      let kept;
      const reading = keys.get("pair");
      reading.onsuccess = () => {
        kept = reading.result;
        if (kept === undefined) {
          keys.put(pair, "pair");
          kept = pair;
        }
      };
      await done;
      return kept;
    },
    async accept(envelope) {
      const transaction = db.transaction("outbox", "readwrite", STRICT);
      const done = finished(transaction);
      const adding = requested(transaction.objectStore("outbox").add(envelope));
      await done;
      return await adding;
    },
    async outbox() {
      const transaction = db.transaction("outbox", "readonly");
      const done = finished(transaction);
      const entries = [];
      const cursor = transaction.objectStore("outbox").openCursor();
      cursor.onsuccess = () => {
        const at = cursor.result;
        if (!at) return;
        entries.push({ number: at.key, bytes: new Uint8Array(at.value) });
        at.continue();
      };
      await done;
      return entries;
    },
    async release(numbers) {
      if (numbers.length === 0) return;
      const transaction = db.transaction("outbox", "readwrite", STRICT);
      const done = finished(transaction);
      const outbox = transaction.objectStore("outbox");
      for (const number of numbers) outbox.delete(number);
      await done;
    },
    close() {
      db.close();
    },
  };
}

// ── Bytes as the module writes them ─────────────────────────────────────────────────────────────

export function toBase64(bytes) {
  let text = "";
  for (let at = 0; at < bytes.length; at += 0x8000) {
    text += String.fromCharCode(...bytes.subarray(at, at + 0x8000));
  }
  return btoa(text);
}

export function fromBase64(text) {
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let at = 0; at < binary.length; at += 1) bytes[at] = binary.charCodeAt(at);
  return bytes;
}

function sameBytes(a, b) {
  if (a.length !== b.length) return false;
  for (let at = 0; at < a.length; at += 1) if (a[at] !== b[at]) return false;
  return true;
}

// ── The key (D3) ────────────────────────────────────────────────────────────────────────────────

/**
 * The node's key pair: the one its store holds, or one made now, not extractable, and kept unless
 * another page kept one first. A platform without Ed25519 is refused by name: a key in the page's
 * own memory could leave it, and the node does not fall back to one.
 */
async function nodeKey(platform, store) {
  let pair = await store.key();
  if (pair === undefined) {
    let made;
    try {
      made = await platform.crypto.subtle.generateKey(KEY_ALGORITHM, false, ["sign", "verify"]);
    } catch (error) {
      if (error?.name === "NotSupportedError") {
        throw new NodeRefused(
          "ed25519",
          "this platform's Web Cryptography API makes no Ed25519 key, and a browser node does " +
            "not keep a key the page's own memory holds",
        );
      }
      throw error;
    }
    pair = await store.keepKey(made);
  }
  if (pair.privateKey.extractable) {
    throw new NodeRefused("extractable", "the node's stored key can leave the platform");
  }
  return pair;
}

// ── The node ────────────────────────────────────────────────────────────────────────────────────

const LANES = ["poll", "send"];

class BrowserNode {
  #settings;
  #platform;
  #Client;
  #listeners = new Set();
  #store = null;
  #key = null;
  #channel = null;
  #names;
  #client = null;
  #lanes = new Map();
  /** What the core was given, by the store's number, until its server holds it. */
  #given = new Map();
  /** The inputs the core is given, one at a time, each step's effects before the next's. */
  #inputs = [];
  #stepping = false;
  /** Reads of the outbox and its releases, one at a time. */
  #storeWork = Promise.resolve();
  #release = null;
  #lockRequest = null;
  #lockHeld = null;
  #closed = false;
  #stopping = false;
  #lanesOver = null;

  leading = false;
  persisted = null;

  constructor(settings, platform, Client, onEvent) {
    this.#settings = settings;
    this.#platform = platform;
    this.#Client = Client;
    if (onEvent) this.#listeners.add(onEvent);
    // The node's name: what its lock, its store and its channel are named by, one per pair of a
    // server and a machine.
    const node = `sce-node/${settings.server}/${settings.own}`;
    this.#names = { lock: node, store: node, channel: node };
  }

  /** Listen to what the node publishes; answers a function that stops listening. */
  on(listener) {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  async open() {
    const platform = this.#platform;
    this.#store = await platform.openStore(this.#names.store);
    try {
      this.#key = await nodeKey(platform, this.#store);
    } catch (error) {
      this.#store.close();
      throw error;
    }
    const publicKey = new Uint8Array(await platform.crypto.subtle.exportKey("raw", this.#key.publicKey));
    this.#settings = {
      ...this.#settings,
      origin: platform.origin,
      public_key: toBase64(publicKey).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, ""),
      durable: true,
    };
    // The settings are the module's to judge, here as they will be when this page speaks: a page
    // whose settings the core refuses is refused at once, whichever page speaks.
    try {
      this.#makeClient().free();
    } catch (error) {
      this.#store.close();
      throw new NodeRefused("settings", String(error?.message ?? error));
    }
    this.persisted = await persistedNow(platform.storage);
    this.#channel = new platform.BroadcastChannel(this.#names.channel);
    this.#channel.onmessage = (message) => this.#heard(message.data);
    // D5: the page that holds the exclusive lock of the node's name is the node; the others wait
    // for it in the lock's queue, and one of them takes it when this page ends.
    this.#lockRequest = new AbortController();
    this.#lockHeld = platform.locks
      .request(
        this.#names.lock,
        { mode: "exclusive", signal: this.#lockRequest.signal },
        () => this.#lead(),
      )
      .catch((error) => {
        if (error?.name !== "AbortError") console.error("the node's lock was not granted", error);
      });
  }

  /**
   * Keep `envelope` in the store, and resolve with the number the store gave it once the write is
   * complete: only then is it accepted (D6). The page that speaks carries it.
   */
  async accept(envelope) {
    if (this.#closed) throw new NodeRefused("closed", "the node was closed");
    const number = await this.#store.accept(Uint8Array.from(envelope));
    if (this.leading) this.#pull();
    else this.#channel.postMessage({ kind: "accepted" });
    return number;
  }

  /** The page's store holds what arrived up to `mark`, the mark of an `arrived` event. */
  held(mark) {
    if (this.#closed) return;
    if (this.leading && this.#client) this.#step(() => this.#client.held(JSON.stringify(mark)));
    else this.#channel.postMessage({ kind: "held", mark });
  }

  /** The `keyid` of the node's key, once this page speaks. */
  keyId() {
    return this.#client ? this.#client.keyId() : null;
  }

  /** This page stops being the node: its exchanges end, and the lock goes to another page. */
  async close() {
    if (this.#closed) return;
    this.#closed = true;
    if (this.#client) {
      this.#stopping = true;
      const over = new Promise((resolve) => {
        this.#lanesOver = resolve;
      });
      this.#step(() => this.#client.stop());
      for (const lane of this.#lanes.values()) lane.abort?.abort(new DOMException("closed", "AbortError"));
      await over;
      this.#client.free();
      this.#client = null;
    }
    this.leading = false;
    this.#channel?.close();
    this.#channel = null;
    if (this.#release) this.#release();
    else this.#lockRequest?.abort();
    await this.#lockHeld;
    await this.#storeWork;
    this.#store?.close();
  }

  // ── Speaking (D5) ──────────────────────────────────────────────────────────────────────────

  #makeClient() {
    const seed = this.#platform.crypto.getRandomValues(new Uint8Array(32));
    return new this.#Client(JSON.stringify(this.#settings), seed);
  }

  /** This page holds the lock: it restores what was accepted from the store and speaks. */
  async #lead() {
    if (this.#closed) return;
    const released = new Promise((resolve) => {
      this.#release = resolve;
    });
    this.leading = true;
    this.#client = this.#makeClient();
    for (const lane of LANES) this.#lanes.set(lane, { timer: null, abort: null, over: false });
    this.#emit({ tell: "leading" });
    this.#step(() => this.#client.started());
    // D6: what any page accepted and the server does not hold yet is carried by this page now;
    // and the node asks for a persistent bucket, and publishes which it has, while it speaks.
    await this.#pull();
    const value = await persistence(this.#platform.storage);
    if (!this.#closed) this.#publish({ tell: "persisted", value });
    await released;
  }

  /**
   * Run `work` on the store after the work before it, whatever came of that; answers whether it
   * was done.
   */
  #storeTask(work) {
    this.#storeWork = this.#storeWork.then(work).then(
      () => true,
      (error) => {
        console.error("the node's store failed", error);
        return false;
      },
    );
    return this.#storeWork;
  }

  /** Give the core what the store holds that it was not given, oldest first. */
  #pull() {
    return this.#storeTask(async () => {
      if (!this.#client || this.#stopping) return;
      for (const { number, bytes } of await this.#store.outbox()) {
        if (this.#given.has(number)) continue;
        this.#given.set(number, bytes);
        this.#step(() => this.#client.transmitted(bytes));
      }
    });
  }

  #heard(message) {
    if (!message || this.#closed) return;
    if (message.kind === "accepted" && this.leading) this.#pull();
    else if (message.kind === "held" && this.leading && this.#client) {
      this.#step(() => this.#client.held(JSON.stringify(message.mark)));
    } else if (message.kind === "event" && !this.leading) this.#tell(message.event);
  }

  #publish(event) {
    this.#tell(event);
    this.#channel?.postMessage({ kind: "event", event });
  }

  #tell(event) {
    if (event.tell === "persisted") this.persisted = event.value;
    this.#emit(event);
  }

  #emit(event) {
    for (const listener of this.#listeners) {
      try {
        listener(event);
      } catch (error) {
        console.error("a listener of the node threw", error);
      }
    }
  }

  // ── The core's inputs and effects ──────────────────────────────────────────────────────────

  /** Give the core the input `give` makes, after every input before it and its effects. */
  #step(give) {
    this.#inputs.push(give);
    if (this.#stepping) return;
    this.#stepping = true;
    try {
      while (this.#inputs.length > 0) {
        const next = this.#inputs.shift();
        if (!this.#client) continue;
        let effects;
        try {
          effects = JSON.parse(next());
        } catch (error) {
          console.error("the node's core did not take an input", error);
          continue;
        }
        for (const effect of effects) this.#effect(effect);
      }
    } finally {
      this.#stepping = false;
    }
  }

  #effect(effect) {
    if (effect.count !== undefined) return;
    if (effect.tell !== undefined) return this.#told(effect);
    const lane = this.#lanes.get(effect.lane);
    switch (effect.ask) {
      case "draw":
        return this.#draw(effect.lane, effect.bytes);
      case "sign":
        return this.#sign(effect.lane, effect.base);
      case "carry":
        return this.#carry(lane, effect);
      case "wake":
        return this.#wake(lane, effect);
      case "stop":
        return this.#stopLane(lane);
    }
  }

  #told(effect) {
    switch (effect.tell) {
      case "arrived":
        return this.#publish({
          tell: "arrived",
          peer: effect.peer,
          envelope: fromBase64(effect.envelope),
          mark: JSON.parse(this.#client.received()),
        });
      case "carried":
        return this.#carried(effect.envelopes.map(fromBase64));
      case "unsent": {
        // What the page accepted stays the node's to carry until its server holds it (D6): it
        // goes to the core again, which carries it after the lane's wait. A node that stops
        // leaves it in the store for the page that speaks next.
        if (this.#stopping) return;
        const bytes = fromBase64(effect.envelope);
        return this.#step(() => this.#client.transmitted(bytes));
      }
      default: {
        const { tell, peer, words } = effect;
        return this.#publish(words === undefined ? { tell, peer } : { tell, peer, words });
      }
    }
  }

  /** The server holds `envelopes`: the store releases the oldest accepted ones they are. */
  #carried(envelopes) {
    const released = [];
    for (const envelope of envelopes) {
      for (const [number, bytes] of this.#given) {
        if (!released.includes(number) && sameBytes(bytes, envelope)) {
          released.push(number);
          break;
        }
      }
    }
    for (const number of released) this.#given.delete(number);
    this.#storeTask(() => this.#store.release(released)).then((done) => {
      if (done) this.#publish({ tell: "carried", count: released.length });
    });
  }

  #draw(lane, count) {
    let random = new Uint8Array(0);
    try {
      random = this.#platform.crypto.getRandomValues(new Uint8Array(count));
    } catch (error) {
      console.error("the page's random source gave nothing", error);
    }
    const moment = this.#platform.now();
    this.#step(() => this.#client.drawn(lane, random, moment));
  }

  #sign(lane, base) {
    this.#platform.crypto.subtle.sign(KEY_ALGORITHM, this.#key.privateKey, fromBase64(base)).then(
      (signature) => this.#step(() => this.#client.signed(lane, new Uint8Array(signature))),
      (error) => {
        console.error("the node's key did not sign", error);
        this.#step(() => this.#client.signed(lane, new Uint8Array(0)));
      },
    );
  }

  #carry(lane, { lane: name, url, headers, body, timeout_ms }) {
    const abort = new AbortController();
    lane.abort = abort;
    // A node that stops carries nothing more: an exchange its core began while it stopped ends
    // at once, and the core then ends the lane.
    if (this.#stopping) abort.abort(new DOMException("closed", "AbortError"));
    let timedOut = false;
    const timer = this.#platform.setTimeout(() => {
      timedOut = true;
      abort.abort(new DOMException("the exchange took longer than its bound", "TimeoutError"));
    }, timeout_ms);
    const init = {
      ...this.#platform.fetchInit,
      method: "POST",
      headers,
      body: fromBase64(body),
      signal: abort.signal,
      redirect: "error",
    };
    this.#platform
      .fetch(url, init)
      .then(async (response) => {
        const answer = new Uint8Array(await response.arrayBuffer());
        return { status: response.status, headers: [...response.headers], body: answer };
      })
      .then(
        (response) => {
          this.#platform.clearTimeout(timer);
          lane.abort = null;
          this.#step(() =>
            this.#client.answered(name, response.status, JSON.stringify(response.headers), response.body),
          );
        },
        (error) => {
          this.#platform.clearTimeout(timer);
          lane.abort = null;
          const kind = timedOut ? "timeout" : "other";
          this.#step(() => this.#client.failed(name, kind, `fetch failed: ${error?.message ?? error}`));
        },
      );
  }

  #wake(lane, { lane: name, ms, until_ms }) {
    const delay = ms !== undefined ? ms : Math.max(0, until_ms - this.#platform.now());
    lane.timer = this.#platform.setTimeout(() => {
      lane.timer = null;
      this.#step(() => this.#client.woken(name));
    }, delay);
  }

  #stopLane(lane) {
    if (lane.timer !== null) this.#platform.clearTimeout(lane.timer);
    lane.timer = null;
    lane.over = true;
    if ([...this.#lanes.values()].every((each) => each.over) && this.#lanesOver) this.#lanesOver();
  }
}

/** Whether the store's bucket is persistent: null without a storage manager. */
async function persistedNow(storage) {
  if (!storage?.persisted) return null;
  try {
    return await storage.persisted();
  } catch (error) {
    console.error("the storage manager did not answer", error);
    return false;
  }
}

/** Ask for a persistent bucket and answer which the store has (D6): null without a manager. */
async function persistence(storage) {
  if (!storage?.persist || !storage?.persisted) return null;
  try {
    if (await storage.persisted()) return true;
    return await storage.persist();
  } catch (error) {
    console.error("the storage manager did not answer", error);
    return false;
  }
}
