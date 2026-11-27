# Running an SCE server

A server, in the sense of `docs/adr/0107-a-server-keeps-a-durable-session-per-key-and-a-client-reaches-it-by-mesh-over-http.md`,
is a statechart served as a pool of sessions, one per key, kept across restarts in a session store, and reached by
clients over the Mesh `https` binding. This document is the runbook for the operator of such a server: what to
configure, what a restart and a standby process do, what the clients need of their network, what to back up, and how
to tell a session that is stuck from one that is idle. Every key name and default below is taken from the code or
from SCE_MESH.md, and each says where.

Amended by ADR 0112 (`docs/adr/0112-a-generated-machine-is-served-as-a-pools-session-by-sces-adapter-and-one-kit-holds-every-session-machine-to-the-pools-contract.md`):
a generated machine is served by SCE's adapter (section 1), and the upgrade in place is held under load (section 4.2).

Amended by ADR 0116 (`docs/adr/0116-a-person-stands-behind-an-instance-by-a-delegation-the-server-holds-and-what-a-document-reads-of-them-no-sender-can-write.md`):
the store keeps who each fact of the journal was said for (layout 13, section 3), and the audit log who each entry a
run appended was made for (D7).

Amended by ADR 0109 (`docs/adr/0109-a-server-on-the-public-network-keeps-what-it-owes-bounds-what-its-clients-cost-and-is-upgraded-in-place.md`),
and brought to every step of it that landed by its step 15:

- D1 and D2: what a client may cost the server, and the edge's rate limits (section 2);
- D3: the credential store, its tokens and its pins (sections 0, 1, 3 and 5);
- D4: the server's outbound HTTP, built with a destination policy (sections 0 and 1);
- D5 to D7: the key ring, its key-encryption key and its keys (sections 0, 1, 3 and 8);
- D8: the store sealed at rest, its keys, their rotation and what an SQL read of it shows (sections 3, 8 and 9);
- D9: message signatures, the `key` authenticator and the deploy keys `sign`, `verify` and `max_skew_ms` (sections 0,
  1, 2, 5 and 7);
- D10: `persistence:` and `receipt:`, a durable client's outbox in the store, its window and its expiry, and the
  orderly stop (sections 0 to 4, 8 and 9);
- D11: a server upgraded in place, each session migrated by the migration its document declares (section 4.2);
- D12: the values a machine's deployment supplies, `config:` and `config_file:` (sections 1 and 2.4);
- D13 and D14: the key-value feature's durable store and the audit log, kept in the session store's file (sections 0,
  1, 3, 4.1 and 8);
- D15: the checkpoints of many sessions committed together, `sessions.commit_delay_us`, and the host told per client
  (sections 0 to 3);
- D16: the metrics port, the registry of the names it reports and the health checks (sections 0, 1, 9 and 10);
- D17: the named bounds of the signing and sealing records, which a build moves and the host is given (sections 1 and
  11).

Amended by ADR 0114
(`docs/adr/0114-a-keyed-sessions-facts-are-kept-in-a-journal-written-with-its-state-and-a-follower-reads-them-in-an-order-that-misses-nothing.md`),
D1 to D6 and D8: the event journal's store contract, kept in the session store's file, the sessions of each machine
of a deployment kept apart in it, the facts a pool's sessions say appended with their checkpoints, and a pool that
follows a journal (sections 0, 1, 3 and 8).

