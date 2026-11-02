# The peer core's C ABI

A device that holds a shared list does it through one peer core (`backends/rust/peer`,
`sce-rust-peer`, ADR 0079): the trust layer, the relay's client side, the device of the list's MLS
group and the state the device keeps, composed once in Rust and doing no I/O itself. An application
lane that is not Rust links it through one C ABI: `backends/rust/peer-capi` (`sce-rust-peer-capi`, a
shared and a static library named `sce_peer_capi`), header `include/sce_peer.h`. The machine that
holds the list stays in the application's language and speaks to the core only in the record list's
own events; what crosses the boundary is texts.

The ABI follows the group key's (`docs/SCE_SHARED_LIST_MLS.md` section 6) in every convention, so a
reviewer learns one set of rules:

- the header is written by hand, and a test keeps it equal to the library's exports and its numbers
  (`the_header_declares_exactly_what_the_library_exports`); `SCE_PEER_ABI_VERSION` is in the header
  and the crate, and `sce_peer_abi_version()` says which the library is;
- a handle (`ScePeer`) is made and freed by the library and used by one thread at a time;
- a function that does something returns `int32_t`, `SCE_PEER_OK` (0) or a status, and says why in
  `sce_peer_last_error()`, a text the library owns, per thread, good until the thread's next call;
- a pointer is checked for null and read for exactly the length it comes with; a text given to the
  library is NUL-terminated UTF-8;
- a panic is caught at the boundary and is a status (`SCE_PEER_ERR_PANIC`), never an unwinding;
- this is the one crate of the peer core that holds `unsafe` code.

A Swift host reaches the header as the Clang module `CScePeer` (`include/module.modulemap`), a Kotlin
host through JNA, a Python host through ctypes, a Go host through cgo, a C or C++ host by including
it. Each writes its thin binding over the same header; no second interface language defines the
boundary.

## 1. Buffers

What the library gives back is a `ScePeerBuf` (a pointer and a length), the library's, given back with
`sce_peer_buf_free`. A buffer's text is its `len` bytes of UTF-8 and is **not** NUL-terminated. **An
empty buffer (a null pointer and a length of 0) is how a call says there is nothing to give**: no
request to put to the relay, no outcome to take, no settlement of an instant, no reason (the send was
not refused). It is not a failure, and the status is `SCE_PEER_OK`. A call that fills a buffer sets it
to empty first, so a host may free it whatever the status.

The place a call fills with what it is for (the request, the outcome, the save, the group's state,
the answer to a send) must not be null: the call refuses with `SCE_PEER_ERR_NULL_ARGUMENT` before it
does anything, so that nothing is taken from the core that would be lost (an outcome is given once).
The reason of a refusal and the count of a restore may be null.

## 2. Statuses

| Status | Value | When |
|---|---|---|
| `SCE_PEER_OK` | 0 | the call did what it was asked |
| `SCE_PEER_ERR_NOTHING_ASKED` | 1 | an answer or a failure was given and no request of an exchange waits for one |
| `SCE_PEER_ERR_OUTCOME_NOT_TAKEN` | 2 | an exchange was begun while the last one's outcome waits; take it first |
| `SCE_PEER_ERR_NO_GROUP_DEVICE` | 3 | the call needs the group's device and the core holds none |
| `SCE_PEER_ERR_GROUP` | 4 | the group's device refused (a state that is not one, a key package it could not make) |
| `SCE_PEER_ERR_UNREADABLE` | 5 | a saved text is not what `sce_peer_save` writes, or does not hold for this list |
| `SCE_PEER_ERR_NULL_ARGUMENT` | 20 | a pointer, a handle or a place for the result is null |
| `SCE_PEER_ERR_BAD_TEXT` | 21 | a text is not UTF-8 |
| `SCE_PEER_ERR_PANIC` | 22 | the library panicked; the panic was caught |
| `SCE_PEER_ERR_BAD_ARGUMENT` | 23 | a number is outside what it may be (a kind of round) |

A refusal by the trust of what the machine sent, and a failure of the transport or the relay in a
round, are not statuses: they are answers, in the answer to a send and in the outcome (sections 4
and 5).

## 3. Functions

