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
  before calls none of it. A status is appended and never renumbered. Version 2: the outcome holds
  `owed` and the save holds what the relay is owed (sections 5 and 6), which came with the
  administrator's acts (section 7). The links, the run over a link and its cancellation (section 11)
  came after as functions and statuses added alone, which no function of before takes or returns,
  and left it 2. The version is 3: what the device keeps is one sealed state, its secrets with the
  rest, under a data key the host wraps (section 6); `sce_peer_save`, `sce_peer_restore` and
  `sce_peer_group_state` are gone, and `sce_peer_with_group` no longer takes a group's state, since
  a host no longer holds one. The version is 4: a send the device may not sign yet is kept and not
  refused, so `sce_peer_outbound` gives an answer a lane of version 3 does not know
  (`SCE_PEER_KEPT_WAITING`, section 4), a changed meaning of an output that the policy above makes
  a new number; the outcome holds `taken` and `dropped` (section 5), the standing `waiting`
  (section 8), and the sealed state is the core's version 4, which holds what waits (section 6).
  The link that believes a deployment's own authority (`sce_peer_link_wss_believing`,
  `SCE_PEER_ROOTS_*`, section 11) is a function added alone, and moves no number. The version is 5
  (ADR 0080): a device logs in (`sce_peer_login`, section 5) and gives its link the token
  (`sce_peer_link_set_token`, section 11), two functions added alone; but a link made with a null or
  empty token now dials a connection that presents none, where version 4 presented the empty text
  as a token the relay refused, a changed meaning of an input, and the outcome of a login holds
  `token`, which the policy above makes a new number;
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
boundary. The Kotlin binding is a library of its own, `backends/kotlin/peer` (`:sce-kotlin-peer`):
the header's functions and numbers declared once for JNA, and the core, a link and a cancellation as
classes that free their handles (`PeerCore`, `PeerLink`, `PeerCancel`), with a host's transport given
as a Kotlin function (`HostTransport`). The Kotlin replay below and the world's Kotlin device are
built on it, and an application's Kotlin lane is built on it as they are.

## 1. Buffers

What the library gives back is a `ScePeerBuf` (a pointer and a length), the library's, given back with
`sce_peer_buf_free`. A buffer's text is its `len` bytes of UTF-8 and is **not** NUL-terminated. **An
empty buffer (a null pointer and a length of 0) is how a call says there is nothing to give**: no
request to put to the relay, no outcome to take, no settlement of an instant, no reason (the send was
neither refused nor kept waiting). It is not a failure, and the status is `SCE_PEER_OK`. A call that fills a buffer sets it
to empty first, so a host may free it whatever the status.

