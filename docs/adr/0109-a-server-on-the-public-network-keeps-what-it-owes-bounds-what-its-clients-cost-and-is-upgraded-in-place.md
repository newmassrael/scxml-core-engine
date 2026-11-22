# ADR 0109 — A server on the public network keeps what it owes, bounds what its clients cost, and is upgraded in place

- Status: Accepted (2026-11-19, by the owner: "start", with its open questions as recommended). Written under the owner's standing rule of 2026-10-26 and 2026-10-27 (the most
  long-term correct structure, textbook in its foundations, cost ignored; decided, not asked), from a comparison of
  the parts ADR 0107 built with a mature server API whose clients run on machines their operators control, and from
  requests of a downstream project. Every decision is a general facility of SCE; none of it is the logic of any one
  application. The owner set its scope the same day (D0): new facilities are built for Rust, the reference and the
  server's host core, and for C++, the client side; every other backend refuses by name what it does not serve.
  "Order of work" lists the steps, each with the test that holds it.
- Landed: steps 1 to 15 of "Order of work", D1 to D17, on Rust and C++ as D0 scopes them, each in the landing that
  added the "Amended by: ADR 0109" lines of the documents it moved:
  - the Rust host core: `sce-rust-mesh` (D1, D2, D9, D10, D15, D16), `sce-rust-session-store` (the stores of D3, D7,
    D10, D13, D14, D15; layout 7), `sce-rust-crypto` (D3, D5 to D9, D14), `sce-rust-host-features` (D3 to D7, D13,
    D14, D16, D17), `sce-rust-host-http` (D4) and `sce-rust-metrics` (D16);
  - the C++ side: the reference host of `sce/include/host` (D3 to D8, D13, D14, D16, D17) and the generated
    `TransportRouter`'s `https` arm (D9, D16);
  - D1's window bound on every Mesh core: the Rust, Kotlin and Swift host cores and the generated C++ router;
  - the generated machines of Rust and C++: the restore's upgrade that applies a migration (D11;
    `saved_state::upgrade`, `upgradeSavedState`), the `<Machine>Deployment` reader (D12) and `<Machine>Limits` (D17);
  - `sce-build`: the deploy parser's keys (D1, D2, D9, D10, D12, D15), the migration and the shape file (D11), the
    deployment value (D12), the enum field of a typed request (step 4) and the named bounds' relations (D17);
  - the documents: `docs/SCE_SERVER_DEPLOY.md` and `docs/SCE_HOST_FEATURES.md` section 9 brought to all of it (step
    15).
  D8's re-seal passes of the outboxes and the audit log (`Sealed::reseal_outboxes`, `Sealed::reseal_audit`) landed
  after step 15, so that every row a sealed store holds has one. D9's record-level half (`sce:payload` on the four
  COSE operations, Rust and C++) landed after ADR 0101's steps 4a and 4b, as its "Landed" note under D9 says, and
  Open question 9 (a `<log>` honours `sce:sensitivity`, on every lane) as its own note says. Not landed: step 16
  (D18, on measured need); an in-place upgrade under load in the server suite ("Conformance"); Open questions 5, 7
  and 8; and the later backend steps of D0. Where a landing differs from the text of a decision, an "Amended on
  landing" note under it says what landed and why.
- Amended by: ADR 0114 (D8, D10): a sealed store seals the event journal, with a re-seal pass of its own
  (`Sealed::reseal_journal`), and the write that carries a session's outbox appends carries its facts too
  (`Write::journal`); `sce-rust-session-store`'s layout is 8.
