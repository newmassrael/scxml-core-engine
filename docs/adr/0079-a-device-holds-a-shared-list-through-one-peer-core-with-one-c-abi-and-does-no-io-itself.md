# ADR 0079 — A device holds a shared list through one peer core with one C ABI, and the core does no I/O itself

- Status: Accepted (2026-11-02). Written and accepted at the owner's order of that day, under the rule of
  2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored. The
  order of work below starts with ADR 0042's own steps, which this ADR does not change.
- Date: 2026-11-02
- Related:
  - ADR 0034 (a shared list is held by its members, and the hub only relays), whose section 6 already rules that
    cryptography is one Rust implementation behind a C ABI;
  - ADR 0042 (the shared record list is a standard part), steps 2 and 3;
  - ADR 0043 (a child session is a machine of the deploy, and its parent gives it its mesh endpoint);
  - ADR 0046 (a change to a shared list is one unit);
  - `docs/SCE_SHARED_LIST_TRUST.md`, `docs/SCE_SHARED_LIST_MLS.md` (section 6, the other lanes);
  - the owner's decision of 2026-10-18 that the Mesh's transport for deployment is a downstream zenoh
    implementation (called here the deployment transport).
- Scope: what a device runs to hold a shared list with other devices through a relay: the boundary between the
  machine (generated in the application's language) and the host's trust, key and relay code; the one C ABI an
  application lane links; how the relay is reached; where a device's secrets live; and the order in which the
  pieces that exist are joined. Not in scope: the relay's own storage and hosting, the cryptographic review, the
  user interface of linking a device.

## Context

Every piece a device needs to hold a shared list exists, and they do not yet make one thing an application can use.

- **The trust layer** (`sce-rust-trust`, type `Trust`) admits another device's operations (`take_operation`, and as
  the mesh `Admission`) and signs its own (as the mesh `Sealer`); it keeps the membership log, the per-device chains,
  checkpoints and settlement, and saves and restores its state as text. It knows the events it secures **by name**:
  `SIGNED_EVENTS = ["todo.write", "todo.supersede"]`, `FLOOR_EVENT = "todo.floor"`, and the signed bytes of a write
  and a note are laid out from those events' fields (`message.rs`). Those are the events of the test fixture
  `todo_peer`. The standard part an application invokes, `sce:std/shared/record_list.scxml` (ADR 0042), speaks
  `record_list.*`, which the trust layer does not accept.
- **The relay** (`sce-rust-relay`) verifies, keeps and hands back signed records; its protocol is request and
  response JSON texts (`protocol::VERSION = 1`). A host reaches it through a `Courier<L: Link>`, where
  `Link::call(&Request) -> Result<Response, LinkError>` is synchronous; the WebSocket link (`WssLink`) owns a
  current-thread runtime and blocks on each call. The relay admits only what `Trust::take_operation` admits, so it is
  as bound to `todo.*` as the trust layer is.
- **The group key** (`sce-rust-mls`) is reached by other languages through `sce-rust-mls-capi`, a hand-written C
  header checked against the exports, opaque handles, `int32_t` statuses with a per-thread message, buffers owned
  by the library, an ABI version, and one recorded scenario replayed from C, C++, Python, Go and Kotlin (JNA).
  Nothing else of the device has a C ABI.
- **The Mesh** (`sce-rust-mesh`) carries a machine's `<send>` through a `Sealer`, a router and a
  `Transport::transmit(peer, bytes)`, and takes what arrives through `receive`, `Admission` and `peer_ready`. Its
  WebSocket binding (`wss`) is the one in this repository; the owner decided that deployment runs on the
  deployment transport, a downstream project that depends on this repository, so its adapter cannot live here.
- **The world** runs `todo_peer` with `Trust`, `Courier` and an in-process relay between Rust devices
  (`a_shared_list_runs_through_a_relay_with_no_hub_machine.rs`, and the MLS variant). A Kotlin copy shares the to-do
  list with Rust copies without the trust layer or a relay.