The place a call fills with what it is for (the request, the outcome, the sealed state and its data
key, the answer to a send, the genesis of a list opened, the standing) must not be null: the call refuses
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
| `SCE_PEER_ERR_UNREADABLE` | 5 | bytes given as a sealed state are not one (another form, or cut short of the least a sealed state holds), or what it holds does not hold for this core |
| `SCE_PEER_ERR_NO_ADMINISTRATOR` | 6 | an administrator's act, and the core holds no administrator's key (`sce_peer_set_administrator`) |
| `SCE_PEER_ERR_NO_PERSON` | 7 | the act needs the device's person and the core has none (`sce_peer_set_identity`) |
| `SCE_PEER_ERR_OPENED` | 8 | the list is open already: the core's log holds its genesis |
| `SCE_PEER_ERR_LOG` | 9 | the membership log refused the entry the act wrote (an administrator's key the log does not name, a genesis that is not the one the device was invited with); nothing was written |
| `SCE_PEER_ERR_NOT_A_MEMBER` | 10 | the device of that node is no member of the list now |
| `SCE_PEER_ERR_ADMITTED_THROUGH_GROUP` | 11 | the list is keyed by its group, and a grant would make a member the group does not hold: the device is admitted with `sce_peer_add_member` |
| `SCE_PEER_ERR_NO_KEY_PACKAGE` | 12 | no key package of the device was read from the relay yet |
| `SCE_PEER_ERR_DRAWN_KEYS` | 13 | the list's keys are drawn by an administrator and not its group's, and the core does not hand the members left the new key a removal needs |
| `SCE_PEER_ERR_NOT_OPENED` | 14 | a sealed state did not open under the data key given, for this list and the node given: another key, a changed or cut state, or another device's (a cipher cannot tell these apart); nothing of it was taken (section 6) |
| `SCE_PEER_ERR_NO_RANDOMNESS` | 15 | the operating system gave no randomness for a data key; no weaker source is used |
| `SCE_PEER_ERR_NULL_ARGUMENT` | 20 | a pointer, a handle or a place for the result is null |
| `SCE_PEER_ERR_BAD_TEXT` | 21 | a text is not UTF-8 |
| `SCE_PEER_ERR_PANIC` | 22 | the library panicked; the panic was caught |
| `SCE_PEER_ERR_BAD_ARGUMENT` | 23 | a number is outside what it may be (a kind of round, a profile, a role); an address a link cannot be made to; a token given to a host's own transport |
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
| `sce_peer_with_group(peer, node, seed)` | `Peer::with_group` | once, before the first exchange of a device with no sealed state yet: the core holds a new device of `node` in the list's MLS group, which the sealed state keeps from then on |
| `sce_peer_outbound(peer, event, data, kept, reason)` | `Peer::outbound` | what the machine sent, and what the core did with it (section 4) |
| `sce_peer_start(peer, kind)` | `Peer::start` | begins a round: `SCE_PEER_ROUND_SYNC`, `_RESYNC` or `_REREAD` |
| `sce_peer_publish_checkpoint(peer, before, digest)` | `Peer::publish` of a checkpoint | begins the exchange that gives the relay the person's checkpoint up to `before` |
| `sce_peer_publish_key_package(peer)` | `Peer::publish_key_package` | begins the exchange that leaves a key package of the group's device with the relay |
| `sce_peer_login(peer, audience)` | `Peer::login` | begins the exchange that logs the device in at the relay named `audience` (the name it was configured with), whose outcome holds the token (section 5); `SCE_PEER_ERR_NO_PERSON` with no person |
| `sce_peer_poll_request(peer, out)` | `Peer::poll_request` | the next request text for the relay; empty when none waits |
| `sce_peer_respond(peer, response)` | `Peer::respond` | the relay's response text to that request |
| `sce_peer_fail(peer, reason)` | `Peer::fail` | the transport did not carry the request or its answer |
| `sce_peer_take_outcome(peer, out)` | `Peer::take_outcome` | what the exchange came to, once; empty when none ended |
| `sce_peer_floor_event(peer, before, digest, out)` | `Peer::floor_event` | the event that says an instant is settled; empty when it is not |
| `sce_peer_save_sealed(peer, state, key)` | `Peer::save_sealed` | what the device keeps, its secrets with the rest, sealed under a data key drawn for this save; `key` is set to that key (32 bytes) for the host to wrap (section 6) |
| `sce_peer_save_sealed_under(peer, key, state)` | `Peer::save_sealed_under` | the same, under a 32-byte data key the host gives: one it drew itself, or a recording's fixed key |
| `sce_peer_restore_sealed(peer, node, state, len, key, count)` | `Peer::restore_sealed` | takes a sealed state back for the device of `node`, under the unwrapped data key, into a core just made for the list and given nothing else |
| `sce_peer_link_wss(base, token)` | `WssLink::new` | a link to the relay over its WebSocket binding (section 11), with no token when `token` is null or empty; null on failure |
| `sce_peer_link_wss_believing(base, token, authorities, len, roots)` | `WssLink::believing` | the same, believing the certificate authorities in the bytes given (PEM or DER), alone or with the public ones (section 11); null on failure |
| `sce_peer_link_host(exchange, release, context)` | a `Transport` over the host's function | a link whose transport the host carries itself (section 11); null when `exchange` is null |
| `sce_peer_link_free(link)` | | frees a link, releasing a host's context; null is ignored |
| `sce_peer_link_network_changed(link)` | the link's `Transport::network` raised | the device's network changed; any thread, at any time |
| `sce_peer_link_take_notices(link, out)` | `Transport::take_notices` | what became of the connection and what the relay pushed, as one JSON array; empty when nothing |
| `sce_peer_link_set_token(link, token)` | `Transport::set_token` | the token the link presents from its next dial, null or empty for none (section 11); `SCE_PEER_ERR_BAD_ARGUMENT` for a host's own transport |
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
| `SCE_PEER_KEPT_REFUSED` | 3 | the trust will not sign it, and `*reason` says why: another content under a stamp the device signed or that waits, another person's write the device holds no signature of, a person the log the device holds does not let write (a reader, a device taken off), a frozen chain, a device with no person; nothing is kept |
| `SCE_PEER_KEPT_WAITING` | 4 | the device may not sign it yet, and `*reason` says why: it has not read the log that lets it write (an invited device before its first round, a log that grants it nothing yet), the key of the epoch the list's content is in, or, restored, its own chain; it is kept (below) |

**A send the device may not sign yet is kept, not refused.** A machine sends whenever its person
edits, offline too, and right after its application starts, which for a device that keeps a sealed
state is a restore: a refusal there would lose the write for every other device. The core keeps it,
and every send after it, in the order they were made, in what it seals (section 6). Each round signs
what waits, in that order, as far as the device may, once the logs are level and again once the
device's own chain is read back, before the relay is given what it lacks: the round that reads what a
send waited for gives it the relay, and its outcome names it in `taken` (section 5). The same send
made again while it waits waits once (`SCE_PEER_KEPT_WAITING` again), and one of another content under
the stamp of a write that waits is refused at once. When a round's log says the person never may (a
reader, a device taken off before it read the list), every send that waited is refused then, each with
why, in the outcome's `dropped`, and `rebuild` is true: the machine took the write in as made and no
other copy will hold it, so the host builds its machine again and makes a resync, as for an evicted
operation, and a host that keeps its own log of commands takes the dropped send out of that log (and
may tell its person). What waits counts in the standing (`waiting`, section 8).

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
| `taken` | ended | the sends that waited (section 4) and were signed and given the relay, in the order they were made, each `[name, payload text]` as the machine sent it |
| `dropped` | ended | the sends that waited and never will be signed, each `[name, payload text, reason]` (section 4) |
| `rebuild` | ended | whether the host is to build its machine again and make a resync (an operation was evicted or restored, or a send was dropped) |
| `stalled` | ended | whether the round stopped before the end of what the relay holds; a later round goes on from there |
| `reread` | ended | whether the group's device needed a second read from the start, whose events are in `events` after the round's |
| `steps` | ended | what the group's device did with what was read, each `{"step": "joined" or "applied", "epoch": text}`, `{"step": "removed"}` or `{"step": "refused", "reason": text}` |
| `owed` | ended | how many commits and Welcomes of the administrator's acts the relay is still owed once the exchange ended (section 7); one the relay did not take is among `refused` |
| `token` | a login that ended | `{"token": text, "expires_at_ms": text}`: the token the relay issued, and when it expires by the relay's clock; the member is not written for any other exchange |
| `failure` | failed | `"link"` (the transport), `"relay"` (the relay refused the round as a whole) or `"unexpected"` (an answer that is not the answer to the request) |
| `code` | failed by the relay | the relay protocol's word for the refusal (`"forbidden"`, `"rate_limited"`, ...), and `retry_after_ms` as text when it is a rate limit |
| `reason` | failed | why |