| Function | The core's operation | What it does |
|---|---|---|
| `sce_peer_open(list)` | `Peer::open` | the core of the device of whoever makes the list; null on failure |
| `sce_peer_invited(list, genesis)` | `Peer::invited` | the core of a device invited with the hash (32 bytes) of the list's genesis entry; null on failure |
| `sce_peer_free(peer)` | | frees a core; null is ignored |
| `sce_peer_set_identity(peer, node, seed)` | `Peer::set_identity` | the device's person: their node and the key a 32-byte seed makes |
| `sce_peer_for_a_hub(peer)` | `Peer::for_a_hub` | before the first exchange: the machine is a hub's |
| `sce_peer_with_group(peer, node, seed, state, len)` | `Peer::with_group` | before the first exchange: the core holds the device of `node` in the list's MLS group, new when `len` is 0, or the one `sce_peer_group_state` gave |
| `sce_peer_group_state(peer, out)` | `Device::to_bytes` of the group's device | what the group's device keeps, secret, for the host's store |
| `sce_peer_outbound(peer, event, data, kept, reason)` | `Peer::outbound` | what the machine sent, and what the core did with it (section 4) |
| `sce_peer_start(peer, kind)` | `Peer::start` | begins a round: `SCE_PEER_ROUND_SYNC`, `_RESYNC` or `_REREAD` |
| `sce_peer_publish_checkpoint(peer, before, digest)` | `Peer::publish` of a checkpoint | begins the exchange that gives the relay the person's checkpoint up to `before` |
| `sce_peer_publish_key_package(peer)` | `Peer::publish_key_package` | begins the exchange that leaves a key package of the group's device with the relay |
| `sce_peer_poll_request(peer, out)` | `Peer::poll_request` | the next request text for the relay; empty when none waits |
| `sce_peer_respond(peer, response)` | `Peer::respond` | the relay's response text to that request |
| `sce_peer_fail(peer, reason)` | `Peer::fail` | the transport did not carry the request or its answer |
| `sce_peer_take_outcome(peer, out)` | `Peer::take_outcome` | what the exchange came to, once; empty when none ended |
| `sce_peer_floor_event(peer, before, digest, out)` | `Peer::floor_event` | the event that says an instant is settled; empty when it is not |
| `sce_peer_save(peer, out)` | `Peer::save` | what the device keeps, as one text |
| `sce_peer_restore(peer, saved, count)` | `Peer::restore` | takes a save back into a core just made for the list, after its person was given again |

## 4. A send of the machine

The machine's outbound `record_list.write` and `record_list.supersede`, addressed to `#hub` or
`#people`, are given as the event's name and its JSON payload, as the machine wrote them.
`*kept` says what the core did:

