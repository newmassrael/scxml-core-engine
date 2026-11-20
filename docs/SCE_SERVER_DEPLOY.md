# Running an SCE server

A server, in the sense of `docs/adr/0107-a-server-keeps-a-durable-session-per-key-and-a-client-reaches-it-by-mesh-over-http.md`,
is a statechart served as a pool of sessions, one per key, kept across restarts in a session store, and reached by
clients over the Mesh `https` binding. This document is the runbook for the operator of such a server: what to
configure, what a restart and a standby process do, what the clients need of their network, what to back up, and how
to tell a session that is stuck from one that is idle. Every key name and default below is taken from the code or
from SCE_MESH.md, and each says where.

Amended by ADR 0109 (`docs/adr/0109-a-server-on-the-public-network-keeps-what-it-owes-bounds-what-its-clients-cost-and-is-upgraded-in-place.md`),
decisions D1 and D2: what a client may cost the server (section 2); D8: the store sealed at rest, its keys and what an
SQL read of it shows (sections 3, 8 and 9); D9: message signatures, the `key` authenticator and the deploy keys
`sign`, `verify` and `max_skew_ms` (sections 0, 1, 2, 5 and 7); D10: `persistence:` and `receipt:`, a durable client's
outbox in the store, its window and its expiry, and the orderly stop (sections 0 to 4, 8 and 9).

SCE builds the parts of a server, as libraries of the Rust host core; it builds no server program. Each deployment
writes the program that joins the parts to its machine, its keys and its credentials (section 1). What this document
says of "the program" is what such a program must do.

## 0. What is built and what is not

| Part | Where | Specified by |
|---|---|---|
| The keyed pool of sessions | `sce_rust_mesh::pool` (`backends/rust/mesh/src/pool.rs`) | SCE_MESH.md section 14.4, "Keyed session pool" |
| The session store contract, with lease and fence | `sce-rust-session-store` (`backends/rust/session-store/src/lib.rs`) | ADR 0107, D6 |
| The reference session store | `SqliteStore`, feature `sqlite` of `sce-rust-session-store` | ADR 0107, D6 |
| The credential store: installation pins and bearer tokens, each as its digest | `CredentialStore` of `sce-rust-session-store`, durable in `SqliteStore`; the `bearer` check `StoredTokens` of `sce-rust-crypto` | ADR 0109, D3; SCE_MESH.md section 22.5 |
| A store sealed at rest: every value a sealed record, every key an entry, the keys rotated | `Sealed<S>` of `sce_rust_crypto::sealed` over any of the stores above; its keys `SealingKeys` | ADR 0109, D8; `docs/SCE_SEALED_RECORD.md` |
| The checkpoint of a session | `sce_rust_mesh::checkpoint`, schema `schemas/sce-mesh-checkpoint.v1.schema.json` | SCE_MESH.md section 10.11 |
| The `https` server | `sce_rust_mesh::https::server::HttpsServer`, feature `https` | SCE_MESH.md section 22 |
| The `https` client, Rust | `sce_rust_mesh::https::client::HttpsClient` over `HyperTransport` | SCE_MESH.md sections 22.6 and 22.8 |
| The `https` client, C++ | the generated `TransportRouter`'s `https` arm over `SCE::Mesh::HttpsTransport`, with `CurlHttpsTransport` or `WinHttpHttpsTransport` | SCE_MESH.md sections 22.6 and 22.8 |
| Message signatures: a `200` signed by the server's `sign` key and verified by a client under `verify:`, a request signed under the `key` authenticator | `sce_rust_mesh::https::signature` (server and Rust client); `SCE::Mesh::Https::Signatures` (C++ client), on libsodium | ADR 0109, D9; SCE_MESH.md section 22.9 |
| A durable client's outbox: what a session sends it kept in the store, appended by the write of the session's checkpoint, until the client acknowledges it; its window and its expiry | `SessionStore::write` and the outbox calls of `sce-rust-session-store` (`SqliteStore` from layout 5, sealed under `Sealed`); `sce_rust_mesh::outbox::Outboxes`, kept by the pool (`Pool::outboxes`, `Pool::expire`) and answered from by the server (`HttpsServer::keep_outboxes`) | ADR 0109, D10; SCE_MESH.md sections 10.11 and 22.3 |
| The orderly stop: the server answers what it holds and takes nothing in, the pool saves every session as a clean checkpoint | `HttpsServer::stop`, `Pool::stop` | ADR 0109, D10; SCE_MESH.md sections 14.4 and 22.4 |
| The key-value feature's values kept durably, each put-if's first result in the transaction of its write, the first results of a session deleted with it | `KvStore` of `sce-rust-session-store` (`SqliteStore` from layout 6, sealed under `Sealed`), given to the host by `ReferenceHost::with_kv_store`; the pool names the sessions a removal ends (`Write::sessions`) | ADR 0109, D13; docs/SCE_HOST_FEATURES.md section 3 |
| An audit log: entries redacted by their schema, chained, checkpointed under the audit key, durable before the append answers, retained by segments | `AuditStore` of `sce-rust-session-store` (`SqliteStore` from layout 7, sealed under `Sealed`), `sce_rust_crypto::audit` (the chain, the checkpoints, `verify`, `retain`), given to the host by `ReferenceHost::set_audit_store` and `set_audit_streams`; `audit_stop` at the orderly stop, `audit_retain` on the host's schedule | ADR 0109, D14; docs/SCE_HOST_FEATURES.md section 15; docs/SCE_AUDIT_LOG.md |
| The dialing side of a durable `https` binding in Rust: acknowledges only what a durable checkpoint of its own holds, keeps what it sent until a `200` carried it | `HttpsClient::received`, `HttpsClient::held`, `LinkEvent::Carried`, and the router `Machine::router` builds | ADR 0109, D10; SCE_MESH.md section 10.11 |

Not built, and a server must not count on them:

- The rest of the suite of ADR 0107 D10 (ten thousand sessions killed at random points, two processes against one
  store, the binding through proxies with responses lost on purpose) is under `tests/server`, the outbox's kills with
  it (ADR 0109, D10); it is a test suite, not a guarantee of a deployment's own composition.
- The Windows CI lane of ADR 0107 D11. The WinHTTP adapter is built and has its conformance test
  (`tests/cmake/mesh_https.cmake`), but no CI job runs it.
- A durable receipt on any backend but Rust's: the generated C++ router, whose `https` client holds no envelope for a
  checkpoint, and the Kotlin and Swift host cores refuse `persistence:` and `receipt: durable` by name
  (`mesh/codegen-durable-receipt-unsupported`).