A 64-bit number is written as text, as the relay protocol writes it. The MLS records a round read are
taken in by the group's device inside the core and are not in the outcome; what the device did with
them is in `steps`.

The floor event is the pair `[name, payload text]` the machine is told.

**A login** (`sce_peer_login(peer, audience)`, ADR 0080) is an exchange driven the same way: a
`challenge` for the device's node, and a `login` signed with the device's person's key for
`audience`, the relay's name the device was configured with (shipped with the relay's address, or
carried in an invitation), never the one the relay says. A relay that names itself otherwise is
signed nothing, and the outcome is a failure, `"unexpected"`. The login brings the entry of the
node's latest grant the core holds, so that a relay that was not given it takes it. Its outcome is
`"ended"` with `token` and tells the machine nothing (`events` empty), or a failure in the relay's
word: `unauthorized` (the nonce, its time or the signature), `forbidden` (the log gives the node no
role: a device taken off), `behind` (the relay lacks the log the login stands on, or holds no log of
the list), `rate_limited`. It is carried over a link that presents no token, or one that expired
(section 11). The core holds no token and logs in only when its host begins a login: a round the
relay refuses `unauthorized` (an expired token) ends as any refused round does, and the host logs in
and runs the round again.

## 6. What the device keeps

What a device keeps between runs is **one sealed state**: its person's key, the administrator's key
it holds, the keys of the epochs it holds (none of an epoch the list had destroyed), the group's
device (its MLS state, and the commits, Welcomes and key packages it was given and waits to use), the
membership log, what the trust holds, the state of the relay's client as it would be if the exchange
that runs stopped there, the commits and Welcomes the relay is still owed (section 7), and what the
machine sent that waits to be signed (section 4), with what became of the sends that waited that an
outcome has not said yet. The keys
are given once, when the device is set up (`sce_peer_set_identity`, `sce_peer_set_administrator`,
`sce_peer_with_group`); from then on a host never handles a protocol secret in the clear, and gives
no key again to restore (ADR 0079 section 6).