An application that generates its machine in Kotlin (a phone) or Swift, and later in Rust for a desktop and the web,
needs all of this on its devices. What is missing is not a mechanism but a **shape**: which side of a boundary each
piece is on, what crosses it, who drives time and I/O, and in what order the existing pieces are joined so that the
protocol exists once.

## Decision

**A device holds a shared list through one peer core: one Rust library that composes the trust layer, the group key,
the relay's client side and the device's saved state, written once and reached by every lane through one C ABI. The
core performs no I/O and starts no thread; the host gives it bytes and time and carries away the bytes it emits. The
machine, generated in the application's language, stays where it is and talks to the core only in the record list's
own events.**

### 1. Two sides, one boundary

| Side | Holds | Written in |
|---|---|---|
| The machine | the record list and the application's rules: registers, observed-remove sets, digest, merge (ADR 0042, 0046) | the application's language, generated from the documents |
| The peer core | identity, membership log, per-device chains, sealing and opening, MLS, checkpoints and settlement, the relay's client protocol, the device's saved state | Rust, once |

What crosses the boundary is exactly what ADR 0034 section 3 says the host maps: the machine's outbound
`record_list.write` and `record_list.supersede` (addressed to `#hub` or `#people`) become the core's operations; an
operation the core admits becomes the inbound event the machine takes. Both are the event's name and its payload as
JSON text, the form the Mesh and the relay already use. The machine is not changed by having a core behind it, and a
machine with no core (a single device, the scenarios) runs as it runs today.

This is the textbook separation of an application's replicated data type from its authentication and delivery
layer: the merge is commutative and needs no trust, and the trust needs no knowledge of what is merged beyond the
fields it signs.

### 2. The trust layer secures the standard part, and the protocol exists once

Before the core is built, the trust layer is bound to the standard part:

1. ADR 0042 step 1 is verified (the standard part on every backend `todo_peer` runs on). Done 2026-11-02.
2. ADR 0042 step 3: the trust layer and the relay name the standard part's events, and only those. The table of
   signed events, the floor event and the field layout of the signed bytes follow **one declaration that belongs to
   the standard part** (its event schemas), held to it by a test, not written as names in `message.rs` alone. A list
   built on the standard part is secured without the trust layer learning the application. The tests that ran
   `todo_peer` under the trust layer or through a relay run the standard list as a copy a host starts.
3. ADR 0042 step 2 (`todo_peer` rebuilt on the standard part, its duplicated body deleted) follows ADR 0043, as that
   ADR orders; a device's core does not wait for it (ADR 0042, "The order of steps 2 and 3", the owner's order of
   2026-11-02).

The signed bytes keep their domain separation and layout rules (length-prefixed parts, the list's name and the
event's name first); the event names in them change, which is a new domain version (`sce-trust/event/2` in place
of `sce-trust/event/1`), since a signature must not verify under two meanings. The fields the list's second
revision added to a write and a supersede (`parts`, `atWallTime`, `atCounter`) are signed, and a field an event
leaves out is signed as its schema default (ADR 0046, "The trust layer").

### 3. The core is sans-I/O

The core is a state machine over inputs and outputs, in the manner of protocol libraries that leave I/O to their
host (QUIC's and TLS's state-machine cores are the reference examples):

- **Inputs:** an outbound event from the machine; a response or pushed record from the relay; the current time
  (wall clock and monotonic), given with every call; a request to start a round (`sync`, `resync`, `reread`).
- **Outputs:** an inbound event for the machine; a request for the relay, as the protocol's text; the instant at
  which the core next wants to be called; refusals, each with its reason (ADR 0046's refusal is the model).
- The core reads no clock, opens no socket, spawns no thread and blocks on nothing. A round is a sequence of
  requests the core emits and responses the host returns; the host may hold a response for minutes, lose the
  connection, or be suspended by its operating system, and the core resumes from what it was given.

Why: a phone's operating system decides when an application may run (a scheduled background job, a push, the
foreground) and kills it without notice; a core that owns a runtime fights that, a core that is driven fits it. The
same core then runs under a test's virtual clock and network in the world, deterministically, which is how the
protocol is verified at all. The present `Courier` keeps its synchronous `Link` form as a convenience for Rust hosts,
built over the sans-I/O core, and `WssLink` becomes one transport among others (section 5) rather than the only way a
round can run.

