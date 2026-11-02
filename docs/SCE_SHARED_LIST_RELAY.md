# The relay of a shared list

A shared list whose members do not trust the host between them (`docs/SCE_SHARED_LIST_TRUST.md`)
needs somewhere to leave what a member signed for the others to collect, because devices are not
online together. That is the relay (`backends/rust/relay`, `sce-rust-relay`, ADR 0034 stage 5). It
verifies and keeps what members sign and gives it back; it holds no key, reads nothing it holds, and
decides nothing a member's host would then believe, since a host checks what a relay gives it exactly
as it checks what anyone gives it.

## 1. What it holds and what it is held to

For each list the relay holds three kinds of record, in the order they arrived:

| Record | What it is | What the relay checks before keeping it |
|---|---|---|
| Entry | an entry of the membership log | it is the genesis of a list the relay holds none of, or the next entry, signed by the administrators it needs (`sce-rust-trust`, `membership`) |
| Operation | one device's numbered operation, as it travels | the entry of the log it names is the one the relay holds, its author was a writer under it, its number is not past the end the log set for the device, the signature is true, it stands consistently in the device's chain, and its content is sealed under an epoch the log has begun when the list keeps content from the hub |
| Checkpoint | a member's signed statement of what it holds | it names an entry the relay holds, its device was a writer under it, the key the log gave it signed it |

That is the whole of the relay's judgement, and it needs no key: it is the rule of `Trust` as a
verifier with no person. What it can therefore refuse is spam from outside the list, a reader that
writes, a device that was taken off past its last number, a change to what a member signed, and
plaintext in a list that is sealed. It cannot tell a useful write from a useless one by a member, and
it does not try; it bounds how much any one device asks.

The relay is specific to the events whose rules `sce-rust-trust` knows: the writes and notes of the
standard shared record list, `sce:std/shared/record_list.scxml` (`record_list.write`,
`record_list.supersede`, docs/adr/0042), which is the one document it carries; anything else
submitted as an operation is `invalid`. It is generic over where it keeps records (`Store`) and how
it is reached (a `Transport`, section 9).

## 2. Sessions: login-bound tokens

Who a person is (OIDC, a password, a passkey) is the application's and is not the key of anything.
After a person logs in, the application asks the relay's `Authority` for a token naming one list and
the node of the device that person is using, with an expiry (`Authority::issue`), and the device
presents it. The relay therefore knows a device by the node number the membership log gave it in that
list, and by nothing that names an account. A token is `payload.tag`, the tag HMAC-SHA256 of the
payload under the relay's secret, with an audience so that a token for one relay is no use at another.
Short expiries are the revocation a token has; what a device taken off may still do is decided at each
request by the log, not by the token.

What a session may ask is decided by the log at the relay's head: any session may ask for the log and
give it entries (they verify from the genesis, so a stranger's are refused); a device that is a member
reads (feed, ranges, heads); a device that may write submits operations and checkpoints. Creating a
list is giving a relay the genesis of a name it holds none of, so who may create one is who the
application issues a token for that name to.

## 3. Requests

A request and its response are JSON texts with a version, every 64-bit number written as text. A
transport carries them one for one (section 9): the WebSocket binding (`wss` and `serve`), `DirectLink`
in process, a host's own through the peer core's C ABI, or the deployment transport's adapter.

| Request | Gives |
|---|---|
| `entries from` | the log's entries from a position, and how long the relay's log is |
| `append entry` | an entry, taken when it verifies |
| `submit event data` | an operation, taken when it verifies; `new: false` when the relay held it already |
| `checkpoint` | a checkpoint, as above |
| `feed cursor limit` | the records after the first `cursor`, in arrival order, bounded by the relay |
| `operations node from to` | a range of one device's chain |
| `heads` | the head of each chain the relay holds in full, the log's length and the number of records |

A refusal says why: `unauthorized`, `forbidden`, `rate_limited` (with when to ask again),
`too_large`, `invalid`, `behind` (it names an entry of the log the relay has not been given yet: send
the log first) and `unavailable`. A relay that cannot write a record down takes nothing more until its
operator has looked, rather than hold in memory what it did not keep.

## 4. Limits

Each device of each list has an allowance that refills with time (a token bucket in whole
milli-requests, with the clock passed in by the caller); a write costs more than a read; a request,
a feed and a range are bounded; a list holds at most a set number of records. The defaults suit a
household list and an operator sets their own (`Limits`).

## 5. Storage

A relay's state is the records it was given, in order, and nothing else. A store is an append-only
list of them: `MemoryStore`, and `FileStore`, a file for each list named by the hexadecimal of the
list's name (no name a device sends makes a path), a line for each record, flushed to the disk before
the relay answers. A relay that starts reads the records back **through the checks that took them
in**, so a store that was tampered with does not make the relay hold what members did not sign: the
relay does not start, and says which record failed. A line cut off by a crash was never acknowledged
and is dropped; a whole line that is not a record is corruption and is not read past.

The relay keeps its records in memory and replays them at start; it never forgets one. A settlement
lets the members' machines forget tombstones, and does not let the relay forget operations (that is a
compaction, which is not built).