The state is sealed by **envelope encryption**, the pattern of a platform key store whose keys are not
exportable:

1. `sce_peer_save_sealed(peer, state, key)` draws a data key of 32 bytes from the operating system for
   this save alone, seals the state under it, and gives both;
2. the host wraps the data key with a key its platform's key store holds and does not export (below),
   keeps the wrapped key beside the state, and zeroes its copy of the data key;
3. to restore, the host makes a core for the list (`sce_peer_open`, or `sce_peer_invited` with the
   genesis), unwraps the data key, gives `sce_peer_restore_sealed(peer, node, state, len, key, count)`
   the state, the key and the device's node, and zeroes the key. The core calls no platform's API.

The state and its wrapped key are a pair: a state does not open under the key of another save. A host
writes them together, as one file (the wrapped key, then the state) written to a new name and renamed
over the old, so that a stop in the middle leaves the old pair or the new one, never one of each.

**The sealed form.** `SCES1`, a nonce of 24 bytes, and the state sealed with XChaCha20-Poly1305 (the
cipher of the passphrase backup, `sce_rust_trust::backup`). The cipher's key and the key the nonce is
made with are drawn from the data key by HKDF-Expand under labels of their own, and the nonce is
HMAC-SHA-256 under the second of what is sealed: a synthetic nonce, so the same key and the same state
give the same bytes (which the recorded scenario holds), and a key a host gives twice gives away only
whether two states were the same, never the keystream. The list's name, the device's node and the
form (`sce-peer/state/4`) are associated data: a state opens only for the device it was sealed for,
and a state of another list or node, another key, a changed byte and a cut are each
`SCE_PEER_ERR_NOT_OPENED`, which a cipher cannot tell apart; bytes cut short of any sealed state or of
another form are `SCE_PEER_ERR_UNREADABLE`. Either way the core took nothing. The core zeroes the data
key, the keys drawn from it and the plain state once they are used; what the MLS library keeps of the
group in memory is the library's and is not zeroed.

The form is pinned by a known-answer vector, `backends/rust/trust/tests/fixtures/sealed_state.json`:
a fixed data key, list, node, seed and extras, what is made on the way (the contents as sealed, the
associated data, the cipher's key and the nonce's key, the nonce) and the sealed bytes, for an
implementation in another language to check itself step by step. Its test
(`a_sealed_state_is_the_bytes_its_construction_makes`) holds the bytes the library seals equal to the
recording, and equal to what the construction gives when it is made again from the primitives alone:
`SCEB1` and each part of the contents after its length (a big-endian 64-bit number), the associated
data as `sce-trust/state/1`, the list, the node and the form each after its length, the two keys as
HKDF-Expand with SHA-256 of the data key under `sce-trust/state/1 seal` and `sce-trust/state/1 nonce`,
the nonce as HMAC-SHA-256 under the nonce's key of the associated data's length, the associated data and
the contents, cut to 24 bytes, and `SCES1`, the nonce and XChaCha20-Poly1305 under the cipher's key of
the contents with the associated data. A change of the form is recorded again with
`SCE_SEALED_KAT=write cargo test -p sce-rust-trust --test a_sealed_state_is_the_bytes_its_construction_makes`,
which is a decision, not a drift.