Amended by ADR 0113
(`docs/adr/0113-a-machine-of-a-deployment-may-run-as-an-open-set-of-instances-and-a-server-binds-each-as-it-arrives.md`),
D1 to D7: a machine that runs as an open set of instances, whose server binds each as it arrives, keeps their
membership in the session store, fences an ended incarnation `412`, keeps an outbox per instance that ends with its
membership, and whose client joins and is told an unknown outcome (sections 0 to 2).

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
| A record signed and sealed by name and kept beyond the exchange: a machine's record signed, verified, sealed and opened as its deterministic CBOR, a record of an earlier revision read with the later revision's defaults | `sce:payload` on the four COSE operations; the record codecs `sce_rust_runtime::record_cbor` and `common/RecordCbor.h` | ADR 0109, D9; docs/SCE_HOST_FEATURES.md section 10 |
| A durable client's outbox: what a session sends it kept in the store, appended by the write of the session's checkpoint, until the client acknowledges it; its window and its expiry | `SessionStore::write` and the outbox calls of `sce-rust-session-store` (`SqliteStore` from layout 5, sealed under `Sealed`); `sce_rust_mesh::outbox::Outboxes`, kept by the pool (`Pool::outboxes`, `Pool::expire`) and answered from by the server (`HttpsServer::keep_outboxes`) | ADR 0109, D10; SCE_MESH.md sections 10.11 and 22.3 |
| The orderly stop: the server answers what it holds and takes nothing in, the pool saves every session as a clean checkpoint | `HttpsServer::stop`, `Pool::stop` | ADR 0109, D10; SCE_MESH.md sections 14.4 and 22.4 |
| Group commit: the checkpoints of many sessions and the acknowledgements of durable clients written in one transaction, each answered once it is durable; the server tells its host per client | `sce_rust_mesh::commit::Committer` (`Pool::committer`) over `SessionStore::write_many` (`SqliteStore`: one savepoint per write, one sync; sealed under `Sealed`); `ServerHost::deliver` called at once for different clients | ADR 0109, D15; SCE_MESH.md sections 10.11, 14.4 and 22.8 |
| The key-value feature's values kept durably, each put-if's first result in the transaction of its write, the first results of a session deleted with it | `KvStore` of `sce-rust-session-store` (`SqliteStore` from layout 6, sealed under `Sealed`), given to the host by `ReferenceHost::with_kv_store`; the pool names the sessions a removal ends (`Write::sessions`) | ADR 0109, D13; docs/SCE_HOST_FEATURES.md section 3 |
| An audit log: entries redacted by their schema, chained, checkpointed under the audit key, durable before the append answers, retained by segments | `AuditStore` of `sce-rust-session-store` (`SqliteStore` from layout 7, sealed under `Sealed`), `sce_rust_crypto::audit` (the chain, the checkpoints, `verify`, `retain`), given to the host by `ReferenceHost::set_audit_store` and `set_audit_streams`; `audit_stop` at the orderly stop, `audit_retain` on the host's schedule | ADR 0109, D14; docs/SCE_HOST_FEATURES.md section 15; docs/SCE_AUDIT_LOG.md |
| The event journal's store: a session's facts appended by the write that keeps its state, in one order of positions read up to the horizon and as of an earlier moment, and what another system's adapter observed, once for each id it gave | `JournalStore` of `sce-rust-session-store` (`SqliteStore` from layout 8, sealed under `Sealed`), appended to by `Write::journal` and `JournalStore::observe`; a pool appends what its sessions say with the checkpoint that holds it, and a pool that follows a journal reads it (`Pool::follow_from`, `Pool::follow`), each session keeping its position in its checkpoint and the reader its mark in the store (`JournalStore::keep_read`, layout 12); the machines of a deployment keep their sessions in one store, each by its own handle (`SessionStore::machine`, `SqliteStore::of_machine`, layout 12) | ADR 0114 |
| The key ring: the keys a host generates and derives, its issuing key of tokens and its audit keys, and the index and sealing keys of a sealed store, each kept wrapped under a key-encryption key the store never holds | `Keyring` of `sce_rust_crypto::keyring` over a `KeyStore` of `sce-rust-session-store` (`SqliteStore` from layout 2); the `KeyWrap` port and its reference adapter `KeyEncryptionKey` of `sce_rust_crypto::wrap` (`from_file`, `from_variable`); given to the host by `ReferenceHost::set_keyring` | ADR 0109, D7 and D8; docs/SCE_HOST_FEATURES.md section 13; section 3.2 |
| The standard host features a server's machine asks for, the clock, key-value, HTTP, signing and sealing, random bytes, keyed digests, keys, credentials and the audit log among them, served by one reference host | `ReferenceHost` of `sce-rust-host-features` (`ReferenceHost::new` or `with_kv_store`, and its setters, section 1), joined to a generated machine by `sce_rust_host_features::serve` | docs/SCE_HOST_FEATURES.md; ADR 0109, D3 to D7, D13, D14 |
| Outbound HTTP from a server: each request judged by a destination policy before anything is sent, connected to the address judged, no redirect followed, bounded in time and size | `NetworkTransport::new(policy, settings)` of `sce-rust-host-http`, the reference host's transport; `DestinationPolicy::allow_list` or `public_internet` of `sce_rust_host_features::destination` | ADR 0109, D4; docs/SCE_HOST_FEATURES.md section 5 |
| The edge's bounds and rate limits: client entries, connections and their timeouts, a response's size, the four limiters, `Forwarded` read from trusted proxies only; the pool's bounds on the sessions in memory and on one session's inbox | `ServerSettings` and `sce_rust_mesh::https::rate` (each decision a call of `sce:std/rate/gcra_admit`, generated into the crate); `sessions.max_live` and `max_inbox`, read by `Pool` | ADR 0109, D1 and D2; section 2 |
| The dialing side of a durable `https` binding in Rust: acknowledges only what a durable checkpoint of its own holds, keeps what it sent until a `200` carried it | `HttpsClient::received`, `HttpsClient::held`, `LinkEvent::Carried`, and the router `Machine::router` builds | ADR 0109, D10; SCE_MESH.md section 10.11 |
| An upgrade in place: a session saved by one version of its document restored into the next by the migration the document declares, from the shape files the author commits | `sce-codegen shape`; `<sce:migration>`, planned by the build and applied by the restore's upgrade (`saved_state::upgrade` in Rust, `upgradeSavedState` in C++) | ADR 0109, D11; SCE Accepted Subset §2.15, "A declared migration"; section 4.2 |
| The values a machine's deployment supplies, checked against the types its document declares | deploy.yaml's `config:` (checked by the build and written beside the machine as `<machine>.config.json`) or `config_file:`; the generated `<Machine>Deployment::read`, which every constructor of the machine takes the result of | ADR 0109, D12; SCE Accepted Subset §2.15, "A value the deployment supplies"; section 2.4 |
| The named bounds of the signing and sealing records, moved by `--limit` and given to the host as the build resolved them | the records' bounds `cose_payload`, `cose_signed`, `cose_plaintext`, `cose_sealed`, `mac_message`, `random_bytes`; `<Machine>Limits`; `NamedBounds` and `ReferenceHost::set_named_bounds` (C++: `CoseFeature(crypto, bounds)`) | ADR 0109, D17; docs/SCE_HOST_FEATURES.md section 10; section 11 |
| The metrics port: what the server, the pool, the store's committer, the reference hosts and the `https` clients report, by fixed names and closed label sets, and the reference adapter that renders the OpenMetrics text exposition | `sce-rust-metrics` (`Metrics`, `openmetrics::OpenMetrics`, `recording::Recording`), given by `HttpsServer::observed`, `Pool::observed`, `ReferenceHost::set_metrics` and `HttpsClient::start_observed`; C++ `host/Metrics.h`, given by `ReferenceHost::setMetrics`, `AuditFeature::setMetrics` and `ClientHost::metrics` | ADR 0109, D16; section 10 |
| Health: whether a part takes work in, is stopping, and reaches its store | `HttpsServer::health`, `Pool::health` (`sce_rust_mesh::health::Health`) | ADR 0109, D16; section 10.4 |
| The members of an open machine: a join offered an incarnation, confirmed by the next exchange, leased, and an exchange under an incarnation the store does not hold fenced `412`; a run kept only under the incarnations it was made under; the person an incarnation acts for | `MembershipStore` of `sce-rust-session-store` (`SqliteStore`, sealed under `Sealed`); `HttpsServer::keep_members` and `keep_principals`, `Listening::admits`; `Pool::with_members`, `Pool::receive_member_for`, `Pool::stand`, `Pool::end_members` | ADR 0113, D3 to D6; ADR 0116; SCE_MESH.md sections 10.9 (invariant 11), 10.11 and 22.3 |
| An outbox per instance of a durable open binding, with its window, ended with its membership | `sce_rust_mesh::outbox::Outboxes` (`Outboxes::end`), kept by the pool and answered from by the server (`HttpsServer::keep_outboxes`) | ADR 0113, D4; SCE_MESH.md sections 10.11 and 22.3 |
| The dialing side of an instance: joins, makes every exchange under its incarnation with its low-water mark, and on a fence tells its machine an unknown outcome and joins anew | `ClientSettings::instance`, `Router::speak_as_instance`, `LinkEvent::Fenced`, `Endpoint::peer_fenced` | ADR 0113, D4, D5 and D7; SCE_MESH.md sections 16.7 row 20 and 22.3 |

The suite of ADR 0107 D10 holds these parts together, under `tests/server`: sessions killed at random points and
restored, two processes against one store, the binding through proxies with responses lost on purpose, a durable
outbox through kills of its commit, acknowledgement and expiry, an orderly stop under load, the edge's rate limits,
an input killed at every write making one entry of the journal or none, and a follower of many sessions killed again
and again repeating entries and missing none. It is a test suite, not a guarantee of a deployment's own composition.

Not built, and a server must not count on them:

- The journal's retention (ADR 0114, D8): a journal keeps every entry, and a reader's mark every position. An
  observed stream's adapter is the application's (D6): no part of SCE calls `JournalStore::observe`.
- A server program (ADR 0107; ADR 0109, "Not in scope"): each deployment writes its own (section 1).
- A store on media several machines share, and an exchange routed to the process that holds its key's lease (ADR
  0109, D18): its direction is decided and it is built only once a deployment measures one active process with a
  standby as not enough (section 6).
- The Windows CI lane of ADR 0107 D11. The WinHTTP adapter is built and has its conformance test
  (`tests/cmake/mesh_https.cmake`), but no CI job runs it.
- A durable receipt on any backend but Rust's: the generated C++ router, whose `https` client holds no envelope for a
  checkpoint, and the Kotlin and Swift host cores refuse `persistence:` and `receipt: durable` by name
  (`mesh/codegen-durable-receipt-unsupported`).
- A client of the `https` binding on the Kotlin and Swift host cores (SCE_MESH.md section 22.8).
- An open machine on any backend but Rust's (ADR 0113, D7), which refuses it by name
  (`mesh/codegen-open-instances-unsupported`).
- The build's refusal of an open machine's binding under `authenticator: key` that names no `sign:` on the
  server's side or no `verify:` on the instance's (ADR 0113, D4): a deployment that declares a document reading
  the person an instance acts for is refused without them (`mesh/codegen-principal-unproved`), and any other is
  not. The program names both.
- An instance's host that restores its router from a checkpoint of its own carries no incarnation across the
  restore: its client joins anew, and what the restored router held of the incarnation before is not ended as a
  fence ends it (SCE_MESH.md section 16.7 row 20).
- The removal of expired and revoked tokens from the credential store: a revoked token is kept so that an issue
  repeated under its identity cannot make it live again, and no call of the store removes one (section 5).
- What ADR 0109 leaves to records or steps of their own (its Open questions 5, 7 and 8): a migration from a state of
  another product-line configuration, a signed payload of megabytes, and a trace context in an envelope.
- The facilities of ADR 0109 on Kotlin, Go, Python, Swift and C11 (its D0): a build for one of them refuses a
  document that asks for one by name.

## 1. What the program supplies

