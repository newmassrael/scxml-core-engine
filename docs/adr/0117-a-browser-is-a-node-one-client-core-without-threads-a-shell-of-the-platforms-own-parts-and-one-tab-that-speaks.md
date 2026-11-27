# ADR 0117 — A browser is a node: one client core without threads, a shell of the platform's own parts, and one tab that speaks

- Status: Accepted (2026-11-22, by the owner), as amended before it was accepted: the freshness step is a row of ADR
  0113 D4's table (D4), and a change of person is a change of incarnation whose store is not carried over (D6). Its
  model (D10) was written and checked before any code it is of, and amended D5; the separation of the core (step 1)
  needs none.
- Date: 2026-11-22
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: the wasm32 target is the Rust lane, not another backend; the
  contract is general; what is special to one application stays in the application.
- Amends, once accepted (each amended document gains an "Amended by: ADR 0117" line in the landing that moves it):
  `SCE_MESH.md` §18, §22.6, §22.8 and §22.9; ADR 0115 (D13 names its first module: the client core);
  `docs/SCE_ACCEPTED_SUBSET.md` where a generated surface is listed.
- Scope: the `https` binding's dialing side, the Rust lane on native and on `wasm32-unknown-unknown` in a browser, and a
  surface generated from a document for TypeScript.
- Related: ADR 0113 (an instance), ADR 0116 (a person behind it), ADR 0115 (D13), ADR 0109 (D9, D10).

## Context

A machine already runs on wasm32 as a test harness (`backends/rust/scenarios`), with no sockets. The `https` client of
the Rust core holds threads and waits on a condition variable (`backends/rust/mesh/src/https/client.rs`), and its port
is a call that blocks until the exchange ends (`HttpsTransport::exchange`). A browser's main thread may not block and
has no such thread, so no adapter of that port can be written there: the fault is in the shape of the core, not in a
missing adapter. ADR 0115 D13 asks the same core to be separated for another reason, that decisions be a synchronous
function a proof can be of.

## The textbook structure

- **Decisions are a pure step; input and output are at the edge** (IronFleet, 2015: a host is a state machine and a
  small loop moves its packets). One core, many shells.
- **An application decides what to do when an outcome is unknown** (Saltzer, Reed and Clark 1984). A browser that loses
  its storage has an outcome nobody else can know.
- **A key that cannot leave limits an attacker to the time they are inside** (RFC 9449 section 11.4), and a value the
  server chose limits what they can prepare (section 11.2). Neither stops code that runs in the page, and the OAuth
  working group's draft for browser applications says that nothing does but not running it.
- **What is a hint is not a guarantee**, and the platform's own words decide which is which.

## Decision

### D1. One client core, which moves nothing

The core of the `https` client is a state machine with no thread, lock or clock: it takes *inputs* (an event to send, an
answer, a thing stored, a signature made, a moment) and gives *effects* (carry this exchange, sign these bytes, store
this, deliver this event, wake me at this moment). It is the first module of ADR 0115 D13, and its trace is D4's.
Nothing of a browser is in it. The order of effects that matters is in their data: an exchange names the stored write it
waits for.

### D2. Two shells over it, held to one kit

- **Native**: the threads of today, moved out of the core. `HttpsTransport` stays as the port of this shell, so an
  embedding that implements it does not move.
- **Browser**: `fetch`, WebCrypto, IndexedDB and timers, on one event loop, written once and held to the kit. The
  request headers of the binding (`If-Match`, `Content-Digest`, `Signature`, `Signature-Input` and the `Sce-Mesh-` ones)
  are not among those the Fetch standard forbids a page to set.

The conformance kit of the binding runs on both, in Node and in headless browsers of two engines.

### D3. The key cannot leave the browser

