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
  and the crate, and `sce_peer_abi_version()` says which the library is. As the group key's ABI rules,
  a change to what a function takes or returns makes a new number, and so does a change to the shape
  of a text a function gives; a function added alone does not, since a lane built against the number
  before calls none of it. A status is appended and never renumbered. The version is 2: the outcome
  holds `owed` and the save holds what the relay is owed (sections 5 and 6), which came with the
  administrator's acts (section 7). The links, the run over a link and its cancellation (section 11)
  came after as functions and statuses added alone, which no function of before takes or returns,
  and left it 2;
- a handle (`ScePeer`, `ScePeerLink`, `ScePeerCancel`) is made and freed by the library and used by
  one thread at a time, but for the two calls section 11 lets any thread make at any time (raising a
  run's cancellation, telling a link the network changed);
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
the answer to a send, the genesis of a list opened, the standing) must not be null: the call refuses
with `SCE_PEER_ERR_NULL_ARGUMENT` before it does anything, so that nothing is taken from the core that
would be lost (an outcome is given once).
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
| `SCE_PEER_ERR_NO_ADMINISTRATOR` | 6 | an administrator's act, and the core holds no administrator's key (`sce_peer_set_administrator`) |
| `SCE_PEER_ERR_NO_PERSON` | 7 | the act needs the device's person and the core has none (`sce_peer_set_identity`) |
| `SCE_PEER_ERR_OPENED` | 8 | the list is open already: the core's log holds its genesis |
| `SCE_PEER_ERR_LOG` | 9 | the membership log refused the entry the act wrote (an administrator's key the log does not name, a genesis that is not the one the device was invited with); nothing was written |
| `SCE_PEER_ERR_NOT_A_MEMBER` | 10 | the device of that node is no member of the list now |
| `SCE_PEER_ERR_ADMITTED_THROUGH_GROUP` | 11 | the list is keyed by its group, and a grant would make a member the group does not hold: the device is admitted with `sce_peer_add_member` |
| `SCE_PEER_ERR_NO_KEY_PACKAGE` | 12 | no key package of the device was read from the relay yet |
| `SCE_PEER_ERR_DRAWN_KEYS` | 13 | the list's keys are drawn by an administrator and not its group's, and the core does not hand the members left the new key a removal needs |
| `SCE_PEER_ERR_NULL_ARGUMENT` | 20 | a pointer, a handle or a place for the result is null |
| `SCE_PEER_ERR_BAD_TEXT` | 21 | a text is not UTF-8 |
| `SCE_PEER_ERR_PANIC` | 22 | the library panicked; the panic was caught |
| `SCE_PEER_ERR_BAD_ARGUMENT` | 23 | a number is outside what it may be (a kind of round, a profile, a role); an address a link cannot be made to |
| `SCE_PEER_ERR_CANCELLED` | 24 | `sce_peer_run` stopped because its cancellation was raised; the exchange ended as a failure of the link, and its outcome waits (section 11) |
| `SCE_PEER_ERR_DEADLINE` | 25 | `sce_peer_run` stopped because its deadline passed, with the same end |

A refusal by the trust of what the machine sent, and a failure of the transport or the relay in a
round, are not statuses: they are answers, in the answer to a send and in the outcome (sections 4
and 5). A run that stopped says why in its status, and its outcome is a failure of the link as
well, so a host that reads only outcomes reads it there. An administrator's act that is refused
wrote nothing: what can refuse it is asked, and the group's commit is made, before an entry of the
log is written (section 7).

## 3. Functions

| Function | The core's operation | What it does |
|---|---|---|
| `sce_peer_open(list)` | `Peer::open` | the core of the device of whoever makes the list; null on failure |
| `sce_peer_invited(list, genesis)` | `Peer::invited` | the core of a device invited with the hash (32 bytes) of the list's genesis entry; null on failure |
| `sce_peer_free(peer)` | | frees a core; null is ignored |
| `sce_peer_set_identity(peer, node, seed)` | `Peer::set_identity` | the device's person: their node and the key a 32-byte seed makes |
| `sce_peer_set_administrator(peer, seed)` | `Peer::set_administrator` | the administrator's key the device holds, from a 32-byte seed |
| `sce_peer_open_list(peer, profile, genesis)` | `Peer::open_list` | opens a new list (section 7); `genesis` is set to the hash (32 bytes) a person is invited with |
| `sce_peer_grant(peer, node, role, key)` | `Peer::grant` | grants the device of `node`, which signs with a 32-byte public key, a role of 1 to 3 |
| `sce_peer_add_member(peer, node, role, key)` | `Peer::add_member` | admits a device to a list keyed by its group, with the grant and the commit in one change |
| `sce_peer_remove_member(peer, node)` | `Peer::remove_member` | takes a device off the list, and off its group when the list is keyed by one |
| `sce_peer_standing(peer, out)` | `Peer::standing` | what the device stands at in its list, as one JSON text (section 8) |
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
| `sce_peer_link_wss(base, token)` | `WssLink::new` | a link to the relay over its WebSocket binding (section 11); null on failure |
| `sce_peer_link_host(exchange, release, context)` | a `Transport` over the host's function | a link whose transport the host carries itself (section 11); null when `exchange` is null |
| `sce_peer_link_free(link)` | | frees a link, releasing a host's context; null is ignored |
| `sce_peer_link_network_changed(link)` | the link's `Transport::network` raised | the device's network changed; any thread, at any time |
| `sce_peer_link_take_notices(link, out)` | `Transport::take_notices` | what became of the connection and what the relay pushed, as one JSON array; empty when nothing |
| `sce_peer_reply(reply, text, len)` | | from a host's transport: the response of the exchange, or the reason it has none |
| `sce_peer_cancel_new()`, `sce_peer_cancel_free(cancel)` | a `Signal` | a cancellation of a run, and freeing it |
| `sce_peer_cancel_raise(cancel)` | `Signal::raise` | cancels the run given it; any thread, at any time |
| `sce_peer_run(peer, link, within_ms, cancel)` | `drive` | runs the exchange begun on the core over the link, on the calling thread, to its end, its deadline or its cancellation |

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
| `owed` | ended | how many commits and Welcomes of the administrator's acts the relay is still owed once the exchange ended (section 7); one the relay did not take is among `refused` |
| `failure` | failed | `"link"` (the transport), `"relay"` (the relay refused the round as a whole) or `"unexpected"` (an answer that is not the answer to the request) |
| `code` | failed by the relay | the relay protocol's word for the refusal (`"forbidden"`, `"rate_limited"`, ...), and `retry_after_ms` as text when it is a rate limit |
| `reason` | failed | why |

A 64-bit number is written as text, as the relay protocol writes it. The MLS records a round read are
taken in by the group's device inside the core and are not in the outcome; what the device did with
them is in `steps`.

The floor event is the pair `[name, payload text]` the machine is told.

## 6. What the device keeps

`sce_peer_save` gives one text: the membership log, what the trust holds, the state of the relay's
client, as it would be if the exchange that runs stopped there, and the commits and Welcomes the relay
is still owed (section 7). It does not hold the device's secrets: the person's seed is given again
with `sce_peer_set_identity`, the administrator's with `sce_peer_set_administrator`, and the group's
device with `sce_peer_with_group` from what `sce_peer_group_state` gave, before `sce_peer_restore`.
Sealing them in one state under a key from the platform's key store is ADR 0079 section 6. The save is
the core's version 2; a save of version 1, which owed the relay nothing, is still read.

**A restored device signs nothing new until its own chain is read back.** The core cannot tell the
last save from an older one, so a restored core refuses a send that would make a new operation (and
still gives what it holds as held) until a round has read the device's own operations back from the
relay (`docs/SCE_SHARED_LIST_RELAY.md` section 6). A host that rebuilds its machine from its own log
after a restore makes that round first, then gives the core the machine's sends, then makes a resync.
Whether it still waits is in what it stands at (`awaits_own_chain`, section 8).

## 7. An administrator's acts

A device that holds an administrator's key (`sce_peer_set_administrator`) runs the list. Each act is
one call, and writes the entries of the membership log and makes the change of the group that goes
with them together, so that no host can do one without the other:

- `sce_peer_open_list` writes the genesis (the administrator's key the device holds as the list's one
  administrator, a quorum of one, the hub's profile `SCE_PEER_PROFILE_UNTRUSTED` or `_TRUSTED_HUB`)
  and a grant that makes the device's person a member with the highest role (3), and, when the core
  holds a device of the group (`sce_peer_with_group`, called before), makes the group with that device
  alone in it: a list is keyed by its group from its first entry, or not at all. The hash of the
  genesis is what a person is invited with (`sce_peer_invited`).
- `sce_peer_grant` gives a device a role of 1 (may read), 2 (may also write) or 3 (may also do what an
  owner does), or another role or key to a member; one that takes writing away carries where the
  device's chain ends, as the log requires. On a list keyed by its group it is refused for a device the
  group does not hold (`SCE_PEER_ERR_ADMITTED_THROUGH_GROUP`): a grant alone would make the log's
  members differ from the group's, and every later commit would be refused for it.
- `sce_peer_add_member` admits a device to a list keyed by its group from the newest key package of it
  a round read from the relay (the device leaves one with `sce_peer_publish_key_package`): the grant,
  the commit that adds the device and the entry that names the commit.
- `sce_peer_remove_member` takes a device off, its chain ending at the last operation of it the
  administrator's device holds; on a list keyed by its group the same call makes the commit that moves
  the group to an epoch without it, since a removal the group did not follow would leave what is
  written after it readable by the device. A list whose content keys an administrator draws (not its
  group's) is refused (`SCE_PEER_ERR_DRAWN_KEYS`): the members left would need a new key the core does
  not hand over.

The log is append-only and a commit made and not merged can be given up, so an act asks everything
that can refuse it and makes its commit before it writes an entry: an act that is refused wrote
nothing. The entries go to the relay with the next round. A commit and a Welcome go after them, since
the relay keeps a commit only once its log names it: the round that ends gives the relay what it is
owed, in order, before its outcome is given, and `owed` in the outcome says how many it still is. One
the relay did not take (the transport failed, the relay refused it) stays owed, is among the
outcome's refusals, and is given again after the next round; it is in the save, so a device stopped
before its next round gives it once it runs again.

## 8. What a device stands at

`sce_peer_standing` gives one JSON object, for a host to show the list as it is without reading a
refusal's words:

| Member | What |
|---|---|
| `genesis` | the hash of the list's genesis in the URL alphabet, as the log writes hashes; `null` while the core holds no entry |
| `node`, `key` | the device's person: their node as text, their public key in the URL alphabet; `null` with no person |
| `role` | the person's role now, 0 when they are no member |
| `administrator` | whether the core holds an administrator's key the log names as one |
| `awaits_own_chain` | whether the device signs nothing new until a round has read its own chain back (section 6) |
| `members` | the members now, in the order of their nodes, each `{"node": text, "role": number}` |
| `epoch` | the latest epoch the log begins, 0 when none |
| `keys` | the epochs whose keys the device holds: what it can open |
| `owed` | how many commits and Welcomes the relay is still owed (section 7) |

## 9. The recorded scenario every lane replays

`backends/rust/peer-capi/tests/fixtures/two_devices_and_a_relay.json` is a run of two devices of one
list through the ABI against the real relay in one process: alice's device, which holds the
administrator's key, opens the list and grants bob writing, and bob's, invited with the hash of the
genesis alice's core gave. Every call each core is given and every text it gives back is recorded in
order: the opening and its genesis, the grant, what each device stands at, bob's writes and his note
through `sce_peer_outbound`, an event that is not signed, another content under a stamp he signed
(refused), each request and the relay's answer, the outcomes, bob's save and his core made again and
restored from it, a write of his that comes back held and a new one that waits for his chain to be
read back, a resync, a round of alice's whose transport fails, and bob taken off: a write his core
signs before it read the removal reaches no one, since the relay refuses a device the log took off.

Because the core does no I/O a lane replays it with no relay: it makes the same calls, compares each
request, outcome, standing, genesis, save and refusal reason with the recording, and gives the core the
recorded answers.
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
not record the same texts twice, and the group's own C ABI has its own recorded scenario. The
administrator's acts on a group are held through this ABI without a recording, by their properties
(`the_administrators_acts_on_a_group_go_through_the_abi`): a device admitted from its key package joins
from the Welcome and holds the key of the epoch the administrator's device holds, and once taken off
holds no key of the epoch after.

## 10. What the ABI does not hold, because the core does not

The ABI has one function per operation of `Peer` a host needs, and nothing `Peer` does not do:

- **A list with several administrators.** A list the core opens has one administrator and a quorum
  of one. An entry that needs the signatures of a quorum above one (a change to the administrators,
  a profile, a shredded epoch) is collected across hosts with the trust's draft, which no lane reaches.
- **A list whose keys an administrator draws.** The core keys a list's content with its group; the
  drawn keys of `Trust::rotate_epoch` are handed to the members by a channel the core does not have,
  so the core takes no member off such a list (section 7).
- **A device linked by another of the same person, and an amendment of a removal.** The group's
  device makes both (`Member::add_linked_member`, `Member::amend_removed`); they are not acts of the
  core yet.
- **The time.** Nothing the core does depends on the time today; when it comes to, it enters as an
  argument of the calls that need it (ADR 0079 section 3), and the ABI's version moves.

## 11. A run over a link the library holds

A lane may carry the texts itself (section 5). It need not: the library holds a link to the relay,
and runs an exchange of the core over it, so that the framing, the device's authentication and the
reconnection are written once, in Rust, and the host gives only what it alone has, the network's
reachability and the permission to run (ADR 0079 section 5). The link is one of the transports of
`docs/SCE_SHARED_LIST_RELAY.md` section 9, and the run is the one driver every transport is run by.

- `sce_peer_link_wss(base, token)` is the relay's WebSocket binding at `base` (`ws://host:port` or
  `wss://host:port`), presenting `token`, the token the device's login earned. Nothing is dialled
  until a run asks; the link dials again after anything that did not come to a response and after
  the network changed. A token the relay refuses at the upgrade comes back as the relay's refusal
  (`failure: "relay"`, `code: "unauthorized"`), as one that expires on an open connection does: the
  host asks its application for a new token and makes a new link. Only `ws://` runs today: the
  library links `rustls` with no cryptographic provider, so the first exchange over `wss://` panics
  inside the library and the run is `SCE_PEER_ERR_PANIC`, until a provider is chosen (ADR 0079,
  Status of the plan).
- `sce_peer_link_host(exchange, release, context)` is a transport the host carries itself, for a
  host whose platform has the only usable network stack, or a test. `exchange` is
  `int32_t (*)(void *context, const uint8_t *request, size_t len, uint64_t timeout_ms, ScePeerReply
  *reply)`: it carries the request (`len` bytes of UTF-8, not NUL-terminated, valid for the call),
  gives the response text with `sce_peer_reply(reply, text, len)` (copied), and returns
  `SCE_PEER_TRANSPORT_ANSWERED` (0); or it returns `SCE_PEER_TRANSPORT_LINK` (1, not carried),
  `_UNAUTHORIZED` (2, the token was refused), `_DEADLINE` (3) or `_CANCELLED` (4), with the reason
  given by `sce_peer_reply` when there is one. It may take at most `timeout_ms` (`UINT64_MAX` when
  the run has no deadline). `release`, which may be null, is called once with `context` when the link
  is freed.
- `sce_peer_run(peer, link, within_ms, cancel)` runs the exchange begun on the core
  (`sce_peer_start`, a publication) over the link **on the calling thread, blocking**, until it ends,
  `within_ms` milliseconds pass (`UINT64_MAX`: no deadline) or `cancel` (which may be null) is
  raised. It returns `SCE_PEER_OK` when the exchange ended, whatever it came to (and at once when
  none runs), and `SCE_PEER_ERR_CANCELLED` or `SCE_PEER_ERR_DEADLINE` when it stopped; either way the
  outcome is then taken with `sce_peer_take_outcome`. A run that stopped, or whose transport failed,
  leaves an outcome of a failure of the link (`"the exchange was cancelled"`, `"the deadline passed
  before the relay answered"`, `"the network changed"`, or the transport's words), and what the
  exchange took in is handed over by the next round, as section 5 says of any exchange that does not
  end.
- `sce_peer_cancel_new()` makes a cancellation, `sce_peer_cancel_raise(cancel)` raises it, and it
  stays raised: a host makes one for each run it may want to stop.
- `sce_peer_link_network_changed(link)` says the device's network changed (an interface came or
  went, the system said the path is another): the link gives its connection up, and an exchange that
  waits fails as the link's failure, so the next round dials on the new path.
- `sce_peer_link_take_notices(link, out)` gives a JSON array, oldest first, of what became of the
  connection and what the relay pushed since it was last asked: `{"notice": "connected"}`,
  `{"notice": "lost", "reason": text}`, `{"notice": "pushed", "text": text}`; an empty buffer when
  there is none. A push is a reason to begin a round, never an input of the core; version 1 of the
  relay protocol pushes nothing. A host's own transport gives no notices: its host knows them.

**Threads.** A run uses the core and the link on the thread that called it, and no other thread
uses either until it returns, but for two calls any thread may make at any time, the run's
included: `sce_peer_cancel_raise` on the run's cancellation and `sce_peer_link_network_changed` on its
link. A host's `exchange` is called on the running thread, one call at a time, and calls nothing of
the library on that link or core but `sce_peer_reply`; its context is used from whichever thread
runs, until `release`. A link and a cancellation are freed only when no run uses them.

**Why a blocking run with a deadline, and not a run the host pumps.** A phone's operating system
runs an application's background work as a bounded job on a thread of its own: Android's
WorkManager calls `doWork()` on a background thread and stops the worker when its time is up
(`onStopped()`, on another thread), and iOS gives a background task an expiration handler, called on
another thread, before it suspends the application. A run that blocks on the job's thread, ends by
the job's deadline, and is cancelled from the expiration handler is that shape exactly, in every
host language, with nothing to integrate. A pumped run would hand the host the transport's event
loop (its sockets, its timers, its wakers) to fit into each language's own, which is the per-language
transport code ADR 0079 section 5 rules out. A host that wants to drive each request itself still
can, with the calls of section 5 and a transport of its own; the core is the same exchange either
way.
