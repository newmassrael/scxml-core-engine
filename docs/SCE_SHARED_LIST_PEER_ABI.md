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
  `token`, which the policy above makes a new number. The invitations and the join of ADR 0080
  section 4 (section 12) are functions added alone (`sce_peer_invite`, `_invite_under`,
  `_invitation_read`, `_invited_by`, `_join`, `_joined`), with statuses appended
  (`SCE_PEER_ERR_GRANTED`, `_INVITATION`, `_NOT_JOINED`) that only those functions give, and the
  administrator's acts on a node the core invited, which only they make: a lane of version 5 that
  calls none of them sees nothing change, and the number stays 5. The sealed state is the core's
  version 5, which holds the invitations (section 6). The creation of a list with no token and its
  lapse (ADR 0080 section 3, section 13) move no number either: `sce_peer_certified_name` is a
  function added alone, and `sce_peer_login` creates a list only at a relay whose challenge asks
  work, which no relay asked before, with an outcome of a login's shape. The version is 6
  (2026-11-03): `sce_peer_run` waits out a relay's refusal for the rate and puts the same request
  again while the wait ends before its deadline (section 11), where version 5 ended the round with the
  refusal, a changed meaning of a function; a host's transport may say its connection was refused for
  the rate (`SCE_PEER_TRANSPORT_RATE_LIMITED`) and give the wait on its own clock
  (`sce_peer_link_set_pause`, `ScePeerPauseFn`, `SCE_PEER_PAUSE_*`); and a host installs the sink of
  what the library logs (`sce_peer_set_log_sink`, `ScePeerLogFn`, `SCE_PEER_LOG_*`, section 14),
  with a status appended (`SCE_PEER_ERR_SINK_TAKEN`). The outcome's `conflicted` (section 5, ADR
  0083 section 4) moves no number: it is written only for an exchange that froze the device's own
  chain, which a lane of version 6 reads by members' names and meets only when a relay of protocol 2
  answers `conflict` or a round finds the device signed two histories; every other outcome's text is
  the one it was. The version is 7 (2026-11-09, docs/adr/0085 landing 4c): a change of the list is
  one event and one signed operation, `record_list.change`, so what `sce_peer_outbound` takes and the
  events an outcome holds are of another shape than the parts `record_list.write` and
  `record_list.supersede` of version 6, which the core no longer signs (section 4), a changed shape
  of a text that the policy above makes a new number. The version is 8 (2026-11-10, docs/adr/0085
  landing 5, which is ADR 0083's landing 3 at change units with ADR 0093's core folded in, what ADR
  0083 counted among "version 7"): the change a copy that keeps a journal offers is signed in the
  call that takes it (`sce_peer_made`) and is a record the host journals (`sce_peer_take_records`),
  and a round gives the relay no operation of the device's own past the first the host has not said
  it recorded (`sce_peer_recorded`), which a lane of version 7 never says, a changed meaning of a
  round; the outcome holds `withdrawn` in place of `dropped`, a send that never will be signed
  withdrawn with every send after it and the machine built again for none of them, and `raised`
  (section 5); `sce_peer_floor_event` is gone, since the list reads no floor, and the stable cut and
  a device's mark say what every member was told (`sce_peer_stable_cut`, `sce_peer_mark`); a host
  started again gives back its journal's records (`sce_peer_restore_records`), recalls the history
  (`sce_peer_history`) and raises its copy's clock to the device's own stamp (`sce_peer_own_stamp`)
  (section 4). The sealed state is the core's version 6, which holds the outbox (section 6). The
  fold of a list held before a change was one unit (docs/adr/0085 landing 5b) moves no number: the
  fold (`sce_peer_fold`, section 4) is a function added alone, and `sce_peer_restore_sealed` opens
  and upgrades a state of the core's version 5 (section 6), which it refused as not opened before:
  a state no core of version 8 seals, and no output of before changes its shape;
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
| `SCE_PEER_ERR_NO_ADMINISTRATOR` | 6 | an administrator's act, and the core holds no administrator's key (`sce_peer_set_administrator`), or, for an invitation, none the log names as an administrator's |
| `SCE_PEER_ERR_NO_PERSON` | 7 | the act needs the device's person and the core has none (`sce_peer_set_identity`) |
| `SCE_PEER_ERR_OPENED` | 8 | the list is open already: the core's log holds its genesis |
| `SCE_PEER_ERR_LOG` | 9 | the membership log refused the entry the act wrote (an administrator's key the log does not name, a genesis that is not the one the device was invited with); nothing was written |
| `SCE_PEER_ERR_NOT_A_MEMBER` | 10 | the device of that node is no member of the list now |
| `SCE_PEER_ERR_ADMITTED_THROUGH_GROUP` | 11 | the list is keyed by its group, and a grant would make a member the group does not hold: the device is admitted with `sce_peer_add_member` |
| `SCE_PEER_ERR_NO_KEY_PACKAGE` | 12 | no key package of the device was read from the relay yet |
| `SCE_PEER_ERR_DRAWN_KEYS` | 13 | the list's keys are drawn by an administrator and not its group's, and the core does not hand the members left the new key a removal needs |
| `SCE_PEER_ERR_NOT_OPENED` | 14 | a sealed state did not open under the data key given, for this list and the node given: another key, a changed or cut state, or another device's (a cipher cannot tell these apart); nothing of it was taken (section 6) |
| `SCE_PEER_ERR_NO_RANDOMNESS` | 15 | the operating system gave no randomness for a data key or an invitation's secret; no weaker source is used |
| `SCE_PEER_ERR_GRANTED` | 16 | an invitation of a node the log has granted (a removal included): an invitation is for a node it never granted (section 12) |
| `SCE_PEER_ERR_INVITATION` | 17 | an invitation that is not one (another form, a text cut or changed), one of another list, genesis or node than the core's, a lifetime outside 1 ms to a day, or a list not opened yet |
| `SCE_PEER_ERR_NOT_JOINED` | 18 | the core invited the node, and no join a round read proves the key given under its invitation: a key package or a key the relay made up, or a join not read yet; nothing was written |
| `SCE_PEER_ERR_NULL_ARGUMENT` | 20 | a pointer, a handle or a place for the result is null |
| `SCE_PEER_ERR_BAD_TEXT` | 21 | a text is not UTF-8 |
| `SCE_PEER_ERR_PANIC` | 22 | the library panicked; the panic was caught |
| `SCE_PEER_ERR_BAD_ARGUMENT` | 23 | a number is outside what it may be (a kind of round, a profile, a role); an address a link cannot be made to; a token given to a host's own transport |
| `SCE_PEER_ERR_CANCELLED` | 24 | `sce_peer_run` stopped because its cancellation was raised; the exchange ended as a failure of the link, and its outcome waits (section 11) |
| `SCE_PEER_ERR_DEADLINE` | 25 | `sce_peer_run` stopped because its deadline passed, with the same end |
| `SCE_PEER_ERR_SINK_TAKEN` | 26 | `sce_peer_set_log_sink`: another sink of the library's records was installed in the process first (a Rust library in the same image installed its own); nothing was installed (section 14) |

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
| `sce_peer_certified_name(administrator, salt, salt_len, out)` | `Peer::certified_name`, `Peer::draw_certified_name` | the name of a list to open at a relay that takes a creation with no token, which certifies the administrator's key a 32-byte seed makes, under the host's salt of 1 to 32 bytes or 16 the library draws when `salt_len` is 0 (section 13); `SCE_PEER_ERR_BAD_ARGUMENT` for a salt longer |
| `sce_peer_set_identity(peer, node, seed)` | `Peer::set_identity` | the device's person: their node and the key a 32-byte seed makes |
| `sce_peer_set_administrator(peer, seed)` | `Peer::set_administrator` | the administrator's key the device holds, from a 32-byte seed |
| `sce_peer_open_list(peer, profile, genesis)` | `Peer::open_list` | opens a new list (section 7); `genesis` is set to the hash (32 bytes) a person is invited with |
| `sce_peer_grant(peer, node, role, key)` | `Peer::grant` | grants the device of `node`, which signs with a 32-byte public key, a role of 1 to 3; for a node the core invited, only the key a join proved (section 12) |
| `sce_peer_add_member(peer, node, role, key)` | `Peer::add_member` | admits a device to a list keyed by its group, with the grant and the commit in one change; for a node the core invited, only from the key package of a join that proved `key` (section 12) |
| `sce_peer_remove_member(peer, node)` | `Peer::remove_member` | takes a device off the list, and off its group when the list is keyed by one |
| `sce_peer_standing(peer, out)` | `Peer::standing` | what the device stands at in its list, as one JSON text (section 8) |
| `sce_peer_for_a_hub(peer)` | `Peer::for_a_hub` | before the first exchange: the machine is a hub's |
| `sce_peer_with_group(peer, node, seed)` | `Peer::with_group` | once, before the first exchange of a device with no sealed state yet: the core holds a new device of `node` in the list's MLS group, which the sealed state keeps from then on |
| `sce_peer_outbound(peer, event, data, kept, reason)` | `Peer::outbound` | what the machine of a copy that keeps no journal sent, and what the core did with it (section 4) |
| `sce_peer_made(peer, data, kept, reason)` | `Peer::made` | the change a copy that keeps a journal offered (`record_list.made`'s payload), signed in the call; `kept` is the verdict the host gives the copy (section 4) |
| `sce_peer_take_records(peer, out)` | `Peer::take_records` | the records the host has not said it recorded, as a JSON array of `{"seq", "event", "data"}`, the number as text (section 4) |
| `sce_peer_recorded(peer, through)` | `Peer::recorded` | the host recorded every record up to `through`: the next round gives the relay what it lacks of them |
| `sce_peer_restore_records(peer, records, count)` | `Peer::restore_records` | a journal's records past the sealed state restored, in the form `sce_peer_take_records` gives, taken back as the device's own; `count` (may be null) is how many the core did not hold; `SCE_PEER_ERR_BAD_ARGUMENT` for a text that is not that array, `SCE_PEER_ERR_UNREADABLE` for a record that is not the device's or does not hold |
| `sce_peer_history(peer, after, out)` | `Peer::history` | what a machine built again is given, as one JSON object (section 4), from the cut `after` (a JSON array of `[node, seq]` pairs as texts) on, or the whole of it when `after` is null |
| `sce_peer_stable_cut(peer, out)` | `Peer::stable_cut` | the stable cut, as a JSON array of `[node, seq]` pairs as texts: for each device, the highest number every member is known to have been told |
| `sce_peer_fold(peer, parts, out)` | `Peer::fold` | what the parts of changes signed before a change was one unit make together, as one JSON object `{"restores", "cut"}` (section 4): the state as `record_list.restore` payload texts of at most `parts` values each, and each device's highest number among the parts as `[node, seq]` pairs as texts |
| `sce_peer_mark(peer, seq)` | `Peer::mark` | signs a mark of what the machine was told when one is owed; `seq` (may be null) is its number, 0 when none was |
| `sce_peer_own_stamp(peer, wall_time, counter, read_back)` | `Peer::own_stamp` | the highest stamp of the device's own changes the core holds (0 and 0 for none), and 1 once it has read its own chain back; each out may be null |
| `sce_peer_start(peer, kind)` | `Peer::start` | begins a round: `SCE_PEER_ROUND_SYNC`, `_RESYNC` or `_REREAD` |
| `sce_peer_publish_checkpoint(peer, before, digest)` | `Peer::publish` of a checkpoint | begins the exchange that gives the relay the person's checkpoint up to `before` |
| `sce_peer_publish_key_package(peer)` | `Peer::publish_key_package` | begins the exchange that leaves a key package of the group's device with the relay |
| `sce_peer_login(peer, audience)` | `Peer::login` | begins the exchange that logs the device in at the relay named `audience` (the name it was configured with), whose outcome holds the token (section 5); at a relay that takes a creation with no token, the login of a list the relay does not hold creates it (section 13); `SCE_PEER_ERR_NO_PERSON` with no person |
| `sce_peer_invite(peer, relay, audience, node, now_ms, valid_ms, out)` | `Peer::invite` | an administrator's invitation of the device of `node`, which the log never granted, at the relay of address `relay` and name `audience`, from `now_ms` for `valid_ms` (at most a day): `out` is the text for a QR code (section 12) |
| `sce_peer_invite_under(peer, relay, audience, node, now_ms, valid_ms, secret, out)` | `Peer::invite_under` | the same under a 32-byte secret the host gives: one it drew, or a recording's fixed one |
| `sce_peer_invitation_read(invitation, len, out)` | `Ticket::from_text` | what the invitation's text says, as one JSON text: `relay`, `audience`, `list`, `node`, `genesis`, `expires_at_ms`; no secret |
| `sce_peer_invited_by(invitation, len)` | `Peer::invited` | the core of the device the invitation invites, for its list and genesis; null on failure |
| `sce_peer_join(peer, invitation, len, fingerprint)` | `Peer::join` | begins the exchange that leaves the device's identity key, and its key package when the core holds a device of the group, under the invitation; `fingerprint` is set to the digits the person compares (section 12) |
| `sce_peer_joined(peer, node, out)` | `Peer::joined` | the devices that joined under the core's invitation of `node`, as a JSON array of `{"key", "fingerprint"}`; empty when none was read, and once the node is admitted |
| `sce_peer_poll_request(peer, out)` | `Peer::poll_request` | the next request text for the relay; empty when none waits |
| `sce_peer_respond(peer, response)` | `Peer::respond` | the relay's response text to that request |
| `sce_peer_fail(peer, reason)` | `Peer::fail` | the transport did not carry the request or its answer |
| `sce_peer_take_outcome(peer, out)` | `Peer::take_outcome` | what the exchange came to, once; empty when none ended |
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
| `sce_peer_link_set_pause(link, pause)` | the run's `driver::Clock` | the wait of a run over a host's own transport, on the host's clock, null for the system's (section 11); `SCE_PEER_ERR_BAD_ARGUMENT` for a link of the library's own transport |
| `sce_peer_reply(reply, text, len)` | | from a host's transport: the response of the exchange, or the reason it has none |
| `sce_peer_set_log_sink(max_level, sink, user)` | the `log` facade's sink | the process's sink of what the library logs, replaceable, null or `SCE_PEER_LOG_OFF` for none (section 14) |
| `sce_peer_cancel_new()`, `sce_peer_cancel_free(cancel)` | a `Signal` | a cancellation of a run, and freeing it |
| `sce_peer_cancel_raise(cancel)` | `Signal::raise` | cancels the run given it; any thread, at any time |
| `sce_peer_run(peer, link, within_ms, cancel)` | `drive` | runs the exchange begun on the core over the link, on the calling thread, to its end, its deadline or its cancellation |

## 4. A send of the machine

The machine's outbound `record_list.change`, addressed to `#people`, is given as the event's name
and its JSON payload, as the machine wrote it: one change whole, signed as one operation
(docs/adr/0085, section 1).
`*kept` says what the core did:

| Answer | Value | Meaning |
|---|---|---|
| `SCE_PEER_KEPT_TAKEN` | 0 | signed and remembered; the next round carries it to the relay |
| `SCE_PEER_KEPT_HELD` | 1 | the device holds it already (a machine built again from its application's own log sends its changes again): nothing is signed, and the relay is given nothing new |
| `SCE_PEER_KEPT_NOT_SIGNED` | 2 | it is no event that is signed (a part `record_list.write` or `record_list.supersede` of version 6 included); nothing is kept |
| `SCE_PEER_KEPT_REFUSED` | 3 | the trust will not sign it, and `*reason` says why: another content under a dot the device signed or that waits, another person's change the device holds no signature of, a change that names nothing or writes one register twice, a person the log the device holds does not let write (a reader, a device taken off), a frozen chain, a device with no person; nothing is kept |
| `SCE_PEER_KEPT_WAITING` | 4 | the device may not sign it yet, and `*reason` says why: it has not read the log that lets it write (an invited device before its first round, a log that grants it nothing yet), the key of the epoch the list's content is in, or, restored, its own chain; it is kept (below) |

**A send the device may not sign yet is kept, not refused.** A machine sends whenever its person
edits, offline too, and right after its application starts, which for a device that keeps a sealed
state is a restore: a refusal there would lose the write for every other device. The core keeps it,
and every send after it, in the order they were made, in what it seals (section 6). Each round signs
what waits, in that order, as far as the device may, once the logs are level and again once the
device's own chain is read back, before the relay is given what it lacks: the round that reads what a
send waited for gives it the relay, and its outcome names it in `taken` (section 5). The same send
made again while it waits waits once (`SCE_PEER_KEPT_WAITING` again), and one of another content under
the dot of a change that waits is refused at once. When a send that waited never will be signed (the
round's log says the person never may: a reader, a device taken off before it read the list; or it
would conflict with what the device signed, a stamp that goes back along its chain among them), it is
withdrawn, and so is every send after it, in order, each with why, in the outcome's `withdrawn`
(docs/adr/0093, section 6.2): each was made on a state that held the one before it. The copy took
each in provisionally, and the host withdraws them in place (below); no machine is built again for
it. What waits counts in the standing (`waiting`, section 8).

**A change a copy that keeps a journal offers** (docs/adr/0083 section 3, docs/adr/0093 section 3)
is given to `sce_peer_made` as `record_list.made`'s JSON payload: the change (`by`, `wallTime`,
`counter`, `ops`) and the `command` that closed it, which is the host's and is not signed. The core
signs it in the call, as one operation, and `*kept` is the verdict the host gives the copy
(`record_list.verdict`): `SCE_PEER_KEPT_TAKEN` is `accepted`, `_HELD` `held`, `_WAITING` `waiting`
and `_REFUSED` `refused`, with `*reason` for the last two. What is signed is a **record**: its number,
its event and the text the relay is given (`sce_peer_take_records`). The host appends it to its
journal in the transaction that holds the change and its verdict, says so (`sce_peer_recorded`), and
only then gives the copy the verdict; no round gives the relay an operation of the device from the
first record not recorded on, so nothing the person is shown as saved is held by anyone before the
journal holds it. A change that waited and is signed by a round is a record too, held back the same
way, and the round's `taken` names it: the host records it and gives the copy `record_list.confirm`;
the next round gives it the relay. The records are in the sealed state until the host says it
recorded them. A host that keeps no journal gives the core what its copy sends to `#people`
(`sce_peer_outbound`), which is released at once.

**A start.** A host started again restores the sealed state (section 6), gives back the records its
journal holds past it (`sce_peer_restore_records`), which are verified and taken as the device's own,
recorded already, before it signs anything, and gives the changes its journal holds that the core
does not (each `held` or kept again, never prepared again). It recalls the history into a fresh
machine (`sce_peer_history`): `{"roster", "changes"}`, `roster` the pairs `[name, payload text]` a
round tells the roster in, `changes` every change the core holds that a round would tell, the
device's own included, whole and in causal order, each `{"event", "data", "provisional": false,
"author", "seq", "context"}` with its tag read from its link (numbers as text, `context` pairs
`[node, seq]`), from the stable cut the host folded its journal at; then the device's own changes that
wait to be signed, each `{"event", "data", "provisional": true}`, which the copy recalls
provisionally. It gives the copy `record_list.raise` with the device's own stamp
(`sce_peer_own_stamp`), and again whenever a round's outcome holds `raised`. **The stable cut**
(`sce_peer_stable_cut`) is, for each device, the highest number every member is known to have been
told, read from each member's latest operation; a member that has written nothing for a while says
what it was told by a mark (`sce_peer_mark`), an operation of its chain no machine is told, which the
next round gives the relay. When "a while" is up is the host's to judge on its own clock. The host
folds its journal at the cut (docs/adr/0085, section 4.3).

**A list held before a change was one unit** (docs/adr/0085, section 8.3) holds its changes as
parts, signed under the second event domain, which the core reads as they were signed and never
signs (`docs/SCE_SHARED_LIST_TRUST.md` section 3). No machine is told a part: what they hold
together is one state, the fold (`sce_peer_fold`): `{"restores", "cut"}`, `restores` the state as
`record_list.restore` payload texts, each of at most `parts` values (the build's `shared_parts`;
none when no value is live), every value with its own dot and every payload with the state's clock,
and `cut` each device's highest number among the parts. A machine built again is given the restores
first and then the history, so a copy whose host has not folded its journal yet, a device upgraded
from a state of version 5 among them (section 6), holds what the list held. A round that takes in a
part the core did not hold says the machine is to be built again (`rebuild`, section 5), since the
fold moved. Two devices that hold the same parts fold the same state, whatever order they read them
in. The fold is a function added alone, and moves no number.

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
| `taken` | ended | the sends that waited (section 4) and were signed, a new operation each, in the order they were made, each `[name, payload text]` as the machine sent it or the copy offered it: one of a host that keeps no journal was given the relay, and one a copy offered is a record the next round gives it once the host recorded it; a send that waited and that the device held already when the round came to it (a device restored from an older state, whose chain the round read back) is in neither `taken` nor `withdrawn`, since nothing was signed for it and the host has nothing to do (the Rust outcome names it in `held`) |
| `withdrawn` | ended | the sends that waited and never will be signed, the first and every one after it, earliest first, each `{"event", "data", "wallTime", "counter", "why"}`, the wall time as text (section 4): the host journals the withdrawal and gives the copy one `record_list.withdraw` naming the earliest |
| `rebuild` | ended | whether the host is to build its machine again and make a resync: an operation was evicted or restored, a part of a change of a list held before a change was one unit among those restored, which moves the fold the machine is built from (section 4). A withdrawal is not one |
| `raised` | a round that raised the device's own stamp | `{"wallTime", "counter"}`, the wall time as text: the highest stamp of the device's own changes, which rose in the round (it read the device's own chain back past what the core held, or signed what waited); the host gives the copy `record_list.raise` with it. The member is not written for any other exchange |
| `stalled` | ended | whether the round stopped before the end of what the relay holds; a later round goes on from there |
| `conflicted` | an exchange that froze the device's own chain | the number, as text, the device's own chain is frozen from: the relay answered `conflict` to an operation of the device's own (the round ended there), or a round read one that conflicts with it (`docs/SCE_SHARED_LIST_RELAY.md` sections 3 and 6). The device signs nothing more, every host that holds both freezes it at the same number, and the host tells its person that an administrator has to take the device off and grant a new one; the member is not written for any other exchange |
| `reread` | ended | whether the group's device needed a second read from the start, whose events are in `events` after the round's |
| `steps` | ended | what the group's device did with what was read, each `{"step": "joined" or "applied", "epoch": text}`, `{"step": "removed"}` or `{"step": "refused", "reason": text}` |
| `owed` | ended | how many commits, Welcomes and keys given of the administrator's acts the relay is still owed once the exchange ended (section 7); one the relay did not take is among `refused` |
| `token` | a login that ended | `{"token": text, "expires_at_ms": text}`: the token the relay issued, and when it expires by the relay's clock; the member is not written for any other exchange |
| `failure` | failed | `"link"` (the transport), `"relay"` (the relay refused the round as a whole) or `"unexpected"` (an answer that is not the answer to the request) |
| `code` | failed by the relay | the relay protocol's word for the refusal (`"forbidden"`, `"rate_limited"`, ...), and `retry_after_ms` as text when it is a rate limit: over a run (section 11), only one whose wait would end at the run's deadline or after it, or never, since a run waits out every other |
| `reason` | failed | why |

A 64-bit number is written as text, as the relay protocol writes it. The MLS records a round read are
taken in by the group's device inside the core and are not in the outcome; what the device did with
them is in `steps`.

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
that runs stopped there, the commits, Welcomes and keys given the relay is still owed (section 7),
and what the machine sent that waits to be signed (section 4), with what became of the sends that
waited that an outcome has not said yet. The keys
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
form (`sce-peer/state/5`) are associated data: a state opens only for the device it was sealed for,
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

The state is the core's version 5. Version 3 was the first that is sealed; version 4 holds what waits
to be signed, and since the version is bound into the sealed state, a core of version 3, which would
open a state of version 4 and lose what waits, does not open it. Version 5 holds the invitations an
administrator's device made (the node, the key the secret makes, the expiry) and the joins under them
a round read (section 12), which a core of version 4 would lose. The texts of versions 1 and 2 held no
secret and were taken back only after the host gave every key again; they, and the sealed states of
versions 3 and 4, were never written outside this repository's tests (no device ships before ADR
0034's review), so they are not read. The known-answer vector of the construction keeps the form
string of version 4 as one of its inputs: it pins the construction, which did not change.

**A restored device signs nothing new until its own chain is read back.** The core cannot tell the
last state from an older one, so a restored core keeps a send that would make a new operation
(`SCE_PEER_KEPT_WAITING`, section 4), and still gives what it holds as held, until a round has read the
device's own operations back from the relay (`docs/SCE_SHARED_LIST_RELAY.md` section 6), from where
the relay's heads and the core agree to where the relay holds nothing more, past a conflict or a gap
its heads stop at; that round signs what waited at the numbers after the device's last, and a send
that waited and that the chain read back holds already is the operation it holds, signed once (in
neither `taken` nor `withdrawn`). A device whose chain the relay holds past a conflict finds it there,
and signs nothing more. A host whose copy keeps a journal gives back the records its journal holds
past the state (`sce_peer_restore_records`) and recalls the history (section 4); the sealed state is
the core's version 6, which holds the records not recorded yet and which waiting changes a copy
offered.
Whether it still waits is in what it stands at (`awaits_own_chain`, section 8).

**A state of version 5 is opened and upgraded** (docs/adr/0085, section 8.3). A device that held a
list before a change was one unit sealed its state at the core's version 5, its operations parts of
changes signed under the second event domain. `sce_peer_restore_sealed` takes it as it is, under the
key it was sealed under: a state that does not open as version 6 is tried as version 5, and one that
opens as neither is `SCE_PEER_ERR_NOT_OPENED`, as before. The device keeps its person's key, the
administrator's key, the keys of the epochs, the group's device, the log and every part, each
verified as it was signed and held where it stands in its chain, so its next change is signed at the
number after its last part; it waits for its own chain as every restored device does. What its
machine sent as parts and its person had not signed waits again as the changes they were parts of;
a write-over sent as a change of one carried no stamp of its change and is withdrawn, with every send
after it, in the next round's `withdrawn`. What the state kept for its machine (the events a round
had not handed over, and what became of the sends that waited) is dropped: no machine of today takes
a part. The host builds its machine again from the fold and the history (section 4), and the next
save is of version 6. A state of version 5 opened so is the device's: nothing of it is asked of the
host again.

## 7. An administrator's acts

A device that holds an administrator's key (`sce_peer_set_administrator`) runs the list. Each act is
one call, and writes the entries of the membership log and makes the change of the group that goes
with them together, so that no host can do one without the other:

- `sce_peer_open_list` writes the genesis (the administrator's key the device holds as the list's one
  administrator, a quorum of one, the hub's profile `SCE_PEER_PROFILE_UNTRUSTED` or `_TRUSTED_HUB`)
  and a grant that makes the device's person a member with the highest role (3), and, when the core
  holds a device of the group (`sce_peer_with_group`, called before), makes the group with that device
  alone in it: a list is keyed by its group from its first entry, or not at all. A list keyed by its
  group is sealed from its first write: the same call moves the group of one to its first epoch with a
  commit that adds no one (an MLS self-update) and writes the entry that names it after the grant, so
  the log is three entries, the relay is owed that commit (`owed` is 1), and nothing the device writes
  is ever in the relay's clear. The first device admitted joins at epoch 2. The hash of the genesis is
  what a person is invited with (`sce_peer_invited`).
- `sce_peer_grant` gives a device a role of 1 (may read), 2 (may also write) or 3 (may also do what an
  owner does), or another role or key to a member; one that takes writing away carries where the
  device's chain ends, as the log requires. On a list keyed by its group it is refused for a device the
  group does not hold (`SCE_PEER_ERR_ADMITTED_THROUGH_GROUP`): a grant alone would make the log's
  members differ from the group's, and every later commit would be refused for it.
- `sce_peer_add_member` admits a device to a list keyed by its group from the newest key package of it
  a round read from the relay (the device leaves one with `sce_peer_publish_key_package`): the grant,
  the commit that adds the device and the entry that names the commit. The device is given the list's
  history: the key of every epoch before its own that the administrator's device holds, each sealed to
  the device's key, which the relay is owed after the Welcome and the device's next round takes in, so
  that it opens what was written before it was admitted (a Welcome gives the key of its own epoch
  alone).
- `sce_peer_remove_member` takes a device off, its chain ending at the last operation of it the
  administrator's device holds; on a list keyed by its group the same call makes the commit that moves
  the group to an epoch without it, since a removal the group did not follow would leave what is
  written after it readable by the device. A list whose content keys an administrator draws (not its
  group's) is refused (`SCE_PEER_ERR_DRAWN_KEYS`): the members left would need a new key the core does
  not hand over.

The log is append-only and a commit made and not merged can be given up, so an act asks everything
that can refuse it and makes its commit before it writes an entry: an act that is refused wrote
nothing. The entries go to the relay with the next round. A commit, a Welcome and a key given go after
them, since the relay keeps each only once its log names or begins its epoch: the round that ends
gives the relay what it is
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
| `owed` | how many commits, Welcomes and keys given the relay is still owed (section 7) |
| `waiting` | how many sends of the machine wait to be signed (section 4): what a host shows as not sent yet |

The standing says nothing of a token: the core holds none, and the token a login earned is the
host's, given to its link (section 11).

## 9. The recorded scenario every lane replays

`backends/rust/peer-capi/tests/fixtures/two_devices_and_a_relay.json` is a run of two devices of one
list through the ABI against the real relay in one process: alice's device, which holds the
administrator's key, opens the list and invites bob's device by the text of a QR code (section 12),
and bob's, made from that text alone (`make_invited`), which joins under the invitation with his key
and shows a fingerprint; alice's round reads the join, her device shows the same digits (`joined`),
and she grants writing to the key the proof names. Every call each core is given and every text it
gives back is recorded in order: the opening and its genesis, the invitation (`invite`, with the
secret the recording fixes) and what it says (`invitation`), the join and its fingerprint, the
devices that joined before the grant and none after it, the grant, what each device stands at, a
change of bob's made
before his device read the log, kept waiting (and sent again, waiting once) and signed by his first
round, his change that writes his first write over and the field anew, one operation, through
`sce_peer_outbound`, an event that is not signed, another content under a dot he signed (refused),
each request and the relay's answer, the outcomes, bob's
sealed state and his core made again given nothing but the list and the genesis and restored from it
with no key given again, a change of his that comes back held and a new one kept waiting until his
chain is read back, which the round that reads it signs and which then comes back held, a resync;
bob's core started again from the same older state, whose copy now keeps a journal and offers two
changes (`made`) that wait, the first stamped before the change of his the state lacks: the round
that reads his chain back says his stamp rose (`raised`) and withdraws both, earliest first
(`withdrawn`); a round of alice's whose transport fails; alice's copy keeping a journal: a change she
offers signed in the call and held back from the relay as a record (`records`) until her host
recorded it (`recorded`), a change offered in bob's name refused, her core started again from the
state saved before the change and given back the record (`restore_records`), a change offered before
her chain is read back waiting, the history her rebuilt machine is given (`history`, the waiting
change provisional), her own stamp (`own_stamp`) before and after the round that reads her chain
back, signs what waited as a record and says the stamp rose; bob marking what he was told (`mark`)
and the stable cut it moves (`stable_cut`); and bob taken off: a change his core signs before it read the
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
what it seals; the relay's challenges are made under a key the recording fixes; the invitation is
made with `sce_peer_invite_under` under a secret the recording fixes, at an instant it fixes; nothing
reads a clock or draws a random number), so a run now records the same file, and
`the_recorded_scenario_is_what_a_fresh_run_records` fails, naming the first step that differs, when the
recording and a fresh run do not agree. It is recorded again with `SCE_PEER_SCENARIO=write cargo test
-p sce-rust-peer-capi --test the_recorded_scenario_is_what_a_fresh_run_records --
the_recorded_scenario_is_what`. The list's MLS group is not in it: MLS draws randomness, so a run would
not record the same texts twice, and the group's own C ABI has its own recorded scenario. That is why
the recorded list is one no group keys, whose invited device joins with its key alone and is admitted
by a grant (section 12). The administrator's acts on a group are held through this ABI without a
recording, by their properties (`the_administrators_acts_on_a_group_go_through_the_abi`): a device
admitted from its key package joins from the Welcome and holds the key of the epoch the
administrator's device holds, and once taken off holds no key of the epoch after; an invited device's
join on a list keyed by its group, the fingerprint the same on both sides, and the refusal of a key
package a relay made up are held by `a_device_is_invited_by_the_text_of_a_qr_code_and_joins`.

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
- **The time.** Nothing the core does depends on the time but the expiry of an invitation, which
  enters as an argument of the call that makes it (`now_ms`, section 12); when more comes to, it
  enters as an argument of the calls that need it (ADR 0079 section 3), and the ABI's version moves.

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
  `_UNAUTHORIZED` (2, the token was refused), `_DEADLINE` (3), `_CANCELLED` (4) or `_RATE_LIMITED`
  (5, the relay's endpoint refused the connection for the rate before it carried the request, an
  HTTP 429), with the reason given by `sce_peer_reply` when there is one; for `_RATE_LIMITED` the text
  given is the wait the endpoint asked for, in milliseconds, as decimal digits (a host reading
  `Retry-After: 2` gives `2000`), which the run waits out as it waits out the relay's `rate_limited`,
  and a text that is no such number makes the exchange the link's failure. It may take at most
  `timeout_ms` (`UINT64_MAX` when the run has no deadline). `release`, which may be null, is called
  once with `context` when the link is freed.
- `sce_peer_link_set_pause(link, pause)` gives a link whose transport the host carries the wait its
  runs take on the host's clock: `int32_t (*)(void *context, uint64_t wait_ms)`, called with the
  transport's context on the running thread between two exchanges, which waits `wait_ms` of the
  host's time and returns `SCE_PEER_PAUSE_WAITED` (0), or returns `SCE_PEER_PAUSE_WOKEN` (1) earlier,
  at the latest once the host raises the run's cancellation. Null is the system's clock, which a host's
  transport has until it is given a pause: the run's thread then sleeps the relay's wait, woken by the
  cancellation. That is what a host whose time is the system's wants, and what a host whose time is
  its own does not: a simulation whose relay counts the host's time would see the run sleep while its
  relay's allowance never refills, and the request asked again at the same instant of its own clock
  refused again; it gives a pause that moves its clock (the world's step server does, as `peer_pause`
  of `sce_rust_world::external`, `backends/kotlin/world`). A pause that returns early
  without the cancellation costs one more request, which the relay refuses with the wait that is left.
  `SCE_PEER_ERR_BAD_ARGUMENT` for a link of the library's own transport, whose time is the system's.
  Not while a run uses the link.
- `sce_peer_run(peer, link, within_ms, cancel)` runs the exchange begun on the core
  (`sce_peer_start`, a publication) over the link **on the calling thread, blocking**, until it ends,
  `within_ms` milliseconds pass (`UINT64_MAX`: no deadline) or `cancel` (which may be null) is
  raised. It returns `SCE_PEER_OK` when the exchange ended, whatever it came to (and at once when
  none runs), and `SCE_PEER_ERR_CANCELLED` or `SCE_PEER_ERR_DEADLINE` when it stopped; either way the
  outcome is then taken with `sce_peer_take_outcome`. A run that stopped, or whose transport failed,
  leaves an outcome of a failure of the link (`"the exchange was cancelled"`, `"the deadline passed
  before the relay answered"`, `"the network changed"`, or the transport's words), and what the
  exchange took in is handed over by the next round, as section 5 says of any exchange that does not
  end. **A refusal for the rate is waited out** (version 6): when the relay answers a request
  `rate_limited`, or the endpoint refuses the connection for the rate (the WebSocket link's `429`, a
  host's `SCE_PEER_TRANSPORT_RATE_LIMITED`), and its wait ends before the run's deadline (or there is
  none) and is not `u64::MAX` (the relay's word for never), the run waits it out, on the host's clock
  when the link was given a pause and on the system's otherwise, and puts the same request again, as
  many times as the relay asks: the relay refuses for the rate before it does anything of a request
  (`docs/SCE_SHARED_LIST_RELAY.md` section 4), so the request asked again is a request asked for the
  first time. A wait that would end at the deadline or after it ends the exchange with the refusal,
  whose outcome holds `code: "rate_limited"` and `retry_after_ms`; a cancellation raised during a wait
  stops the run as during an exchange (`SCE_PEER_ERR_CANCELLED`). A host that gives no deadline
  gives the relay the word on how long a run lasts; its cancellation is what ends it.
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

## 12. An invitation and the join (ADR 0080 section 4)

A device that is not in the list's log yet is admitted with no operator and no token: an
administrator's device invites it, it leaves its key (and its key package) with the relay under the
invitation, and the administrator's device admits the key the proof names after the two people
compared a fingerprint.

1. **The invitation.** `sce_peer_invite(peer, relay, audience, node, now_ms, valid_ms, out)`, on a
   device that holds an administrator's key the log names, for a node the log never granted
   (`SCE_PEER_ERR_GRANTED` otherwise), from `now_ms` by the host's clock for `valid_ms`, at most a
   day (86 400 000 ms; `SCE_PEER_ERR_INVITATION` otherwise). The core draws a secret `s` of 32 bytes,
   makes an Ed25519 key pair whose seed is HKDF-SHA256 of `s` with no salt and the info
   `sce-relay/invitation-key/1`, and signs with the administrator's key the parts
   `sce-relay/invitation/1`, the audience, the list, the genesis's hash in the URL alphabet, the node
   and the expiry in decimal, and the public key `P_s` in the URL alphabet, each in the trust layer's
   framing (its length as eight bytes big-endian, then its UTF-8). It keeps the node, `P_s` and the
   expiry in its sealed state; the secret leaves it in `out` alone. An invitation is not withdrawn
   before it expires (the owner's answer to ADR 0080): its day is its bound.
2. **The text for the QR code.** `SCEI1.` (the form and its version) and, in the URL alphabet with
   no padding, each part its length as two bytes big-endian and then its bytes, in this order: the
   relay's address (UTF-8, the `wss://host:port` a device dials), the audience (UTF-8), the list
   (UTF-8), the genesis's hash (32 bytes), the node and the expiry in milliseconds (8 bytes
   big-endian each), the administrator's public key (32 bytes), the signature (64 bytes) and `s` (32
   bytes). `P_s` is not carried: the reader makes it from `s`. About 350 characters, which a QR code
   holds in byte mode. A host gives the text, as it scanned it, to `sce_peer_invitation_read` for
   what it says (`relay`, `audience`, `list`, `node` and `expires_at_ms` as text, `genesis` in the URL
   alphabet; never `s`), and to `sce_peer_invited_by` for the core of the invited device; it shows
   the text and does not keep it, since whoever holds it may join once.
3. **The join.** On that core, given the person of the invitation's node (`sce_peer_set_identity`)
   and, on a list keyed by its group, a device of the group (`sce_peer_with_group`),
   `sce_peer_join(peer, invitation, len, fingerprint)` begins an exchange run as a round is, over a
   link that presents no token: a `challenge` for the invitation's list and node, and
   `{"op":"join","invitation":{..},"nonce","expires_at_ms","key","key_package"?,"proof"}`, the proof
   a signature with the key `s` makes of the parts `sce-relay/join/1`, the audience the invitation
   carries, the list, the node, the nonce in the URL alphabet, its expiry, the identity key in the
   URL alphabet and the SHA-256 of the key package in the URL alphabet (of no bytes when the core
   holds no group: a list no group keys leaves no key package, and its device is admitted by a
   grant). A relay that names itself by another audience than the invitation's is given nothing.
   The relay checks the invitation's administrator against the log it holds, the signature, that the
   invitation is for its audience and the list's genesis, the expiry, that the node was never granted
   and has no key package kept under an invitation (its record of the invitation's use), and the
   proof under `P_s`; it keeps the key package with the invitation and the proof
   (`docs/SCE_SHARED_LIST_RELAY.md` section 2) and answers `accepted`. The outcome is `"ended"` and
   tells the machine nothing; a refusal is the relay's (`unauthorized`: the signature, the expiry,
   the audience, the proof; `forbidden`: a node granted, an invitation used; `too_large`: as many
   joins waiting as the list keeps). No token is issued: once admitted, the device logs in (section
   5) to read its log, and its Welcome.
4. **What the administrator's device admits.** A round reads the key package record; the core keeps
   it when its proof holds under a `P_s` it made for that node, and a record under an invitation it
   made whose proof does not hold is among the round's refusals (a key package the relay made up: the
   relay knows `P_s` and not `s`). `sce_peer_joined(peer, node, out)` gives the devices that joined,
   each `{"key": .., "fingerprint": ..}`; two are a relay that took two joins of one invitation, or
   a person who photographed the code and raced the device, and the fingerprint tells which is the
   device in front of the administrator. `sce_peer_add_member(peer, node, role, key)` then takes only
   the key package of the join that proved `key`, and `sce_peer_grant` only that key
   (`SCE_PEER_ERR_NOT_JOINED` for any other); once the node is admitted the core forgets its
   invitations. A node the core did not invite is admitted as before, by the key its host gives.
5. **The fingerprint.** SHA-256 of the parts `sce-relay/fingerprint/1`, the list, the node in
   decimal, `P_s` and the identity key (both in the URL alphabet), in the trust layer's framing; of
   its first twenty bytes, each four as a big-endian number modulo 10 000, written as four decimal
   digits, the five groups joined by a space. Twenty digits, about 66 bits: the invited device shows
   them from `sce_peer_join`, the administrator's device from `sce_peer_joined`, and every lane
   shows the same digits since each reads them from the core. For the list `home`, node 2, `P_s` of
   32 bytes each `0x11` and the identity key of 32 bytes each `0x22`, the digits are
   `2074 5706 5351 1674 5217` (`a_fingerprint_is_the_digits_the_documentation_gives`).

## 13. A list created with no token, and its lapse (ADR 0080 section 3)

A relay whose creation policy is `open` takes the first entries of a list from a device on a
connection with no token, for some work, under a name that certifies the list's genesis; a list so
created lapses when no login touches it for the relay's days, and its devices create it again. The
core does both through the login, so a lane adds one call.

1. **The name.** `sce_peer_certified_name(administrator, salt, salt_len, out)` gives
   `n1.<salt>.<digest>`: the salt in the URL alphabet, and the first 16 bytes, in the URL alphabet,
   of SHA-256 of the parts `sce-relay/list-name/1`, the salt in the URL alphabet and the public key
   the 32-byte `administrator` seed makes in the URL alphabet, in the trust layer's framing. The salt
   is the host's (1 to 32 bytes; one administrator's two lists take two), or 16 bytes the library
   draws when `salt_len` is 0. The host makes the core with this name (`sce_peer_open`), gives it the
   same seed (`sce_peer_set_administrator`) and its person, and opens the list (`sce_peer_open_list`)
   as for any list. A relay takes no creation of the name with a genesis whose first administrator
   key is another, so no one takes a name another device made, before the list lapses or after.