- A client of the `https` binding on the Kotlin and Swift host cores (SCE_MESH.md section 22.8).
- The removal of expired and revoked tokens from the credential store (section 5).
- A record signed by name and kept beyond the exchange (`sce:payload`, ADR 0107 Open question 1): ADR 0109 D9's
  record-level half, a later step after ADR 0101's steps 4a and 4b. Message signatures cover one exchange.

## 1. What the program supplies

| The program supplies | To | Read in |
|---|---|---|
| The generated machine and its peer table (`sce-codegen generate --deploy`, `--lang rust`) | `Pool::new` | SCE_MESH.md section 19, "Peer table" |
| How a session's machine is started and restored (`SessionFactory`, `SessionMachine`) | `Pool::new` | `pool.rs` |
| Its name as the holder of leases, the lease's length and its renewal period (`PoolConfig`: `owner`, `lease_ttl_ms`, `renew_every_ms`) | `Pool::new` | section 4 |
| The session store, opened on a file, sealed under the keys of its key ring | `Pool::new` | section 3 |
| The key-encryption key (`KeyEncryptionKey::from_file` or `from_variable`), held apart from the store | the key ring the sealing keys are read from, and the host's | sections 3 and 8 |
| The authenticator's check (`StoredTokens` or a `BearerCheck`, a `PinStore`, `CertificateCheck`, or `KeyPins` over a `PinStore`), and the credential store it and the machine's host share | `HttpsServer::new`, `ReferenceHost::set_credentials` | section 5 |
| The key that signs every `200` when the binding names `sign` (`ServerSettings::signer`, read from the key ring) | `HttpsServer::new` | section 5, "Message signatures" |
| The server's certificate chain and key, and for `mtls` the client authorities | `server_tls` | section 5 |
| The clock loop: `Pool::due`, `Pool::renew_leases`, `Pool::evict_idle` and `Pool::tick` on its own schedule, sleeping until `Pool::next_wake_ms` between them, and `Pool::run` for each key they answer | the pool | `pool.rs`, module documentation |
| The checkpoint each exchange waits for: `ServerHost::deliver` answers the endpoint's `sealed_through()`, and `HttpsServer::durable` is told each checkpoint reported durable | the server | SCE_MESH.md section 22.8 |
| The standard host features its machine asks for (section 9 of `docs/SCE_HOST_FEATURES.md` names what their reference hosts keep in memory) | the machine | `docs/SCE_HOST_FEATURES.md` |

`PoolConfig::renew_every_ms` must be above zero and below `lease_ttl_ms`; `Pool::new` refuses any other pair
(`PoolConfigError::Renewal`). Neither has a default: the program chooses both (section 4).

## 2. deploy.yaml

### 2.1 The `https` binding

The side that keeps the mailboxes, the server, carries `listen`; the side that posts the exchanges, the client, carries
`url`. A binding with both or neither is refused, and so is a key of the other side (`mesh/topology-invalid-binding-field`).
Both sides carry `authenticator`, and a binding without one is refused as `mesh/topology-binding-without-authenticator`.
The keys and their defaults are SCE_MESH.md section 22's table; the defaults are applied once, by the deploy parser
(`HTTPS_DEFAULT_*` in `sce-build/src/mesh/topology.rs`), and the same numbers are the `DEFAULT_*` constants of
`sce_rust_mesh::https`.

| Side | Key | Meaning | Default | Accepted values |
|---|---|---|---|---|
| server | `listen` | the `host:port` the server accepts on; an IPv6 host in brackets (`[::1]:8443`) | required | a host, and a port from 1 to 65535 |
| server | `authenticator` | who may claim a name (section 5) | required | `bearer`, `install`, `mtls`, `key` |
| server | `max_hold_ms` | the longest an exchange is held while the mailbox is empty | 30000 | a positive integer that fits 32 bits |
| server | `max_body_bytes` | the largest request body taken in; a larger one is answered `413` | 1048576 | a positive integer |
| server | `absence_ms` | how long a client may go without an exchange and stay ready | 120000 | a positive integer that fits 32 bits |
| server | `mailbox` | how many envelopes wait for one client at most | 1024 | a positive integer that fits 32 bits |
| server | `max_clients` | how many client entries the server holds in memory; an exchange for a name past it is answered `503` | 100000 | a positive integer that fits 32 bits |
| server | `max_response_bytes` | the most mailbox bytes one response carries; the rest go at the next exchange | `max_body_bytes` | a positive integer |
| server | `rate` | the edge's limiters, below | on, at the defaults below | `none`, or a map of `address`, `client`, `failed_auth`, `new_names` (each `{ interval_ms, burst }` or `none`) and `max_tracked` |
| server | `trusted_proxies` | the proxies whose `Forwarded` header names the client's address | none | a list of CIDR blocks (`10.0.0.0/8`, `2001:db8::/32`, or a bare address) |
| server | `sign` | the `sign` key of the server's key ring every `200` is signed with (section 5, "Message signatures") | none: nothing is signed | a key's id, 1 to 64 bytes |
| server | `max_skew_ms` | how far a signed request's `created` may lie from the server's clock | 300000 | a positive integer that fits 32 bits, beside `authenticator: key` only |
| both | `receipt` | `durable` keeps each client's mailbox as an outbox in the session store (section 3), and a client acknowledges only what a checkpoint of its own holds | `transport` | `transport`, `durable`; `durable` needs both machines checkpointed (section 2.3) |
| server | `retain_ms` | how long an outbox entry is kept unless its client acknowledges it | 2592000000 (thirty days) | a positive integer, at least `absence_ms`, beside `receipt: durable` only |
| client | `url` | `https://<host>[:<port>][<base>]`, before the binding's path, with no query or fragment; `http://` only to a loopback host | required | as said |
| client | `authenticator` | as on the server | required | as on the server |
| client | `hold_ms` | how long a poll asks the server to hold it | 25000 | a positive integer that fits 32 bits |
| client | `verify` | the trust set of the client's host every `200` is verified under (section 7) | none: a `200` is taken as it comes | a trust set's name, 1 to 64 bytes |
| client | the retry keys every binding spells | SCE_MESH.md section 10.10; a request that failed at the transport is sent again under them | none | as section 10.10 says |

What these mean for an operator:

- **One address, one rule.** Several bindings may listen on one address only when they name one authenticator and one
  `sign` key (or none); a server built for the address keeps, of each bound, the smallest any of them declares (`Listening::on` in
  `backends/rust/mesh/src/https/server.rs`), so no binding is given more than it declared.