| Answer | Value | Meaning |
|---|---|---|
| `SCE_PEER_KEPT_TAKEN` | 0 | signed and remembered; the next round carries it to the relay |
| `SCE_PEER_KEPT_HELD` | 1 | the device holds it already (a machine built again from its application's own log sends its writes again): nothing is signed, and the relay is given nothing new |
| `SCE_PEER_KEPT_NOT_SIGNED` | 2 | it is no event that is signed; nothing is kept |
| `SCE_PEER_KEPT_REFUSED` | 3 | the trust would not sign it, and `*reason` says why: another content under a stamp the device signed, a device with no person, a restored device whose own chain was not read back yet |

## 5. A round

A round is driven by its host, and the core does no I/O:

1. `sce_peer_start` (or a publication) begins the exchange;
2. `sce_peer_poll_request` gives a request text (asked again before it is answered, it is the same
   one); the host carries it to the relay by whatever transport it has and gives the answer with
   `sce_peer_respond`, or says with `sce_peer_fail` that the transport did not carry it;
3. once the poll gives an empty buffer the exchange is over, and `sce_peer_take_outcome` gives what it
   came to, once. The next exchange is not begun until it is taken (`SCE_PEER_ERR_OUTCOME_NOT_TAKEN`),
   so nothing it hands over is lost.

The requests and responses are the relay protocol's texts (`docs/SCE_SHARED_LIST_RELAY.md` section 3).
A response that is not one of the protocol is taken as a failure of the transport. A host may hold a
request for minutes, lose its connection, or be stopped in the middle of a round: what the round took
in and did not hand over is handed over by the next round.

The outcome is a JSON object, read by its members' names:

| Member | When | What |
|---|---|---|
| `outcome` | always | `"ended"` or `"failed"` |
| `events` | ended | the events for the machine, in order, each `[name, payload text]`: the roster when the log moved, then the operations the device did not hold (a resync tells every operation) |
| `refused` | ended | each record the core or the relay refused, and why; the rest of the round went on |
| `pushed` | ended | how many entries, operations and checkpoints the relay was given that it lacked |
| `evicted`, `restored` | ended | the operations, each `[node, number]` as texts, the machine was told and is to be without, or was not told and is to be with |
| `rebuild` | ended | whether the host is to build its machine again and make a resync (an operation was evicted or restored) |
| `stalled` | ended | whether the round stopped before the end of what the relay holds; a later round goes on from there |
| `reread` | ended | whether the group's device needed a second read from the start, whose events are in `events` after the round's |
| `steps` | ended | what the group's device did with what was read, each `{"step": "joined" or "applied", "epoch": text}`, `{"step": "removed"}` or `{"step": "refused", "reason": text}` |
| `failure` | failed | `"link"` (the transport), `"relay"` (the relay refused the round as a whole) or `"unexpected"` (an answer that is not the answer to the request) |
| `code` | failed by the relay | the relay protocol's word for the refusal (`"forbidden"`, `"rate_limited"`, ...), and `retry_after_ms` as text when it is a rate limit |
| `reason` | failed | why |

A 64-bit number is written as text, as the relay protocol writes it. The MLS records a round read are
taken in by the group's device inside the core and are not in the outcome; what the device did with
them is in `steps`.

The floor event is the pair `[name, payload text]` the machine is told.

## 6. What the device keeps

`sce_peer_save` gives one text: the membership log, what the trust holds and the state of the relay's
client, as it would be if the exchange that runs stopped there. It does not hold the device's secrets:
the person's seed is given again with `sce_peer_set_identity`, and the group's device with
`sce_peer_with_group` from what `sce_peer_group_state` gave, before `sce_peer_restore`. Sealing them in
one state under a key from the platform's key store is ADR 0079 section 6.

**A restored device signs nothing new until its own chain is read back.** The core cannot tell the
last save from an older one, so a restored core refuses a send that would make a new operation (and
still gives what it holds as held) until a round has read the device's own operations back from the
relay (`docs/SCE_SHARED_LIST_RELAY.md` section 6). A host that rebuilds its machine from its own log
after a restore makes that round first, then gives the core the machine's sends, then makes a resync.

## 7. The recorded scenario every lane replays

`backends/rust/peer-capi/tests/fixtures/two_devices_and_a_relay.json` is a run of two devices of one
list through the ABI against the real relay in one process: alice's device, which holds the
administrator's key and opens the list from a save that holds the genesis and the grants, and bob's,
invited with the hash of the genesis and granted writing. Every call each core is given and every text
it gives back is recorded in order: bob's writes and his note through `sce_peer_outbound`, an event
that is not signed, another content under a stamp he signed (refused), each request and the relay's
answer, the outcomes, bob's save and his core made again and restored from it, a write of his that
comes back held and a new one that waits for his chain to be read back, a resync, and a round of
alice's whose transport fails.

Because the core does no I/O a lane replays it with no relay: it makes the same calls, compares each
request, outcome, save and refusal reason with the recording, and gives the core the recorded answers.
Rust, C, C++, Python (ctypes), Go (cgo), Kotlin (JNA, `backends/kotlin/peer-replay`) and Swift (the
module map) replay the same file
(`a_c_abi_is_what_the_lanes_call_and_replays_the_recorded_scenario`); a lane whose toolchain is
missing is skipped unless `SCE_REQUIRE_TOOLS` is set, and the Kotlin lane needs
`./gradlew :sce-kotlin-peer-replay:installDist`.

Everything in it is deterministic (Ed25519 signs deterministically; nothing reads a clock or draws a
random number), so a run now records the same file, and
`the_recorded_scenario_is_what_a_fresh_run_records` fails, naming the first step that differs, when the
recording and a fresh run do not agree. It is recorded again with `SCE_PEER_SCENARIO=write cargo test
-p sce-rust-peer-capi --test the_recorded_scenario_is_what_a_fresh_run_records --
the_recorded_scenario_is_what`. The list's MLS group is not in it: MLS draws randomness, so a run would
not record the same texts twice, and the group's own C ABI has its own recorded scenario.

## 8. What the ABI does not hold, because the core does not

The ABI has one function per operation of `Peer` a host needs, and nothing `Peer` does not do:

- **An administrator's operations.** `Peer` has none of its own: the entries of the membership log
  (the genesis, a grant, a removal) are drafted and signed with the trust (`Peer::trust`), and the
  group's changes (making the group, adding and removing a member, the commit and the Welcome to
  publish) are the group's device's (`Peer::group_mut`). A lane cannot open a list or grant a device
  through this ABI; the recorded scenario starts the administrator's device from a save that holds the
  log for that reason.
- **Whether a restored device still waits for its own chain.** It is the trust's
  (`Trust::awaits_own_chain`); a lane learns it from a refusal's reason.
- **The time.** Nothing the core does depends on the time today; when it comes to, it enters as an
  argument of the calls that need it (ADR 0079 section 3), and the ABI's version moves.