2. **The creation.** `sce_peer_login(peer, audience)` asks a challenge as it does; a relay that does
   not hold the list and takes a creation says so (`"held": false`) and names the work (`"work":
   bits`, 20 by default). The core then sends, in place of the login,
   `{"op":"create","list","node","nonce","expires_at_ms","work","entries","signature"}`: the login's
   members and signature, the entries of its log from the genesis to its person's latest grant, and
   the first counter, from 0, such that SHA-256 of the parts `sce-relay/work/1`, the nonce in the URL
   alphabet, the genesis's hash in the URL alphabet and the counter in decimal begins with that many
   zero bits. The core does no more than 24 bits whatever a relay asks. The relay checks the
   challenge, the work, the name, its bounds and the entries and signature as for a login, keeps the
   list and issues the token: the outcome is a login's (`"token"`). At a relay that takes no
   creation, the login of a list it does not hold is refused `behind`, as before.
3. **What the next round gives back.** The relay holds the log as far as the grant, and nothing
   more, and its `heads` say since when its history of the list runs (`"since"`); a device whose
   courier last read another `since` reads the relay from its start, so every device's next round,
   the one that created the list again and the others, gives the relay what it lacks of what the
   device holds: the rest of the log, every device's operations it holds, its checkpoints and the
   proof of a conflict. The courier's state holds `since` once it has read one, in the sealed state
   with the rest; a list created with a token has none, and its state's text is the one it was.