- **The hold and the intermediaries.** A held exchange is an HTTP request that stays open for up to `max_hold_ms`
  with nothing on the wire. Every intermediary between client and server (a load balancer, a reverse proxy, a client's
  explicit proxy) must let a request stay open longer than that, or it cuts held polls, which the client sees as a
  transport failure (SCE_MESH.md section 22.4) and answers with its backoff. Where an intermediary's limit cannot be
  raised, lower the client's `hold_ms` below it; the server grants the smaller of `Sce-Mesh-Hold-Ms` and
  `max_hold_ms` (SCE_MESH.md section 22.3).
- **A full mailbox drops; a full durable outbox refuses.** Under `receipt: transport` a send to a client whose
  mailbox holds `mailbox` envelopes is dropped as SCE_MESH.md section 16.7 row 9 (`backpressureDrop`). Under
  `receipt: durable` `mailbox` is the client's durable window: a send admitted while the client's outbox and what the
  pool's sessions produced for it and no write appended yet hold `mailbox` envelopes is refused at admit as row 9,
  told to the session that sent it, and nothing accepted is ever dropped. A client absent for longer than
  `absence_ms` is told to its machine as row 1 (`transportUnavailable`), and its mailbox stays (SCE_MESH.md section
  22.4).
- **A durable outbox expires.** An entry kept longer than `retain_ms` without its client acknowledging it is deleted
  by `Pool::expire`, which the clock loop calls on its schedule; the program is told (`PoolError::OutboxExpired`), and
  the session that sent it is told at its next run as row 15 (`outboundStaleDrop`). Choose `retain_ms` as the longest
  a client may be gone and still be owed what was sent to it.
- **A client costs a stated bound** (ADR 0109, D1). A client's entry is removed once its mailbox is empty, no
  exchange of it is in flight and it is not ready, so the names that make one exchange and go away are forgotten
  `absence_ms` after; `max_clients` bounds the entries held at once, and a name past it is answered `503` with
  `Retry-After`. A response carries at most `max_response_bytes` of the mailbox, the rest at the next exchange. A
  mailbox made again after its entry was removed numbers anew, in an incarnation of its own (SCE_MESH.md section 22.3),
  which its client follows as it follows a restart.
- **The edge limits rate** (ADR 0109, D2), each limiter by the standard library's GCRA (`sce:std/rate/gcra_admit`),
  and a refusal is `429` with `Retry-After`, the limiter's wait rounded up to a second:

  | Limiter | Counts | Keyed by | Default |
  |---|---|---|---|
  | `address` | every exchange, before it is authenticated | the remote address (an IPv4 address, an IPv6 /64) | 100 a second, burst 200 (`interval_ms: 10, burst: 200`) |
  | `client` | every exchange that authenticated | the client's name | 20 a second, burst 40 (`interval_ms: 50, burst: 40`) |
  | `failed_auth` | each `401`; while it refuses, an exchange is answered `429` without running the authenticator | the name, and the address | 10 a minute, burst 10 (`interval_ms: 6000, burst: 10`) |
  | `new_names` | each name the `install` authenticator pins for the first time | the remote address | 20 an hour, burst 20 (`interval_ms: 180000, burst: 20`) |

  Each limiter tracks at most `rate.max_tracked` keys (default 100000), dropping the key of the earliest arrival to
  make room; a restart starts every limiter fresh. `rate: none` turns every limiter off, and `<limiter>: none` one.
  A guess at another client's name locks that name out for as long as the `failed_auth` limiter refuses it, which is
  the price of costing the guesser the rate; choose the rate with that in mind.
- **Behind a load balancer.** Every client of a server behind a reverse proxy arrives from the proxy's address, so the
  `address` and `new_names` limiters would count every client as one. Name the proxy's addresses in
  `trusted_proxies`: a connection from one of them has its client's address read from `Forwarded` (RFC 7239), the
  rightmost address it names that is not itself a trusted proxy, and its connections count toward
  `max_connections` alone. The proxy must set `Forwarded` (not `X-Forwarded-For`, which is not read) and strip any a
  client sent. A connection from any other address is keyed by its own address, whatever it sends.

The server has bounds deploy.yaml does not carry, the program's own (`ServerSettings` in `server.rs`):

| Setting | Default | What it does |
|---|---|---|
| `base_path` | empty | the path before `/sce-mesh/`, matching the `<base>` of the clients' `url` |
| `max_exchanges` | 10000 | the most exchanges in flight at once; one more is answered `503` |
| `max_exchanges_per_client` | 3 | the most exchanges in flight for one client; one more is answered `429` |
| `retry_after` | 1 s | what a `429` or `503` asks the client to wait (`Retry-After`) |
| `handshake_timeout` | 10 s | the longest a TLS handshake may take; the connection is closed past it |
| `header_timeout` | 10 s | the longest a request's head may take to arrive; the connection is closed past it |
| `body_timeout` | 30 s | the longest a request's body may take to arrive whole; the exchange is answered `408` |
| `max_connections` | 20000 | the most connections open at once; one more is closed at accept |
| `max_connections_per_address` | 64 | the most connections open at once from one address (an IPv4 address, an IPv6 /64), a trusted proxy's excepted; one more is closed at accept |

A program that accepts its connections itself rather than calling `HttpsServer::serve` hands each to
`HttpsServer::serve_accepted` with its remote address, so that the edge's address limits apply, and applies the
connection bounds and the handshake timeout itself; `serve_connection`, which takes no address, holds its exchanges
to no limiter of the address.

The Rust client has settings of its own beside `hold_ms` (`ClientSettings` in `backends/rust/mesh/src/https/client.rs`):
`exchange_timeout`, how long an exchange may take beyond the hold it asks for, 30 s; and `backoff`, the wait between
polls after a failed one, 250 ms doubling to 30 s with jitter (`Backoff::default` in
`backends/rust/mesh/src/reconnect.rs`, and SCE_MESH.md section 22.4).

### 2.2 The pool: `sessions:`

A machine entry declares (SCE_MESH.md section 14, schema; section 14.4, "Keyed session pool"):

```yaml
sessions:
  key_from: data.<field> | client   # where an envelope's key is read
  idle_ms: <integer>                # an idle session is saved and dropped after this
  max_live: <integer>               # optional, 100000: sessions held in memory at most
  max_inbox: <integer>              # optional, 64: envelopes waiting in one session's inbox
```

`key_from` and `idle_ms` are required when the block is present, and neither has a default; `max_live` and
`max_inbox` are at least 1 (ADR 0109, D1). `key_from: client` keys a session by the
name the client posts under, so one client is one session; `key_from: data.<field>` keys it by a field of the
request's data, which the event schema of every `service.request.*` the machine handles must declare as a
`string[N]` of at most 256 bytes or as an integer (`mesh/deploy-sessions-key-not-in-request`). The block stands beside no `server:` block, no binding with
`ordering: required` and no binding with `delivery: causal` (`mesh/deploy-invalid-sessions`). Only the Rust host core
serves it; every other backend refuses it (`mesh/codegen-keyed-pool-unsupported`).