### 4. One C ABI, the conventions of the MLS ABI

`sce-rust-peer-capi` is the one ABI an application lane links. It follows `sce-rust-mls-capi` exactly, so a reviewer
learns one set of rules:

- a hand-written C header kept equal to the exports by a test; an ABI version constant in the header and the crate;
- opaque handles created and freed by the library, used by one thread at a time;
- every function returns `int32_t`, 0 for success; the reason is per thread (`sce_peer_last_error`); a panic is
  caught and reported and never crosses the boundary;
- buffers the library returns are the library's and are freed by a call;
- events, relay requests and relay responses cross as UTF-8 JSON texts in the forms the Mesh and the relay already
  define; no structure is laid out twice in C.

The group key's functions are inside the core and are not called by an application lane separately; the MLS ABI
stays as the surface the group's own replay tests drive. One recorded scenario of two devices, a relay and a removal
is replayed through the peer ABI from C, Kotlin (JNA), Swift, Python and Go, as the MLS scenario is; a lane whose
toolchain is missing is skipped unless the tools are required.

Bindings are thin and written per lane over the C header (a Kotlin class over JNA, a Swift module over a module map),
not generated by a second interface language: the C header is the contract, and every lane, including those a
generator does not serve, reads the same one.

### 5. The relay is reached through a transport the core does not know

The relay's protocol is transport-independent: a request text, a response text, and records the relay pushes. A
transport carries those texts and says when it is connected; it knows nothing of what they mean.

- **WebSocket** (`/sce-relay/1`, the existing `serve` and `WssLink`) is the reference binding: it is in this
  repository, it is what the world and the relay's own tests use, and it is the fallback for a deployment that has
  no other.
- **The deployment transport** (a downstream zenoh implementation) is what deployment runs on, by the owner's
  decision. Since it depends on this repository, its adapter (the relay's request and response over its key space,
  and the Mesh `Transport` where the Mesh is used) is a crate in that project, written against the transport trait
  defined here.
- A transport is Rust where it can be, linked with the core into the native library an application ships, and
  driven through the same ABI, so that framing, authentication and reconnection are not written again in each host
  language. A host language gives only what it alone has: the network's reachability, and the permission to run.

The token that binds a device to a list at the relay (`token.rs`) is carried by the transport's authentication, as it
is by the WebSocket's `Authorization` today.

### 6. A device's secrets are one sealed state, wrapped by the platform's key store

The core holds a device's secrets (its identity seed, its MLS state, the keys of the epochs it holds, its chain
position) and saves them as one opaque state it seals with a key the host gives it. The host keeps the sealed state
in its own storage and keeps the wrapping key in the platform's key store (Android Keystore, iOS Keychain, the
desktop's key store), where it is not exportable. The core never calls a platform API; the platform's key store never
sees a protocol secret in the clear. The passphrase backup of `sce-rust-trust::backup` is the recovery path, and is
the application's to offer.

### 7. A replay of the application's own log does not write twice

An application that keeps its own command log as its original rebuilds its machine by replaying it from the start,
and the machine sends its writes again. The core makes sealing **idempotent on the write's identity** (register and
stamp): a write it has already sealed and published is recognised and produces no new operation, so a device's chain
never grows by a replay, and never signs two different things at one number (which ADR 0034 freezes a device for).
Operations of other devices that the core admitted are kept by the core and handed to the machine again after a
replay; since the merge is commutative, the order in which a replayed device takes its own writes and the admitted
ones does not change what it holds.

### 8. Verification

- The world runs the core on every device of a list, with the relay, under the virtual network: a Rust device, a
  Kotlin device through the C ABI, and the relay's own policy, with loss, delay, reordering, a device suspended
  mid-round, a relay that withholds and a relay that forges. The present `todo_peer` relay tests move to the
  standard part in ADR 0042 step 3 and are the first of these.