## 6. What a host does: the courier

A host that has no hub puts `Keep` behind its machine: the host signs what the machine sends and
remembers it (the operation is in its `Trust` from then on), and the machine's send is not delivered
to any peer. A `Courier` holds the host's `Trust` and a `Link` to one relay, and a round does three
things:

1. brings the two logs level: the relay's entries come in and are checked, the entries the relay lacks
   go out;
2. gives the relay what it lacks, by comparing heads and sending the missing ranges and the
   checkpoints the host holds, so that nothing needs a queue that could be lost in a crash. Before
   it gives anything, when the relay's heads name more of the host's own person's chain than the host
   holds, it asks for those operations (`operations`, the device's node) and takes them in, so that a
   device whose state came back from an older store numbers its next operation past every one it
   made (below);
3. reads what the relay holds after its cursor and takes each record in as it would from anyone
   (`Trust::take_operation`). It returns the events the machine is to be told: the roster when the
   log moved, then each operation the host did not hold. A record that names an entry of the log the
   host lacks stops the round after one more try at the log, with the cursor before it.

`Courier::resync` reads from the start and tells the machine every operation, for a machine that lost
its state. `publish_checkpoint(before, digest)` signs what the machine froze for an instant
(`record_list.propose` of the machine, which a hub would send) and gives it to the relay;
`floor_event(before, digest)` is the word `record_list.floor` for the machine once the checkpoints the host
holds make a settlement. No machine of a hub and no administrator takes part.

A host connected to several relays has one `Trust` and a courier for each: a relay that withholds an
operation is seen by the heads the members' checkpoints name (`Trust::divergences`) and routed around
by the next relay's round.

A round is a state machine that does no I/O itself (`RoundMachine`): it says which request it wants
put to the relay, is given the answer, and comes to the `Round` and the errors a `Courier` comes to,
in the same requests and the same order; a `Courier` is that machine driven over a synchronous `Link`
by the one driver every exchange is run by (section 9).
What a courier keeps between rounds (`CourierState`: the cursor, the roster told, the operations parked
for want of a key, the proof of a conflict given) changes only where a step of a round was completed,
and what a round took in and did not hand over, because the link failed, the answer never came or the
host abandoned it, is handed over first by the next round; the state is one text for a host's storage.
`Publication` builds and checks the requests that give the relay a key package, a commit, a Welcome or
a checkpoint (`a_round_driven_by_hand_asks_what_a_courier_asks`).

The peer core (`sce-rust-peer`, ADR 0079) composes a device's `Trust`, the round machine and the
device of the list's MLS group, and speaks to its host in texts. The machine's outbound
`record_list.write` and `record_list.supersede` are signed and remembered (`Peer::outbound`, through
the same `keep` that `Keep` uses); a round is request texts out and response texts in
(`Peer::poll_request`, `Peer::respond`, `Peer::fail`), and its outcome is the events for the machine in
order, the refusals, whether the machine is to be built again, whether it stalled, and what the group's
device did with what was read; the device's state is saved and restored as one text. It reads no clock
and starts no thread, so the C ABI every application lane links is a thin layer over it
(`two_devices_hold_one_list_through_a_relay_with_a_core_each`). That ABI is `sce-rust-peer-capi`
(`docs/SCE_SHARED_LIST_PEER_ABI.md`): the same texts through a hand-written header, and a recorded
scenario of two devices and this relay that C, C++, Python, Go, Kotlin and Swift replay with no relay.

**A host opens and runs a list through the core.** The device that holds an administrator's key
opens the list (`Peer::open_list`: the genesis, the grant of its own person, and the group when it
holds a device of it), grants a role (`Peer::grant`), admits a device to a list keyed by its group
from the key package a round read (`Peer::add_member`) and takes one off (`Peer::remove_member`). Each
act writes the entries of the log and makes the group's change in one call, and makes the commit
before it writes an entry, so that a refused act wrote nothing. The entries reach the relay with the
next round, as every entry the relay lacks does (step 1 above); a commit and a Welcome cannot go before
them, since the relay keeps a commit only once its log names it, so the core keeps them as owed and
gives them after the round that ends (`Publication`, the exchange `Peer::publish` makes), keeps one
the relay did not take for the round after, and saves them with its state. What a device stands at
(its role, the members, whether it waits for its own chain, what the relay is still owed) is
`Peer::standing`. A host that drives a `Courier` and a `Member` itself does what these acts do with
`Member::add_member`, `Member::remove_member` and the courier's `publish_commit` and `publish_welcome`,
in that order (`a_shared_list_keys_its_content_with_mls_through_a_relay`).

**A replay writes nothing twice.** An application that keeps its own log of commands as its original
builds its machine again from it, and the machine sends its writes and notes again, as it made them.
`keep` (and so `Keep` and `Peer::outbound`) signs a write or a note once: one the host holds already
(the same register, stamp and content for a write, the same written-over write for a note) is
`Kept::Held`, the operation with the number and the signature it has, and a courier gives the relay
nothing it holds (`Trust::seal_sent`, `docs/SCE_SHARED_LIST_TRUST.md` section 5). Another content under
a stamp the device signed is refused before anything is sealed or signed. The rebuilt machine is told
the list by a resync, and the other members' machines are told nothing again
(`a_copy_built_again_from_its_own_commands_writes_nothing_twice`).

**A device whose state came back from a store reads its own chain back before it signs.** A saved
state may be older than what the device published since (a backup, a copy of its storage, a save it ran
past before it was stopped), and its next operation would then take a number it used, which every host
given both takes as the device signing two histories and freezes it for. `Peer::restore` makes the trust
wait (`Trust::await_own_chain`): a send that would make a new operation is refused with that reason, one
the device holds is still `Kept::Held`, and the next round reads the device's own operations back
(step 2 above) and tells them to the machine; the wait ends once the trust holds the chain as far as the
relay's heads say (`Trust::confirm_own_chain`), and the device signs at the number after its last. The
rule is the core's, so that no host forgets it, and it uses only `heads` and `operations`, which the
relay served already. A host that drives a `Courier` and restores its `Trust` itself calls
`await_own_chain` after `restore_from_text`. A host that rebuilds its machine after a restore makes a
round, gives the core the machine's sends again, then makes a resync. What the rule rests on is the
relay's word of how far the chain goes: a relay that withholds the device's own operations from its
heads as well as from what it gives back makes the device sign at a used number, and every host given
both holds the conflict as proof, which is the limit ADR 0034 states for a relay that withholds.

## 7. Two profiles, one relay

The relay does not know which profile a list is in and does not need to: the profile is a fact in the
membership log that every host reads. Under `Untrusted` the members seal their content (a write is the
same length whether it holds a number, a word or a line, so the size says nothing) and the relay holds
sealed text; under `TrustedHub` nothing is sealed and a member device that is the hub's holds the keys
of a member. The same relay serves both (`a_shared_list_runs_through_a_relay_with_no_hub_machine`).

## 8. What the relay learns, and what is not built

The relay sees the list's name, each device's node number in that list (a number the log gave it, not
an account), the stamp of each write (which row, which field, when, by whom), each operation's size
(constant for a text a person types) and the membership log. It does not see what was written, or what
was settled and forgotten. It does not record arrival times, and it does not batch.

Not built, and not claimed:

- **Compaction.** The relay keeps every record of a list up to its bound.
- **Hiding stamps from the relay.** That is another design (a relay that does not run the order of the
  merge); today the stamp is in the clear because the machines order and compare by it.
- **TLS.** The binding runs on any stream the host hands it, one its TLS acceptor wrapped included;
  the deployer provides the certificate. The client (`WssLink`) links `rustls` with no cryptographic
  provider, so it dials `ws://` only until one is chosen.
- **Login.** Who a person is, and when to issue them a token, is the application's.
- **Other lists' rules.** The relay verifies the standard record list's events, and no other
  document's: an application builds its list on that part.

## 9. The transport: the contract a binding implements

What carries the protocol's texts between a device and a relay is a transport (ADR 0079 section 5).
Its contract is the trait `Transport` of the crate `sce-rust-relay-transport`
(`backends/rust/relay-transport`), which `sce-rust-relay` re-exports. It is a crate of its own with
no dependency beyond the standard library, so that a binding written in another repository (the
deployment transport's adapter, in that transport's repository) depends on the contract and on
nothing of the relay, the trust layer or the Mesh, and so that the contract changes only when it
does. A transport knows nothing of what the texts mean; the peer core, which does no I/O, does.

| Method | What it is |
|---|---|
| `exchange(request, deadline, cancel)` | carries one request text and returns the relay's response text, or a `TransportError` |
| `take_notices()` | what became of the connection and what the relay pushed since last asked (`Notice::Connected`, `Lost(why)`, `Pushed(text)`), oldest first, never blocking, at most `NOTICES_KEPT` (`Notices` keeps them) |
| `network()` | the `Signal` the host raises when the device's network changed |

The contract, which every binding keeps:

- **One exchange at a time, in order.** `exchange` is called by one thread at a time, and the next
  begins only after the last returned. The response it returns is the response to its request: a
  medium that could deliver a late response to an earlier request gives up what would carry it (the
  connection) rather than give it to a later one.
- **What it may block on.** Connecting (dialling, the medium's handshake, the device's
  authentication), sending, and waiting for the response; never past the deadline (`Deadline`), and
  not past the moment the cancellation is raised by any thread (`Cancelled`): the transport watches
  the `Signal` it is given (`Signal::watch`) for the length of the exchange.
- **Reconnection is the transport's.** It connects when an exchange needs a connection and has none,
  and after a failure, a deadline, a cancellation or a change of the network the next exchange
  connects again. It never sends a request twice: whether to ask again is the core's, which knows
  what the request means (every request of the protocol may be asked again, and the core's next
  round asks what it still needs).
- **A relay's refusal is a response, never an error.** An `Ok` is a text the relay sent, its
  refusals included. `TransportError::Link(why)` says the medium did not carry the request or the
  response (the relay may or may not have had the request, and the core assumes neither);
  `Unauthorized(why)` says the relay's endpoint refused the device's token before it carried the
  request, which the core reads as the relay's refusal of the token (`unauthorized`), the answer it
  gives a token that expires on a connection that stays open, so that a host renews its token on one
  sign; `Deadline` and `Cancelled` say which stop came first.
- **The token is the transport's.** The token of section 2 is carried as the medium carries
  credentials: the WebSocket binding sends it as `Authorization: Bearer` at the upgrade.
- **The network is the host's word.** Any thread raises `network()` at any time; a transport gives
  its connection up and connects again on its next exchange, and an exchange that waits when it is
  raised fails as the link's failure rather than wait on a path that may be gone.
- **A push is a reason to sync, not an input.** The core reads what the relay holds by its cursor; a
  `Notice::Pushed` tells the host to begin a round. Version 1 of the protocol pushes nothing, so the
  WebSocket binding gives no push.
- **Moved, not shared.** A transport is `Send`: it is moved to the thread that runs the rounds, and
  what another thread does to it while an exchange blocks is raise the two signals.

**One driver.** The exchange the core makes, a courier's round (`RoundMachine`) or the peer core's
(`Peer`, which composes a round, the second read the group's device owes and what the relay is owed),
is an `Exchange`: the request it wants put, and the answer or the link's failure. `drive(exchange,
link, deadline, cancel)` is the one loop that carries it: it reads the cancellation and the deadline
before each request and fails a request asked after either, gives every other request to the link
(`Link::call_within`, which a transport honours), and returns how it ended (`Driven::Ended`,
`Cancelled`, `DeadlinePassed`); the outcome waits in the exchange. A `Courier` is `drive` over its
`Link` with no deadline; a Rust host runs `Peer` over a transport with `drive`; a host in another
language runs it through the C ABI (`sce_peer_run`, `docs/SCE_SHARED_LIST_PEER_ABI.md` section 11),
which is `drive` too. Every transport is a `Link` (its texts written and read with `Request::to_json`
and `Response::from_json`); a `Link` a test writes over another, to change what a relay seems to
answer, stays a `Link`. A host that carries the texts itself (its platform's own network stack, a
test) still drives the core by hand (`Peer::poll_request`, `respond`, `fail`), and the core is the
same exchange.

**The bindings in this repository.** `wss::WssLink` (feature `wss`, which `serve` includes) is the
reference: the client of the WebSocket binding at `/sce-relay/1`, with a runtime of its own, blocking
on each exchange, giving `Connected` and `Lost` notices, dialling again after anything that did not
come to a response and when the network's signal moved. `DirectLink` is a relay in the same process,
which never blocks. A host's own transport is reached through the C ABI (`sce_peer_link_host`).

**What the deployment transport's adapter is.** A crate in that transport's repository that depends
on `sce-rust-relay-transport` and implements `Transport` over its medium: the request carried to the
relay's endpoint in its key space and the response brought back to the same exchange (never to
another), the token presented as its medium presents credentials and its refusal given as
`Unauthorized`, the deadline and the cancellation honoured by watching the signal, its session's
events given as `Connected` and `Lost`, and whatever the relay publishes given as `Pushed`. Its relay
side hands each request text and the token to the relay's `Hub` (`authenticate`, `handle`), as
`serve::serve_connection` does. Where it also carries the Mesh, it implements the Mesh's `Transport`
there too: the two traits are not one because the shapes are not, a Mesh binding moving an envelope
one way to a named peer and feeding arrivals and readiness back (`LinkEvent`), a relay transport
asking and being answered; what the two share they say alike (a binding keeps no decision, says what
the medium could not do, and tells its host when a peer is ready or lost).