- Date: 2026-11-19
- Amends, once accepted (each amended document gains an "Amended by: ADR 0109" line in the landing that moves it):
  - `SCE_MESH.md` §10.5 (the windows an endpoint keeps are bounded by its links, D1), §10.11 (`persistence:` and
    `receipt:` are read, the `https` acknowledgement is the durable receipt of that binding, and checkpoints are
    committed in groups, D10, D15), §14.4 "Keyed session pool" (bounds, a session that is never kept, the orderly
    stop, D1, D10), §16.7 (rows 9 and 15 under a durable outbox, D10; a new row for a response that is not
    authenticated, D9), §22.3 (the outbox, D10), §22.4 (rate and connection limits, D1, D2), §22.5 (the credential
    store and the `key` authenticator, D3, D9), §22.8 (the host contract delivers per client, D15), and a new §22.9
    for message signatures (D9);
  - `docs/SCE_HOST_FEATURES.md` section 3 (a durable reference store, D13), section 5 (the destination policy of a
    server's transport and its failure code, D4; why `method` stays a text, step 4 of "Order of work"), section 9
    (what this record builds leaves the list), section 10 (keys generated, exported and named stably, and named
    bounds, D7, D17), and new sections for random bytes (D5), keyed digests (D6), keys (D7; `key_use` an enum, step 4),
    credentials (D3) and the audit log (D14);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 "Saving and restoring" (a declared migration, D11) and a new paragraph of
    §2.15 for a value the deployment supplies (D12); §2.8 (the deploy keys D1, D2, D9, D10 and D12 add); §2.12
    (an enum field in a typed host-run request, read by its declared name, step 4 of "Order of work"); the
    appendix (every diagnostic code this record adds, each with its row);
  - `SCE_WIRE_CONTRACTS.md`: the surfaces this record adds, each `pre-release` (the shape file of D11, the sealed
    row of D8, the audit entry and its signed checkpoint of D14, the signature profile of D9);
  - `docs/SCE_SERVER_DEPLOY.md` sections 0, 2 to 6, 8 and 9.
  - Amended on landing: the landings moved more than this list names, each with its "Amended by: ADR 0109" line.
    `docs/SCE_SERVER_DEPLOY.md` gained sections 1 (what the program supplies, D3 to D17), 2.4 (deployment values,
    D12), 3.1 (group commit, D15), 3.2 (the key ring, D7, D8), 4.1 and 4.2 (the stop, D10; the upgrade, D11), 10 (the
    metrics and health checks, D16) and 11 (the named bounds, D17). `docs/SCE_HOST_FEATURES.md` sections 7 and 8 name
    the new operations' lanes, scenarios, stores, metrics and bounds (D3 to D7, D13, D14, D16, D17). New documents
    fix the bytes of two surfaces, `docs/SCE_SEALED_RECORD.md` (D8) and `docs/SCE_AUDIT_LOG.md` (D14), and
    `docs/SCE_CODEC_CONSUMER_MIGRATION.md` has a section per landing that changed a consumer's code.
- Scope:
  - `sce-rust-mesh` (the `https` server and client, the keyed pool, the endpoint's windows, the checkpoint's commit),
    `sce-rust-session-store` (the contracts and SQLite implementations of every durable facility here),
    `sce-rust-host-features` (the new host operations and their reference host);
  - the C++ side: the generated `TransportRouter`'s `https` arm (message signatures, the `key` authenticator), the
    C++ reference host of the new host operations on libsodium and SQLite, and the libcurl adapter of
    `SCE::Transport` with its destination policy;
  - `stdlib/host` (the new request and result records), `sce-build` (the migration and the deployment value: grammar,
    checks, the plans the runtimes read; the standard-feature table; the deploy parser's new keys);
  - every backend for D0's two exceptions only.
- Builds on: ADR 0107 (the parts this record hardens), ADR 0045 (checkpoints and durable receipt), ADR 0044 (schema
  revisions in a saved state), ADR 0057 §7 (a state of another configuration), ADR 0101 (CBOR records; its step 4
  orders D9's record-level half), ADR 0062 (the bounded log D14 is not), ADR 0084 (one inbox per session), ADR 0108
  D4 (a limit set at run time is a value of the machine).

## Context

ADR 0107 built what a server is made of: a keyed pool of sessions kept in a session store under a lease and a fence,
a checkpoint per session, a reply cache that answers a repeated request with its first reply, COSE operations, and a
Mesh binding over HTTP that a client on an unknown network can use. That is enough to answer requests correctly while
the process runs and the clients behave. A server on the public network runs for years, is upgraded many times, is
reached by clients that do not behave, and is operated by people who must see what it does. What it then needs and
what exists, checked on origin/main `69afab97d7c`:

- **What the server sends a client lives in the server's memory, and is dropped when it does not fit.** The mailboxes
  are a `HashMap<String, Client>` inside `HttpsServer` (`backends/rust/mesh/src/https/server.rs`, `Mailboxes`).
  `MailboxTransport::transmit` refuses an envelope once a client's entries reach `mailbox`, as
  `TransportFailure::queue_full`, which the core raises as §16.7 row 9 (`backpressureDrop`). `HttpsServer::new` draws
  a new incarnation (`draw_epoch`), so a restart starts every mailbox empty, and what waited in one is lost
  (`docs/SCE_SERVER_DEPLOY.md` section 4). Durable receipt exists in the core (`Router::send_durably`,
  `Router::receive_durably` in `router.rs`) and is called only by the tests of `checkpoint.rs`: `Machine::router`
  (`peers.rs`) applies no receipt, the deploy parser refuses `persistence:` and `receipt:` as unknown keys
  (SCE_MESH.md §10.11, "Status"), and a pool's bindings take `receipt: transport` (§14.4, "What a pool does not
  keep").
- **No orderly stop.** `HttpsServer::serve` accepts until its listener fails and has no stop; no call of `Pool` saves
  every session it holds as a clean checkpoint, so every process stop restores its live sessions with `RESTORED`
  (`docs/SCE_SERVER_DEPLOY.md` section 4).
- **A saved session cannot move to a new version of its document.** A saved state is bound to the document's shape,
  a SHA-256 over every state with its kind and parent, every history, invoke and variable with its type and bound
  (`static_lowering::saved_shape` in `sce-build/src/forge/static_lowering.rs`); a state, history or variable renamed,
  re-typed, re-parented or re-bounded refuses the restore, which `docs/SCE_ACCEPTED_SUBSET.md` §2.15 leaves "the
  author's to avoid until a document can declare a migration". Two growths are already accepted: a record schema or
  an enum that grows by revision (ADR 0044; `saved_state::upgrade` in `backends/rust/runtime/src/saved_state.rs`
  and its counterparts in Kotlin, Go, Python and `sce/include/saved/SavedState.h`), and `T` widened to `optional<T>`.
  ADR 0057 §7 refuses a state of another configuration: "Carrying a state across configurations is a migration,
  which a restore does not do."
- **A deployment cannot hand a machine a typed value.** A bound moved by `--limit name=N` is a number fixed at build
  time; a value read at start reaches a machine only as the data of an event or of an `<invoke>` param (ADR 0108 D4).
  deploy.yaml is read with `deny_unknown_fields` (§2.8) and has no block for a machine's values.
- **No document can draw a random value, compute a keyed digest or compare a secret.** `stdlib/host` has no
  random-bytes or MAC record; the standard library's only digest is `stdlib/hash/murmur3_32.scxml`. The runtimes read
  randomness only to draw a root's session id (§2.15), and the COSE seal draws its nonce inside the host
  (`SystemNonces` in `backends/rust/host-features/src/cose.rs`).
- **The host's durable state is in memory.** The key-value store is a `BTreeMap`
  (`backends/rust/host-features/src/kv.rs`), and so is the table of first results a repeated `sce-host-kv-put-if` is answered from
  (`docs/SCE_HOST_FEATURES.md` section 9). The COSE keys are `[u8; 32]` arrays in `BTreeMap`s (`Keys`, `CoseKeys` in
  `cose.rs`); the five operations (`cose::OPERATIONS`) neither generate a key nor export a public one; a derived key
  is named `derived:<n>`, numbered per process, so a `derived:<n>` a saved session holds names no key, or another
  key, after a restore, and the Rust host zeroes none of its keys (section 9, "A key store").
- **Credentials are the program's, in memory.** `PinStore` (`server.rs`) has `pinned` and `pin` and no way to unpin;
  `MemoryPins` is its only implementation, so a restart lets the first client to post under a name own it.
  `BearerCheck` is a closure the program writes: SCE keeps no token, expiry or revocation. The pins are compared in
  constant time (`same_digest`); what a `BearerCheck` compares is the program's.
- **A client can make the server's tables grow without bound.** `server.rs` makes a `Client` entry for every name
  the authenticator admits and for every peer the machine sends to (`clients.entry(..).or_insert_with(Client::new)`),
  and removes none; the `install` authenticator pins any new name it is shown. `inbound.rs` keys the §10.5 window by
  the envelope's own `source` (`senders.entry(source)`), so every source an envelope claims makes a window, released
  only by `Router::remove_peer`. `pool.rs`'s `post` makes a session slot for every key, its inbox a `VecDeque` with no
  bound, the pool holds no bound on its live sessions, and a key the store holds no session for starts the machine
  (`SessionFactory::start`) and is written at its first checkpoint. `serve` and `serve_connection` set no TLS
  handshake, header or body timeout and no limit on connections.
- **The edge limits concurrency, not rate.** `ServerSettings` has `max_exchanges` (`503`) and
  `max_exchanges_per_client` (`429`). A client may make exchanges, and fail authentication, as fast as it can.
- **Outbound HTTP has no destination policy.** The HTTP feature is "the semantics over a transport the embedding
  supplies" (`backends/rust/host-features/src/http.rs`); section 5 leaves trust and proxies to the host and section 9
  records that Rust has no network transport for it. Nothing refuses a loopback, private or link-local destination.
- **Nothing is encrypted at rest.** `SqliteStore` writes a checkpoint as plain JSON in `sessions.state` and its key as
  `sessions.session TEXT PRIMARY KEY` (`LAYOUT` in `backends/rust/session-store/src/sqlite.rs`;
  `docs/SCE_SERVER_DEPLOY.md` section 3, "Not encrypted"); §10.11 asks the host to encrypt.
- **A client cannot tell its server's message from an intermediary's.** It believes the operating system's
  authorities, an inspecting proxy's included, and "a server key pin is not a key of this version of the binding"
  (§22.6). The bearer token and the installation secret travel in `Authorization`, which such a proxy reads. A record
  signed by name (`sce:payload`, ADR 0107 Open question 1) is not read by the build (`docs/SCE_HOST_FEATURES.md`
  section 9), and needs ADR 0101's step 4, which has landed on no backend (`cbor_codec::lowers_record_list` is still
  in `sce-build/src/forge/cbor_codec.rs`).
- **The signing and sealing bounds are fixed:** a payload of 4096 bytes, a plaintext of 4352
  (`PAYLOAD_MAX`, `PLAINTEXT_MAX` in `cose.rs`; `bytes[4096]` in `stdlib/host/cose_sign1_request.scxml`).
- **No durable record of what happened.** ADR 0062's log is bounded and forgets its oldest entry; the host features
  have no append-only log.
- **Every input costs one synchronous write, one at a time.** The pool checkpoints after every input that took an
  envelope in or ran a macrostep (§14.4); `SqliteStore` is one connection behind a `Mutex`, with `synchronous=FULL`;
  the server holds one `telling` lock (`tokio::sync::Mutex<()>` in `server.rs`) across the host's `deliver` of every
  exchange that carried events, so the server's throughput is that of one durable write at a time.
- **The program sees nothing but errors.** The mesh, store and host-feature crates carry no metrics; the program reads
  `Pool::take_errors`, `Pool::is_live`, `Pool::next_wake_ms` and the store by SQL (`docs/SCE_SERVER_DEPLOY.md`
  section 9).
- **One machine per store.** WAL needs a local file system, the mailboxes are in one process's memory, and routing a
  key to the process that holds its lease is not built (ADR 0107 Open question 2; `docs/SCE_SERVER_DEPLOY.md`
  section 6).

Two related matters are decided elsewhere and are not reopened here. **An envelope's `source` is bound to the
authenticated link**: a fix in progress on the branch `mesh-source-bound-to-link`, not yet on main; D1 builds on it.
**A trust set rotates keys by `kid`**: a key id an object carries selects the signer of that id alone
(`cose::verify1`, `docs/SCE_HOST_FEATURES.md` section 10, "Verify"); D7 and D9 use it as it is.

## The textbook structure

**Transactional outbox, at-least-once delivery, idempotent consumer.** A service that changes its state and sends a
message writes the message into an outbox table in the same transaction as the state, and a relay delivers it and
deletes it on acknowledgement (C. Richardson, "Transactional outbox"; the same pattern as a database's change log
read by its replicas). Delivery is then at least once, and the consumer makes the effect once by remembering what it
took in (the idempotent consumer). SCE already has the second half (§10.5, the reply cache) and the hold until
durable (§10.11); the outbox is the half it lacks.

**Backpressure, not loss.** A bounded queue between a producer and a consumer either slows the producer or drops; a
system that promises delivery may only do the first, and tells the producer at the point it would otherwise drop
(the Reactive Streams rule; §10.11's durable receipt already refuses an admit past its window rather than drop an
accepted envelope). Load shedding at the edge (`429` of RFC 6585 section 4, `503` of RFC 9110 section
15.6.4, each with `Retry-After`) is the same rule applied to what has not been accepted yet.

**An orderly stop drains.** A server told to stop stops accepting, finishes what it accepted, makes its state durable,
and exits; the next start then has nothing to recover. Erlang/OTP's application stop and the drain of every
load-balanced service follow it.

**Schema evolution is a reader's schema meeting a writer's.** Avro resolves data written under a writer's schema into
a reader's schema by a stated set of rules (fields matched by name, a default for a field the writer lacked, a field
the reader lacks skipped, numeric promotion), and refuses what the rules do not cover; it needs the writer's schema
at read time. Erlang/OTP upgrades a running state machine's data in place with `code_change/3` under a release's
`appup`, step by step from version to version. Database migrations (Flyway, Rails) are an ordered, immutable chain
from each released version to the next. All three are total over what they map and refuse what they do not.

**Configuration is typed input, checked before the program serves.** A value the deployment supplies is read once,
validated against a declared type, and refused at start, not discovered wrong at the first request that reads it
(fail fast; the Twelve-Factor App's "config in the environment", with the type check it leaves out).

**Randomness is an input.** A deterministic machine is a function of its inputs; a random value is one of them, as
the time is, and comes from the operating system's cryptographic generator (RFC 4086; `getrandom(2)`,
`BCryptGenRandom`, libsodium's `randombytes_buf`).

**A keyed digest is HMAC, and a secret is compared in constant time where it is held.** HMAC-SHA-256 (RFC 2104,
FIPS 198-1, with RFC 4231's test vectors) authenticates a message under a key; its verification compares the tag in
time that does not depend on where it differs (libsodium's `crypto_auth_verify`, RustCrypto's `Mac::verify_slice`).
Generated code in seven languages cannot promise that for an ordinary comparison, so the comparison stays in the code
that holds the key.

**Keys have a lifecycle.** NIST SP 800-57 Part 1: a key has one use (key separation), is generated by an approved
generator, is protected at rest by a key-encryption key (envelope encryption, the structure of every key management
service), is zeroised when it leaves memory, and is retired before it is destroyed. A public key travels as a
structure that names its algorithm and its id: COSE_Key (RFC 9052 section 7) for SCE's COSE objects, its thumbprint
by RFC 9679.

**Data at rest is sealed by an AEAD bound to where it is stored.** An authenticated cipher with associated data (RFC
5116) keeps a row confidential and detects its change; binding the row's identity into the associated data detects a
row moved to another key. A random nonce is safe only below a stated number of messages per key: 2^32 for a 96-bit
nonce (NIST SP 800-38D section 8.3), which a server that writes a checkpoint per input reaches; XChaCha20-Poly1305's
192-bit nonce removes the bound. A key that is itself a secret or a personal identifier is indexed by its keyed
digest (pseudonymisation).

**A random token is stored as its digest.** OWASP's guidance on secrets: a high-entropy random token is stored as a
fast cryptographic digest, since there is no dictionary to guess it from; a slow password hash (Argon2) is for values
people choose. Tokens carry a prefix so that a scanner finds one leaked in a log.

**Rate is limited by a token bucket, in the generic cell rate algorithm's form.** GCRA (ITU-T I.371) holds a token
bucket's whole state in one time per key, and answers a refusal with the exact wait. SCE has it as a standard
algorithm already (`stdlib/rate/gcra_admit.scxml`).

**Server-side request forgery is prevented by checking the address that is connected to.** OWASP's SSRF prevention:
an allow-list of destinations; every address a name resolves to checked against the special-purpose ranges (RFC
6890: loopback, private, link-local, shared, documentation, multicast, and their IPv4-in-IPv6 forms); the connection
made to the address that was checked, never to a second resolution (which a rebinding name defeats); no redirect
followed; bounded time.

**Message security survives intermediaries that transport security does not.** HTTP Message Signatures (RFC 9421)
sign named components of a request or a response, a response's signature may cover components of its request, and
Digest Fields (RFC 9530) bind the body. OSCORE (RFC 8613) is the same idea for CoAP with COSE; DPoP (RFC 9449) is
proof of possession of a key in place of a bearer value an intermediary can copy.

**A durable log is append-only and tamper-evident.** Each entry chains the digest of the one before it; a signed
checkpoint over the chain's head lets a reader prove an edit or a truncation (Crosby and Wallach, "Efficient Data
Structures for Tamper-Evident Logging"; Certificate Transparency's signed tree heads, RFC 9162). Retention deletes
whole segments and keeps the checkpoint that anchors the rest.

**Group commit.** A database writes the commits of many transactions in one log flush and answers each after it
(PostgreSQL's `commit_delay`, InnoDB's binary-log group commit). The durability of each is unchanged; the cost of a
flush is shared.

**Observability through a port.** A component reports named counters, gauges and histograms to an interface its host
implements; the exposition format (OpenMetrics) and the collector are the host's. The RED method (rate, errors,
duration) and the USE method (utilisation, saturation, errors) say which to report.

## Decision

### D0. New facilities are built for Rust and C++; every other backend refuses them by name until a consumer needs them

Every facility this record adds is built for **Rust**, the reference and the server's host core, and for **C++**, the
client side (the generated `TransportRouter`'s `https` arm and the C++ reference host). Kotlin, Go, Python, Swift and
C11 get a facility when a consumer needs it, each as a step of its own, and until then **refuse a document that asks
for it by name**, never silently: `generate/unsupported-feature` naming the facility, or the facility's own code where
one exists (`codegen/standard-host-type-not-served` for a host operation, ADR 0107 D8;
`mesh/codegen-keyed-pool-unsupported` for the pool).

Refusal by name is already the repository's rule for a form a backend does not lower (ADR 0101 section 5, item 4),
so a backend that lacks a facility is a stated gap, not a wrong program. A later backend step is mechanical: the
printers read plans the build makes once (the migration plan of D11, the deployment value's reader of D12, the record
schemas of every host operation), and the shared corpora (`tests/host_features/scenarios/`, the saved-state fixtures
of `sce-build/tests/fixtures/static_datamodel/saved/`) hold a new backend to the same bytes the first two pass.

Two exceptions land on every backend that has the surface or the path:

1. **A change to a surface every backend already shares** (a saved-state format, a wire format several backends write
   byte for byte) lands on each backend that has the surface, as ADR 0107 D4's saved-state format 4 did. None of this
   record's decisions changes such a surface: D11 adds a step before the restore and leaves the saved state as it is,
   and D9 signs what the `https` binding already carries.
2. **A security fix lands on every backend that has the affected path.** D1's bound on the §10.5 windows is one: the
   window keyed by a source an envelope claims exists in every Mesh core (`inbound.rs`, the generated C++
   `DedupRouter`, the Kotlin and Swift host cores), and the bound lands in each, with the source binding it builds on.

### D1. Every table a client can make grow has a bound, and a key the server has never seen costs it nothing durable

- **The §10.5 windows are as many as the links.** With `source` bound to the authenticated link (decided elsewhere),
  an endpoint keeps one window per link name its authenticator admitted, and an envelope whose `source` is not its
  link's is refused before a window is made for it. Every Mesh core (D0, exception 2).
- **A client entry of the `https` server is removed when it holds nothing.** An entry with an empty mailbox, no
  exchange in flight and not ready (absent past `absence_ms`, §22.4) is removed; under D10 the outbox is in the store
  and the entry is a cache of it. A new accepting-side key `max_clients` (default 100000) bounds the entries in memory;
  an exchange for a name past it is answered `503` with `Retry-After`.
- **The connection is bounded in time and number.** New `ServerSettings`: `handshake_timeout` (default 10 s, the TLS
  handshake), `header_timeout` (10 s, the request's head), `body_timeout` (the whole body: 30 s), `max_connections`
  (default 20000) and `max_connections_per_address` (default 64); a connection past either is closed at accept. A
  response carries at most `max_response_bytes` of the mailbox (an accepting-side key, default `max_body_bytes`), the
  rest at the next exchange, as §22.3's numbering already allows.
- **The pool has a bound on what it holds in memory.** `sessions:` gains `max_live` (sessions in memory) and
  `max_inbox` (inputs waiting in one session's inbox). At `max_live`, the session idle longest that holds nothing
  pending is saved clean and dropped to make room; when none can be, a request for a key not in memory is answered
  `RpcStatus::Unavailable` and its envelope's receipt is lost, so the `https` exchange is answered `503` and the
  client carries it again. An input past `max_inbox` is shed the same way. No new `RpcStatus` is added (see
  "Alternatives rejected").
- **A session is kept only once its document keeps it.** A session started for a key the store holds nothing for is
  provisional until its first run ends: if the machine reached a top-level final state, the session is never written,
  and what the run sent leaves at once, since nothing durable changed (§14.4 already removes a finished session and
  releases what its last input sent once the removal answered; a session never written has nothing to remove). A
  document refuses a key it does not know by answering and finishing, at the cost of one run in memory, which D2's
  rate bounds; the store is written only for keys a document chose to keep.
- **Wire, API and spec:** the accepting-side keys `max_clients` and `max_response_bytes` (§22's table, §2.8);
  `sessions: { max_live, max_inbox }` (§14.4; both optional, defaults 100000 and 64); `ServerSettings`' new fields;
  §10.5 and §14.4 amended.
- **Tested by:** an endpoint fed envelopes claiming ten thousand sources over one link keeps one window and refuses
  the rest (every Mesh core); a server shown ten thousand names that each make one exchange and go away holds none of
  them after `absence_ms`; a slow-header and a slow-body client are closed at their bounds; a pool at `max_live`
  evicts the longest-idle quiet session and answers `Unavailable` when none is quiet; a request for an unknown key to
  a document that answers and finishes leaves the store unwritten.

### D2. The edge limits rate, and failed authentications, by the standard library's GCRA

- **One rule.** The server judges every limit with `sce:std/rate/gcra_admit`, generated into `sce-rust-mesh` as the
  §10.5 rules already are (`crate::generated::dedup_admit`), so a document's rate limit and the edge's are one
  algorithm.
- **Four limiters**, each `{ interval_ms, burst }`, under an accepting-side key `rate:`:
  - `address`: every exchange, keyed by the remote address before authentication (an IPv4 address, an IPv6 /64);
  - `client`: every exchange that authenticated, keyed by the client name;
  - `failed_auth`: each `401`, keyed both by the name and by the address; while either is refused, an exchange of that
    key is answered `429` without running the authenticator, so a guess costs the guesser the rate and the server no
    store read;
  - `new_names`: each name the `install` or `key` authenticator pins for the first time, keyed by the address.
- **The answer** is `429` with `Retry-After` the limiter's wait rounded up to a second (§22.4's "for this client").
- **The remote address behind a proxy.** Only a connection from an address of the accepting-side key
  `trusted_proxies` (a list of CIDR blocks, default empty) has its client address read from `Forwarded` (RFC 7239),
  the rightmost address not itself trusted.
- **Bounded state.** Each limiter keeps at most `rate.max_tracked` keys (default 100000); a key whose arrival time is
  at or before now is a fresh limiter and is dropped first, then the oldest. A restart starts every limiter fresh.
- **Defaults** (on unless the deployment writes `rate: none`): `address` 100 per second, burst 200; `client` 20 per
  second, burst 40; `failed_auth` 10 per minute, burst 10; `new_names` 20 per hour, burst 20 (Open question 2).
- **Tested by:** each limiter's refusal and `Retry-After` against the GCRA's own cases; a `401` storm from one
  address stops reaching the authenticator; `Forwarded` read only from a trusted proxy; the tracked table at its
  bound.

### D3. Credentials are kept in a durable store, revocable, digested at rest, compared in constant time

- **The store.** A credential contract in `sce-rust-session-store`, with a memory and a SQLite implementation and a
  conformance kit: pins (a name to the SHA-256 of an installation secret, or to a public key under D9's `key`
  authenticator), and tokens. `PinStore` gains `unpin(name)`; the durable implementation is `SqliteStore`, in the
  session store's file (layout 3), as D7's key store is, and `MemoryCredentials` the one in memory. The store keeps
  rows and compares no secret: `pin` answers the digest a name is pinned to, which its caller compares.
- **Tokens.** `StoredTokens` is the reference `BearerCheck`: a token is `sce1.` followed by the base64url of 32
  bytes; the store keeps its SHA-256 with the name, the moment it was issued, its expiry and whether it is revoked; a
  presented token is digested, looked up by its digest, and its name compared in constant time; an expired or revoked
  one is `401`.
- **A document decides who gets one.** Two host operations, so that the authority is the document's and the storage
  the host's:

  | Operation | `type` | Request | Result |
  |---|---|---|---|
  | Issue | `sce-host-credential-issue` | `name` (256), `ttl_s` uint32 (at least 1) | `token` string (64) |
  | Revoke | `sce-host-credential-revoke` | `name` (256) | `revoked` uint32 (the tokens and pin removed) |

  A token is the HMAC-SHA-256, under the host's issuing key (D7), of the invocation's identity (ADR 0107 D4) and the
  name, the deterministic CBOR array `[session id, invoke id, start, name]`, so a repeat under one identity gives the
  same token without the host keeping it in the clear, and a token is bound to its name by the tag itself. The
  issuing key is a MAC key of the key ring (`issuing:credentials`), drawn at the store's first issue. A revoked token
  is kept as revoked, so a repeated issue cannot make it live again. The token passes through
  the document once, to be sent to its client, and so through that session's checkpoint and the client's outbox: a
  deployment that issues tokens this way stores under D8.
- **One constant-time comparison.** Every comparison of a secret or a digest in SCE's code calls one helper per
  language (`subtle::ConstantTimeEq` in Rust, `sodium_memcmp` in C++); `same_digest` becomes a call of it, and so
  does the relay's own `equal`. A token's text is written and read in constant time too (`base64ct`,
  `sodium_bin2base64`).
- **Spec:** §22.5 and a new section "Credentials" of `docs/SCE_HOST_FEATURES.md`; `docs/SCE_SERVER_DEPLOY.md`
  section 5.
- **Tested by:** the conformance kit on both implementations (pin, unpin, a second secret refused, expiry, revocation,
  a process restart that keeps them); a token issued twice under one identity is one token; scenarios of the two
  operations on the Rust and C++ reference hosts.

### D4. Outbound HTTP from a server checks where it goes

- **A network transport for section 5** in Rust, on the hyper and rustls stack the `https` binding's adapter already
  uses, and the libcurl adapter of `SCE::Transport` in C++ (the debt ADR 0107's Consequences recorded), each built
  with a `DestinationPolicy` and with no constructor that lacks one.
- **The policy:** an allow-list of host patterns and ports, or `public_internet()` (any public address, `https`
  only); every address a name resolves to checked against RFC 6890's special-purpose ranges (unspecified, loopback,
  RFC 1918, shared `100.64.0.0/10`, link-local `169.254.0.0/16` and `fe80::/10`, unique local `fc00::/7`, multicast,
  broadcast, documentation, and the IPv4-mapped, IPv4-compatible, 6to4 and NAT64 forms by the address they embed);
  the name resolved for each request and the connection made to the address checked, so a name that rebinds between
  a check and a connect cannot pass; TLS verified against the name; no redirect followed (section 5 already); a
  configured proxy reached as itself, the target still checked against the allow-list; connect, TLS, header and total
  timeouts.
- **A new failure code** of section 5, `destination_refused`: the name resolved only to refused addresses, or the
  target is not on the allow-list. A URL that is no URL stays a refusal (`invalid_argument`).
- **Tested by:** a matrix of literal and resolved addresses in every refused range, both lanes; a resolver that
  answers a public address and then a loopback one connects to neither the second nor by a second resolution; a
  redirect to a private address is answered as the `3xx` it is and not followed.

### D5. A document draws random bytes from its host

| Operation | `type` | Request | Result |
|---|---|---|---|
| Draw | `sce-host-random-bytes` | `length` uint16 (1 to the bound) | `bytes` bytes (`[random_bytes=256]`) |

From the operating system's generator; a test replaces the source as `NonceSource` replaces the seal's. Failure
`unavailable`; refusal `invalid_argument` for a length of 0 or past the bound. Not idempotent: an invocation a restore
starts again draws again, which no one can tell, since a completion the machine took is in its saved state and one it
did not take was seen by nothing. It is a host operation and not an expression because a machine is a function of
its inputs (scenarios, replay, the judge) and a random value is an input, as the clock of section 2 is.
**Tested by:** scenarios with a fixed source on Rust and C++; two draws from the system source differ.

### D6. A keyed digest and its verification are host operations, and a document compares no secret

| Operation | `type` | Request | Result |
|---|---|---|---|
| Tag | `sce-host-mac` | `key_id` (64), `message` bytes (`[mac_message=4096]`) | `tag` bytes (32) |
| Verify | `sce-host-mac-verify` | `key_id` (64), `message` bytes, `tag` bytes (32) | `valid` bool |

HMAC-SHA-256, the whole 32-byte tag, under a key of use `mac` (D7). A tag that does not verify is an answer (`valid`
false), as a signature that does not verify is in section 10. The comparison is the host's, in constant time.
A document holds no constant-time equality of its own (Open question 4): where it would compare a secret, it asks the
host that holds the key to verify. An unkeyed digest is a pure function and belongs in the standard library, not
here. **Tested by:** RFC 4231's test cases 1 to 4 and 6 to 7 (case 5 truncates the tag, which this operation does
not) on Rust and C++; a scenario that tags in one host and verifies in the other.

### D7. Keys have a lifecycle in a durable key store: generated, exported as public, sealed at rest, zeroised, named stably

- **The store.** A key contract in `sce-rust-session-store` with a SQLite implementation: each key has an id, one
  use (`sign`, `seal`, `mac`, `derive`), its algorithm, its material sealed under a key-encryption key, its creation,
  and a state (`active`, `retired`). The key-encryption key is never in the store: the program supplies it through a
  `KeyWrap` port (wrap, unwrap), whose reference adapter reads a 32-byte key from a file or a variable the operator
  names, and which a key management service implements as well.
- **Operations:**

  | Operation | `type` | Request | Result |
  |---|---|---|---|
  | Generate | `sce-host-key-generate` | `key_use` (8): `sign`, `seal`, `mac` or `derive`; `label` (64) | `key_id` (64) |
  | Export | `sce-host-key-public` | `key_id` (64) | `cose_key` bytes (128): a COSE_Key (RFC 9052 section 7), `kty` OKP, `crv` Ed25519, `x`, `kid` |
  | Retire | `sce-host-key-retire` | `key_id` (64) | `retired` bool |

  The use is named `key_use`, which every target language can spell as a field (`use` is a keyword of Rust). Until
  step 4 of "Order of work" lands, it is a text of at most 8 bytes that the host judges: any other text is refused
  (`invalid_argument`, the refusal of docs/SCE_HOST_FEATURES.md section 1: `error.execution`, and nothing is
  generated), as an HTTP request's `method` is. The build refuses a host-run record with an enum field
  (`typed_invoke::validate`, SCE Accepted Subset §2.12), so the closed set is a value the host checks when the
  invocation starts rather than a type the build checks; step 4 makes it the enum D7 means, the same set.

  A generated key's id is drawn at random (D5's source) and is the same in every process that shares the store. A
  repeat of a generation under one identity answers the first id (the store keeps the identity in the generation's
  transaction). A retired key verifies and opens, and no longer signs, seals or tags; destroying one is the operator's.
- **Derived keys are named by what derived them and kept.** `sce-host-key-derive` answers `derived:` and the first 16
  bytes, in hexadecimal, of the HMAC-SHA-256 under the store's naming key of the input (the input key's id, or the
  SHA-256 of the given secret), the salt and the info, and keeps the key in the store under that id. The same
  material derived twice is one id, and a `derived:` id in a restored session names the same key in any process
  that shares the store. Section 10's "Deriving the same material twice gives two ids" changes accordingly.
- **Key separation.** A key generated by this store has one use; a `mac` key never seals. The configured keys of
  `CoseKeys` keep their meaning (a symmetric key seals, opens and derives), so no existing configuration changes.
- **In memory.** Key material is held in a zeroising type (`zeroize::Zeroizing` in Rust, `sodium_memzero` on release
  in C++) and is never printed by a `Debug` or a log.
- **Tested by:** the store's conformance kit (generate, export, retire, the material unreadable without the
  key-encryption key, a restart that keeps them, one id for one derivation); an exported COSE_Key that verifies a
  signature of its key in the other lane; a key-encryption key rotated by re-wrapping every key.

### D8. Everything a server stores is sealed at rest, and a key that is itself a secret is stored as its keyed digest

- **A sealing store wraps any store.** `Sealed<S>` implements each store contract of this record and ADR 0107 over
  another implementation of it: every value is sealed with XChaCha20-Poly1305 under a 192-bit random nonce (RustCrypto
  `XChaCha20Poly1305`, which `chacha20poly1305` already carries; libsodium's
  `crypto_aead_xchacha20poly1305_ietf` in C++), the associated data the store's name and the row's indexed key, so a
  row moved to another key does not open. A sealed value is a framed record (`SCE-SEALED-1`, the sealing key's id, the
  nonce, the ciphertext with its tag), a wire surface of its own with test vectors.
- **Rotation.** Each row names its sealing key; new writes take the active one; a re-seal pass rewrites rows sealed
  under a retired key through the ordinary versioned and fenced write.
- **What it does not give.** Freshness against someone who can write the medium: a row replaced by an older sealing
  of the same row opens. The lease's fence and the version stop a process that is wrong, not a medium that is
  written by hand; the runbook says so.
- **A keyed index.** Under `Sealed`, every indexed key is stored as its HMAC-SHA-256 under the store's index key,
  and the key itself is inside the sealed value. `due_before` answers digests; the pool loads by digest and reads the
  session's key from the opened checkpoint. A key that is a personal identifier or a bearer value is then never in
  the clear on the medium.
- **Spec:** §10.11's "A host stores the checkpoint encrypted at rest" names the facility; `docs/SCE_SERVER_DEPLOY.md`
  sections 3, 8 (the key-encryption key is backed up apart from the store) and 9 (the SQL reads show digests; the
  program has `Sealed::digest_of(key)`).
- **Tested by:** the session store's conformance kit run over `Sealed<MemoryStore>` and `Sealed<SqliteStore>`; a row
  copied to another key, a flipped byte and a wrong key each refused; a file read without the key holds none of the
  saved values; the C++ and Rust sealings of the vectors agree.

### D9. A client verifies what its server sent, across any intermediary

- **Responses are signed.** An accepting-side key `sign: <key_id>` (a `sign` key of the server's host, D7) and a
  dialing-side key `verify: <trust_set>` (a trust set of the client's host, rotated by `kid` as trust sets are) turn
  on HTTP Message Signatures (RFC 9421) with Ed25519 (its `ed25519` algorithm) for every `200`. The signature covers
  `@status`, `content-digest` (RFC 9530, `sha-256` of the body), `sce-mesh-epoch` and `sce-mesh-first`, and from the
  request (`;req`) `@method`, `@target-uri`, `sce-mesh-ack` and `sce-mesh-nonce`, a new request header of 16 random
  bytes in base64url the client draws for each exchange. A response therefore answers one request, and an old one
  replayed to a new request does not verify. The numbering headers are covered because a forged `Sce-Mesh-First`
  would make the client acknowledge envelopes it never took in, which the server would then delete.
- **A client under `verify:` takes nothing of a response that does not verify.** It acknowledges nothing from it,
  raises a new §16.7 row 19, `responseUnauthenticated` (`transport`, `target`), once per edge, and retries on its
  backoff. The check is in the binding's semantics layer, so every adapter, SCE's or an embedding's, gets it.
- **Requests are signed by a key the client holds: the `key` authenticator.** The client's host generates an Ed25519
  key pair once (D7) and signs each request (RFC 9421: `@method`, `@target-uri`, `content-digest`, `sce-mesh-ack`,
  `sce-mesh-epoch`, `sce-mesh-hold-ms`, `sce-mesh-nonce`, with `created`), `keyid` the key's COSE thumbprint (RFC
  9679). The server pins a name to the public key at its first `200`, as `install` pins a secret, or takes a key the
  host registered (D3's store), and refuses a `created` further than `max_skew_ms` (default 300000) from its clock.
  Unlike a bearer token or an installation secret, nothing an inspecting proxy reads lets it act as the client; a
  replayed request carries envelopes the §10.5 window drops and an acknowledgement that drops nothing new.
- **What it does not give.** Confidentiality from a proxy the client's operator trusts: it reads the exchange. A
  document that must keep a value from it seals the value to a key the proxy lacks (`sce-host-cose-encrypt0`).
- **A record signed by name**, kept or forwarded beyond the exchange, is ADR 0107 Open question 1's `sce:payload`, as
  decided there. It lands after ADR 0101's steps 4a (Rust) and 4b (C++), with the build's writer of a record in that
  form and its reader. Until then a document signs bytes it holds.

  Landed (2026-11-21), Rust and C++ only (D0): `sce:payload` on the four COSE operations. The build writes a
  description of the record's type once (`sce_build::forge::payload_record::describe`), each record's fields in the
  order their keys encode, and each lane's record codec is the one map between the record's JSON, as a machine
  writes and lifts a payload, and its deterministic CBOR (`sce_rust_runtime::record_cbor`, the runtime's feature
  `record-cbor`; `common/RecordCbor.h`): the machine encodes the record where a signing or a seal starts, and reads
  the bytes a verification or an opening answers as the record where the completion is lifted, into the result's
  record the build derives (`<result alias>Of<invoke>`, its byte string an `optional<record:...>`). Bytes that are
  no value of the record are the operation's failure `not_a_record`. The record's largest CBOR is held to
  `cose_payload` or `cose_plaintext` at the parse (`validation/payload-record-schema`). The C++ reference host's
  trust sets name public keys it holds no seed of (`CoseFeature::setTrustedKeys`), so a client verifies a record its
  server signed offline, with the server's public key alone. Every other lane refuses the machine by name. Held by
  `tests/host_features/payload_record_cross_language.json` on both lanes, by the record steps of `static_host_cose`'s
  scenario, and by the build's tests in `sce-build/tests/the_standard_host_features_are_one_set.rs`.
- **Lands before it:** D7 (the key pairs and the export a client's trust set is made from), D3 (the store the `key`
  authenticator pins in). The binding's signature code calls the COSE and Ed25519 primitives the host features
  already have; they move to a crate both `sce-rust-mesh` and `sce-rust-host-features` depend on, so there is one.
- **Spec:** a new §22.9, "Message signatures"; §22.5 gains `key`; §16.7 row 19; the signature profile registered in
  `SCE_WIRE_CONTRACTS.md`.
- **Tested by:** RFC 9421's Ed25519 example (Appendix B.2.6) in Rust and C++; a conformance-kit item that runs the
  exchange through the inspecting proxy, which alters `Sce-Mesh-First`, the body, and replays an earlier response,
  each refused by the client; a request signed by the C++ client verified by the Rust server.

### D10. What a server sends a client is kept until the client acknowledges it, across restarts, and a full outbox pushes back

- **An outbox in the store, written with the checkpoint.** Under `receipt: durable` on an `https` accepting binding,
  what a session releases to a client is appended to that client's outbox by the same store transaction that writes
  the session's checkpoint: the store contract's `put` takes the outbox appends with the checkpoint. Either both are
  durable or neither; once they are, the endpoint holds nothing for them, the session can be quiet and dropped, and
  the server's in-memory mailbox, now a cache of the outbox, wakes a held exchange.
- **Numbering survives.** A client's outbox row keeps its incarnation and its next number; a restart keeps both, which
  §22.3 already allows ("one that restores its mailboxes keeps theirs").
- **The acknowledgement is the binding's receipt.** `Sce-Mesh-Ack` deletes the outbox entries it covers, in the store.
  On the dialing side under `receipt: durable` with `persistence: checkpoint`, the client names in `Sce-Mesh-Ack`
  only numbers a durable checkpoint of its own holds, and keeps an envelope it sent in its checkpoint until a `200`
  carried it. That is §10.11's rule with the binding's header and status as the acknowledgement: no `DurableReceipt`
  envelope (pattern 22) is sent on this binding.
- **A full outbox refuses the send, and drops nothing accepted.** `mailbox` becomes the durable window of §10.11 for
  the client: a `<send>` admitted while the client's outbox and the appends not yet committed hold `mailbox`
  envelopes is refused at admit as row 9, as §10.11 already says of a durable binding ("a send that was never accepted
  is told, and nothing accepted is dropped"). The pool counts each client's entries across its sessions, reserving at
  admit and releasing at acknowledgement.
- **An outbox of a client that never returns expires.** An accepting-side key `retain_ms` (default 30 days, Open
  question 6): an entry older than it is deleted, reported to the host (`PoolError::OutboxExpired`) and raised as row
  15 in the session that sent it, at its next run, through a per-session note written in the expiry's transaction.
- **The deploy parser reads `persistence:` and `receipt:`** with the refusals §10.11 lists, each with its code, and a
  pool's bindings take the receipt their deployment declares (§14.4's "does not read `receipt:`" is removed).
- **An orderly stop.** `HttpsServer::stop(deadline)` stops accepting, answers every new exchange `503` with
  `Retry-After`, and answers every held exchange at once with what its mailbox holds. `Pool::stop(deadline)` stops
  admitting, runs what each inbox holds to completion, waits for the writes in flight, saves every session in memory
  as a clean checkpoint (its endpoint closed to admits first, §10.11's definition of clean), releases its leases and
  answers a report of the sessions saved and of those not saved with the reason. A session not saved by the deadline
  restores with `RESTORED`, as today. Catching a signal is the program's; the runbook shows it.
- **Tested by:** extending ADR 0107 D10's suite: the server killed (`SIGKILL`) at random points of commit,
  acknowledgement and expiry, with clients absent for a while: after each restart every envelope a client was sent is
  delivered to it at least once and taken in once, and none lost; a send to a client whose outbox is full is refused
  as row 9 and nothing accepted is missing; a stop under load leaves only clean checkpoints and a start raises no
  `RESTORED`; the parser's refusals each by code.
- **Amended on landing.** The server's stop takes how long it waits, not a moment: `HttpsServer::stop(within:
  Duration) -> bool`, asynchronous, answers whether every exchange in flight ended within it. The server runs on an
  asynchronous runtime, whose timers count a duration, and the program gives the pool the moment
  (`Pool::stop(deadline: Instant) -> StopReport`) once the server's stop answered, so the two together stay within
  what the service manager allows (`docs/SCE_SERVER_DEPLOY.md` section 4.1).

### D11. A saved session is restored into a new version of its document by a migration the document declares

- **The writer's shape is a file.** The build writes, beside a machine, its shape file: the text
  `static_lowering::saved_shape` hashes (`sce-saved-state-shape 1` and its lines). An author commits the shape of each
  released version (`shapes/<version>.shape`); it is the writer's schema of every state that version saved. A shape
  file is a wire surface of its own, `pre-release`.
- **A migration is a step from one shape to the next**, declared in the document:

  ```xml
  <sce:migration from="shapes/2.shape" to="shapes/3.shape">
    <sce:rename kind="state" from="waiting" to="pending"/>
    <sce:rename kind="data" from="count" to="attempts"/>
    <sce:map-state from="legacy" to="idle"/>
    <sce:drop-data id="scratch"/>
    <sce:add-data id="limit" expr="16"/>
    <sce:convert data="due" algorithm="due_from_seconds"/>
    <sce:drop-invoke id="poller"/>
  </sce:migration>
  ```

  `rename` covers states, histories, variables, invokes and the events a waiting send names; `map-state` moves a
  configuration that holds a removed state to a state of the new document; `drop-data` discards a removed variable;
  `add-data` gives an added variable its value, a constant (in the last step the `<data>` initialiser serves when it
  is a constant); `convert` names a pure algorithm (`sce:kind="algorithm"`) from the old value's type to the new;
  `drop-invoke` lets an invocation the new document no longer runs end without a start. A widening needs no element:
  an integer to a wider one of the same sign or an unsigned one to a wider signed one, `float32` to `float64`, a
  larger bound of a text, a byte string, a list or a map, `T` to `optional<T>` (already accepted), and an enum or a
  record schema to a later revision (ADR 0044, which the step leaves to `upgrade`).
- **The build checks each step is total** against its two shape files: every old state, history, variable and invoke
  is kept (same name, kind, parent and a type it widens to), renamed, mapped or dropped; every new variable is old,
  renamed or added; the steps form one chain whose last `to` is the current document's shape. New codes, each with its
  appendix row: `validation/migration-not-total`, `validation/migration-type-change` (neither a widening nor
  converted), `validation/migration-shape-unreadable`, `validation/migration-chain-broken`.
- **Where it runs.** The build emits the chain as a plan of the machine (the renames, the maps, the constant values,
  the conversion calls); each runtime's `upgrade` applies it to a saved state whose shape is a step's `from`, before
  it judges the shape, and then restores as it does today, refusing a configuration the mapping made invalid with the
  existing configuration rule. A restored machine saves under its new shape at its next save. Rust and C++ (D0);
  every other backend refuses a document with `<sce:migration>` by name.
- **A server is upgraded in place**: stop it (D10), start the new build on the same store, and each session migrates
  when it is loaded. A deployment that runs the old and the new build at once is not supported: the old build refuses
  a state the new one saved, by its shape.
- **Not migrated here:** a state of another configuration of a product line (ADR 0057 §7 stays; Open question 5);
  the Mesh endpoint sections of a checkpoint, which are keyed by the deployment's machine names, not by the document.
- **Tested by:** fixtures `static_migration_1`, `_2` and `_3` with saved instances of the first two restored into the
  third, a running invoke dropped and a delayed send whose event was renamed delivered under its new name, on Rust and
  C++; each refusal code; a chain with a gap refused; a mutation casefile that drops a rename from the plan.
- **Amended on landing.** The shape file is written by a command of its own, `sce-codegen shape <document>` (`-o`
  writes it, `--host-invoker` as `generate` takes it), and not beside every generated machine: an author writes it
  once per release, when the version is cut, and a file written at every build would be one nobody commits. The
  fixtures are `static_migration_1` and `static_migration_2`, with their shape files under `shapes/`: one step holds
  every kind of element, and a state of the first version restored into the second is the test
  (`a_saved_state_is_migrated_into_the_next_version` on Rust, `ASavedStateIsMigratedIntoTheNextVersionTest` on C++).
  A `convert` names the algorithm by the alias its document imports it under, and on Rust and C++ converts a
  fixed-width number or a `bool`; another type is refused by name. The in-place upgrade under load of "Conformance"
  is not built: no test of `tests/server` stops a loaded server and starts the next build on its store.

### D12. A machine declares the configuration it reads, and the deployment supplies it, checked

- **The declaration.** `<data id="limits" sce:type="record:ServerLimits" sce:from="deployment"/>` declares a value of
  any value type (records, lists, ADR 0108's maps, enums, optional values) that the deployment supplies. It is
  read-only: an `<assign>`, `<sce:append>` or map write to it is refused at build time
  (`validation/deployment-value-assigned`).
- **The value** is written in deploy.yaml under the machine, `config: { limits: { ... } }`, and checked when the
  deployment is read (`mesh/deploy-config-missing`, `mesh/deploy-config-unknown`, `mesh/deploy-config-invalid`, by
  the type's payload schema), or in a file the machine entry names (`config_file:`), read by the host at start through
  the generated reader of the type with the lift's refusals (ADR 0087 section 2.3). A value that does not read
  refuses the machine's start and tells the host why, as ADR 0096 does for a machine that cannot start.
- **It is part of the machine's interface, not of its saved state.** The build's manifest lists each deployment
  value with its type, which deploy validation reads. A saved state holds no value of it, so a restore takes the
  deployment's current value and a changed configuration takes effect at the next start; for that reason it is not a
  line of the saved-state shape, which judges only what a state holds (Open question 3).
- **Not for secrets.** A deployment value is in the machine's memory and in reach of `<log>`; keys and credentials
  stay in the host, named by id.
- **Lanes:** Rust and C++ (D0); every other backend refuses `sce:from="deployment"` by name.
- **Tested by:** a machine reading a record and a list from deploy.yaml and from a file, on Rust and C++; each
  refusal code; a file with a value of the wrong type refusing the start with the field named.

### D13. The key-value feature has a durable reference store, with its first results in the same transaction

A key-value contract in `sce-rust-session-store` (get, put, put-if, delete, next-key, the quota) with a memory and a
SQLite implementation, and the table of first results by invocation identity written in the same transaction as the
effect it records, so a put-if repeated after a crash is answered with its first result whether or not the crash came
after the write. The rows of a session's identities are deleted in the transaction that removes the session from the
store. The Rust and C++ reference hosts take a store at construction, memory by default as today. Section 3's "Left
to the host: durability" becomes: a host states whether its store is durable, and the reference durable store is
this one. **Tested by:** the contract's conformance kit; section 3's scenarios run against the SQLite store; a
put-if killed between its write and its completion, answered with its first result after the restart.

### D14. A durable audit log: append-only, chained, redacted by the schema, kept for a stated time

| Operation | `type` | Request | Result |
|---|---|---|---|
| Append | `sce-host-audit-append` | `stream` (64), `entry`: a record of the schema `sce:entry` names | `sequence` string (20) |

- **The entry is typed.** `<invoke type="sce-host-audit-append" sce:entry="SignInRecord">` names a record schema, as
  `sce:answer` names one for `sce-host-model-generate`; the build writes the record as its JSON payload, the form
  every lane already writes for an event, so this needs no CBOR record form.
- **Redaction is declared once, on the schema.** A field may carry `sce:sensitivity="secret"` (never written; the
  entry holds a marker in its place) or `"personal"` (written as its HMAC-SHA-256 under the log's pseudonymisation
  key, so equal values still correlate; a stream may keep personal values in the clear with `personal: clear`).
- **Storage.** Append-only rows `(stream, sequence, at, entry, chain)`, `chain` the SHA-256 of the previous chain and
  the entry; every `checkpoint_every` entries, and at a stop, a `COSE_Sign1` over the stream, the sequence and the chain
  under the host's audit key (D7). A reader verifies from any signed checkpoint forward.
- **Retention.** Per stream `retain_days`; entries older are deleted by whole segments, each ending at a signed
  checkpoint that then anchors what remains.
- **Durable before it answers**, and a repeat under one identity answers the first sequence and appends nothing.
- It is not ADR 0062's log, which is bounded and forgets, nor `<log>`, which is diagnostic output.
- **Tested by:** append, verify, a changed entry and a truncation detected, retention keeping the chain verifiable,
  redaction of both marks, on Rust and C++.
- **Amended on landing.** The request record, `stdlib/host/audit_append_request.scxml`, has three fields: `stream`
  (64), `entry` a text of the named bound `audit_entry` (4096), the record's JSON payload, and `sensitivity` a text
  of the named bound `audit_marks` (1024), the schema's marks (`/password=secret;/email=personal`). The build writes
  `entry` from the variable the `<param>` names and `sensitivity` from the schema `sce:entry` names, and a document
  writes no `sensitivity`: the host redacts by marks it is given, since it reads no schema. The result is `sequence`
  string (20), as decided. A record nested in an entry is refused by the build until its own marks are written.

### D15. The checkpoints of many sessions are committed together, and nothing is released before its commit

- **A committer per store.** A session seals its checkpoint (§10.11 step 1) and hands the write to the committer,
  which writes everything handed to it since its last commit in one transaction, one savepoint per write so that a
  write whose version or fence is refused is answered refused and the others commit, syncs once, and answers each
  (§10.11 step 3 for its session). A commit starts as soon as the previous one ended; the batch is what arrived
  meanwhile, so no delay is added (an optional `commit_delay_us`, default 0, as PostgreSQL's `commit_delay`).
  D10's acknowledgement deletes join the same batches.
- **The contract** gains `put_many`, durable as a whole on return and decided item by item; the conformance kit holds
  both. It landed as `write_many`: the contract's one write is `write` (D10), of which `put` is the narrower form that
  only keeps a state, and a batch also removes sessions, appends to outboxes and deletes what acknowledgements cover.
- **The server delivers per client, not one at a time.** The `telling` lock is replaced by an order per client: what
  it protected is the order of what one client's exchanges tell the host, and exchanges of different clients reach the
  host at once. `ServerHost::deliver` is called concurrently for different clients and one call at a time per client
  (§22.8 amended); the pool already runs different keys in parallel.
- **The durability rule does not move:** a `200` only after the commit that holds its envelopes, a reply released only
  after the commit that holds it.
- **Tested by:** many clients' exchanges committed in fewer transactions than inputs, each answered only after its
  commit; a fence refused inside a batch refusing only its item; D10's kill suite; a measured throughput recorded in
  the runbook, not gated.

### D16. A server reports what it does through a metrics port

- **A port, not a library.** A `Metrics` trait in Rust and an interface in C++, with counters, gauges and histograms
  by fixed names and closed label sets; the core links no metrics library. A reference adapter renders the OpenMetrics
  text exposition for the program to serve on a route of its own.
- **The names** form a registry in `docs/SCE_SERVER_DEPLOY.md`: exchanges by status; held exchanges; sheds by reason
  (rate, concurrency, durability, a bound of D1); authentication failures by authenticator; outbox depth and expiries;
  live sessions, loads, restores (`RESTORED` among them), evictions; commit duration and batch size; lease renewals
  refused; host operation duration and failure code by type; audit appends. No label carries a client name or a
  session key.
- **Health.** `HttpsServer::health()` and `Pool::health()` (accepting, stopping, store reachable) for the program's
  readiness and liveness checks.
- **Tested by:** each counter moved by the event that names it, through a recording adapter; the exposition parsed
  by an OpenMetrics parser.

### D17. The bounds of the signing and sealing records are named, so a build moves them

The records' bounds become named bounds (`bytes[cose_payload=4096]`, `cose_signed`, `cose_plaintext`, `cose_sealed`,
`mac_message`, `random_bytes`), moved by `--limit` as every named bound is, and read by the reference hosts as the
build's constants in place of `PAYLOAD_MAX` and its neighbours. The overheads between them (a signed object 256 bytes
above its payload, a sealed one 128 above its plaintext) are relations the standard-feature table checks
(`codegen/standard-host-bound-relation`). The defaults are today's numbers, so no byte moves. A payload of megabytes
(a detached payload over a stored blob) is Open question 7. **Tested by:** a build with larger limits signing and
sealing at the new bound on Rust and C++; a build that breaks a relation refused.

### D18. A store on shared media, and a key served where its lease is held: the direction, built on measured need

Every contract of this record is written so that an implementation on a networked transactional database can serve
it, the fence checked where the data is. With D10's outbox in the store, any process can answer a client's poll; an
exchange that carries envelopes for a key another process leases is forwarded to that process over an internal
binding between the server's processes, whose address the lease row carries. None of it is built by this record's
steps until a deployment measures one active process with a standby as insufficient (ADR 0107 Open question 2 stands).

### Not in scope

The logic of any application (what a name may do, how many of something a key admits, what a record means) stays
in its documents. A server program is still each deployment's (ADR 0107). Volumetric attacks below HTTP are the
network operator's. An unkeyed digest as a pure algorithm of the standard library is a record of its own.

## Conformance

- **Contracts.** Each store contract (credentials, keys, key-value, outbox, audit, and the session store's `put_many`)
  has a conformance kit in `sce-rust-session-store`, run against its memory and SQLite implementations and under
  `Sealed`; a store written elsewhere runs the same functions.
- **Host operations.** Scenarios under `tests/host_features/scenarios/` for D3, D5, D6, D7, D13 and D14, run by the
  Rust and C++ reference hosts; a cross-language test for D6's tags, D7's exported keys and D8's sealed rows as
  `cose_cross_language.json` does for COSE; vectors: RFC 4231 (HMAC), RFC 9421 Appendix B.2.6 (signatures), RFC 9679
  (thumbprints), and D8's own.
- **The server suite** of ADR 0107 D10 (`tests/server/`), extended: D10's kills with absent clients and full
  outboxes; an in-place upgrade under load (stop, start the next build, every session migrates, D11); D1's growth
  attempts; D2's floods; D9's inspecting proxy that alters, replays and reads.
- **Deploy and build checks.** Every new code with a test that triggers it and its appendix row
  (`acceptance_doc_covers_every_code`).
- **Other backends** (D0) are held to the refusal by name: a test per facility generates a document that asks for it
  for Kotlin, Go, Python, Swift and C11 and asserts the code; D1's window bound is held on every Mesh core.

## Order of work

Security and correctness first, then durability, then operability. Every step is Rust and C++ unless it says otherwise.

1. This ADR, accepted. Documentation only.
2. D1 and D2: the bounds and the rate limits in the Rust server and pool, their deploy keys, and D1's window bound on
   every Mesh core (D0, exception 2), after `mesh-source-bound-to-link` lands.
3. D5, D6 and D7: random bytes, keyed digests and the key store, with the shared crypto crate D9 needs.
4. Enum fields in a typed host-run request: `typed_invoke::validate` admits an enum field in a request (not yet in
   a result), the Rust and C++ adapters read it by its declared name, every other lane refuses it by name;
   `sce-host-key-generate`'s `key_use` becomes an enum; SCE_ACCEPTED_SUBSET.md §2.12 amended. A general capability
   of the build, which D7's `key_use` is the first to need (step 3 lands it as a text the host judges).
   `sce-host-http-fetch`'s `method` stays a text the host judges, by D0's first exception: the operation is served
   on every lane, its request record is a surface every lane shares, and an enum field there while only Rust and
   C++ read one would take HTTP from Kotlin, Go, Python, Swift, C11 and the Interpreter. It becomes an enum only in
   a later step that gives every lane serving `sce-host-http-fetch` enum fields in a request, so that no lane loses
   the operation. What changes:
   1. `typed_invoke::validate` keeps refusing an enum field in a result, which the payload lift cannot read, and
      admits one in a request; a build for a lane other than Rust and C++ refuses a typed request with an enum field
      as `generate/unsupported-feature`, beside `standard_host::refuse_unserved` (D0).
   2. `static_datamodel::request_field_value` holds the `<param>` of an enum field to a value of that field's own
      enum, so a wrong value is refused where it is written.
   3. `host_invoker_interface`: an enum field is held at the start site to the declared name it crosses as; the
      Rust and C++ record structs type it with the machine's enum (the `StaticEnumType` list handed to
      `render_rust` and `render_cpp` at both their call sites), and each adapter reads it back by its declared
      name, which needs a name-to-variant function beside the generated `declared_name`. The `unreachable!` arms
      of the other renderers stay unreachable behind the refusal of item 1.
   4. A machine imports the enum its request's record names (`sce:std/host/key_use.scxml`, beside the record in
      `stdlib/host`, which `the_standard_host_features_are_one_set` admits as an enum a record of the table
      imports). The record is typed with the machine's own type for the enum, which the static lowering declares
      in the machine as it does for an enum a variable or a payload field holds, so no unit of the enum document
      is generated beside the machine for it, in the Rust tree or for `standard_host_scenario_test`.
   5. The records, the reference hosts (which still read the declared name), the scenarios, a build test that
      refuses a wrong value of `key_use`, docs/SCE_HOST_FEATURES.md sections 5 (why `method` stays a text) and 13
      and SCE Accepted Subset §2.12 (an "Amended by: ADR 0109" line).
5. D3: the credential store, `StoredTokens`, the two operations; the constant-time helper replacing every comparison.
6. D8: `Sealed` and the keyed index; the runbook's sections 3, 8 and 9.
7. D4: the Rust network transport and the C++ libcurl adapter with the destination policy.
8. D9: §22.9, response and request signatures in the Rust server and client and the C++ client, the `key`
   authenticator, row 19. The record-level half after ADR 0101's steps 4a and 4b.
9. D10: the parser's `persistence:` and `receipt:`, the outbox in the store, backpressure, expiry, the orderly stop.
10. D11: shape files, the migration grammar and checks, the plan, the Rust and C++ `upgrade` steps.
11. D12: deployment values in the grammar, the deploy parser, the manifest and the Rust and C++ readers.
12. D13 and D14: the durable key-value store and the audit log.
13. D15: the committer, `put_many`, the per-client delivery.
14. D16 and D17: the metrics port and the named bounds.
15. `docs/SCE_SERVER_DEPLOY.md` and `docs/SCE_HOST_FEATURES.md` section 9 brought to what landed.
16. D18, only on measured need.

Kotlin, Go, Python, Swift and C11 take each facility when a consumer needs it, a step of its own per backend and
facility, and refuse it by name until then (D0).

## Open questions

Each has a recommendation; the owner's acceptance decides them as recommended unless it says otherwise.

1. Whether the durable stores share `sce-rust-session-store` or move to a crate named for the server's stores.
   Recommended: share it, one layout version for the file (2, with layout 1 upgraded in place on open in one
   transaction); a rename moves every dependant for no change of meaning.
   Amended on landing: the stores share `sce-rust-session-store` and its one file, as recommended, but the layout
   moved once per step that added tables, not once for the record: 2 (the keys, D7), 3 (the credentials, D3), 4 (a
   token's sealed bytes, D8), 5 (the outboxes, D10), 6 (the key-value values, D13) and 7 (the audit log, D14). Each
   step landed apart, and a file a landed build wrote has to name what it holds, so a build of the next step brings
   any earlier layout to its own in one transaction on open (`LAYOUT_1` to `LAYOUT_7`, `lay_out` in
   `backends/rust/session-store/src/sqlite.rs`).
2. The default rates of D2. Recommended: the numbers D2 gives, each a deploy key, revisited with D16's measurements.
3. Whether a deployment value belongs in the saved-state shape. Recommended: no (D12): the shape judges what a saved
   state holds, and it holds no deployment value; the manifest carries the declaration for deploy validation.
4. A constant-time equality as a document expression. Recommended: no (D6); the code of seven lanes and their
   compilers cannot promise it, and a document that holds no secret needs none.
5. A migration from a state of another product-line configuration (ADR 0057 §7). Recommended: a record of its own,
   once a consumer carries a session across a change of features; a shape file would then record the features of
   the build that wrote it.
6. The default `retain_ms` of D10. Recommended: 30 days, required to be at least `absence_ms`; no unbounded default,
   since a server that runs for years would keep every vanished client's outbox.
7. Signing a payload of megabytes. Recommended: a record of its own, a detached payload (RFC 9052: a `nil`
   payload) over a blob the host stores, once a consumer measures the need.
8. Trace context (W3C Trace Context) in an envelope. Recommended: a record of its own: it is a field of the envelope,
   an ADR 0044 revision of a surface every Mesh core shares (D0, exception 1).
9. Whether `<log>` honours `sce:sensitivity`. Recommended: yes, as a later step of this record, so a mark declared for
   the audit log holds wherever the value is written. **Decided: yes, by the build, on every lane.**
   - **Where it is honoured.** Under `datamodel="sce-static"` the build knows the type of every place an expression
     reads, so a `<log expr>` that reads a marked field at any depth (a payload's, a record variable's, one a list, a
     map or a union holds), or a value that holds one, is lowered as the literal of its marker. The whole value is
     replaced, since a value computed from a marked one (its length, a text joined to it, a comparison) says
     something of it, and the stronger mark wins where both are read. The marker replaces the expression where the
     one walk every lane shares lowers it, so Rust, C++, Kotlin, Go, Python, Swift, C11, the Interpreter's lowering,
     a test component's log and the TTCN-3 export write the same text and no runtime differs. This is not a change
     to a surface the lanes share in D0's sense (exception 1): no runtime, port or wire format moves.
   - **The marked value is not computed.** SCXML leaves the form of a logged value to the processor ("the SCXML
     processor may convert this value to a convenient format before logging it", `<log>`'s attribute details), and
     the conversion writes the marker without computing the value: whether an operation on a secret fails (a length
     less one, an overflow) is itself a signal of the value, and the `error.execution` it would raise is an event the
     machine and its host observe. A log of a marked value neither writes it nor fails on it.
   - **`secret`** is written `sce-secret`, the marker the audit entry holds.
   - **`personal`** is written `sce-personal`, without a digest. The audit log keys its digest so that equal values
     correlate under a key the host's key store holds (D7), with a retention and a reader of its own; a diagnostic
     log has none of those, an unkeyed digest of an identifier is reversed by trying identifiers, and a keyed one
     would put the pseudonymisation key in every lane's log sink (ADR 0059, ADR 0060) to make a diagnostic line
     name a person by proxy. Correlation is what the audit log is for, not a log line.
   - **Where the type cannot be established.** Under `ecmascript` no value has a type: a marked value read from
     `_event.data` can be assigned to a variable, handed to a `<script>` or logged in a `<finalize>`, and the event
     an `<onentry>`'s `<log>` sees is not known before it runs. Redaction there would need every runtime to know
     the marks; instead a statechart under any data model but `sce-static` that imports an event schema or a union
     marking a field, at any depth, is refused at the `<sce:import>` (`validation/attribute-rule-violated` on its
     `src`, naming the field and the mark), as a schema that grew is refused there under the same data models.
   - **What it does not follow.** A value carried out of a marked field into a place of no mark (a variable, an
     unmarked field, a child's parameter) is no longer marked, as an audit entry's unmarked field is written as it
     stands: the mark is a property of the field, held by type.
   - **Landed** (2026-11-21): `sce-build/src/forge/sensitivity.rs` (the marks of every record a statechart's types
     reach, read in the walk that reads its type scope; the marker; the refusal), the `<log>` arms of
     `crate::forge::static_lowering` and of the TTCN-3 export; held by
     `sce-build/tests/a_log_writes_a_marked_value_as_its_marker.rs` and the module's own tests. SCE Accepted Subset
     §2.12 and §2.15 ("What a `<log>` shows") and docs/SCE_HOST_FEATURES.md section 15 amended.

## Alternatives rejected

- **A mailbox in memory saved by a periodic snapshot.** It loses what arrived between snapshots and is a second
  write beside the checkpoint that can disagree with it; the outbox is written in the checkpoint's transaction.
- **`DurableReceipt` envelopes on the `https` binding.** The binding already acknowledges by header and status; a
  second acknowledgement would be two mechanisms for one promise.
- **Dropping on a full outbox with an alert.** An accepted send lost is the loss the outbox exists to prevent.
- **A new `RpcStatus` (`ResourceExhausted`) for load.** `RpcStatus` is a closed enum of a surface every Mesh core
  decodes; a value an older client does not know is refused as malformed. `Unavailable` already tells the client to
  try again.
- **A migration written in the host's language** (an OTP `code_change` per backend). It is unchecked against the
  shapes and written once per language; the declared migration is checked by the build and planned once.
- **Running the old build until its sessions drain.** Sessions live for years.
- **A random value or a constant-time comparison as an expression.** The first makes the engine a function of
  something other than its inputs; the second promises what generated code cannot keep.
- **Bearer tokens stored under Argon2.** A slow hash on every request, for a 256-bit random value that has no
  dictionary.
- **Self-contained signed tokens (JWT) for clients.** They cannot be revoked without a list the server must keep
  anyway, and their algorithm negotiation has a record of confusion; this server holds state already.
- **Pinning the server's key in TLS.** It excludes the inspecting proxies the binding admits by design (§22.6);
  message signatures work through them.
- **Signing each envelope as a COSE object instead of the exchange.** It leaves the numbering headers unprotected, and
  a forged number makes a client acknowledge what it never took in.
- **SQLCipher for encryption at rest.** A second build of SQLite against the rule that the system's is linked, and no
  help to a store that is not SQLite; `Sealed` serves every implementation.
- **Rate limits left to a reverse proxy.** It cannot see a client's name after authentication or the outcome of the
  authenticator.
- **An allow-list without address checks.** A name on the list that resolves to a private address passes it.

## Consequences

- A server can be stopped and upgraded in place: it stops clean, the next build migrates each session when it loads.
- Under `receipt: durable`, nothing a server accepted for a client is lost by a restart, and a full outbox tells the
  document that sent instead of dropping; an outbox of a client gone longer than `retain_ms` expires, and the sender
  is told.
- A client's tables, connections, rate and failed attempts each cost the server a stated bound.
- The deploy schema gains keys (`rate:`, `sign:`, `verify:`, `retain_ms`, `max_clients`, `max_response_bytes`,
  `sessions.max_live`, `sessions.max_inbox`, `persistence:`, `receipt:`, `config:`, `config_file:`), with defaults
  that are safe for a public server.
- New host operations and their records in `stdlib/host`, served by the Rust and C++ reference hosts and refused
  elsewhere by `codegen/standard-host-type-not-served` (ADR 0107 D8).
- New wire surfaces, each `pre-release` and registered: the shape file, the sealed row, the audit entry and its
  signed checkpoint, the binding's signature profile.
- The store's layout moves to 2; a build before it refuses the file, as `docs/SCE_SERVER_DEPLOY.md` section 3 already
  says of a layout change. Amended on landing: it moved to 7, one layout per step that added tables (Open question 1);
  a build before any of them refuses a file a later one opened.
- More durable writes (the outbox, its acknowledgements, the audit log) are paid for by group commit, which the
  server's per-client delivery lets fill.
- The runbook gains the key-encryption key, the credential and key stores, the stop, the upgrade, the metrics and the
  limits.