- The C ABI's recorded scenario is replayed from every lane that has a toolchain.
- The core's state machine is fuzzed on its inputs (events, responses, times), as the trust layer is.
- ADR 0034's rule stands: nothing built on this is called production until the independent cryptographic review is
  done.

## Order of work

1. ADR 0042 step 1, then step 3 (section 2); step 2 follows ADR 0043 and is not on this path.
2. The sans-I/O peer core, with `Courier` rebuilt on it; the world's relay tests pass unchanged.
3. Idempotent sealing on replay (section 7), with a world test that replays a device's log against a relay.
4. The C ABI and its recorded scenario, replayed from C, Kotlin, Swift, Python and Go.
5. The transport trait; the WebSocket binding on it; the deployment transport's adapter in its own repository.
6. The world test with a Kotlin device through the ABI (section 8).
7. The sealed device state and the key-store wrapping, specified for each platform lane.

An application starts linking the core after step 4 and ships it after step 6; the review gate of ADR 0034 decides
when it is called production.

## Status of the plan

- **Step 1** landed 2026-11-02 (ADR 0042 step 1 verified, step 3 landed; see ADR 0042).
- **Step 2** landed 2026-11-02. A courier's round is `client::RoundMachine`, a state machine over the relay's
  requests and answers that does no I/O; what a courier keeps between rounds is `client::CourierState`, one text
  when saved; a publication (key package, commit, Welcome, checkpoint) is `client::Publication`. `Courier` keeps its
  API as a loop over the machine, and every existing relay and world test passed unedited. The device's core is the
  crate `sce-rust-peer` (`Peer`): the trust layer, the courier's state, the round machine and the MLS member
  (`with_group`), with events, data, requests and answers as JSON texts; it reads no clock and starts no thread.
  The one place a send is signed and remembered is `client::keep`, which step 3 makes idempotent.
- **A revision made in building it.** A round that fails or is abandoned used to lose the events and MLS records
  it had already taken in: the trust held them and the cursor had passed them, so the machine was told them only by
  a resync. They are now kept in the courier's state and handed over first by the next round (a resync drops them,
  since it reads everything again). A round that follows no failure makes the same requests, in the same order,
  with the same result as before.
- **Step 3** landed 2026-11-02. A write or note the device already sealed comes back as the operation it holds
  (`Trust::seal_sent`, `Kept::Held`): same number, same signature, nothing new for the relay. Another content at the
  same register and stamp is refused before anything is sealed or signed. A device whose state came back from a
  store (`Peer::restore`, or a host that calls `Trust::await_own_chain`) signs nothing new until a round has read
  back from the relay any of its own chain the relay holds beyond its own (`heads`, then `operations`): before this,
  a device restored from an older save signed its next write at a number it had already used, and froze itself on
  every host that saw both. What this rests on is the relay's word on how far a chain goes, which is ADR 0034's limit
  for a relay that withholds. Not done here: handing a rebuilt machine the other devices' operations from the core
  alone; a resync round from the relay tells it the list.
- **Step 4** landed 2026-11-02. `sce-rust-peer-capi` (`include/sce_peer.h`, ABI version 1, documented in
  `docs/SCE_SHARED_LIST_PEER_ABI.md`) gives `Peer` to a lane function for function, with the MLS ABI's rules: the
  two constructors return a handle or NULL and `free`, `abi_version`, `last_error` and `buf_free` return what the
  MLS ABI's do, as there; every other function returns `int32_t`. "Nothing waits" is an empty buffer with status 0.
  The recorded scenario (`tests/fixtures/two_devices_and_a_relay.json`) is two devices and the in-process relay:
  writes and a supersede taken, an unsigned event, another content under a signed stamp refused, a transport failing
  mid-round, a save and a restore that waits for its own chain, a replayed write held, a resync. A fresh run is held
  equal to it, and it replays with no relay from Rust, C, C++, Python, Go, Kotlin (JNA) and Swift (a module map).
  The CI lane that builds the Kotlin replays now starts when one of them alone changes. Two departures from section
  4: the scenario has no removal, and the group key is not in it. A removal is an administrator's operation and
  those are not yet on `Peer` (below), and the group key draws randomness, so the MLS ABI keeps its own scenario.
