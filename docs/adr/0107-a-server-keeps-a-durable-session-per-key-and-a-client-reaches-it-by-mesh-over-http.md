# ADR 0107 — A server keeps a durable session per key, and a client reaches it by Mesh over HTTP

- Status: Accepted (2026-11-18). Written at the owner's order of that day, under the standing rule of 2026-10-26 and
  2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored. The owner chose the
  structure in conversation the same day: the programming model is Mesh, the transport that reaches a client on an
  unknown network is HTTP request and response, and every part is a general facility of SCE, not one shaped for the
  application that asked for it ("제너럴한 형태로 받아서 구현해야해"). On review the same day the owner had the
  binding's transport made a port with adapters that follow each platform's HTTP stack (D2), and then ordered the
  work started, which accepts this record with the recommendation of each open question. Nothing of it has landed;
  "Order of work" lists the steps, each with the test that holds it.
- Date: 2026-11-18
- Amends, once accepted (each amended document gains an "Amended by: ADR 0107" line in the landing that moves it):
  - `SCE_MESH.md` §10.5 (a duplicate of a request is answered with the reply the first one produced, D4), §10.11
    (built in the Rust host core, D5), §14 (a keyed session pool on a host core, D3), §19 (the host core stops
    refusing a `server:` block it now serves), and a new section for the `https` binding (D2);
  - `docs/SCE_HOST_FEATURES.md` section 3 (a read answers a version, and a conditional write is added, D6), section 7
    (a standard feature is served by a stated set of lanes, D8), section 9 (what this record builds leaves the list),
    and a new section for the COSE operations (D7);
  - ADR 0045's order of work, step 6 (SCE_MESH.md §10.11): it lands here, in the Rust host core, as step 6 of this
    record.