4. **What is not given back.** The group's commits and Welcomes, and a key package left under an
   invitation, are the relay's alone once they were read: a device whose group had not read a commit
   before the list lapsed is admitted to the group again, and a device that had joined under an
   invitation not yet acted on joins again under a new one.

The recorded scenario keeps creation closed (alice's device gives the list's first entries with a
token of the authority); the creation, its lapse and its creation again through the ABI are
`a_list_created_with_no_token_lapses_and_a_device_that_holds_it_creates_it_again`.

## 14. What the library logs, and the sink a host installs

Each decision of the library is a record (version 6): of the relay's WebSocket link (a dial, with the
endpoint and whether a token is presented; a connection made; an upgrade refused, with its status and
the relay's words, and the wait of a `429`; a connection given up and why), of the driver (each wait
for the rate, with the request's kind, how long and how many refusals in a row; a refusal given over
because its wait does not fit, with the time left; a failure of the link; a run stopped by its
cancellation or deadline; each request answered, at debug), of a host's pause (at debug), and of the
core (an exchange ended, in counts: the events, what the relay was given, the refusals, whether it
stalled or the machine is to be rebuilt, what the relay is still owed, and whether a token came; an
exchange failed, with the link's words or the relay's code, reason and wait, and an unexpected answer
by its kind alone; what the relay is owed given or kept, by its kind). A relay in the same process
logs its own decisions the same way (`docs/SCE_RELAY_DEPLOY.md` section 2). A record names counts,
codes, node numbers, durations, an endpoint, the relay's reasons and a list by a digest of its name
(`list=1a2b3c4d`); never a token, a key, a signature, a nonce, an invitation's text or anything a list
holds.