**A data key for each save, not one for the device.** A key seals one state, so no nonce is ever at
risk; a key that leaked (a dump of the process's memory) opens one save, not every save before and
after it; and a platform's key the host rotates retires every save wrapped under the old one once
the old one is deleted from the key store, where one data key for the device would be wrapped again
under the new key and keep every old save readable. The cost is one wrapping of 32 bytes at each save.
With a platform key whose public half wraps (the Secure Enclave's, an RSA key of the Android Keystore),
a save in the background asks for no user's presence, and only a restore can.

**What the core cannot tell: an older state.** The core keeps nothing across runs but what it is
given, and no platform gives it a counter no one can turn back, so an older state and its wrapped key
(a copy of the storage, a backup, a save the device ran past before it was stopped) open as well as
the last. What an older state could do is held by the core: a restored device signs nothing new until
its own chain is read back (below), so it never signs at a number it used; and the key of an epoch the
list destroyed since the older save is zeroed when the next round reads the entry that destroys it.
What it cannot hold: someone who has the device's storage and the use of its platform key opens an
older state, as they open the last. Deleting a platform key the host rotated out is what retires every
state wrapped under it.

**The keys of each platform.** The core needs only a wrap and an unwrap of 32 bytes; which platform
key does it is the host's, and no platform code is in this repository.

- **Android.** An AES-256 key of the Android Keystore (`KeyGenParameterSpec.Builder(alias,
  PURPOSE_ENCRYPT or PURPOSE_DECRYPT)`, `BLOCK_MODE_GCM`, `ENCRYPTION_PADDING_NONE`, key size 256),
  used through `Cipher.getInstance("AES/GCM/NoPadding")`: the Keystore draws the IV of each wrapping
  (randomized encryption is required), and the IV is kept with the wrapped key. In StrongBox
  (`setIsStrongBoxBacked(true)`) when the device has one (`FEATURE_STRONGBOX_KEYSTORE`), in the TEE
  otherwise (`StrongBoxUnavailableException`). `setUserAuthenticationRequired` is the host's choice of
  when a device may save: not required, a background job (WorkManager) saves and restores without the
  user, and the key still never leaves the device; required with a window
  (`setUserAuthenticationParameters(seconds, AUTH_BIOMETRIC_STRONG or AUTH_DEVICE_CREDENTIAL)`), a
  save after the window fails until the user authenticates again, so the host saves in the foreground;
  required at each use (0 seconds), each wrapping needs a `BiometricPrompt` with the `Cipher`, which
  no background job has. A host that wants the user's presence for a restore and none for a save uses
  an RSA key of the Keystore instead (`PURPOSE_ENCRYPT or PURPOSE_DECRYPT`,
  `ENCRYPTION_PADDING_RSA_OAEP`, SHA-256): it
  wraps with the public half, which needs no authentication, and unwraps with the private half under
  authentication. `setUnlockedDeviceRequired(true)` stops either while the screen is locked. A
  Keystore key is not in a backup, so a backup of the files holds a state no other device opens; the
  recovery path is the passphrase backup (ADR 0079 section 6). The Kotlin replay
  (`backends/kotlin/peer-replay`) wraps the data key with a software AES key through the same `Cipher`
  calls.
- **iOS, iPadOS and macOS.** The Secure Enclave where there is one: a P-256 key made in it
  (`kSecAttrTokenIDSecureEnclave`, with `SecAccessControlCreateWithFlags(...,
  kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly, .privateKeyUsage, ...)`, adding `.userPresence`
  when a restore should ask for the user); the data key is wrapped with `SecKeyCreateEncryptedData` and
  `.eciesEncryptionCofactorVariableIVX963SHA256AESGCM` under the public key, which needs neither the
  enclave nor the user, so a background task saves; it is unwrapped with `SecKeyCreateDecryptedData`
  in the enclave. Without an enclave: a 256-bit key kept as a Keychain item with
  `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`, and the data key sealed under it with AES-GCM
  (CryptoKit `AES.GCM.seal`, its combined form kept). `AfterFirstUnlock` lets a background task (a
  `BGProcessingTask`, a push) save and restore after the device was unlocked once since it started,
  where `WhenUnlocked` would stop it while locked; `ThisDeviceOnly` keeps the key out of every backup
  and off every other device, which is what makes the state the device's alone.
- **Desktop.** macOS as above. Windows: a key of the TPM through the Microsoft Platform Crypto
  Provider (`NCryptCreatePersistedKey`, RSA, the data key wrapped with OAEP), or DPAPI
  (`CryptProtectData` in the user's scope) where there is no TPM. Linux: a key sealed to the TPM 2.0
  (tpm2-tss), or a wrapping key kept by the Secret Service (libsecret: GNOME Keyring, KWallet). A
  desktop with none of these asks its user for a passphrase and makes the wrapping key from it with
  Argon2id, which is the passphrase backup's derivation (`sce_rust_trust::backup::Cost`), and the
  state is then as strong as the passphrase. The web: a WebCrypto AES-GCM key made with
  `extractable: false` and kept in IndexedDB, which wraps and unwraps and is never read out.

The state is the core's version 4. Version 3 was the first that is sealed; version 4 holds what waits
to be signed, and since the version is bound into the sealed state, a core of version 3, which would
open a state of version 4 and lose what waits, does not open it. The texts of versions 1 and 2 held no
secret and were taken back only after the host gave every key again; they, and the sealed states of
version 3, were never written outside this repository's tests (no device ships before ADR 0034's
review), so they are not read.

**A restored device signs nothing new until its own chain is read back.** The core cannot tell the
last state from an older one, so a restored core keeps a send that would make a new operation
(`SCE_PEER_KEPT_WAITING`, section 4), and still gives what it holds as held, until a round has read the
device's own operations back from the relay (`docs/SCE_SHARED_LIST_RELAY.md` section 6); that round
signs what waited at the numbers after the device's last, and a send that waited and that the chain
read back holds already is the operation it holds, signed once. A host that rebuilds its machine from
its own log after a restore gives the core the machine's sends when it likes, and makes a resync.
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
| `waiting` | how many sends of the machine wait to be signed (section 4): what a host shows as not sent yet |

The standing says nothing of a token: the core holds none, and the token a login earned is the
host's, given to its link (section 11).

## 9. The recorded scenario every lane replays

`backends/rust/peer-capi/tests/fixtures/two_devices_and_a_relay.json` is a run of two devices of one
list through the ABI against the real relay in one process: alice's device, which holds the
administrator's key, opens the list and grants bob writing, and bob's, invited with the hash of the
genesis alice's core gave. Every call each core is given and every text it gives back is recorded in
order: the opening and its genesis, the grant, what each device stands at, a write of bob's made
before his device read the log, kept waiting (and sent again, waiting once) and signed by his first
round, his writes and his note through `sce_peer_outbound`, an event that is not signed, another
content under a stamp he signed (refused), each request and the relay's answer, the outcomes, bob's
sealed state and his core made again given nothing but the list and the genesis and restored from it
with no key given again, a write of his that comes back held and a new one kept waiting until his
chain is read back, which the round that reads it signs and which then comes back held, a resync, a
round of alice's whose transport fails, and bob taken off: a write his core signs before it read the
removal reaches no one, since the relay refuses a device the log took off. Bob's device is handed no
token: it logs in (a `login` step, which names the audience, and its requests, answers and outcome)
before its first round, and the relay's answers to its rounds are made with the token that login
earned; once taken off, his device logs in again and is refused (`forbidden`). Alice's device, which
gives the relay the list's first entries, presents a token of the relay's authority, as a relay
whose creation of lists is closed takes. A `save` step holds the
data key and the sealed state, in hex; a `restore` step holds the same and the count, and a lane makes
the device's core again before it restores.

Because the core does no I/O a lane replays it with no relay: it makes the same calls, compares each
request, outcome, standing, genesis, sealed state and refusal reason with the recording, and gives the
core the recorded answers. The Kotlin lane keeps the data key between the two steps wrapped, as a host
does (section 6).
Rust, C, C++, Python (ctypes), Go (cgo), Kotlin (JNA through the binding, `backends/kotlin/peer-replay`)
and Swift (the module map) replay the same file
(`a_c_abi_is_what_the_lanes_call_and_replays_the_recorded_scenario`); a lane whose toolchain is
missing is skipped unless `SCE_REQUIRE_TOOLS` is set, and the Kotlin lane needs
`./gradlew :sce-kotlin-peer-replay:installDist`.

Everything in it is deterministic (Ed25519 signs deterministically; bob's state is sealed with
`sce_peer_save_sealed_under` under a data key the recording fixes, and its nonce is a keyed hash of
what it seals; the relay's challenges are made under a key the recording fixes; nothing reads a clock
or draws a random number), so a run now records the same file, and
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
  `wss://host:port`), presenting `token`, the token the device's login earned, or none when `token`
  is null or empty. Nothing is dialled until a run asks; the link dials again after anything that did
  not come to a response and after the network changed. A token the relay refuses at the upgrade
  comes back as the relay's refusal (`failure: "relay"`, `code: "unauthorized"`), as one that expires
  on an open connection does: the host logs in (`sce_peer_login`) over the same link and gives it the
  new token, or asks its application's service for one. A relay that offers login takes a link with
  no token, and one whose token it issued and that expired, as a connection that logs in; a token it
  did not issue (its key changed) it refuses still, so a host that logs in after a refusal first gives
  the link no token (`sce_peer_link_set_token(link, NULL)`). Over
  `wss://` the link runs TLS
  on `ring`'s cryptography, which the library names to rustls itself, so a host installs and
  initialises nothing, on Android as anywhere; and it believes Mozilla's certificate authorities,
  compiled into the library, not the device's trust store. A relay whose certificate none of them
  issued fails the run as the link's (`failure: "link"`, with a `reason` that names the
  certificate). What was chosen and why, the device's store included, is
  `docs/SCE_SHARED_LIST_RELAY.md` section 9 (decided 2026-11-02).