- **The administrator's acts** landed 2026-11-02, on `Peer` and on the ABI (version 2). `Peer::open_list` writes
  the genesis (the device's administrator's key, a quorum of one) and the grant of the device's person as owner, and
  makes the group when the core holds a device of it; `grant` gives a role; `add_member` admits a device to a list
  keyed by its group from the key package a round read; `remove_member` takes a device off; `standing` and
  `awaits_own_chain` say what the device stands at, so a lane no longer reads it from a refusal's reason. An act that
  writes the log and changes the group is one call, because the log is append-only and the group's roster must be
  the log's members at every commit: a grant without its commit, or a removal the group did not follow, would leave
  the two apart for good. The commit is made before any entry is written (the group's device now makes it against
  the log as it will be), so a refused act writes nothing. The entries go with the next round, and the commit and
  the Welcome after it, since the relay keeps a commit only once its log names it: the core keeps them as owed,
  gives them after each round that ends, keeps one the relay did not take, and saves them (save version 2). The
  recorded scenario now opens the list and grants bob through the ABI, and ends with bob taken off and a write of
  his reaching no one; the acts on a group are held through the ABI by their properties, without a recording.
  `Peer::trust` stays, for what a host reads of the trust; `Peer::group_mut` is gone. Not done: a list with several
  administrators, a linked device and an amendment through the core, and a removal from a list whose keys an
  administrator draws, which the core refuses.