| The program supplies | To | Read in |
|---|---|---|
| The generated machine and its peer table (`sce-codegen generate --deploy`, `--lang rust`) | `Pool::new` | SCE_MESH.md section 19, "Peer table" |
| How a session's machine is started and restored (`SessionFactory`, `SessionMachine`): for a generated machine, `sce_rust_mesh::served::Served` given a `SessionHost` (the policy of each session and the host operations it registers), which owns the engine's clock, registers the endpoint and the host before the first macrostep, and restores through the declared migration; for a machine written by hand, a factory of the program's own, held to the kit `sce_rust_mesh::session_conformance` | `Pool::new` | ADR 0112; `served.rs`, `pool.rs` |
| For a machine that declares values its deployment supplies, those values: `<Machine>Deployment::read` of the `<machine>.config.json` the build wrote from `config:`, or of the file `config_file:` names, read once at start. A text that does not read is the lift's refusal and builds no machine | the `SessionFactory`, which builds every session's machine from it | section 2.4 |
| Its name as the holder of leases, the lease's length and its renewal period (`PoolConfig`: `owner`, `lease_ttl_ms`, `renew_every_ms`) | `Pool::new` | section 4 |
| The session store, opened on a file, sealed under the keys of its key ring; a handle of it for each machine of the deployment that keeps sessions (`SqliteStore::of_machine`, each sealed), since a session is its machine and its key | `Pool::new` | section 3 |
| For a machine that follows journals (`follows:`), the store they are kept in: the file the followed machines' pools write, through a handle or a connection of its own (`SqliteStore::open` on the same file), sealed as they are | `Pool::follow_from` | SCE_MESH.md section 14.4, "The event journal" |
| The key-encryption key (`KeyEncryptionKey::from_file` or `from_variable`), held apart from the store | the key ring the sealing keys are read from, and the host's | sections 3, 3.2 and 8 |
| The authenticator's check (`StoredTokens` or a `BearerCheck`, a `PinStore`, `CertificateCheck`, or `KeyPins` over a `PinStore`), and the credential store it and the machine's host share | `HttpsServer::new`, `ReferenceHost::set_credentials` | section 5 |
| The key that signs every `200` when the binding names `sign` (`ServerSettings::signer`, read from the key ring) | `HttpsServer::new` | section 5, "Message signatures" |
| The server's certificate chain and key, and for `mtls` the client authorities | `server_tls` | section 5 |
| The clock loop: `Pool::due`, `Pool::renew_leases`, `Pool::evict_idle` and `Pool::tick` on its own schedule, `Pool::expire` when a binding is durable, `Pool::end_members` when the machine is bound to an open machine, and `Pool::follow` when the machine follows journals, sleeping until `Pool::next_wake_ms` between them (which counts the outbox entries read in this process as they come to expire, and nothing a followed journal comes to hold: a follower's loop calls `Pool::follow` at an interval of its own, the longest its sessions may lag the journal), and `Pool::run` for each key they answer | the pool | `pool.rs`, module documentation; `pool/follow.rs` |
| The checkpoint each exchange waits for: `ServerHost::deliver` answers the endpoint's `sealed_through()`, and `HttpsServer::durable` is told each checkpoint reported durable. The server calls `deliver` at once for different clients and one at a time for each: a host over a keyed pool runs the sessions it is handed on the calling thread, and one over a single endpoint takes a lock of its own | the server | SCE_MESH.md section 22.8 |
| The standard host features its machine asks for: a `ReferenceHost` over a clock and a transport (`ReferenceHost::new`), or over the key-value store it is given (`ReferenceHost::with_kv_store(clock, transport, store)`, the sealed store of section 3 for one that outlives the process) | the machine, through `sce_rust_host_features::serve` | `docs/SCE_HOST_FEATURES.md` sections 3, 8 and 9 |
| The transport of the host's HTTP feature, built with the destination policy that says where the machine may reach: `NetworkTransport::new(DestinationPolicy::allow_list(..), NetworkSettings::default())`, or `public_internet()` | `ReferenceHost::new`, `with_kv_store` | `docs/SCE_HOST_FEATURES.md` section 5, "Destination policy" |
| The host's stores beside the key-value one, each the sealed store of section 3 for a server: its key ring (`Keyring::new(Box::new(store), Box::new(kek))`), its credentials and its audit log, with the streams it keeps and their policies | `ReferenceHost::set_keyring`, `set_credentials`, `set_audit_store`, `set_audit_streams` | section 3.2; `docs/SCE_HOST_FEATURES.md` sections 13 to 15 |
| The audit log's two passes: retention on the program's schedule (`ReferenceHost::audit_retain(now_ms)`, which deletes each stream's segments older than its `retain_days`), and the signature of every stream's head at the orderly stop (`ReferenceHost::audit_stop(now_ms)`) | the host | section 4.1; `docs/SCE_HOST_FEATURES.md` section 15 |
| The numbers of the named bounds the machine's build resolved, read from the machine's `<Machine>Limits` | `ReferenceHost::set_named_bounds` | section 11 |
| The adapter of the metrics port, and the route its exposition is served on; the readiness and liveness checks | `HttpsServer::observed`, `Pool::observed`, `ReferenceHost::set_metrics` | section 10 |

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
| server | `fresh_ms` | the freshness step a signed request of an instance signs a value of (SCE_MESH.md section 22.9) | none: required on the binding of an open machine under `authenticator: key` | a positive integer that fits 32 bits, beside `authenticator: key` only, on the binding of an open machine only |
| both | `receipt` | `durable` keeps each client's mailbox as an outbox in the session store (section 3), and a client acknowledges only what a checkpoint of its own holds | `transport` | `transport`, `durable`; `durable` needs both machines checkpointed (section 2.3) |
| server | `retain_ms` | how long an outbox entry is kept unless its client acknowledges it | 2592000000 (thirty days) | a positive integer, at least `absence_ms`, beside `receipt: durable` only |
| client | `url` | `https://<host>[:<port>][<base>]`, before the binding's path, with no query or fragment; `http://` only to a loopback host | required | as said |
| client | `authenticator` | as on the server | required | as on the server |
| client | `hold_ms` | how long a poll asks the server to hold it | 25000 | a positive integer that fits 32 bits |
| client | `verify` | the trust set of the client's host every `200` is verified under (section 7) | none: a `200` is taken as it comes | a trust set's name, 1 to 64 bytes |
| client | the retry keys every binding spells | SCE_MESH.md section 10.10; a request that failed at the transport is sent again under them | none | as section 10.10 says |

A value outside the last column is refused when the deployment is read, as `mesh/topology-invalid-binding-field`
naming the key and why; so is `max_skew_ms` or `fresh_ms` beside another authenticator, `fresh_ms` on the binding of a
machine that runs once, and `retain_ms` under `receipt: transport` or below `absence_ms`. The binding of an open machine
under `authenticator: key` that names no `fresh_ms` is `mesh/deploy-invalid-instances`. A key no side of the binding
reads is `mesh/deploy-unknown-binding-field`, with the nearest key it may have meant.

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
  | `new_names` | each name the `install` authenticator, or the `key` authenticator on first use, pins for the first time | the remote address | 20 an hour, burst 20 (`interval_ms: 180000, burst: 20`) |

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
  commit_delay_us: <integer>        # optional, 0: how long a commit waits for more writes
```

`key_from` and `idle_ms` are required when the block is present, and neither has a default; `max_live` and
`max_inbox` are at least 1 (ADR 0109, D1); `commit_delay_us` is at most 100000 (ADR 0109, D15; section 3.1). A
`key_from` that is neither form below, an `idle_ms` past what a Mesh core counts to, a `max_live` or `max_inbox` of 0,
and a `commit_delay_us` past its bound are each refused as `mesh/deploy-invalid-sessions`, with the reason.
`key_from: client` keys a session by the name the client posts under, so one client is one session; `key_from:
data.<field>` keys it by a field of the request's data, which the event schema of every `service.request.*` the
machine handles must declare as a `string[N]` of at most 256 bytes or as an integer
(`mesh/deploy-sessions-key-not-in-request`). The block stands beside no `server:` block, no binding with `ordering:
required` and no binding with `delivery: causal` (`mesh/deploy-invalid-sessions`). Only the Rust host core serves it;
every other backend refuses it (`mesh/codegen-keyed-pool-unsupported`).

`idle_ms` is how long a session stays in memory after its last event. A session is dropped only when it also holds
nothing pending (section 9), so `idle_ms` bounds the memory of quiet sessions and nothing else.

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

### 2.4 What the deployment hands a machine: `config:` and `config_file:`

ADR 0109, D12; SCE Accepted Subset §2.15, "A value the deployment supplies". A document declares each value its
deployment supplies, `<data id="limits" sce:type="record:ServerLimits" sce:from="deployment"/>`, and the machine reads
it and never writes it. The machine's entry gives the values in one of two places:

```yaml
machines:
  ledger:
    source: ledger.scxml
    config:                              # checked when the machine is generated
      limits: { max_rows: 200, label: "main" }
      hosts: [ "a.example", "b.example" ]
    # or: config_file: /etc/ledger/config.json   # read by the program at start
```

`config:` is checked when the machine is generated with the deployment, against the type each value's declaration
states, and written beside the generated machine as `<machine>.config.json`: a value the document declares and the
entry leaves out is `mesh/deploy-config-missing`, a key the document declares no value of is
`mesh/deploy-config-unknown`, and a value that is no value of its type is `mesh/deploy-config-invalid`, naming the
place by its JSON pointer. `config_file:` names a file of the same JSON, the operator's, which the build cannot see: the
program reads it at start through the same reader, and a value that does not read refuses the machine's start with
the lift's code, pointer and bound. An entry with both, a `config_file:` that names no file, and a `config_file:` on a
machine that declares no value are refused as `mesh/deploy-config-invalid`.

The values are part of the machine's interface and not of its state: no checkpoint holds them, so a session restored
after a change of configuration runs on the new values, which take effect at the next start of the process. They are
in the machine's memory and in reach of `<log>`, so a key or a credential is never one: it stays in the host, named by
its id (sections 3.2 and 5). Rust and C++ build a machine from them; every other backend refuses the declaration by
name.

### 2.5 An open set of instances: `instances: open` and `members:`

A machine entry that runs as any number of copies, each an instance no deployment lists, declares (SCE_MESH.md
section 14; ADR 0113, D1, D4 and D6):

```yaml
instances: open
members: { lease_ms: <integer>, provisional_ms: <integer>, max_instances: <integer> }
```

None of the three has a default. A member a session kept a message of lives `lease_ms` past its last exchange; one
no session kept yet, and an offer, `provisional_ms`, at most `lease_ms`; and at most `max_instances` stand at
once. The machine's bindings are `https` and dialed by the instance; the server's binding to it names `fresh_ms`
under `authenticator: key` (ADR 0117, D4), and `sign:` with `verify:` on the instance's side are the program's to
name (section 0). Refusals: `docs/SCE_ACCEPTED_SUBSET.md` section 2.8.

What the program does for it:

- **The server.** It builds the pool of the machine bound to the open one with the store that keeps its sessions
  (`Pool::with_members`), gives the server the same store (`HttpsServer::keep_members`) and the pool's outboxes
  (`HttpsServer::keep_outboxes`), and takes `Listening::admits` as the server's `is_bound`, so no list of
  instances is written. Its `ServerHost::deliver_member` hands each envelope to the pool with the edge's judgement
  and the person it read (`Pool::receive_member_for`), tells the pool a confirmation (`Pool::stand`), and answers
  `Durability::Fenced` when a session found an envelope fenced. Its clock loop calls `Pool::end_members`, which lets
  go of each member whose lease ended, with its records and its outbox.
- **An instance.** Its host draws the instance id once and keeps it, as it keeps a credential; it builds its router
  speaking as the instance (`Router::speak_as_instance`) and its client with `ClientSettings::instance`. The client
  joins, makes every exchange under its incarnation, and on a fence (`412`) tells the machine an unknown outcome
  for each request that left (SCE_MESH.md section 16.7 row 20); the document decides whether to ask again.
- **What a restart does.** The membership, the completion records and the outboxes are in the store, so a server
  started again on it answers a request asked again under a standing incarnation from its record, once; an
  incarnation whose lease ran out while the server was down is fenced.

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
- **Many writes, one sync** (ADR 0109, D15). The pool writes the checkpoints of its sessions, and the acknowledgements
  of its durable clients, in batches (`SessionStore::write_many`): one `IMMEDIATE` transaction, one savepoint per write,
  one sync of the log at the commit. A write refused for its version or its fence, or one the database refuses part
  of the way through, is rolled back to its savepoint and answered alone; the others commit. Section 3.1 says when a
  batch holds more than one write, and how to measure what it buys.
- **Its layout.** The database's `user_version` names the layout, `13` today. A file of layout `1` to `12` is brought
  to `13` when it is opened, in one transaction, its sessions, leases, keys, credentials, outboxes, values, audit log,
  journal and members kept; layout 12 keeps a session by its machine and its key (ADR 0114, D2), and the sessions of
  an earlier file are the one machine's the empty name stands for, which `SqliteStore::open` gives a handle of;
  layout 13 keeps who each fact of the journal was said for (ADR 0116, D7), and an entry of an earlier file was said
  for nobody. A file of a layout this build does not know, or one that holds tables the store did not make, is
  refused. A later build that changes the layout makes a rollback to an earlier build refuse the file: a build before
  layout 13 refuses a file this one opened.
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
  (`Keyring::new(Box::new(SqliteStore::open(path)?), Box::new(kek))`, `kek` the key-encryption key of section 3.2).
  Every value the sealed store writes is a record of XChaCha20-Poly1305 under a sealing key, bound to its store and
  its row, and every key (a session's key, a client's name, a key's id, a token's digest) is kept as its **entry**,
  the HMAC-SHA-256 of the key under the store's index key. The sealed store is the one handed to `Pool::new`, to the
  server's authenticator and to the host (`ReferenceHost::with_kv_store`, `set_credentials`, `set_audit_store`, and
  its key ring's store): it is a `SessionStore`, a `CredentialStore`, a `KeyStore`, a `KvStore`, an `AuditStore` and
  a `JournalStore`, as the file is, so one `Arc` of it serves the pool, the authenticator and each store the host
  takes by `Arc`, and the key ring, which owns its store, takes a handle of its own (section 3.2). A store opened
  without the sealing keys reads every row as a record it cannot open; a record moved to another row, changed by a byte, or read under
  another key is refused (`Failure`), and the pool answers that session's requests as it answers a store that
  failed.
- **What the file shows.** Every value is a record, and every key an entry; in the clear are what the store compares
  or sets: a session's machine, version, earliest due and lease (owner, fence, expiry), a token's revocation, a key's
  use, algorithm, creation and state, an outbox's incarnation and numbers, a value's version, the audit log's
  sequences, moments and chain, and the journal's positions and recorded moments (the table below). The index key
  (`index:sealed`) and the sealing keys (`sealing:<hex>`) are rows of the `keys` table wrapped under the key-encryption key, the one thing the sealed store
  keeps as it is given.
- **Rotation.** `SealingKeys::rotate(&keyring, now_ms, &mut SystemRandom)` makes a new sealing key and retires the
  one before, which still opens what it sealed. Give each process the keys read again (`Sealed::set_keys`, or a
  restart), then run the re-seal passes: `Sealed::reseal_sessions(owner, lease_ttl_ms, now_ms)`, which takes each
  session's lease under the name `owner` and writes it again from the version it read, leaving one another process
  holds; `Sealed::reseal_credentials()`; `Sealed::reseal_values()`, the key-value feature's;
  `Sealed::reseal_outboxes()`, every durable client's outbox and every expiry's note; `Sealed::reseal_audit()`, every
  audit row's entry and every checkpoint's signature, each row's sequence, moment and chain kept, so a reader verifies
  the stream as before; `Sealed::reseal_journal()`, every journal entry's data, its position and its moments kept, so
  no answer changes; and `Sealed::reseal_keys()`, never beside a rotation of the key-encryption key. Each writes a
  row again by the store's replacement from the bytes it read, so a row another process changed or deleted in between
  is left, not overwritten. Each answers `Resealed`: rows written again (`resealed`), rows `left` (run the pass again
  later), rows `refused` (they do not open: changed on the medium, or sealed under a key the process was not given).
  Once every pass answers no row left and none refused, the retired key opens nothing the store holds and may be
  destroyed; a backup carries the keys its own rows were sealed under (section 8). Each record names the key that
  sealed it in the clear, after its magic (`docs/SCE_SEALED_RECORD.md` section 1), so a read confirms it:
  `SELECT count(*) FROM audit_rows WHERE CAST(substr(entry, 14, 40) AS TEXT) = '<the retired id>'`, and the same of
  `sessions.state`, `outbox_entries.envelope`, `outbox_notes.envelope`, `audit_checkpoints.signature`,
  `journal_entries.data` and the other sealed columns of the table below. The index key is never rotated.
- **What sealing does not give.** Freshness against someone who can write the medium: a row replaced by an earlier
  sealing of the same row opens, and a token's revocation, kept in the clear, can be set back by hand. The lease's
  fence and the version stop a process that is wrong, not a medium written by hand; keep the file where only the
  server's account writes it.

The tables, by the layout that added them (the `LAYOUT_` constants of `sqlite.rs`), and what each column holds under
a sealed store (`docs/SCE_SEALED_RECORD.md` section 3). An entry is the keyed digest of a key, a record a sealed
value; a column the last column does not name is kept in the clear, since the store compares or sets it.

| Layout | Tables | Under a sealed store |
|---|---|---|
| 1, the sessions (ADR 0107) | `sessions` (`session`, `state`, `version`, `earliest_due`); `leases` (`session`, `owner`, `fence`, `expires_ms`); `numbers`, the one counter versions and fences are drawn from | `session` an entry, `state` a record |
| 2, the key ring (`KeyStore`, docs/SCE_HOST_FEATURES.md section 13) | `keys` (`id`, `key_use`, `algorithm`, `wrapped`, `label`, `created_ms`, `state`), each key's material wrapped under a key-encryption key the file never holds; `key_generations` (`generation`, `key`), which key each generation made; `key_naming`, the wrapped naming key of derived keys | `id`, `generation` and `key` entries, `wrapped` and the naming key records, `label` empty; the sealed store's own keys (`index:sealed`, `sealing:<hex>`) are kept as they are given, wrapped |
| 3 and 4, the credentials of section 5 (`CredentialStore`) | `credential_pins` (`name`, `digest`); `credential_tokens` (`digest`, `name`, `issued_ms`, `expires_ms`, `revoked`, and from layout 4 `sealed`); each digest the SHA-256 of a secret the file never holds | `name` an entry, a pin's `digest` and a token's `sealed` records; a token's `digest` the keyed digest of its digest, its moments 0 |
| 5, the outboxes of section 2.3 | `outbox_heads` (`client`, `incarnation`, `next`, `touched_ms`); `outbox_entries` (`client`, `number`, `at_ms`, `sender`, `envelope`); `outbox_notes` (`id`, `session`, `client`, `number`, `at_ms`, `envelope`), an expiry's note for the session that sent the entry | `client`, `sender` and a note's `session` entries, `envelope` a record |
| 6, the key-value feature (`KvStore`, docs/SCE_HOST_FEATURES.md section 3) | `kv_values` (`key`, `value`, `version`); `kv_numbers`, the feature's own counter of versions; `kv_first_results` (`session`, `invoke`, `start`, `outcome`, `version`), the first result of each put-if by its invocation's identity, deleted with the session that started it | `key`, and a first result's `session` and `invoke`, entries; `value` a record |
| 7, the audit log (`AuditStore`, docs/SCE_HOST_FEATURES.md section 15, docs/SCE_AUDIT_LOG.md) | `audit_heads` (`stream`, `sequence`, `chain`), each stream's last row, which retention leaves; `audit_rows` (`stream`, `sequence`, `at_ms`, `entry`, `chain`), never updated and deleted only with their segment; `audit_checkpoints` (`stream`, `sequence`, `chain`, `signature`); `audit_identities` (`session`, `invoke`, `start`, `sequence`), the sequence each append's identity was appended at, deleted with its session | `stream`, and an identity's `session` and `invoke`, entries; a row's `entry` and a checkpoint's `signature` records; the chain is kept as it is, so the log is verified without the sealing keys |
| 8, the event journal (`JournalStore`, ADR 0114) | `journal_head` (`issued`, `recorded_ms`), the last position given and the last moment recorded; `journal_entries` (`position`, `recorded_ms`, `name`, `key`, `origin`, `schema`, `revision`, `valid_ms`, `holds_ms`, `data`), a session's entry under its machine's `name` and its `key`, an observed stream's under its `name` and the other system's `origin`; `holds_ms` the valid time, or the recorded moment where there is none; never deleted | `name`, `key` and `origin` entries, `data` a record; `schema` empty, `revision` 0, `valid_ms` null and `holds_ms` the recorded moment |
| 12, a session by its machine and its key, and the journal's readers (ADR 0114, D2 and D4) | `sessions` and `leases` keyed by `machine` and `session`, the rows of an earlier file under the empty `machine`; `outbox_notes` and `outbox_entries` gain `machine`, that of the session a note is for or an entry was sent by; `journal_readers` (`reader`, `position`), how far each reader of the journal has taken in every entry it reads, which only rises | `machine` kept as the handle names it, and a session's record bound to it (`docs/SCE_SEALED_RECORD.md` section 3); a reader's `reader` an entry |
| 13, who a fact of the journal was said for (ADR 0116, D7) | `journal_entries` gains `actor_instance`, `actor_issuer`, `actor_subject`, `actor_proved_ms` and `actor_method`: the instance whose event the run that said the fact took in and the person its incarnation acted for, all five or none; none for an observed entry and for an entry of an earlier file | all five null: who a fact was said for is inside the entry's `data` record |

Every moment is milliseconds since the Unix epoch.

### 3.1 Group commit

ADR 0109, D15; SCE_MESH.md sections 10.11 and 22.8. With `synchronous=FULL` every commit waits for the disk, so a
pool that wrote one checkpoint at a time would take as many inputs a second as the disk takes syncs. The pool instead
hands each write to its committer (`Pool::committer`), which writes everything handed to it since its last commit in
one transaction and answers each write once that transaction is durable. A commit starts as soon as the one before it
ended, and its batch is what arrived meanwhile: under load a batch holds as many writes as arrived while the previous
one was synced, and alone a write is committed at once, so nothing waits that would not have waited anyway. A reply is
released, and a `200` answered, only after the commit that holds it, as before.

- **What fills a batch.** Sessions of different keys that run at once. The `https` server tells its host the exchanges
  of different clients at once and those of one client one at a time (`ServerHost::deliver`), so a host that runs
  each session it is handed on the calling thread, as the pool's hosts do, lets many clients' checkpoints share a
  commit. One client's inputs are one session's, and are committed one after another.
- **`sessions.commit_delay_us`** (default `0`, at most `100000`): how long a commit waits before it takes its batch,
  as PostgreSQL's `commit_delay`. Leave it at `0` unless a measurement below shows that batches stay small under the
  load the server carries and the disk's sync is what bounds it; every reply then waits that much longer.
- **What it does not change.** Each write's durability, its version and fence checks, and what a crash loses: a
  process killed before a commit ended has released nothing that commit held, and its clients send again.

**Measuring it.** The suite has a measurement that is not run by default:
`tests/server/tests/group_commit_throughput.rs`. Each client is a thread with a key of its own that sends requests to a
pool on the reference store, one after another; one client is the serial baseline. Run it on the machine and disk the
server will run on, with nothing else writing to that disk:

```sh
SCE_SERVER_SUITE_CLIENTS=1,4,16,64 SCE_SERVER_SUITE_REQUESTS=200 \
  cargo test --release -p sce-server-suite --test group_commit_throughput -- --ignored --nocapture
```

It prints, for each count of clients, the inputs, the transactions they cost, the inputs a transaction held and the
inputs taken in a second. Its scratch store is under the system's temporary directory; point `TMPDIR` at the disk the
store will live on. Record the result here, with the machine, the file system and the date:

Measured 2026-11-20 on a build machine, not a server's: 32 cores (Intel Core i9-14900KF), the store on btrfs over a
SATA SSD (Samsung 870 EVO), with other builds running on the machine. The shape is what carries over: the commits
stay near what the disk syncs in the run's time while each commit holds more inputs as clients are added.

| Clients | Inputs | Transactions | Inputs per transaction | Inputs per second |
|---|---|---|---|---|
| 1 | 200 | 200 | 1.00 | 130 |
| 4 | 800 | 402 | 1.99 | 335 |
| 16 | 3200 | 411 | 7.79 | 992 |
| 64 | 12800 | 401 | 31.92 | 2539 |

### 3.2 The key ring and its key-encryption key

ADR 0109, D7 and D8; docs/SCE_HOST_FEATURES.md section 13. Every key a server keeps in its store is a row of the
`keys` table, its material wrapped under a **key-encryption key** the store never holds: the keys a machine generated
(`generated:<hex>`) and derived (`derived:<hex>`), the key tokens are issued under (`issuing:credentials`, section 5),
the audit log's signing and pseudonymisation keys (`audit:sign`, `audit:pseudonym`), and a sealed store's index key
and sealing keys (`index:sealed`, `sealing:<hex>`, section 3). A `Keyring` (`sce_rust_crypto::keyring`) reads and
writes them through a `KeyStore` and the `KeyWrap` port.

- **The key-encryption key.** 32 bytes the program supplies. The reference adapter, `KeyEncryptionKey` of
  `sce_rust_crypto::wrap`, reads them from a file the operator names (`KeyEncryptionKey::from_file(path)`: the 32
  bytes, or their 64 hexadecimal digits) or from an environment variable (`KeyEncryptionKey::from_variable(name)`:
  the 64 digits), and wraps each key with XChaCha20-Poly1305 under a 192-bit random nonce, bound to the key's id and
  use, so a row's material moved to another row does not unwrap. It is never printed. A key management service
  serves the same port by implementing `KeyWrap`.
- **Two rings on one file.** The sealed store's keys are read from a ring over the file itself (section 3,
  `SealingKeys::from_keyring`), and the host's from a ring over the sealed store, given with
  `ReferenceHost::set_keyring(Keyring::new(Box::new(Sealed::new(SqliteStore::open(path)?, keys.clone())),
  Box::new(kek)))`, so its keys' ids and labels are entries and records like every other row. Each ring takes the
  key-encryption key read for it.
- **Without a store.** A host given no key ring keeps one in memory (`Keyring::in_memory`) under a key-encryption key
  drawn for the process: its keys, its issuing key and its audit keys are gone when the process stops (section 4).
- **Rotating the key-encryption key.** `Keyring::rewrap_all(Box::new(new_kek))` on the ring over the sealed store,
  which holds the sealed store's own keys as they are given and opens every other row, wraps every key and the naming
  key under the new one, each in a write of its own, leaving a key that already unwraps under it; a rotation that
  stopped part of the way is finished by running it again with the same two keys. A ring over the bare file reads
  the host's keys as records it cannot unwrap and stops at the first. Run it with every other process of the store
  stopped and no re-seal pass running, then give each process the new key. A backup wrapped under the old key needs
  the old key to restore (section 8).
- **Retiring a key.** A machine retires the keys it generated (`sce-host-key-retire`); the operator retires the
  issuing key or an audit key with `Keyring::retire(id)` on the host's ring, after which it makes nothing new and
  still verifies and opens what it made (`key_retired`, docs/SCE_HOST_FEATURES.md sections 13 to 15). No call destroys
  a key: destroying one is the operator's, once nothing in the store or its backups was made under it.

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
- **What the host kept in memory is gone.** A host given no store keeps each of its own in memory: pins and tokens in
  `MemoryCredentials` (section 5), the key-value store of `ReferenceHost::new` with its first results of a put-if,
  the key ring of `Keyring::in_memory` (section 3.2), so a `generated:` or `derived:` id a restored session holds
  names no key, and the audit log. The COSE keys the host is configured with (`ReferenceHost::set_cose_keys`) are the
  program's to give again. A host given the sealed store of section 3 for each keeps all of it.

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

A host that keeps an audit log signs the head of every stream it keeps once the pool has stopped,
`ReferenceHost::audit_stop(now_ms)`, so that a cut of the rows written after the last periodic checkpoint is seen by
a reader (docs/SCE_HOST_FEATURES.md section 15). From the moment the server's stop begins,
`server.health().and(pool.health()).ready()` is false, so a load balancer that reads it sends the process nothing
more (section 10.4).

### 4.2 Upgrading in place

ADR 0109, D11; SCE Accepted Subset §2.15, "A declared migration". A server is upgraded by stopping it in order
(section 4.1) and starting the next build on the same store; each session is migrated when it is loaded, at its next
request or due, and saved under its new shape at its next checkpoint. A server that serves its generated machine
through `sce_rust_mesh::served` (section 1) restores each session through the migration by that adapter; the server
suite holds the whole move under load, a SIGKILL partway through it and a rollback the old build refuses
(`tests/server/tests/a_server_upgraded_in_place_under_load_loses_nothing.rs`; ADR 0112).

- **What decides whether a session restores.** A saved state names the shape of the document that saved it: its
  states with their kinds and parents, its histories, invocations and variables with their types and bounds. A
  change of a guard, an action, an initial value or a comment leaves the shape, and a session restores as it is. A
  state, history, invocation or variable renamed, re-typed, re-parented or re-bounded changes it, and a session saved
  before the change restores only through a migration the new document declares.
- **What a release commits.** `sce-codegen shape <document> -o shapes/<version>.shape` writes the shape of each
  version released; the next version declares each step from one released shape to the next, `<sce:migration
  from="shapes/2.shape" to="shapes/3.shape">` with its renames, mapped states, dropped and added variables,
  conversions and dropped invocations. The build refuses a chain that is broken, a step that is not total, and a type
  carried with no widening or conversion (`validation/migration-*`), so a build that generates is one that migrates
  every state a released version saved.
- **What is refused.** A session whose saved shape no step starts from is not restored: each request for its key is
  `PoolError::NotBuilt` and answered `RpcStatus::InternalError` (section 9), and its checkpoint is left as it is for
  a build that reads it. The old build refuses a state the new one saved, by its shape, so the old and the new build
  never serve one store at once, and a rollback after a session was saved under the new shape needs a migration
  back, which the old build does not have. A change of the store's layout (section 3) is a second reason a rollback
  is refused.
- **What is not migrated.** The Mesh endpoint's sections of a checkpoint, keyed by the deployment's machine names,
  which a migration does not rename: keep the names of deploy.yaml's machines across an upgrade. A state of another
  product-line configuration (ADR 0057 §7). A deployment value (section 2.4), which no checkpoint holds.
- **A moved bound is a change of shape.** A build that moves a named bound (`--limit`, section 11) moves the bound of
  every variable that names it: raising one is a widening, which a step from the old shape carries with no element;
  lowering one is refused unless the step drops or converts the variable.

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
  whose server keeps it (under `receipt: durable`, a cache of the client's outbox in the store, whose held exchanges
  and reservations are still that process's), and routing a key to the process that holds its lease is not built
  (ADR 0107, Open question 2; ADR 0109, D18, which builds it only on measured need). Send all traffic to the active
  process, and move it to the standby only once the active one is gone.
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
| The session store: `<file>`, with its `-wal` and `-shm` | every session of every machine, its replies and its timers; every durable client's outbox; the credentials; the key ring's keys, wrapped; the key-value feature's values with their first results; the audit log; the event journal, with how far each reader of it has read | a consistent copy while the server runs, by SQLite's own backup (the `sqlite3` shell's `.backup`, or `VACUUM INTO` on SQLite 3.27 or later); a plain file copy only while no process has the store open. The store has no backup command of its own. |
| The server's TLS key and certificate chain | the clients' trust in the server | as the program keeps them |
| The client authorities of `mtls`, and the material a `BearerCheck` of the program's own verifies with | who may claim a name | as the program keeps them; the pins and tokens of the credential store are in the session store's file, backed up with it |
| The key-encryption key of the key ring (section 3.2) | the sealing keys and the index key of a sealed store (section 3), the issuing key of tokens, the audit keys, and the keys a host generated and derived: without it a copy of the store opens nothing, and with it beside the copy the copy is in the clear | apart from the store and from its backups, which hold every key it protects wrapped and none of them in the clear: in the program's secret store or a key management service, with a copy kept where a loss of the server's machine does not reach it |
| The COSE keys the host is configured with: signing seeds, sealing keys, trust sets (`docs/SCE_HOST_FEATURES.md` section 10) | what the machine signs and opens | as the program keeps them; never in the store, since a checkpoint holds no key (SCE_MESH.md section 10.11) |
| deploy.yaml, the documents, and the build of the program | what the store's checkpoints are a state of | with the release |

Nothing else of a server outlives its process: its mailboxes under `receipt: transport`, and whatever its host keeps
in a store of its own in memory (`MemoryCredentials`, the key-value store of `ReferenceHost::new`, a key ring of
`Keyring::in_memory`, the audit log a host was given no store for) are in memory (section 4).

A store sealed at rest (section 3) keeps its sealing keys and its index key in its own `keys` table, so a backup of
the file carries the keys its rows were sealed under, wrapped, whatever was rotated since: what restores it is that
file and the key-encryption key it was wrapped under, never one without the other. A rotation of the key-encryption
key rewraps the live store's keys and no backup's, so keep each key-encryption key as long as a backup wrapped under
it may be restored.

Restoring a copy of the store moves every session back to the moment of the copy. Stop every process that uses the
store first, and start them after: a process left running holds leases and fences from the newer store, and the copy's
counter of versions and fences is behind the numbers it handed out since. After the restore a session restores from a
checkpoint that is not clean and raises `restored`, and a request it answered after the copy is no duplicate any more:
sent again, it is handled again. The rest of the file moves back with it: a token issued or a key generated after the
copy is gone, a revocation made after it is undone, a durable client's outbox holds what it held at the copy,
the audit log ends at the copy's last entry, and so does the event journal: the positions it gave after the copy are
given again to the entries made after the restore, while each reader's mark and the positions each follower's sessions
took in are the copy's, in step with the journal they read.

## 9. A stuck session or an idle one

What the program can read:

- `Pool::is_live(key)`: whether the session is in memory.
- `Pool::take_errors()`: what went wrong that only the host can act on, in order (`PoolError` in `pool.rs`).
- `Pool::next_wake_ms()`: the earliest moment the pool has work, and so the moment the clock loop must call it again.
- `Pool::health()` and `HttpsServer::health()`: whether each takes work in, is stopping, and reaches its store
  (section 10.4).
- The metrics of section 10: the sessions in memory, what was loaded, restored and evicted, what was shed and why,
  the lease renewals refused, the outboxes' depth and expiries, and how long each commit took.
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

## 10. What the server reports: metrics and health

ADR 0109, D16. The parts of a server report what they do through a port, not a library: the `Metrics` trait of
`sce-rust-metrics` in Rust (`count`, `adjust`, `observe`) and the `SCE::Metrics::Metrics` interface of
`host/Metrics.h` in C++ (`count`, `observe`). SCE links no metrics library; the program gives each part the adapter it
chooses, and a part given none reports to `NoMetrics`, which reports nothing, so every constructor that takes no
adapter works as before.

| Part | Given the adapter by | Reports |
|---|---|---|
| The `https` server | `HttpsServer::observed(settings, authenticator, is_bound, host, metrics)` | `sce_server_*` |
| The keyed pool, its committer and its durable clients' outboxes | `Pool::observed(machine, config, factory, store, transport, environment, metrics)` | `sce_pool_*`, `sce_store_*` |
| The Rust reference host | `ReferenceHost::set_metrics(metrics)` | `sce_host_*` |
| The C++ reference host and its audit feature | `ReferenceHost::setMetrics(metrics)`, `AuditFeature::setMetrics(metrics)` | `sce_host_*` |
| The Rust `https` client | `HttpsClient::start_observed(settings, port, events, metrics)` | `sce_client_exchanges` |
| The C++ `https` client | `ClientHost::metrics` | `sce_client_exchanges` |

### 10.1 The registry of names

Every series is a family of the table below with a value for each of its labels. The names are fixed, and every
label value is a variant of a closed enum of the port (`ResponseStatus`, `ExchangeOutcome`, `ServerShed`, `PoolShed`,
`AuthenticatorKind`, `RestoredFrom`, `Eviction`, `HostOperation`, `HostFailure` in Rust, and their counterparts in
C++), so no series carries a client's name, a session's key or any other text a client chose: a caller has no way to
hand a label a text of its own. Each enum holds only the values its family's events produce: the server builds every
response from a `ResponseStatus`, and a client's `ExchangeOutcome` adds the two outcomes only a client meets. The registry is a wire surface, `sce-metrics-1` (SCE_WIRE_CONTRACTS.md, "Server
metrics"), `pre-release`: a dashboard or an alert written against it reads the registry's name from
`sce_metrics_info`, and a change that renames or removes a family, a label or a value names a new registry. In Rust
the registry is `sce_rust_metrics::FAMILIES`; `host/Metrics.h` holds the families the C++ side reports, value for
value.

| Family (as exposed) | Type | Labels and their values | Moved by |
|---|---|---|---|
| `sce_metrics_info` | info | `registry`: `sce-metrics-1` | always 1, rendered by the reference adapter |
| `sce_server_exchanges_total` | counter | `status`: `200`, `400`, `401`, `404`, `405`, `408`, `412`, `413`, `415`, `429`, `503` | each exchange the server answered, by the status of its answer |
| `sce_server_held_exchanges` | gauge | none | an exchange held open while its client's mailbox is empty, from its first wait until it is answered or its connection drops |
| `sce_server_sheds_total` | counter | `reason`: `rate`, `concurrency`, `max_clients`, `max_connections`, `max_connections_per_address`, `durability`, `store`, `stopping`, `random_source`, `max_instances` | each `429` and each `503` the server answered, and each connection it closed at accept, by why (below) |
| `sce_server_authentication_failures_total` | counter | `authenticator`: `bearer`, `install`, `mtls`, `key` | each `401`, by the authenticator that refused it |
| `sce_pool_live_sessions` | gauge | none | a session brought into memory, and one dropped from it (finished, evicted, lost, stopped) |
| `sce_pool_loads_total` | counter | none | each session brought into memory, started or restored |
| `sce_pool_restores_total` | counter | `checkpoint`: `clean`, `not_clean` | each session restored from its last durable checkpoint; a `not_clean` one raises `restored` (SCE_MESH.md section 16.7 row 17) |
| `sce_pool_evictions_total` | counter | `reason`: `idle`, `max_live` | each session saved clean and dropped: idle past `idle_ms`, or to make room at `sessions.max_live` |
| `sce_pool_sheds_total` | counter | `reason`: `max_live`, `max_inbox`, `stopping` | each envelope shed before a session took it in (`PoolError::Shed`, or `Unavailable`, or a lost receipt) |
| `sce_pool_lease_renewals_refused_total` | counter | none | each lease renewal the store refused: a key this process lost (`PoolError::LeaseLost` from `Pool::renew_leases`) |
| `sce_pool_outbox_depth` | gauge | none | the envelopes the process holds of its durable clients' outboxes: read from the store at first use, appended, acknowledged, expired |
| `sce_pool_outbox_expiries_total` | counter | none | each outbox entry deleted unacknowledged past `retain_ms` (`Pool::expire`) |
| `sce_store_commit_duration_seconds` | histogram, seconds | none; buckets 0.0005 to 30 | each commit of the committer, durable on return (section 3.1) |
| `sce_store_commit_batch_size` | histogram | none; buckets 1 to 1024 | the writes and acknowledgements each commit held |
| `sce_host_operation_duration_seconds` | histogram, seconds | `type`: each standard host operation's type (`sce-host-kv-get`, ...) | each start of an operation the reference host answered at once; a start it holds is not observed |
| `sce_host_operation_failures_total` | counter | `type` as above; `code`: `unavailable`, `quota_exceeded`, `network`, `response_too_large`, `invalid_body`, `destination_refused`, `busy`, `context_exceeded`, `answer_too_large`, `declined`, `unknown_key`, `key_retired`, `unknown_stream`, `invalid_argument`, `other` | each start answered with a failure code, or refused (`invalid_argument`); a code no operation names (one injected) is `other` |
| `sce_host_audit_appends_total` | counter | none | each entry appended to the audit log; a repeat under one identity appends none |
| `sce_client_exchanges_total` | counter | `status`: the server's eleven, `other` and `none` | each exchange an `https` client made, by the status its server answered; `other` for a status no SCE server answers (an intermediary's), `none` when no response came |

Why a server sheds, each a value of `reason`:

- `rate`: a limiter of the edge refused the exchange (`rate:`, section 2.1), `429`;
- `concurrency`: `max_exchanges` in flight (`503`), or `max_exchanges_per_client` for the client (`429`);
- `max_clients`, `max_connections`, `max_connections_per_address`: the bounds of ADR 0109 D1 (`503`, or the connection
  closed at accept);
- `durability`: the envelopes the exchange carried were not made durable, or not within `max_hold` (`503`). An envelope
  the pool shed at one of its own bounds is counted in `sce_pool_sheds_total` where the pool shed it, and its exchange
  here, since the pool told the binding it is not durable;
- `store`: a store the exchange reads could not be read or written: the pin store of `install` or `key`, or a durable
  client's outbox (`503`);
- `stopping`: the server is stopping (`503`, section 4.1);
- `random_source`: no incarnation could be drawn for a mailbox (`503`).

So each `429` and each `503` the server answers is one shed, and the sum of `sce_server_sheds_total` less its
connection reasons is the sum of `sce_server_exchanges_total` over `429` and `503`.

### 10.2 The reference adapter and its route

`sce_rust_metrics::openmetrics::OpenMetrics` keeps every series in memory and renders the OpenMetrics 1.0 text
exposition (`render()`), which the program serves on a route of its own, with the media type
`sce_rust_metrics::openmetrics::CONTENT_TYPE` (`application/openmetrics-text; version=1.0.0; charset=utf-8`). A family
of no label is rendered at zero from the start; a family with labels shows only the series that moved. The same
adapter is handed to every part, and the gauges of parts that share it add up.

```rust
use std::sync::Arc;
use sce_rust_metrics::openmetrics::{OpenMetrics, CONTENT_TYPE};

let metrics = Arc::new(OpenMetrics::new());
let pool = Arc::new(Pool::observed(machine, config, factory, store, transport, environment, metrics.clone())?);
let server = HttpsServer::observed(settings, authenticator, is_bound, host, metrics.clone())?;
reference_host.set_metrics(metrics.clone());
// On a listener of the program's own (here 127.0.0.1:9443, apart from the binding's `listen`), a `GET /metrics`
// is answered `200` with `Content-Type: CONTENT_TYPE` and the body `metrics.render()`.
```

Keep the route off the address the clients reach: it shows how the server is used, which is the operator's to read.
A program on another metrics stack (a Prometheus client, OpenTelemetry) implements `Metrics` over it instead, with
the same names, and chooses its own buckets.

### 10.3 What to watch

- **Load shed**: a rising `sce_server_sheds_total` by `reason` says which bound is met; `rate` and
  `max_connections_per_address` under an attack, `max_clients` or `sce_pool_sheds_total{reason="max_live"}` when the
  server is too small for its clients, `durability` and `store` when the store is slow or failing.
- **Durability**: `sce_store_commit_duration_seconds` is the disk's sync; `sce_store_commit_batch_size` shows whether
  group commit fills (section 3.1).
- **Restarts that were not clean**: `sce_pool_restores_total{checkpoint="not_clean"}` counts the restores that raised
  `restored`; after an orderly stop (section 4.1) it stays flat.
- **A process that lost its keys**: `sce_pool_lease_renewals_refused_total` above zero (section 6).
- **Clients that do not come back**: `sce_pool_outbox_depth` rising and `sce_pool_outbox_expiries_total`.
- **Guessing**: `sce_server_authentication_failures_total` with `sce_server_sheds_total{reason="rate"}`.

### 10.4 Readiness and liveness

`HttpsServer::health()` and `Pool::health()` answer a `Health`: `accepting` (it takes work in; false from the moment a
stop begins), `stopping` (a stop has begun and has not ended: the server still has an exchange in flight, the pool is
in `Pool::stop`) and `store_reachable` (the store it keeps its state in answered a read just now: the pool's store, and
for the server the pool's store when it answers durable clients and the pin store of `install` and `key`).
`Health::and` joins the two.

- **Readiness**: `server.health().and(pool.health()).ready()`, true while both accept and reach their stores. A load
  balancer that reads it sends nothing to a process that stops or cannot reach its store.
- **Liveness**: that `health` answered at all. A store that does not answer makes the process unready, not dead:
  restarting it reaches the same store.
- **Stopped**: `Health::stopped()` once both have stopped (section 4.1); the program may exit then.

`health` reads the store, and blocks for as long as the store does: a program on tokio calls it on the blocking pool
(`tokio::task::spawn_blocking`), as it calls `Pool::stop`.

## 11. The bounds of signing and sealing

ADR 0109, D17; docs/SCE_HOST_FEATURES.md sections 8 and 10 to 12. What a machine signs, seals and tags, and what it
draws, is bounded by named bounds of the library's records, each a number of bytes:

| Bound | What it bounds | Library's number |
|---|---|---|
| `cose_payload` | what `sce-host-cose-sign1` signs and `sce-host-cose-verify1` answers | 4096 |
| `cose_signed` | the object a signing makes and a verification reads | 4352 |
| `cose_plaintext` | what `sce-host-cose-encrypt0` seals and `sce-host-cose-decrypt0` answers | 4352 |
| `cose_sealed` | the object a seal makes and an opening reads | 4480 |
| `mac_message` | the message `sce-host-mac` tags and `sce-host-mac-verify` verifies | 4096 |
| `random_bytes` | the most bytes `sce-host-random-bytes` draws | 256 |

- **A build moves them.** `sce-codegen --limit cose_payload=8192 --limit cose_signed=8448 generate ...` moves each
  bound in the records and in every variable of the machine that names it, and a build given no limit keeps the
  library's numbers. Two relations hold, which the build checks on every `<invoke>` of an operation that reads both:
  `cose_signed` is at least `cose_payload` and 256, and `cose_sealed` at least `cose_plaintext` and 128
  (`sce_build::forge::standard_host::BOUND_RELATIONS`); a build that breaks one is refused as
  `codegen/standard-host-bound-relation`, naming the two bounds, their numbers and the `--limit` that repairs it.
- **The host is given the build's numbers.** The machine declares each bound it names as a constant of the number
  its build resolved (`<Machine>Limits::COSE_PAYLOAD` and its neighbours). The program gives the host those numbers,
  `ReferenceHost::set_named_bounds(NamedBounds { cose_payload: <Machine>Limits::COSE_PAYLOAD as usize, ..,
  ..NamedBounds::LIBRARY })` (C++: `CoseFeature(crypto, bounds)` with a `NamedBounds` of `host/CoseHost.h`); a host
  given none judges by `NamedBounds::LIBRARY`. A host given numbers below its machine's refuses requests the machine
  may make, and one given numbers above them may answer with an object the machine's record cannot hold, so the two
  are given one build's numbers.
- **Moving a bound of a running server** changes the shape of every machine that names it in a variable: a raised
  bound is a widening the next version's migration step carries with no element, and a lowered one is no widening,
  so the build refuses the step (`validation/migration-type-change`) unless it drops or converts the variable
  (section 4.2). The bounds of the `https` binding (`max_body_bytes`, `max_response_bytes`, section 2.1)
  are apart from these and are moved in deploy.yaml.
- **Not this mechanism.** A payload of megabytes, a detached payload over a stored blob, is a record of its own (ADR
  0109, Open question 7).