`idle_ms` is how long a session stays in memory after its last event. A session is dropped only when it also holds
nothing pending (section 7), so `idle_ms` bounds the memory of quiet sessions and nothing else.

`max_live` bounds the sessions in memory whatever their activity. At the bound, a request for a key not in memory
saves and drops the session idle longest that holds nothing pending, and when none can be dropped the request is
shed: over the `https` binding its exchange is answered `503` and the client carries it again, and on a binding that
acknowledges nothing it is answered `RpcStatus::Unavailable`. `max_inbox` bounds the envelopes waiting for one
session, and one past it is shed the same way; each shed the program is told of is `PoolError::Shed`. A session
started for a key the store holds nothing for is written only if its first run does not end in a final state, so a
document that answers a key it does not know and finishes costs the store nothing.

### 2.3 What is kept across a crash: `persistence:` and `receipt:`

SCE_MESH.md section 10.11 (ADR 0109, D10). A machine entry may declare

```yaml
persistence: checkpoint            # the host core checkpoints the machine
receipt: { window: 256, retry_ms: 1000 }   # optional: how its durable bindings send
```

and a binding `receipt: durable` (or `transport`, the default). A durable binding needs both machines it joins
checkpointed: by `persistence: checkpoint`, by being a keyed pool (`sessions:`, whose sessions are kept as their
checkpoints), or by being held in the tree of a machine that is; otherwise the deployment is refused naming the
machine that is not (`mesh/deploy-receipt-without-persistence`). A `receipt:` block with a field below 1, or on a
machine with no durable binding, is refused (`mesh/deploy-invalid-receipt`); `persistence: checkpoint` on a document
that saves nothing is refused when the machine is generated (`mesh/codegen-persistence-without-saved-state`); and
only the Rust host core keeps either, so every other backend refuses the machine by name
(`mesh/codegen-durable-receipt-unsupported`).

On the server, `receipt: durable` on the accepting `https` binding keeps each client's mailbox as an outbox in the
session store (section 3): nothing a session sent the client is lost by a crash or a restart, its numbering included,
until the client acknowledges it or `retain_ms` expires it. On a Rust client, it makes the client acknowledge only what
a durable checkpoint of its own holds and keep what it sent in its checkpoint until a `200` carried it: the program
reads `HttpsClient::received()` before it applies what the client handed it, takes its checkpoint, and, once the
checkpoint is durable, calls `HttpsClient::held(&received)`; it hands each `LinkEvent::Carried` to the endpoint as every
other event.

## 3. The session store

The reference store is `SqliteStore` (`backends/rust/session-store/src/sqlite.rs`): one SQLite database file, opened
with `SqliteStore::open(path)`, which makes the file when it is not there.

- **The system's SQLite.** The store links SQLite 3.14 or later, found through `pkg-config`, or `vcpkg` on Windows
  with MSVC, and compiles no copy of its own (the crate documentation of `sce-rust-session-store` says how to build
  where none is installed). Its security fixes are the system's.
- **Durable on return.** The database is in WAL mode with `synchronous=FULL`: a write that returned survives a power
  loss. The store refuses to open a file it cannot put in WAL mode, or that does not take `synchronous=FULL`.
- **A local file system.** WAL keeps an index in shared memory beside the database, so the file lives on a local file
  system, never a network one. Beside `<file>` SQLite keeps `<file>-wal` and `<file>-shm`; all three are the store.
- **Waiting for a lock.** A call that finds the database locked by another connection's write waits 5 s
  (`SqliteStore::BUSY_TIMEOUT`, or the bound given to `open_with_busy_timeout`) before it fails.
- **Its layout.** The database's `user_version` names the layout, `7` today. A file of layout `1` to `6` is brought
  to `7` when it is opened, in one transaction, its sessions, leases, keys, credentials, outboxes and values kept. A
  file of a layout this build does not know, or one that holds tables the store did not make, is refused. A later
  build that changes the layout makes a rollback to an earlier build refuse the file: a build before layout 7
  refuses a file this one opened.
- **The outboxes** (ADR 0109, D10). Under `receipt: durable` what a session sends a client is appended to the
  client's outbox by the same transaction that writes the session's checkpoint (`SessionStore::write`), so either
  both are durable or neither; an acknowledgement deletes what it covers, and an expiry what is older than
  `retain_ms`, writing in the same transaction a note for the session that sent it and making that session due at
  once, so it is told at its next run. Under the sealed store every envelope is a record sealed for the entry of its
  client's name, the client's name and the sending session's key inside it.
- **Sealed at rest** (ADR 0109, D8; `docs/SCE_SEALED_RECORD.md`). SCE_MESH.md section 10.11 asks the host to store a
  checkpoint encrypted at rest, since the saved state in it holds the document's values in the clear. The program
  opens the store sealed: `Sealed::new(SqliteStore::open(path)?, keys)`, where `keys` is
  `SealingKeys::from_keyring(&keyring, now_ms, &mut SystemRandom)` over a key ring on the same file
  (`Keyring::new(Box::new(SqliteStore::open(path)?), Box::new(kek))`, `kek` the key-encryption key of section 8).
  Every value the sealed store writes is a record of XChaCha20-Poly1305 under a sealing key, bound to its store and
  its row, and every key (a session's key, a client's name, a key's id, a token's digest) is kept as its **entry**,
  the HMAC-SHA-256 of the key under the store's index key. The sealed store is the one handed to `Pool::new`, to the
  server's authenticator and to the host (`ReferenceHost::set_credentials`, and its key ring's store): it is a
  `SessionStore`, a `CredentialStore` and a `KeyStore`, as the file is. A store opened without the sealing keys reads
  every row as a record it cannot open; a record moved to another row, changed by a byte, or read under another key
  is refused (`Failure`), and the pool answers that session's requests as it answers a store that failed.
- **What the file shows.** Every value is a record, and every key an entry; in the clear are what the store compares
  or sets: a session's version, earliest due and lease (owner, fence, expiry), a token's revocation, and a key's use,
  algorithm, creation and state. The index key (`index:sealed`) and the sealing keys (`sealing:<hex>`) are rows of
  the `keys` table wrapped under the key-encryption key, the one thing the sealed store keeps as it is given.