- **Step 5** landed in this repository 2026-11-02. The transport trait is `Transport` of a crate of its own,
  `sce-rust-relay-transport`, with no dependency beyond the standard library, so that a binding written elsewhere
  depends on the contract alone: a request text out and the response text back by a deadline and until a
  cancellation another thread raises (`Signal`), what became of the connection and what the relay pushed
  (`Notice`), and the signal the host raises when the network changed; a relay's refusal is a response, and an
  error says only that the answer did not come back (`TransportError`: the link, the token refused at the
  endpoint, the deadline, the cancellation). Its contract is `docs/SCE_SHARED_LIST_RELAY.md` section 9. The
  core and a transport are joined by one driver (`sce_rust_relay::drive`) over an `Exchange`, which both
  `RoundMachine` and `Peer` are; `Courier` is that driver over its `Link` with no deadline, every transport is
  a `Link`, and every existing relay and world test passed unedited but one assertion: a token refused at the
  WebSocket upgrade is now read as the relay's refusal of the token (`unauthorized`), the answer to one that
  expires on an open connection, where it was a failure of the link. The WebSocket binding's client
  (`wss::WssLink`, feature `wss`, which `serve` includes) is the reference transport: it gives up its
  connection after anything that did not come to a response and when the network changed, and dials again on
  the next exchange. The C ABI gains a link the library holds (`sce_peer_link_wss`, and `sce_peer_link_host`
  for a transport a host carries through one function it gives), a run of the core over it that blocks on the
  calling thread by a deadline (`sce_peer_run`), a cancellation any thread raises (`sce_peer_cancel_*`), the
  network's change (`sce_peer_link_network_changed`) and the link's notices; all added alone, so the ABI's
  version stays 2 (`docs/SCE_SHARED_LIST_PEER_ABI.md` section 11, where the choice of a blocking run over a
  pumped one is argued from the bounded background jobs of both mobile systems). The deployment transport's
  adapter is written in that transport's own repository against `sce-rust-relay-transport`; nothing of it is
  here. `wss://` runs since 2026-11-02: both WebSocket bindings take their TLS from one module
  (`sce-rust-mesh`'s `tls`, re-exported as `sce_rust_relay::tls`), rustls with `ring`'s cryptography, which the
  owner chose over `aws-lc-rs` as the simpler of the two to cross-build for Android and iOS; the provider is
  passed to rustls in each configuration rather than installed for the process, so no host installs anything
  and a library elsewhere in the application that asks for `aws-lc-rs` cannot make the choice ambiguous. A
  client believes Mozilla's certificate authorities compiled into the library, or a deployment's own
  (`WssLink::believing`), and the relay's server terminates TLS with the deployer's certificate (`serve_tls`).
  The operating system's trust store is not read, for the reasons `docs/SCE_SHARED_LIST_RELAY.md` section 9
  gives. Not done: a lane's replay of a run over a link (the Rust tests hold it through the ABI); a call of the
  ABI that names a private authority; and a relay that pushes, which protocol version 1 does not.
- **Step 6** landed 2026-11-02. The world runs the core on every device of a list
  (`a_shared_list_is_held_through_the_core_on_every_device`): three devices, each a copy of the standard
  record list and its core, and the reference relay with its own allowance, nothing composed by hand. Alice's
  device and carol's are Rust, their cores `Peer` in the test's process, driven by the world through the texts a
  host that carries them itself gives (`poll_request`, `respond`, `fail`), which is what a core that does no I/O
  allows. Bob's is Kotlin: his machine and his core run in the JVM of the world's step server, the core reached
  through the C ABI, and each round is the library's own driver, `sce_peer_run`, over `sce_peer_link_host`,
  whose every exchange the step server hands to the world as one more line of the step protocol (`peer_*`,
  `sce_rust_world::external`), so the Kotlin side writes no protocol and reads no outcome. The step server
  that hosted the to-do list hosts the record list and the device beside it; no second mechanism drives the
  JVM. The binding is a library of its own, `backends/kotlin/peer` (`:sce-kotlin-peer`), written against
  ABI 3, which the Kotlin replay now declares nothing beside and an application's Kotlin lane links as the
  world does. Between each device and the relay is a path on the world's clock, drawn from the seed: delays
  spread wide enough to bring one device's request to the relay after another's sent later, requests lost as
  a connection that breaks and responses lost or late as a deadline that passes, and a device whose job's time
  ends with a request on its way, which for bob is the run's cancellation raised and `SCE_PEER_ERR_CANCELLED`,
  while the relay still takes what reached it. Held by their properties: the copies agree on the list that was
  written and no core signs a send twice, through loss, delay and reordering, a suspension and a deadline;
  the allowance refuses rounds for the rate and the devices still agree; a relay that withholds a row leaves
  two devices without it and refuses them nothing, a sync does not read back what the feed's cursor passed,
  and a resync does; a write the relay made up is refused by the core of every device it is given to, once
  each, and reaches no copy; a list keyed by its group admits bob and carol from their key packages, bob
  joining and then applying the commit that admits carol, and the relay holds none of what they write in the
  clear. With no JVM the same tests run with bob's device Rust, unless `SCE_REQUIRE_TOOLS` is set. Not done:
  a deadline of `sce_peer_run` itself in the world (a run's `within_ms` is real time, so the world's deadline
  is the transport's, which ends a run with the same outcome and status `SCE_PEER_OK`); a device's sealed state
  and its restore in the world; a withholding seen by a device through its members' checkpoints, which the
  ABI does not report; a second relay to route around one that withholds; and a relay that pushes.
- **Step 7** landed 2026-11-02. What a device keeps is one sealed state: its person's key, the administrator's
  key it holds, the keys of its epochs, the group's device (its MLS state, and the commits, Welcomes and key
  packages it waits to use, which a member made again had lost), the log, the trust's text, the courier's state
  and what the relay is owed (`Peer::save_sealed`, `Peer::restore_sealed`). It is envelope encryption: the core
  draws a data key for each save, seals the state under it with the passphrase backup's cipher
  (`sce_rust_trust::backup::seal_state`, XChaCha20-Poly1305 under keys drawn by HKDF-Expand, with a synthetic
  nonce that is an HMAC of what is sealed, and the list, the node and the form as associated data), and gives the
  key to the host, which wraps it with a key its platform's key store does not export and keeps it beside the
  state. A key for each save and not for the device: no nonce is ever at risk, a leaked key opens one save, and
  a platform key the host rotates out retires every save wrapped under it. The synthetic nonce makes a state
  the same bytes under one key, which the recorded scenario holds with a data key it fixes
  (`Peer::save_sealed_under`), with no seam a test alone reaches. Another key, a changed or cut state and
  another list's or node's are each refused (`PeerError::NotOpened`), bytes that are none
  (`PeerError::Unreadable`), and the state is read whole before the core takes any of it. What the core cannot
  tell is an older state: nothing it keeps across runs, and no platform counter, says which is the last, so an
  older state and its wrapped key open as the last does; step 3's wait for the device's own chain is what keeps
  a restored device from signing at a number it used, and the key of an epoch destroyed since is zeroed when the
  next round reads the entry. The plain save is gone, not kept for the part that is no secret: a second way to
  keep a device is the one a host would take to keep its state in the clear, and the texts of versions 1 and 2
  were never written outside this repository's tests, so they are not read (save version 3). The C ABI is
  version 3: `sce_peer_save_sealed`, `sce_peer_save_sealed_under` and `sce_peer_restore_sealed` in place of the
  plain save and restore and of the group's state in the clear, `sce_peer_with_group` making a new device only,
  and two statuses appended (`SCE_PEER_ERR_NOT_OPENED`, `SCE_PEER_ERR_NO_RANDOMNESS`). How each platform wraps
  the key (the Android Keystore with StrongBox and its choices of user authentication, the Secure Enclave or a
  Keychain item of `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`, a desktop's TPM, DPAPI or Secret
  Service) is `docs/SCE_SHARED_LIST_PEER_ABI.md` section 6; no platform code is here, and the Kotlin replay wraps
  with a software key through the Keystore's `Cipher` calls. The recorded scenario seals bob's state and restores
  it into a core given nothing but the list and the genesis, and every lane replays it so. Not done: a
  known-answer vector of the sealed form, and the core drawing a device's keys itself, so that a host does not
  handle a seed even once at setup.

## Alternatives considered

- **Each language implements the trust layer and the relay client.** It is what a machine generated in five
  languages would invite. Rejected: it is five places for one flaw in exactly the code that most needs one place, and
  ADR 0034 section 6 already rules it out for the group key.
- **The core owns its I/O and a runtime** (as `WssLink` does today). Simplest to call from Rust. Rejected: it fights
  every mobile operating system's control of when an application runs, makes deterministic tests depend on real
  time, and ties the core to one transport.
- **A generated interface language for the bindings** (a binding generator that emits Kotlin and Swift). It writes
  less by hand. Rejected as the contract: it serves some lanes and not C, Python and Go, adds a second definition of
  the boundary, and the MLS ABI has already shown that a hand-written header checked by a test is cheap to keep. A
  generator may later write the thin per-lane wrappers over the same header without becoming the contract.
- **The transport written in each host language** (a Kotlin WebSocket, a Swift one). Rejected: framing,
  authentication, keepalive and reconnection would be written once per language, and the deployment transport is a
  Rust library in any case.
- **Peer-to-peer without a relay.** Rejected by ADR 0034: devices are not online together, and the relay is what
  lets them not be.
- **Securing the record list beside `todo.*` instead of replacing it** (two tables of signed events). Rejected:
  ADR 0042 step 2 exists so that the protocol is not held twice; a second table would undo it.

## Consequences

- An application links one native library per platform and one C header, and writes only a thin binding, its key-
  store wrapping and its scheduling. Its machine is unchanged.
- The trust layer's event names, the signed bytes' domain version and the relay's admission change once, with
  ADR 0042 step 3; recorded fixtures signed under the old names are re-recorded.
- `Courier` and `WssLink` stop being the only path a round can take; Rust hosts keep them as a convenience.
- The deployment transport's project gains an adapter crate that depends on the transport trait defined here.
- A device's chain no longer depends on its host never replaying; replay is safe by construction.

## Open questions for the owner

- Whether the deployment transport's client side builds for Android and iOS targets as a Rust library; if not,
  which of the two (making it build, or the WebSocket binding as the mobile transport until it does) comes first.
- Where the first relay is hosted (the two servers measured on 2026-10-18 are the candidates).