`sce_peer_set_log_sink(max_level, sink, user)` installs the sink, process-wide: `void (*)(int32_t
level, const char *target, const char *message, void *user)`, given each record at `max_level` and
the levels before it (`SCE_PEER_LOG_ERROR` 1, `_WARN` 2, `_INFO` 3, `_DEBUG` 4, `_TRACE` 5), the
module that decided it (`sce_rust_relay::driver`, `sce_rust_peer`, ...), the record as one line of
`key=value` words (`event=rate_limited_wait op=submit wait_ms=20 limited=1 deadline=true`, a value in
quotes when it holds a space, a quote, an `=` or a control character, which is escaped), both
NUL-terminated UTF-8 valid for the call, and `user`. The sink may be called from any thread that
calls the library and from several at once, so it is thread-safe, and it calls no function of the
library. It is replaceable: a later call replaces it, and a null `sink` or `SCE_PEER_LOG_OFF` (0)
removes it; once the call returns, the sink it replaced is called no more and its `user` may be
freed. The library logs nothing, and formats nothing, until a sink is installed. A level that is none
of these is refused (`SCE_PEER_ERR_BAD_ARGUMENT`); and `SCE_PEER_ERR_SINK_TAKEN` says another sink of
the facade was installed in the process first, which a host that links only this library never meets.
An Android host forwards each record to `android.util.Log`, an iOS host to `os_log`, at the level
given.

A record of another crate the library links (the WebSocket, TLS and MLS libraries, the runtime) is
given only at `SCE_PEER_LOG_WARN` and `SCE_PEER_LOG_ERROR`, whatever `max_level` keeps beyond them:
their `debug` and `trace` are bound by none of the rules above, and on a device they were measured to
hold a WebSocket frame's payload, the context of an MLS key derivation (which holds the list's name)
and the paths of the machine that built the library. The records of this library's own modules
(`sce_*`) are given at `max_level`.