- Scope:
  - `sce-rust-mesh` (host core: keyed pool, checkpoint, durable receipt, reply cache, the `https` binding's server and
    client), the generated C++ `TransportRouter` (the `https` binding's client arm), the Kotlin and Swift host cores
    (the binding's client, in a later step);
  - a session store contract and its reference implementation (new crate beside `backends/rust/relay`);
  - `stdlib/host` (the versioned key-value records and the COSE operation records), `stdlib/cose` (new: the COSE
    structures as event schemas with a CBOR codec);
  - the reference hosts of every lane for the key-value change; the Rust and C++ reference hosts for COSE;
  - `docs/SCE_ACCEPTED_SUBSET.md` (the new diagnostic codes in its appendix), a server runbook beside
    `docs/SCE_RELAY_DEPLOY.md`;
  - the C++ parts (the `https` client arm and the COSE reference host) on Linux and on Windows with MSVC, through
    SCE's CMake and through the Bazel vendoring channel (`scripts/sce_repo.bzl`, ADR 0059), and a Windows CI lane
    that holds them (D11).

## Context

A downstream product asked SCE for what a server needs to hold an application's logic as statecharts: requests from
many clients, each handled by the session its key names; thousands of such sessions loaded when needed and kept
across restarts, their delayed `<send>`s included; a limit that holds under concurrent requests; documents the server
signs and seals and a client verifies and opens; and one source for the format of those documents on both sides. The
request is the downstream's; the design is SCE's, and nothing in it names the downstream application.

What exists, checked on origin/main `359a196cfac`:

- **No server host and no durable store.** `docs/SCE_HOST_FEATURES.md` section 9 says so: "A production server host.
  No durable store, no quota policy ...", and the reference stores are in memory.
- **The key-value feature has no conditional write.** Section 3 serves get, put, delete and next-key. Two sessions
  that read a value and write it back cannot tell that the other wrote in between.
- **Nothing carries an inbound request to a session.** Section 5 serves one outbound exchange. The host-feature
  contract adds that "a statechart reads no event a host raises on its own"; that rule is about host features and
  stays.
- **Signing and sealing exist only inside the shared list's trust layer.** `backends/rust/trust` links Ed25519,
  ChaCha20-Poly1305, X25519, HKDF and Argon2 for its own records. No statechart can ask for a signature, and the C++
  tree has no cryptography at all.
- **A saved state already keeps its timers.** Saved-state format 3 (`backends/rust/runtime/src/saved_state.rs`,
  `sce/include/saved/`) stores each waiting delayed send as the wall-clock moment it comes due, and a restore arms
  one already due as due now, in order. What is missing is a place to keep thousands of saved states and a way to
  wake a session that is not loaded when its earliest send comes due.
- **Mesh is the statechart-to-statechart channel, and has no binding a client on an unknown network can use.**
  `<invoke type="sce:mesh-rpc">` and `<send target="#peer">` (SCE_MESH.md §9.5) are a request and a reply with
  deadlines, duplicate suppression, ordering and retry in the core. Its bindings for a phone or a remote client are
  WebSocket (§18) and Zenoh (§20); both are served by host cores only, and a generated C++ machine with either is
  refused. Neither passes every intermediary a client may sit behind (see "Alternatives rejected").
- **Mesh's durability is specified and not built.** §10.11 (checkpoint, `receipt: durable`) is decided by ADR 0045
  and "lands in its order of work step 6"; `sce-rust-mesh` holds no checkpoint code.
- **A duplicate request is dropped, not answered.** §10.5 drops an envelope whose id the sender's window holds. Over
  a binding where a response can be lost after the server acted, the client's retry then gets no reply and runs into
  its deadline, although the server did the work.
- **A host core serves no server pool.** §14's `server:` block (one machine, many instances) is an arm of the
  generated C++ router; §19 lists it among what a host core refuses by name.

## Decision

### D1. The programming model is Mesh, and nothing in it is specific to HTTP

A client machine asks a server machine with `<invoke type="sce:mesh-rpc">` and tells it with `<send
target="#server">`; the server answers a request with `<send event="service.response.X">` while it processes the
request, as SCE_MESH.md §9.5 and §10.7 already define. The documents name no transport. Which binding carries them is
the deployment's, in deploy.yaml, as it is for every Mesh binding today.

SCE adds no HTTP-specific event processor. The Basic HTTP Event I/O Processor (§scxml-C-2) keeps its meaning, a
`<send>` posted as a form to a URL, and is not extended into a server.

### D2. A Mesh binding over HTTP request and response: `transport: https`

A new binding, defined on HTTP semantics (RFC 9110) and not on one version of HTTP, so that HTTP/1.1, HTTP/2 and
HTTP/3 carry it unchanged.

- **Only the client opens an exchange.** The client POSTs to `<base>/sce-mesh/1/<client>`, where `<client>` is its
  machine name and `1` the binding's version, as §18.1 spells a WebSocket path. The server never dials, because a
  client on an unknown network has no address to dial.
- **Framing.** The request body is zero or more envelopes, each its CBOR bytes (§7.5), as a CBOR sequence (RFC 8742,
  `application/cbor-seq`). The response body is the same: the envelopes the server holds for that client when the
  exchange ends. A client with nothing to send polls with an empty body.
- **What the server holds for a client.** Everything the server sends a client, replies included, goes to that
  client's mailbox, a bounded queue in arrival order. An exchange's response carries what the mailbox holds after the
  server took in the exchange's request envelopes and ran the macrosteps they caused. A request may ask to be held
  (`hold_ms`, at most the deployment's bound) and the server answers it as soon as the mailbox is not empty or the
  hold ends: long polling, so a server's send reaches a waiting client without a second channel. A full mailbox
  drops the send with §16.7 row 9 (`backpressureDrop`), as a full outbound buffer does.
- **An envelope leaves a mailbox once the client acknowledges it.** Each response numbers what it carries; the next
  request names the last number it received. What a lost response carried is carried again, and the client's §10.5
  window drops what it already took in. Delivery to the client is therefore at least once and its effect once.
- **Readiness** (§10.10). The client is ready toward the server from the first exchange answered `200` until an
  exchange fails at the transport; the server holds a client ready while the gap between its exchanges stays under
  the deployment's `absence_ms`, and a client absent longer is `TRANSPORT_UNAVAILABLE` (§16.7 row 1) once per edge.
- **Status codes.** `200` for every exchange the binding took in. `404` for a version it does not speak or a name it
  has no binding for, as §18.1. `401` for a name the authenticator did not accept. `413` for a body over the
  deployment's bound. `429` or `503` with `Retry-After` when the server sheds load; the client waits at least that
  long. Any other status, and a transport failure, is §16.7 row 2 or under the retry policy row 3.
- **Who may claim a name.** A mailbox is read only by the name that owns it, so the binding requires an
  authenticator, chosen per deployment: a bearer token the host verifies, a TLS client certificate, or a pinned
  installation secret (the client draws a 256-bit secret at first run; the server pins the name to the secret's hash
  at the first exchange and refuses any other secret for it, the trust-on-first-use rule of SSH host keys). A binding
  with none is refused at deploy validation.
- **The client takes the platform's network policy.** It uses the operating system's trust store, so a network that
  inspects TLS with its own authority is honoured where the user's machine trusts that authority, and the
  platform's proxy settings, tunnelling through an explicit proxy with `CONNECT`. A deployment may pin the server's
  key instead; pinning and inspection then exclude each other, and the client reports which refused.
- **The binding is three layers: semantics, a port, adapters** (ports and adapters; the dependency points from the
  adapter to the port, never from SCE to a network library). The semantics (framing, acknowledgement, holding,
  readiness, status codes, authentication) are SCE's and the same in every language. The port is one public contract
  that carries a single exchange at the level of HTTP semantics, the level the binding is defined on: a POST with its
  headers and body, a bound on the wait, and an answer that is a response or a failure of a named kind. Every
  language that carries the binding declares it: a C++ interface, a Rust trait, a Kotlin interface, a Swift protocol,
  as the standard host's `SCE::Transport` (`sce/include/host/StandardHost.h`) and the Zenoh binding's `ZenohClient`
  (SCE_MESH.md §20.4) already are. An adapter is any implementation of the port, SCE's or the embedding's. A program
  that has a network stack of its own implements the port on it and links no adapter of SCE's, so it brings no
  second TLS library and its own proxy and trust configuration applies.
- **One conformance kit holds every adapter.** SCE publishes, per language, a kit: a local server, an explicit
  proxy, a proxy that inspects TLS with a test authority the test installs as trusted, a response dropped after the
  server acted, a server that holds an exchange past the bound. The platform-policy rules above are requirements on
  every adapter. SCE's adapters are held to the kit in SCE's lanes; an embedding's adapter is held to it by the
  embedding's tests.
- **SCE's reference adapters use the platform's HTTP stack, and where a platform has none, its de facto system
  client.** Only a platform's stack follows its policy completely (an automatic proxy configuration script, WPAD, a
  certificate an administrator deploys); an adapter that re-implements them follows them partly. Each adapter is a
  separate, optional target, and its libraries are found, not vendored, for the reason D7 gives for libsodium. The
  binding's core links none of them.

  | Platform | Reference adapter | Lands |
  |---|---|---|
  | Windows | WinHTTP (the Windows certificate store, the system's proxy configuration with its script and WPAD) | this record, step 8 |
  | Linux | the system's libcurl (`find_package(CURL)`, a Bazel label): the distribution's CA bundle and TLS library, `HTTPS_PROXY`, `https_proxy`, `NO_PROXY`, `CONNECT`, HTTP/2 | this record, step 8 |
  | Rust, every OS | hyper over rustls with the operating system's verifier (`rustls-platform-verifier`) and the proxy variables | this record, step 8 |
  | Apple | URLSession | the step that brings the binding to the Swift host core |
  | Android | the platform's HTTP stack | the step that brings the binding to the Kotlin host core |

  cpp-httplib, which SCE vendors (`third_party/cpp-httplib`, v0.26), is not an adapter of this binding: it takes a
  proxy only as `set_proxy(host, port)` written in code, reads no proxy variable and speaks HTTP/1.1 only, so it
  cannot meet the platform-policy rule.

Implementations: the server in `sce-rust-mesh` (feature `https`); clients in `sce-rust-mesh` and as an arm of the
generated C++ `TransportRouter` that takes its adapter from the host at construction, then in the Kotlin and Swift
host cores.

### D3. A host core serves a machine as a pool of sessions, one per key

deploy.yaml may declare a machine a keyed pool on a host core: `sessions: { key_from: <field of the request's data>
| client }`. A request whose key names no session starts one from the machine's initial datamodel; a request whose
key names a stored session loads it; an idle session is saved and dropped from memory after the deployment's
`idle_ms`. Each session is a session tree of its own with its own inbox (ADR 0084), so requests for one key are taken
one at a time in arrival order and sessions of different keys run in parallel.

This is the host-core counterpart of §14's server pool, with an open set of keys instead of a declared list. A limit
that one key's requests must respect, such as how many members a key admits, is therefore a rule inside one session
and needs no lock: run to completion (§scxml-3.13) serialises its requests.

### D4. Every effect that crosses a session's boundary is keyed, and a repeat is answered with the first answer

This is the one rule under D2's acknowledgements, D5's durable receipt and D6's conditional write.

- **A duplicate request is answered, not dropped.** §10.5 is amended for an envelope of pattern `RpcRequest`: the
  responder keeps, in the sender's window entry, the reply the first copy produced, and answers a duplicate with
  that reply. A duplicate of a request still being handled is dropped, and the reply goes out once when it is made.
  Under `receipt: durable` the kept replies are part of the checkpoint (D5), so a client that retries across a server
  restart still gets the first answer.
- **A host operation started again after a restore is the same operation.** A restore starts a running invocation
  again (ADR 0045, which carries `invokestarts` and `hostinvoketoken` in the saved state so that a start after a
  restore is numbered on). An operation whose repetition is not harmless (D6's conditional write) carries the
  invocation's identity, the session key with the invoke's id and its start number, and the host answers a repeat of
  that identity with the first result. A start numbered on after a restore is a new identity only when the session's
  checkpoint recorded the first start's completion; otherwise the restore repeats the start under its old number.

### D5. SCE_MESH.md §10.11 is built in the Rust host core

What ADR 0045 decided lands as decided, on the Rust host core first:

- A machine whose deployment says `persistence: checkpoint` is checkpointed after every macrostep that changed it,
  with its endpoint state and the replies D4 keeps.
- An envelope a macrostep produced, a reply included, is released to its binding only once the checkpoint that
  produced it is durable: a client never holds an answer the server could lose.
- `receipt: durable` on the `https` binding: the server answers an exchange only after the checkpoint that took in
  its envelopes is durable, and the client keeps an envelope until an answered exchange carried it.
- A restore raises `RESTORED` (§16.7 row 17).
- **Timers of sessions that are not loaded.** The store keeps, for each saved session, the earliest `due` of its
  waiting delayed sends, written in the same transaction as the saved state. The host core keeps a timer on the least
  of them, loads the session when it comes due and lets its restore arm the send as format 3 already does. A crash
  between a send coming due and its delivery delivers it after the restart, never twice, because the delivery and the
  checkpoint that removes it from the saved state are one transaction.

### D6. A store contract for sessions, and a conditional write for values sessions share

- **The session store** is a host-side contract, not a statechart feature: get a saved state with its version, put
  one if its version is still the one read (compare and set), list sessions by earliest `due`, and take a lease on a
  key. The lease is a version-checked record of the owning process and its expiry, and its version is a fencing token
  every put carries, so two server processes can never both write one session. The reference implementation is
  SQLite in WAL mode, an embedded store whose transactions are atomic and durable, behind the contract so that another
  store can serve it.
- **The key-value feature gains a version and a conditional write.** `kv_get_result` gains `version` (a 64-bit
  integer as the text of its digits, `0` for a key never written). A new operation `sce-host-kv-put-if` (request
  `key`, `value`, `expected_version`; result `written` bool and `version`) writes only when the stored version is
  `expected_version`, `0` meaning absent, and otherwise completes with `written` false: a lost race is an answer of
  the operation, not a failure (section 1's rule). It carries D4's identity, so a repeat answers the first result. A
  store that is durable says so; the reference stores of the lanes stay in memory.

A value two sessions share (a count across keys) is guarded by `sce-host-kv-put-if`; a rule one key owns stays in
its session (D3).

### D7. Signing and sealing are COSE operations of the standard host

The objects are COSE (RFC 9052): the CBOR counterpart of JOSE, the format SCE's codecs already speak (ADR 0101's
deterministic CBOR), and a published structure for what is signed and what is authenticated, instead of a format of
SCE's own.

| Operation | `type` | Request | Result |
|---|---|---|---|
| Sign | `sce-host-cose-sign1` | `key_id`, `payload` bytes, `external_aad` bytes | `cose` bytes (a tagged `COSE_Sign1`) |
| Verify | `sce-host-cose-verify1` | `trust_set`, `cose` bytes, `external_aad` bytes | `valid` bool, `key_id`, `payload` bytes |
| Seal | `sce-host-cose-encrypt0` | `key_id`, `plaintext` bytes, `external_aad` bytes | `cose` bytes (a tagged `COSE_Encrypt0`) |
| Open | `sce-host-cose-decrypt0` | `key_id`, `cose` bytes, `external_aad` bytes | `opened` bool, `plaintext` bytes |
| Derive a key | `sce-host-key-derive` | `secret` bytes or `input_key_id`, `salt` bytes, `info` bytes | `key_id` |

- **Algorithms are fixed, not negotiated:** EdDSA on Ed25519 (RFC 9053 section 2.2), ChaCha20/Poly1305 (COSE algorithm 24) with a 96-bit nonce drawn by the host from the operating system's random source, and
  HKDF with SHA-256 (RFC 5869). An object that names another algorithm does not verify or open.
- **No key enters a statechart.** A machine names keys by id. A signing key and a trust set are the host's
  configuration; a derived key lives in the host for the life of the session that derived it and is named by the id
  the result gives.
- **A signature that does not verify and a ciphertext that does not open are answers** (`valid` false, `opened`
  false), not failures: the document decides what follows. Failures are `unavailable` and `unknown_key`.
- **Bounds** are `sce:max-size` of each bytes field, as every host record states them.
- **Reference hosts:** Rust on the RustCrypto crates `sce-rust-trust` already links; C++ on libsodium, on Linux and
  on Windows with MSVC, and through the Bazel vendoring channel as well as CMake. libsodium is found, not vendored
  (`find_package` on CMake, a label the consumer passes on Bazel), unlike `third_party/cpp-httplib`: a cryptographic
  library takes its security fixes from one place, and a program that already links one gets no second copy. The COSE structures are
  built and parsed by D9's schemas in both languages, so the two cannot disagree on the bytes.

### D8. A standard feature names the lanes that serve it

Section 7 says every host serves every standard type. That stays the goal; it stops being a precondition of adding
one. The table of standard features (`sce-build/src/forge/standard_host.rs`) gains, per feature, the lanes whose
reference host serves it, and a build for another lane refuses a machine that asks for it, naming the feature and the
lanes that serve it (a new diagnostic code, with its row in the appendix of `docs/SCE_ACCEPTED_SUBSET.md`). COSE
starts with Rust and C++; every other lane is a later step of this record, not a permanent gap.

### D9. A signed and sealed document is a record inside standard COSE schemas, and no new Forge kind is made

- `stdlib/cose` holds `COSE_Sign1` and `COSE_Encrypt0` and their header maps as event schemas with a CBOR codec
  binding (ADR 0102), their tags and the deterministic encoding of ADR 0101. Every lane's codec is generated from
  them, so a server and a client of different languages read the same bytes the same way.
- An application's document is its own record schema with the CBOR codec. The order is sign, then seal: the sealed
  plaintext is the `COSE_Sign1` of the record's bytes, so what a holder of the sealing key can read is still the
  signer's statement, and a copy taken to another holder neither opens nor passes as its own.
- How a document gets the bytes of a record to sign is Open question 1.

### D10. A suite holds durability and scale

Under `tests/server/` a scenario suite run against the Rust host core on the reference store:

- ten thousand keyed sessions, each with delayed sends at spread due times; the process killed (`SIGKILL`) at random
  points of load, checkpoint and delivery; after each restart every send is delivered exactly once, at or after its
  due and never before;
- every request a client saw answered has its effect in the restored state; every retried request receives the first
  reply (D4);
- concurrent clients against one key and against a shared value under `sce-host-kv-put-if` never exceed the limit the
  document states;
- two server processes against one store never both write a session (D6's fencing);
- the `https` binding through an explicit proxy, through a TLS-inspecting proxy whose authority the client's platform
  trusts, and with responses lost on purpose.

### D11. A Windows lane holds the C++ client parts

SCE's CMake has an MSVC branch and its headers two `_WIN32` sites, but no CI job builds on Windows, so nothing holds
a Windows build today. The Bazel channel does not build with MSVC as written: the rule `scripts/sce_repo.bzl`
generates passes `-include common/ExternTemplates.h` and `-include common/DisableStdOut.h`, which is the GCC and Clang
spelling (MSVC's is `/FI`); the rule selects the spelling by compiler. A GitHub-hosted Windows job is added: it builds with MSVC the C++ COSE reference host, the
generated C++ machine with an `https` binding and its WinHTTP transport, and the Bazel channel's smoke consumer
(`scripts/smoke_embed_consumer.sh`'s counterpart for Bazel on Windows), and runs the COSE scenarios, the
cross-language COSE test and the `https` client's tests against a local server and a local proxy. It is a lane of
`gate_registry.py` like every other, judged in CI.

### Not in scope

What a downstream keeps outside Mesh is not part of this record: a plain HTTP query an older client makes, and a page
a browser uses to activate a machine that has no network. The server program that hosts the Mesh binding may serve
other routes beside it, which are the downstream's.

## Consequences

- One way for statecharts to talk, whatever the network: a client document is the same over `https`, `wss`,
  `zenoh_inbox` or a C++ transport.
- A server application is a statechart and a deploy.yaml. Its limits, expiries and refusals are the document's; the
  host owns only sessions, storage, keys and the network.
- A server's send to a client waits for the client's next exchange or its held one. Latency to a client is bounded by
  `hold_ms`, not by a push.
- A deployment of the `https` binding must authenticate names. A public server with anonymous clients uses the
  pinned installation secret, and its application decides what a name may do from what the client proves in
  payloads.
- New diagnostic codes (the unserved standard feature, the binding without an authenticator, keyed-pool deploy
  errors) each land with their appendix row.
- The standard host's HTTP operation (SCE_HOST_FEATURES.md section 5) has the C++ adapter D2 refuses for this
  binding, `HttplibTransport`, with the same limits. Its port (`SCE::Transport`) is already public, so the repair is
  a libcurl and a WinHTTP adapter for it; it is debt recorded by this record and repaid by a record of its own.
- The WebSocket binding's client keeps its compiled-in roots; giving it D2's trust store and proxy is right and is
  left to a record of its own, so this one does not change a landed binding.

## Alternatives rejected

- **An HTTP event processor that hands a request to a session and a `<send>` back as its response.** It reaches the
  same networks, but it is a second way for statecharts to talk, with its own pairing of request and reply, beside
  Mesh's, and every application would choose between them. D1 keeps one.
- **Mesh over WebSocket only.** After the upgrade an intermediary can only pass the stream or cut it, and some cut
  it; a client with nothing to say still holds a connection; the client trusts only its compiled-in roots and refuses
  a network's inspecting authority. Kept as a binding for deployments that need a server to push at once.
- **Mesh over Zenoh (`zenoh_inbox`).** Every client joins one publish-subscribe network, so the router's access
  control is all that keeps one client from reading another's inbox; its links are TCP on a port of its own, QUIC
  over UDP, or plain WebSocket, none of which crosses an explicit proxy. Kept for networks the operator controls.
- **The shared list and its relay (ADR 0034).** Its relay decides nothing and holds no key, because its members do
  not trust it. A server here is the authority: it holds the signing key and enforces the limits. The trust models
  are opposite.
- **gRPC.** It requires HTTP/2 end to end, which inspecting proxies that speak HTTP/1.1 to the client break, and it
  brings an interface language that would duplicate the event schemas.
- **JOSE (JWS, JWE).** JSON, where SCE's codecs and the documents' bytes are CBOR; COSE is the same design for CBOR.
- **A sealed format of SCE's own over libsodium's boxes.** Not a published structure; COSE is.
- **Keeping sessions in the key-value feature.** The session store is the host's, written in the same transaction as
  the timers' index and the leases; a statechart operation cannot give that atomicity.
- **A journal of SCE's own as the reference store.** ADR 0093 journals a copy's operations for replication; a
  server's store needs transactions over several records, which an embedded database gives tested.

## Order of work

1. This ADR, accepted. Documentation only.
2. D8: per-feature lanes in the standard-feature table, and the refusal with its diagnostic code. D11: the Windows
   lane, first building what exists (the C++ runtime and the host features' reference host with MSVC), so that the
   steps after it land on a lane that already passes.
3. D9 and D7: `stdlib/cose`, the COSE operations, the Rust and C++ reference hosts (the C++ one on Linux and on
   Windows, through CMake and Bazel), scenarios in
   `tests/host_features/scenarios/`, the vectors of the COSE working group's examples for the algorithms D7 fixes,
   and a cross-language test that signs and seals in each of the two and verifies and opens in the other. Also Open
   question 1's mechanism.
4. D6, the key-value half: `version` and `sce-host-kv-put-if` in every lane's reference host and scenarios.
5. D6, the store half: the session store contract and its SQLite implementation, with the lease and fencing tests.
6. D4 and D5: the reply cache of §10.5, checkpoint, release after durability, `RESTORED`, the timers' index, in the
   Rust host core. SCE_MESH.md §10.5 and §10.11 amended.
7. D3: the keyed pool in the Rust host core, its deploy schema and refusals. SCE_MESH.md §14 and §19 amended.
8. D2: the `https` binding section of SCE_MESH.md with the transport contract, the conformance kit, the Rust server
   and client, the generated C++ client arm, the port and kit in C++ and Rust, and the WinHTTP, libcurl and Rust
   reference adapters, held on the Linux lanes and the Windows lane, with a test adapter outside SCE's run through the
   same kit.
9. D10: the suite.
10. `docs/SCE_HOST_FEATURES.md` section 9 updated, and a server runbook beside `docs/SCE_RELAY_DEPLOY.md`:
    `docs/SCE_SERVER_DEPLOY.md`, for a program built from the parts steps 5 to 8 landed (its deploy keys, the session
    store and a restart, a standby under the lease and the fence, the clients' trust and proxies, backups, and a stuck
    session told from an idle one).
11. The COSE operations and the `https` client on the remaining lanes, one step per lane. Amended 2026-11-19 by the
    owner (and ADR 0109 D0): each such step lands when a consumer of that language needs it; until then the lane
    refuses the operation or the binding by name (`codegen/standard-host-type-not-served`, the binding's refusal).

Steps 3 and 4 do not depend on the Mesh steps and may land in parallel with them.

## Open questions

Decided as recommended by the owner's acceptance (2026-11-18); each is kept here with its reason.

1. How a document gets the bytes of a record it signs or seals, and a record from bytes it verified or opened.
   Recommended: the typed form `sce-host-model-generate` already uses for its answer schema. `<invoke
   type="sce-host-cose-sign1" sce:payload="DeviceFile">` names a record schema; the build encodes the named field
   with the schema's codec at the invoke site, and `sce:payload` on verify and open decodes the result's payload into
   that record, refusing bytes that are no value of it. The alternative, a pure `encode` and `decode` per codec that a
   document calls, puts bytes in the author's hands for no gain.
2. Whether a server is ever more than one process per store. Recommended: D6's lease and fencing make it safe from
   the first step, and routing a key to the process that holds its lease is left until a measured load needs it; one
   process with a standby is the deployment until then.
3. Whether the `https` binding's server should also offer HTTP/3. Recommended: not as part of the binding. It is
   defined on HTTP semantics, so a server may advertise HTTP/3 (`Alt-Svc`) when its stack serves it, and a client
   that cannot reach UDP keeps HTTP/2 or HTTP/1.1.