The instance's key is an Ed25519 key pair of the Web Cryptography API, made with `extractable` false and kept as a
`CryptoKey` in IndexedDB. A browser without that algorithm is refused by name: a key in the page's own memory is one
that can leave, and a deployment does not fall back to it. The API says of a key shared with another origin that access
to it can no longer be taken back; the node never shares one.

### D4. A request cannot be prepared long before it is sent

A response carries `Sce-Mesh-Fresh`, a value the server makes from a key of its own and a step of time, for this machine
and instance; a request under `key` to an open machine signs the last one it was given (`fresh_ms`, the step, has no
default), and the server takes the present step and the one before. It holds no state for it. A request under an older
step, or none, is refused before anything of it is taken in, the refusal carries the present value, and the client signs
again; the value is one more row of the table of what is signed (ADR 0113 D4), so it can be neither changed nor taken
off, and the refusal is signed as every answer that changes what the client holds. A request signed with the page's key
while an attacker was inside is of no use after two steps, and, under ADR 0113 D4, none under an incarnation offered
later. What this gives is told in that form and no larger: an attacker who runs in the page acts as the page while it
runs.

### D5. One tab speaks

Several pages of one origin share its storage and its key; two that each join would fence each other (ADR 0113 D4). The
page that holds the exclusive Web Lock of the node's name is the node; the others are given what it publishes and hand
it their events, over a message channel of the origin. A page that ends releases the lock and another takes it, restores
from the store and joins if it must. The lock is a convenience of one storage bucket and no fence, and a page may not
have noticed that it lost it. So what must be one is in the store, and each is changed in one transaction of it, which
the store orders across pages: the nonce a join is made with, drawn by whichever page writes it first, so that two that
join are offered the same incarnation; the incarnation, written only where the store holds none and waits on the join
the offer answers, so that a page that did not notice does not bring back one that ended; the number of a request; and
the request, before its page is told. Then two pages that both believe they are the node lose nothing but time, and the
incarnation is the fence.

### D6. What was accepted is kept, as far as the browser keeps it

An event the page accepted is in the outbox (ADR 0109 D10) before the page is told so: an IndexedDB transaction with
durability `strict` completed. The standard calls `strict` a hint to flush, and a bucket is *best-effort* until it is
made *persistent*, after which it is not cleared without the person's consent. So:

- the node asks for persistence and publishes which it has;
- **the promise** is that a reload, a closed page or a crashed one loses no accepted event while the bucket stands;
- a bucket that was cleared is found by the absence of the key and the incarnation: the node joins as a new instance,
  and the document is told that the outcome of what it had waited for is unknown, as after a fence.

**One incarnation, one store.** What the node keeps is kept under the incarnation it was made in. A change of person
(ADR 0116 D1) is a new incarnation: when it is confirmed, the events, answers and published state kept under the one
before are deleted, in the transaction that records the new one, before anything of the new one is shown. The other
pages are told, and drop what they were shown. The offer that waits for a proof, and the challenge computed from it,
survive the redirect to an identity provider and back, since they are in the store. A browser gives no promise that what
was deleted is gone from its medium, and the record says no more than that a page is not shown it.

A page that is frozen is a process that stopped. Its timers are due when it runs again, as after a restore.

### D7. The surface is generated, and is not a backend

From the document a page uses: the type of what it publishes, the type of each event it takes, a function that raises
one, and a subscription to the published snapshot, as a JavaScript module with a TypeScript declaration. No rule of the
document is in it. It checks at the boundary what a type cannot hold at run time (a page is also written in JavaScript),
by the same schema, and refuses there what the machine would refuse. A wrong event is a compile error for a page in
TypeScript, held by a test that must not compile.

### D8. What is the application's

The pages, the routes, the framework, the content security policy, how a module is served and held to its digest, and a
service worker if there is one. SCE names what the node needs of them: a secure context, one origin, and a module served
whole.

### D9. Same origin

The binding in a browser is to the origin that served the page. Another origin is a question of its own (a preflight for
each signed exchange, who may read), and is refused where the deployment is built.

