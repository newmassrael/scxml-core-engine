// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The store and the lock of a browser node where the host has neither: Node, which has fetch,
// WebCrypto, timers and BroadcastChannel and no IndexedDB or Web Locks. They hold the contracts
// the shell reads of them (`js/node.mjs`), in one process: a store whose every write is whole
// when it resolves, shared by the nodes that open one name, and an exclusive lock granted in the
// order asked, released when its holder's callback settles, and given up by a request whose
// signal aborts before it is granted. In a browser the kit runs on the platform's own.

/** A function that opens the store of a name, one store per name, in this process's memory. */
export function memoryStores() {
  const stores = new Map();
  return async (name) => {
    let held = stores.get(name);
    if (!held) {
      held = { pair: undefined, outbox: new Map(), next: 1 };
      stores.set(name, held);
    }
    return memoryStore(held);
  };
}

function memoryStore(held) {
  return {
    async key() {
      return held.pair;
    },
    async keepKey(pair) {
      if (held.pair === undefined) held.pair = pair;
      return held.pair;
    },
    async accept(envelope) {
      const number = held.next;
      held.next += 1;
      held.outbox.set(number, Uint8Array.from(envelope));
      return number;
    },
    async outbox() {
      return [...held.outbox]
        .sort(([a], [b]) => a - b)
        .map(([number, bytes]) => ({ number, bytes: Uint8Array.from(bytes) }));
    },
    async release(numbers) {
      for (const number of numbers) held.outbox.delete(number);
    },
    close() {},
  };
}

/** A lock manager of exclusive locks by name, as `navigator.locks` grants them. */
export function memoryLocks() {
  const queues = new Map();
  const grantNext = (name) => {
    const queue = queues.get(name);
    if (queue && queue.length > 0 && !queue[0].granted) queue[0].grant();
  };
  return {
    request(name, options, callback) {
      return new Promise((resolve, reject) => {
        const queue = queues.get(name) ?? [];
        queues.set(name, queue);
        const entry = {
          granted: false,
          grant: () => {
            entry.granted = true;
            Promise.resolve()
              .then(() => callback({ name, mode: "exclusive" }))
              .then(resolve, reject)
              .finally(() => {
                queue.splice(queue.indexOf(entry), 1);
                grantNext(name);
              });
          },
        };
        const signal = options?.signal;
        if (signal?.aborted) {
          reject(signal.reason ?? new DOMException("aborted", "AbortError"));
          return;
        }
        signal?.addEventListener("abort", () => {
          if (entry.granted || !queue.includes(entry)) return;
          queue.splice(queue.indexOf(entry), 1);
          reject(new DOMException("the request was aborted", "AbortError"));
        });
        queue.push(entry);
        grantNext(name);
      });
    },
  };
}