- `sce_peer_link_wss_believing(base, token, authorities, len, roots)` is the same link, believing
  for a `wss://` relay the certificate authorities in the `len` bytes at `authorities`: one or more
  certificates in PEM, as a deployment hands its authority out, or one in DER, so that a lane gives
  the bytes of the file it ships (an Android raw resource, a file of an iOS bundle, read whole) and
  parses nothing. `roots` is `SCE_PEER_ROOTS_ONLY` (0) for those authorities alone, a relay under a
  private authority, or `SCE_PEER_ROOTS_AND_PUBLIC` (1) for those and the public ones, a deployment
  whose relays are under either or move from the one to the other: an application it shipped
  believing its own authority alone would stop reaching a relay the day its certificate came from a
  public authority. The bytes are read when the link is made and not kept. Null, with why, when the
  bytes hold no certificate or one that is not a certificate, when `roots` is neither, and for an
  address `sce_peer_link_wss` refuses.
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
- `sce_peer_link_set_token(link, token)` gives the link the token it presents from its next dial
  (null or empty for none): the one a login's outcome holds. A connection open now that was dialled
  with another is given up, and the next run dials again (a `lost` notice, then `connected`). It is
  called when no run uses the link. A host's own transport presents its own credentials and is
  refused (`SCE_PEER_ERR_BAD_ARGUMENT`). A host's flow is: a link with no token, `sce_peer_login`,
  `sce_peer_run`, the outcome's `token` to `sce_peer_link_set_token`, then rounds; when a round's
  outcome is the relay's `unauthorized`, give the link no token and log in again over it, then give
  it the new token. A host may log in before the token's `expires_at_ms` too (the relay's clock: a
  host compares it with the time it got the token, not with its own clock's reading of it).
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