- **Rotation.** `SealingKeys::rotate(&keyring, now_ms, &mut SystemRandom)` makes a new sealing key and retires the
  one before, which still opens what it sealed. Give each process the keys read again (`Sealed::set_keys`, or a
  restart), then run the re-seal passes: `Sealed::reseal_sessions(owner, lease_ttl_ms, now_ms)`, which takes each
  session's lease under the name `owner` and writes it again from the version it read, leaving one another process
  holds; `Sealed::reseal_credentials()`; and `Sealed::reseal_keys()`, never beside a rotation of the key-encryption key.
  Each answers `Resealed`: rows written again, rows `left` (run the pass again later), rows `refused` (they do not
  open: changed on the medium, or sealed under a key the process was not given). Once a pass answers no row left and
  none refused, the retired key opens nothing and may be destroyed. The index key is never rotated.
- **What sealing does not give.** Freshness against someone who can write the medium: a row replaced by an earlier
  sealing of the same row opens, and a token's revocation, kept in the clear, can be set back by hand. The lease's
  fence and the version stop a process that is wrong, not a medium written by hand; keep the file where only the
  server's account writes it.

The tables (`LAYOUT_1` to `LAYOUT_5` in `sqlite.rs`): `outbox_heads` (`client`, `incarnation`, `next`,
`touched_ms`), `outbox_entries` (`client`, `number`, `at_ms`, `sender`, `envelope`) and `outbox_notes` (`id`,
`session`, `client`, `number`, `at_ms`, `envelope`), the outboxes of layout 5; `sessions` (`session`, `state`, `version`, `earliest_due`),
`leases` (`session`, `owner`, `fence`, `expires_ms`), and `numbers`, the one counter versions and fences are drawn
from; and the keys a host generates and derives (`KeyStore`, docs/SCE_HOST_FEATURES.md section 13): `keys` (`id`,
`key_use`, `algorithm`, `wrapped`, `label`, `created_ms`, `state`), each key's material wrapped under a
key-encryption key the file never holds, `key_generations` (which key each generation made) and `key_naming` (the
wrapped naming key of derived keys); and the credentials of section 5 (`CredentialStore`): `credential_pins` (`name`,
`digest`) and `credential_tokens` (`digest`, `name`, `issued_ms`, `expires_ms`, `revoked`, `sealed`), each digest the
SHA-256 of a secret the file never holds; and the values of the key-value feature a host keeps in the store
(`KvStore`, docs/SCE_HOST_FEATURES.md section 3), the tables of layout 6: `kv_values` (`key`, `value`, `version`),
`kv_numbers`, the feature's own counter of versions, and `kv_first_results` (`session`, `invoke`, `start`,
`outcome`, `version`), the first result of each put-if by its invocation's identity, deleted with the session that
started it; and the audit log (`AuditStore`, docs/SCE_HOST_FEATURES.md section 15, docs/SCE_AUDIT_LOG.md), the
tables of layout 7: `audit_heads` (`stream`, `sequence`, `chain`), each stream's last row, which retention leaves,
`audit_rows` (`stream`, `sequence`, `at_ms`, `entry`, `chain`), `audit_checkpoints` (`stream`, `sequence`, `chain`,
`signature`) and `audit_identities` (`session`, `invoke`, `start`, `sequence`), the sequence each append's identity
was appended at, deleted with its session; a row is never updated, and deleted only with its segment. Under a sealed
store, a stream is its entry, a row's `entry` and a checkpoint's `signature` hold records, and the chain is kept as
it is, so the log is verified without the sealing keys. Under a sealed store, `session`, `id`, `generation`, `key`, `name` and a first result's `session` and
`invoke` hold entries, `state`, `wrapped`, a pin's `digest`, a token's `sealed` and a value's `value` hold records, a
token's `digest` holds the keyed digest of its digest and its moments are 0. Every moment is milliseconds since the
Unix epoch.

## 4. What a restart does

A session is kept as its last durable checkpoint: its machine's saved state and its endpoint's state (the duplicate
window, the replies a duplicate request is answered with, what waits for a link or out a backoff), written with its
earliest due in one transaction (SCE_MESH.md sections 10.11 and 14.4). Nothing a session sends, a reply included,
leaves before the checkpoint that holds it is durable. So, when the process stops, whether it was told to or not:

- **Nothing is loaded at start.** A session is loaded at its next request, or when its earliest wait comes due
  (`Pool::due`, woken by `Pool::next_wake_ms`). A delayed `<send>` that came due while the process was down is
  delivered once, after the load, and a restored machine arms a send already due as due now (SCE_MESH.md section
  10.11, "Timers of sessions that are not loaded").
- **`RESTORED` or not.** A session that was dropped as idle, or saved by an orderly stop (below), was saved as a
  clean checkpoint, and its restore raises nothing. A session that was in memory when the process stopped without
  one restores from a checkpoint that is not clean, and its machine receives SCE_MESH.md section 16.7 row 17
  (`restored`) once per peer source it holds state for: an envelope that source sent after the checkpoint may have
  been taken in and lost.
- **Repeated requests get the first answer.** A request the client's core sends again (under the binding's retry
  policy, after a transport failure) is answered from the reply cache the checkpoint holds (ADR 0107, D4), across the
  restart.
- **A durable client's mailbox outlives the process** (ADR 0109, D10). Under `receipt: durable` the mailbox is the
  client's outbox in the store, so the restarted server answers the client's next exchange with what it held, in the
  same incarnation and numbering, and the client acknowledges on where it stopped; nothing it was sent is lost, and a
  copy it took in before is dropped by its duplicate window. Give the server the pool's outboxes before it serves
  (`server.keep_outboxes(Arc::clone(pool.outboxes().expect("a durable binding")))`).
- **Other mailboxes start empty.** Under `receipt: transport` a mailbox draws a new incarnation when its entry is made
  and numbers from 1 again (SCE_MESH.md section 22.3); a client learns it from `Sce-Mesh-Epoch` and starts its
  acknowledgement over. What waited in such a mailbox at the stop is lost with the process: a `<send>` to the client
  among it is not delivered, and a request whose reply was among it ends at its deadline (SCE_MESH.md section 9.5).
- **Leases.** A process that starts again under the same `owner` takes each key's lease at once, with a new fence that
  refuses its previous life's writes (`SessionStore::acquire`). A process under another name waits for the old leases
  to run out, `lease_ttl_ms` after their last renewal; until then a request for such a key is answered
  `RpcStatus::Unavailable` (section 6).
- **In-memory host state is gone.** Pins and tokens kept by `MemoryCredentials` (section 5), the reference hosts'
  key-value stores and their first results of a put-if, and the COSE keys a host derived (`docs/SCE_HOST_FEATURES.md`
  section 9): a `derived:<n>` id a restored session holds names no key, or another one.

### 4.1 Stopping in order