### D10. The protocol is a model before it is code

`docs/formal/BrowserNode.tla` is the client that is several pages of one store: a lock that moves while its holder goes
on, a page that stops between any two steps, a store that is cleared, a person who changes, a server that forgets. It
refines `AtMostOnceService`, so a page that stops and a lock that moves end no request unanswered, and `Ends` says that
every request a document was told is accepted ends. `docs/formal/Freshness.tla` is D4, with a thief who signs ahead.
Checked by TLC on 2026-11-23, before any code of the browser shell: one request, 268,075 states; two requests, 216,143;
`Ends`, 66,226; the freshness step, 3,482; no violation.

| Taken out | What TLC finds |
|---|---|
| the nonce of a join in the store | a lock that moved, and an outcome lost |
| an offer taken only for the join the store waits on | an answer to another request |
| the number of a request in the store | an answer to another request |
| the store before the page is told | a request accepted and kept nowhere |
| the fence | a request that takes effect twice |
| a cleared store told | a request that never ends |
| the deletion with a change of person | what was one person's shown to another |
| the freshness step | a thief served long after they left |
| the present value in a refusal | a page whose request is never taken |

**What TLC found** that this record did not have: the second row. With two requests, a page that had not noticed held an
offer of an incarnation that was confirmed, and then fenced, by another page; it wrote that incarnation again and
numbered from one, and a late answer to the first request was taken for the second's. **What was seen while writing the
model**, and is shown necessary by taking it out: the first and third rows, where the record said that two pages that
both believe they hold the lock lose nothing but time, which holds only with both.

## Not in scope

Push to a page that is not running, WebTransport and WebSocket from a page (`wss` exists and its browser shell is its
own record), several origins, a shared worker as the node, other wasm hosts, and any one application's pages.

## Consequences

- The client core has one form, with no thread in it; a native consumer sees the same port.
- What proofs and traces ADR 0115 asks of the core hold of the browser too, since it is the same core.
- A page written against the surface is told of a wrong event before it runs.
- A browser without the algorithm is refused, and the deployment must know it.

## Alternatives rejected

- **A browser client of its own, in TypeScript.** A second core, and a second set of rules.
- **A blocking adapter in a worker.** It waits where the standard lets a worker wait, splits the node across two
  memories, and leaves the core as it is.
- **A software key when the platform has none.** It is extractable, and the refusal is the honest answer.
- **A lock as the fence.** It is per storage bucket, and a page may be stopped while it holds it.
- **The last exchange's nonce as freshness.** The server would hold state for each, and two exchanges in flight would
  race.

## Order of work

Each step lands with a test that fails without it.

1. The core separated from the native shell, with no change of behaviour; its trace and its first Verus contracts (ADR
   0115 D4, D13). Landed 2026-11-23: the core (`https/client_core.rs`), whose inputs and effects are the trace, and the
   proof of its numbers (`https/client_mailbox.rs`). Needs nothing of ADR 0116.
2. (2026-11-23) This record's model (D10), checked, with its failing configurations.
3. The freshness step (D4), server and native client. Landed 2026-11-28: the server's values and their key
   (`https/freshness.rs`), the server's refusal and the value every answer gives (`https/server.rs`), the value as one
   more component of each signature that carries it (`https/signature.rs`), the client core's input (a refusal that
   gives a value) and effect (the request signed again at once, or after its backoff) in `https/client_core.rs`, and
   the rules over their numbers proved by Verus (`https/fresh_step.rs`, ADR 0115 D13). deploy.yaml's `fresh_ms`
   reaches the peer table; the server reads it where it serves an open machine's binding, which is ADR 0113 step 5.