An orderly stop leaves only clean checkpoints, so the next start raises no `restored` and has nothing to recover
(ADR 0109, D10; SCE_MESH.md sections 14.4 and 22.4). It is two calls, the server's first: `HttpsServer::stop(within)`
accepts no connection more, answers every new exchange `503` with `Retry-After` (the clients send again to whatever
serves next), answers every held exchange at once with what its mailbox holds, and waits for the exchanges in flight
to end; `Pool::stop(deadline)` admits nothing more, runs what each session's inbox holds, waits for a run another
thread is in the middle of, saves every session as a clean checkpoint, and releases its leases. It answers a report:
`saved`, and `not_saved` with the reason of each (a run still going at the deadline, a write that failed, a lease
lost); a session not saved restores with `restored`, as after a crash, so report them.

Catching the signal is the program's. On Unix a service manager sends `SIGTERM` and waits before `SIGKILL`
(systemd's `TimeoutStopSec=`, 90 s by default; Kubernetes' `terminationGracePeriodSeconds`, 30 s): give the two
calls together less than that. With tokio's `signal` feature:

```rust
let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
tokio::select! {
    _ = terminate.recv() => {}
    _ = tokio::signal::ctrl_c() => {}
}
let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
server.stop(std::time::Duration::from_secs(5)).await;
let report = tokio::task::spawn_blocking({
    let pool = Arc::clone(&pool);
    move || pool.stop(deadline)
})
.await?;
for (key, reason) in &report.not_saved {
    eprintln!("not saved: {key}: {reason}");
}
```

`Pool::stop` blocks on the store, so it runs on a thread of its own (here tokio's blocking pool), not on a runtime's
worker. A process killed before it ends, or a session the report names as not saved, restores as section 4 says.

## 5. Authentication and TLS on the server

Every binding names one of four authenticators (SCE_MESH.md section 22.5; `Authenticator` in `server.rs`):

| `authenticator` | The client sends | The server's program gives | After a restart |
|---|---|---|---|
| `bearer` | `Authorization: Bearer <token>` | `Authenticator::stored_tokens(StoredTokens::new(store))` over the credential store its machine issues tokens into; or a `BearerCheck` of its own, `(client name, token)` to whether the token proves the name | with the store on the session store's file, every token as before |
| `install` | `Authorization: SCE-Install <secret>`, a 256-bit secret the client drew at its first run and keeps, in base64url | a `PinStore`: the server pins a name to the SHA-256 of the first secret it answers `200` and refuses any other with `401` | with the store on the session store's file, every pin as before; with `MemoryCredentials`, every pin is forgotten and the next secret a name brings is pinned |
| `mtls` | a TLS client certificate | client authorities to `server_tls`, and a `CertificateCheck`, `(client name, certificate DER)` to whether it names the client | as before |
| `key` | no `Authorization`: every request signed by an Ed25519 key the client's host generated once, the key offered in `Sce-Mesh-Key` (SCE_MESH.md section 22.9) | `Authenticator::Key(KeyPins::first_use(store))`: the server pins a name to the key of its first exchange answered `200` and refuses another key with `401`; or `KeyPins::registered(store)`, which takes only a key the program pinned for the name (`PinStore::pin(name, &KeyPins::pin_of(&public_key))`) | with the store on the session store's file, every pinned key as before |

- **The credential store** (ADR 0109, D3). `SqliteStore`, opened on the session store's file (section 3), is a
  `CredentialStore`: the pins of `install` and the tokens of `bearer`, each kept as the SHA-256 of its secret, never
  the secret, in the file's layouts 3 and 4; under the sealed store of section 3 a name is kept as its entry and a pin
  or a token as a sealed record. The program opens the store once and hands the same `Arc` (of the sealed store) to
  the server's authenticator (`Authenticator::Install(store.clone())`, or `Authenticator::stored_tokens`) and to the
  reference host of its machine (`ReferenceHost::set_credentials`), so a token the machine issued is the one the
  server takes and a revocation the machine made is the one the server reads. `MemoryCredentials` keeps them in
  memory, for a test or a server whose names do not outlive it. A store that cannot be read answers the exchange
  `503` with `Retry-After`, as load shed, rather than `401`.
- **Tokens.** A machine issues a token with `sce-host-credential-issue` (docs/SCE_HOST_FEATURES.md section 14): `sce1.`
  and 43 base64url digits, the keyed digest of the invocation's identity and the name under the key ring's issuing
  key, so the same invocation repeated after a restore issues the same token. The machine sends it to its client
  once; the store keeps its digest, the name, the moment it was issued and its expiry. `StoredTokens` takes a token
  for the name the path claims until its expiry, and not after it is revoked. The issuing key is a key of the key
  ring (`issuing:credentials`, docs/SCE_HOST_FEATURES.md section 13), wrapped under the key-encryption key in the
  same file: issuing needs that key, and checking does not, since a presented token is compared by its digest.
- **Revocation.** `sce-host-credential-revoke` revokes a name's tokens and removes its pin in one transaction. A
  revoked token stays in the store as revoked, so a repeat of the issue that made it answers it revoked. The
  operator removes a pin by hand with `PinStore::unpin(name)`: a client that lost its secret (reinstalled) is refused
  `401` until then. Expired and revoked tokens stay in the store; no call of the store removes them yet.
- **One comparison.** Every comparison of a secret or a digest (a pin, a token's name, a relay token's tag) is
  `sce_rust_crypto::equal::same_secret`, in time that does not depend on where the two differ. A `BearerCheck` of
  the program's own compares its secret with it too.
- **Message signatures** (ADR 0109, D9; SCE_MESH.md section 22.9). A binding that names `sign: <key_id>` has every
  `200` signed, over its status, its body's digest, its numbering and the request it answers, so that a client under
  `verify:` takes nothing an intermediary altered or replayed. The key is a `sign` key of the server's key ring
  (docs/SCE_HOST_FEATURES.md section 13, `sce-host-key-generate`): the program reads it once
  (`Keyring::key(id, Purpose::Sign)`), sets `ServerSettings::signer` to `ResponseSigner::new(id,
  KeyPair::from_seed(&seed))`, and hands its clients the public key (`sce-host-key-public`, a COSE_Key) to put in the
  trust set their `verify:` names, under the same id. `Listening::on` names the key the bindings on an address name
  (`Listening::sign`) and refuses an address whose bindings name two. To rotate, generate a new key, add its public key
  to the clients' trust set beside the old one, move `sign` to it, and retire the old key once every client holds the
  new one: a client chooses the key by the signature's `keyid`. A request of the `key` authenticator is held to
  `max_skew_ms` of the server's clock, so the server's clock is kept by NTP; a client whose clock is further off is
  refused `401` until it is set.
- **The pin of a key.** Under `key`, the pin is the key's COSE_Key, which names its algorithm, so the pins of
  `install` and `key` cannot be read for each other. A client whose key is lost (reinstalled) is refused until the
  operator removes its pin (`PinStore::unpin(name)`), as with `install`.
- **TLS.** `server_tls(chain, key, client_authorities)` builds the server's TLS: `chain` each certificate in DER, the
  server's own first; `key` in DER as PKCS#8, PKCS#1 or SEC1; `client_authorities`, each DER, for `mtls` only. It
  offers HTTP/2 and HTTP/1.1 (ALPN) on rustls with `ring`. `HttpsServer::serve(listener, None)` serves without TLS,
  which is for a loopback test; a client reaches `http://` only on a loopback host (section 2.1).

## 6. One active process and a standby

A store may be shared by two server processes; the lease and the fence keep their sessions apart (ADR 0107, D6 and
Open question 2). The deployment this supports is one active process with a standby.

- **Distinct owners.** Each process has an `owner` no other process serving the store uses (`PoolConfig::owner`).
  Two processes under one name take each other's leases in turn.
- **One machine for the reference store.** `SqliteStore` serves several connections, in one process or several, but
  only on a local file system (section 3), so an active process and its standby on `SqliteStore` run on one machine. A
  standby on another machine needs a `SessionStore` on a medium both reach; SCE does not provide one. The contract's
  conformance kit, `sce_rust_session_store::conformance`, is what such a store is held to.
- **Every exchange of a client reaches the process that holds its session.** A mailbox is in the memory of the process
  whose server keeps it, and routing a key to the process that holds its lease is not built (ADR 0107, Open question
  2). Send all traffic to the active process, and move it to the standby only once the active one is gone.
- **What the standby does meanwhile.** A request that reaches it for a key the active process leases is answered
  `RpcStatus::Unavailable`, which tells the client to try again, and the key is left alone until that lease runs out
  (`PoolError::LeaseHeld`). A session that comes due is left to the process that leases it (`Pool::due`).
- **Takeover.** When the active process stops, its leases run out `lease_ttl_ms` after their last renewal, and the
  standby then takes each key at its next request or due; the session restores from its last durable checkpoint and
  raises `restored` (section 4).
- **A process that comes back.** A process paused for longer than its lease (a stopped virtual machine, a long
  collection) still believes it holds its keys. Its next renewal is refused, and the pool drops the session without
  saving it and answers what waits for it `RpcStatus::Unavailable` (`PoolError::LeaseLost`); a write it makes before
  it learns is refused by the store, because its fence is no longer the current one.
- **Choosing the lease.** `lease_ttl_ms` is how long keys stay unserved after the active process dies, and also the
  longest pause the process survives without losing its keys. `renew_every_ms` must be below it (section 1); a period
  of a third of the lease leaves room for two renewals that fail or come late before the lease runs out. A lease's
  expiry is read by the wall clock of the process that gives the moment, so processes that share a store keep their
  clocks synchronised; the fence, not the clock, decides whose writes land.

## 7. Clients: TLS and proxies

A client takes its platform's network policy (SCE_MESH.md section 22.6): it believes the authorities the operating
system believes, so a network that inspects TLS with its own authority is honoured where the client's machine trusts
that authority, and it reaches the server through the proxy the platform's settings name, tunnelling with `CONNECT`.
A server key pin is not a key of this version of the binding. Each failure is reported by its kind (`Connect`, `Tls`,
`Proxy`, `Timeout`, `Other`), which is the first thing to read when a client does not reach its server: `Tls` behind an
inspecting proxy is a machine that does not trust the proxy's authority, and `Proxy` a proxy that refused the tunnel.

| Adapter | Trust | Proxy | What the host may set |
|---|---|---|---|
| Rust `HyperTransport` (`backends/rust/mesh/src/https/adapter.rs`) | `rustls-platform-verifier`: the Windows certificate store, the Apple trust settings, the Android platform, the system's certificate bundle on another Unix | `HTTPS_PROXY`, `https_proxy`, `ALL_PROXY`, `NO_PROXY` everywhere, and the system's settings on macOS and Windows (`ProxyPolicy::System`, the default) | `extra_authorities` (refused on Android); `ProxyPolicy::Through(url)` or `Direct`; `client_identity` for `mtls` |
| C++ `CurlHttpsTransport` (`sce/include/mesh/transports/https/CurlHttpsTransport.h`) | the CA bundle the system's libcurl was built to read | libcurl's reading of `https_proxy`, `HTTPS_PROXY`, `no_proxy`, `NO_PROXY` | `ca_bundle` (PEM, instead of the distribution's); `client_certificate` and `client_key` (PEM) for `mtls` |
| C++ `WinHttpHttpsTransport` (`sce/include/mesh/transports/https/WinHttpHttpsTransport.h`) | the Windows certificate store | the current user's proxy configuration, with its automatic configuration script and WPAD | `proxy` (`host:port`, replacing the user's configuration); `client_certificate_thumbprint`, of a certificate with its key in the user's `MY` store, for `mtls` |

- **Android.** The host initialises `rustls-platform-verifier` with the application's context before the first
  exchange, and no additional authority can be added there (`adapter.rs`).
- **The Rust adapter's threads.** It runs its own runtime and is called from threads that are not a tokio runtime's
  (SCE_MESH.md section 22.6).
- **Building the C++ adapters.** `SCE_MESH_HTTPS_CURL` and `SCE_MESH_HTTPS_WINHTTP` (`cmake/SCEMeshHttps.cmake`) are
  each `AUTO` by default: libcurl's adapter is built when `SCE_ENABLE_MESH` is on and the system's libcurl is found,
  WinHTTP's on Windows. `ON` makes the adapter required and `OFF` leaves it out. Neither is installed with SCE's
  headers; a host on an installed SCE implements the port on its own HTTP stack.
- **The installation secret.** A client of `install` draws its secret once (`draw_install_secret` in Rust) and keeps
  it where its host keeps secrets; a client that loses it cannot claim its name again (section 5).
- **What a client verifies** (ADR 0109, D9). Behind a proxy that inspects TLS, TLS says nothing of who wrote a
  response. A client binding that names `verify: <trust_set>` takes a `200` only when a key of that trust set of its
  host signed it for the request it came back on (SCE_MESH.md section 22.9); one that does not verify is taken nothing
  of, acknowledges nothing, and is raised once in the machine as `error.communication` with `reason`
  `responseUnauthenticated` (section 16.7 row 19). The Rust client takes the set's keys in `ClientSettings::verify`
  (`ClientSettings::from_peer` refuses a link that names `verify` without them); the C++ client asks its host's
  `Credentials::trustSet(name)` and refuses to start when the host holds none, or gave the router no
  `ClientHost::crypto` (libsodium's `SodiumCoseCrypto`). A run of `responseUnauthenticated` on a network with an
  inspecting proxy is that proxy rewriting the exchange; a client whose trust set lacks the server's current key is
  refused the same way, which a rotation in the wrong order causes.
- **The key of a client** (`authenticator: key`). The client's host generates the key once and keeps it as it keeps
  an installation secret: the Rust client takes it as `Credential::Key(RequestSigner::new(pair))`, the C++ client asks
  `Credentials::requestKeySeed(peer)`. Every request then carries `Sce-Mesh-Nonce`, `Content-Digest`,
  `Sce-Mesh-Key`, `Signature-Input` and `Signature`, and no `Authorization`; an intermediary must pass them on as they
  are.

## 8. What to back up

| What | Why | How |
|---|---|---|
| The session store: `<file>`, with its `-wal` and `-shm` | every session, its replies and its timers, and every durable client's outbox | a consistent copy while the server runs, by SQLite's own backup (the `sqlite3` shell's `.backup`, or `VACUUM INTO` on SQLite 3.27 or later); a plain file copy only while no process has the store open. The store has no backup command of its own. |
| The server's TLS key and certificate chain | the clients' trust in the server | as the program keeps them |
| The client authorities of `mtls`, and the material a `BearerCheck` of the program's own verifies with | who may claim a name | as the program keeps them; the pins and tokens of the credential store are in the session store's file, backed up with it |
| The key-encryption key of the key ring | the sealing keys and the index key of a sealed store (section 3), the issuing key of tokens, and the keys a host generated and derived: without it a copy of the store opens nothing, and with it beside the copy the copy is in the clear | apart from the store and from its backups, which hold every key it protects wrapped and none of them in the clear: in the program's secret store or a key management service, with a copy kept where a loss of the server's machine does not reach it |
| The COSE keys the host is configured with: signing seeds, sealing keys, trust sets (`docs/SCE_HOST_FEATURES.md` section 10) | what the machine signs and opens | as the program keeps them; never in the store, since a checkpoint holds no key (SCE_MESH.md section 10.11) |
| deploy.yaml, the documents, and the build of the program | what the store's checkpoints are a state of | with the release |

Nothing else of a server outlives its process: its mailboxes under `receipt: transport`, `MemoryCredentials`, and the
keys a host keeps in a key ring in memory are in memory (section 4).

A store sealed at rest (section 3) keeps its sealing keys and its index key in its own `keys` table, so a backup of
the file carries the keys its rows were sealed under, wrapped, whatever was rotated since: what restores it is that
file and the key-encryption key it was wrapped under, never one without the other. A rotation of the key-encryption
key rewraps the live store's keys and no backup's, so keep each key-encryption key as long as a backup wrapped under
it may be restored.

Restoring a copy of the store moves every session back to the moment of the copy. Stop every process that uses the
store first, and start them after: a process left running holds leases and fences from the newer store, and the copy's
counter of versions and fences is behind the numbers it handed out since. After the restore a session restores from a
checkpoint that is not clean and raises `restored`, and a request it answered after the copy is no duplicate any more:
sent again, it is handled again.

## 9. A stuck session or an idle one

What the program can read:

- `Pool::is_live(key)`: whether the session is in memory.
- `Pool::take_errors()`: what went wrong that only the host can act on, in order (`PoolError` in `pool.rs`).
- `Pool::next_wake_ms()`: the earliest moment the pool has work, and so the moment the clock loop must call it again.
- The store, read only. In WAL mode a reader does not stop the server's writes: `SELECT session, earliest_due FROM
  sessions WHERE earliest_due <= <now>`, `SELECT session, owner, fence, expires_ms FROM leases`. Never write the
  tables by hand: a write outside the contract is outside its versions and fences.
- Under a store sealed at rest (section 3) those reads show **entries**, not keys: the `session` column of both
  tables holds the 64 hexadecimal digits of the key's HMAC-SHA-256 under the index key, and `state` a record no SQL
  read opens. To find the row of a key, ask the program's sealed store for its entry, `Sealed::digest_of(key)`, and
  select by it (`WHERE session = '<entry>'`); to read a session's checkpoint, read it through the sealed store
  (`checkpoint::load(&sealed, key)`), never from the column. An entry listed due names its key through
  `SessionStore::key_of(entry)`, which opens the row. A row that does not open is told as a `PoolError::Store` naming
  the refusal.
- Without a sealed store, a session's `state` is its checkpoint; its `clean` member says whether it was saved idle.

A session is **idle**, as it should be, when it is not in memory, holds no lease row, its checkpoint is `clean`, and
its `earliest_due` is empty or in the future. A session **waits**, as it may, when it is in memory past `idle_ms`
because its endpoint is not quiet (`Endpoint::is_quiet`): a request it sent waits for its answer, which its deadline
ends; a transmission waits out its backoff; envelopes are buffered for a peer that is not ready; or something is held
for a checkpoint. Each of these ends by itself.

A session is **stuck** in these shapes, each with what shows it:

| Shape | What shows it | What it means |
|---|---|---|
| Due and not woken | `earliest_due` in the past by more than the clock loop's period, and no lease row for the key | the program does not call `Pool::due` at `Pool::next_wake_ms` |
| Leased by a process that is gone | a lease row whose `owner` runs nowhere; `PoolError::LeaseHeld` on the live process | it is served again once `expires_ms` passes; a process that started again under that owner takes it at once (section 4) |
| Never saved | the session stays in memory past `idle_ms`; `PoolError::NotSaved` or `PoolError::Store` for it, again and again | its checkpoint cannot be written, so it is not dropped; the store's medium, or a machine that cannot be saved now |
| Not restorable | `PoolError::NotBuilt` for the key at each request, each answered `RpcStatus::InternalError` | its checkpoint is one this build does not read (`checkpoint` format other than 1, or a saved state the machine refuses), or the program could not build the machine |
| A client not reached | `transportUnavailable` (row 1) for the client, then `backpressureDrop` (row 9) on its sends | the client has made no exchange for `absence_ms`, and its mailbox filled; the session itself is well. Under `receipt: durable` the sends are refused at admit and nothing accepted is lost; once the client has been gone longer than `retain_ms`, `PoolError::OutboxExpired` and `outboundStaleDrop` (row 15) in the session that sent each, and `SELECT client, count(*) FROM outbox_entries GROUP BY client` shows who is owed what |