4. The browser shell: `fetch`, the key, the store, the lock; the kit in Node and two engines. Landed 2026-11-28: the
   module's half of the shell (`https/browser.rs`, feature `browser`, built as `sce-rust-browser-node`), which steps
   the core and makes the signature base the page's key signs (`signature::RequestKey`, the native signer's own
   code); the page's half (`backends/rust/browser-node/js/node.mjs`): `fetch` in mode `same-origin`, the key of D3
   in IndexedDB, the outbox of D6 in IndexedDB with durability `strict`, the Web Lock and channel of D5, the page's
   timers; and the kit (`backends/rust/browser-node/kit`) against the binding's own server, under Node and in
   pinned headless Chrome and Firefox, in the lane `wasm32-scenarios`. Decided in the landing: a browser node's
   binding is `receipt: durable` and its credential the `key` authenticator's, refused otherwise when the client is
   made, since what the page accepted is released only by the server's `200` that carries it (D6, ADR 0109 D10);
   and what of D5 must be one in the store is, in this step, the outbox and its numbers, the store's. The nonce of a
   join, the incarnation and the number of a request are rows of the store the client's join adds (ADR 0113 step 5),
   which the core of this step does not make. The engines' versions the kit runs in are the pins of
   `scripts/install_chrome.sh` and `scripts/install_firefox.sh`, which is where Ed25519 in WebCrypto is measured.
5. The generated surface, with the test that must not compile. Landed 2026-11-28: `sce-codegen surface`
   (`sce-build/src/page_surface.rs`, the templates `tools/codegen/templates/surface`) writes, from a
   `datamodel="sce-static"` document, `<name>.mjs` and its declaration `<name>.d.mts`. Decided in the landing:
   the events are those the machine takes from outside, the platform's own left out, each typed by the payload
   schema its schema unit declares (ADR 0087, section 1) and checked at run time by that very text, so the module
   and the machine's reader of a payload hold one language; every value is in that wire form (ADR 0086, section
   3.1), a 64-bit integer the text of its digits; what it publishes is the snapshot a host of every backend reads
   (every active state, the published variables, `truncated`); and the machine is reached through a port,
   `MachinePort` (`raise(name, data)` with the JSON text of the payload, `subscribe(listener)` with the JSON text of
   each snapshot), which the page's node gives and the surface does not implement, since it is not a backend. The
   test that must not compile is `backends/rust/browser-node/surface/pages/wrong.mts`: under the TypeScript
   `scripts/install_typescript.sh` pins (5.9.3, the last line of the compiler written in TypeScript, one archive
   that runs on the lane's Node), each line it marks wrong is an error and no other line is, beside `right.mts`,
   which compiles; Node runs the module's refusals of what a page in JavaScript hands it; both in the lane
   `wasm32-scenarios`. The documents named in "Amends" moved in the same landing: `SCE_MESH.md` §18 and §22.6, ADR
   0115, and `docs/SCE_ACCEPTED_SUBSET.md` section 2.15.

Steps 3 and 4 need ADR 0113 step 4.

## Open questions

Decided with the acceptance, each as recommended and with its reason.

1. `fresh_ms` is required of every `key` binding of an open machine, since a native client is as able to be read.
2. The surface carries its run-time checks always; they are the schema's and small.
3. The engines of the kit are Chromium and Firefox; WebKit when a host can run it.

## Sources

Checked in the text on 2026-11-22: the Web Cryptography API Level 2 (Ed25519, `extractable`, and the remark on a key
shared with another origin); the Fetch standard (the forbidden request-headers); the Storage standard (a bucket's modes
and what clears it); Web Locks (exclusive locks, their scope, and a crashed page's); Indexed Database API 3.0 (the
durability hint and its note); RFC 9449 (sections 2, 8, 11.2 and 11.4); `draft-ietf-oauth-browser-based-apps-25`
(section 5, and the sentence that nothing prevents a hijacked client). None was read through. **Not verified:** which
browser versions ship Ed25519 in WebCrypto, how each engine throttles a page that is not seen, and how long it keeps a
best-effort bucket; each is to be measured in step 4 and not assumed.
