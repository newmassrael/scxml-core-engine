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
application issues a token for that name to, up to the relay's bound on lists (`Limits::max_lists`);
an entry the relay refuses for a name it holds none of leaves nothing held.

**Who issues a token.** Whoever holds the relay's key: the relay, and the application's sign-in
service, which holds a copy of the same key, authenticates the person by its own means, decides which
device of which list the request is for, and issues a token good for a short time; the device asks it
again when the relay answers `unauthorized`. No operator is in the loop for a device, and the relay
learns no account. The key is kept as a three-line text (`Authority::to_key_text`,
`Authority::from_key_text`: `sce-relay-authority 1`, `audience <name>`, `secret <32 bytes in the URL
alphabet>`), which `sce-relay keygen` makes and `sce-relay token issue` issues from
(`docs/SCE_RELAY_DEPLOY.md` section 6). A token names a list by a name that is not empty.

A service in another language writes a token as this crate does. The payload is, for each of the
audience, the list, the node in decimal and the expiry in milliseconds since the Unix epoch in
decimal, the length of its UTF-8 bytes as four bytes big-endian and then the bytes; the tag is
HMAC-SHA256 of the payload under the 32-byte secret; the token is the payload and the tag, each in
base64 with the URL alphabet and no padding, joined by a full stop. For the secret of 32 bytes each
`0x01`, the audience `relay.example`, the list `home`, node 7 and expiry `1800000000000`, the token is
`AAAADXJlbGF5LmV4YW1wbGUAAAAEaG9tZQAAAAE3AAAADTE4MDAwMDAwMDAwMDA.z-lS-hdIHQ8_6aqSt6shwplWq3VOPIcCCbj0bLifhwo`
(`a_token_is_the_bytes_the_documentation_gives`).

**A member logs in, and the relay issues its token (ADR 0080).** A relay that offers login
(`Relay::with_login`, `Shards::with_login`, `[login]` of `sce-relay`, on by default there) takes a
connection that presents no token, or one it issued that expired: an anonymous connection. On it a
device asks `challenge` for its list and node and gets the relay's audience, a nonce (16 bytes the
relay draws and a 16-byte HMAC-SHA256 tag under a key the process holds, over the list, the node, the
expiry and the 16 bytes, so the relay keeps nothing per challenge and a nonce made for one node or
list is no use for another) and the expiry, the relay's clock and 60 s. The device signs, with its
node's key, the parts `sce-relay/login/1`, the audience **it was configured with**, the list, the
node in decimal, the nonce in the URL alphabet and the expiry in decimal, each its length as eight
bytes big-endian and then its UTF-8 (`sce_rust_trust::login::signing_bytes`), and sends `login`. The
relay checks the tag, the expiry, that the nonce was not used (used nonces are kept until they
expire, in memory), appends the entries the login brought (the device brings the entry of its node's
latest grant, which a relay that was not given it takes), finds the latest grant of the node in the
log at its head (a role of at least 1, and a key), checks the signature under that key for its own
audience, and answers an ordinary token of its authority for an hour (`[login] token_seconds`). A
refusal says which check failed: `unauthorized` (the nonce, its expiry, its use, the signature),
`forbidden` (no role), `behind` (the relay lacks the log the login stands on, or holds no log of the
list: a login creates no list), `rate_limited` (section 4). The domain is no other signature's, so a
login signature is no operation, entry, checkpoint or link, and the audience binds it to one relay.
The device's side is `LoginMachine`, an exchange as a round is, run by the one driver.

**A device an administrator invited leaves its key package under the invitation (ADR 0080 section
4).** An administrator's device signs an invitation of a node the log never granted, at this relay's
audience, until an instant, under the public key `P_s` an invitation's secret makes
(`sce_rust_trust::invitation`; the bytes, the text a QR code holds and the fingerprint are
`docs/SCE_SHARED_LIST_PEER_ABI.md` section 12). The invited device asks a challenge for the
invitation's list and node on an anonymous connection and sends `join`: the invitation, its identity
key, its key package (none on a list no group keys) and a proof signed with the key the secret makes
over the domain `sce-relay/join/1`, the audience, the list, the node, the nonce, its expiry, the key
and the SHA-256 of the key package. The doorway checks the challenge as for a login and spends its
nonce; the list's side (`Relay::take_join`) checks that the invitation is for this audience, that the
relay holds the list and the invitation names its genesis, that an administrator the log names now
signed it, that it has not expired by the relay's clock, that the log never granted the node, that no
key package is kept for the node under an invitation (that record is the invitation's use, so an
invitation is used once, and a relay read again from its store holds it used), and that the proof
holds under `P_s`. It keeps the key package record with the invitation and the proof (`"j"` beside
`"n"` and `"d"`; `"d"` is empty for a device that left no key package) and answers `accepted`; no
token is issued. Refusals: `unauthorized` (another relay's invitation, another genesis, no
administrator's signature, an expired invitation, a proof that does not hold), `forbidden` (a node
granted, an invitation used), `behind` (no log of the list), `too_large` (a list holds so many joins
waiting for an administrator, `LoginPolicy::joins_outstanding_per_list`, 32 by default: unexpired and
not granted). A source makes so many joins a minute (`joins_per_minute`, 30). Read back from the store,
a join is checked again as it was taken, but for its expiry and audience, which were judged when it
arrived. The relay cannot make a proof for a key package of its own, since it holds `P_s` and not the
secret; an administrator's device admits only a key a proof under its own `P_s` names. The device's
side is `JoinMachine`.

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
| `challenge list node` | a challenge to log in against (section 2): the relay's audience, a nonce, its expiry, whether the relay holds the list; asked on any connection |
| `login list node nonce expires_at_ms signature entries?` | the token of the device of `node`, when its login holds (section 2); asked on any connection |
| `join invitation nonce expires_at_ms key key_package? proof` | `accepted`, when the invitation and the proof hold, and the key package kept with them (section 2); asked on any connection |

On a connection that presents no token the relay takes, `challenge`, `login` and `join` are answered
and every other request is refused `unauthorized`; `create` (ADR 0080 section 3) is refused `invalid`
with a reason that names it, as by a relay that does not offer it, and a relay that offers no login
refuses `challenge`, `login` and `join` `invalid` the same way.

A refusal says why: `unauthorized`, `forbidden`, `rate_limited` (with when to ask again),
`too_large`, `invalid`, `behind` (it names an entry of the log the relay has not been given yet: send
the log first) and `unavailable`. A relay that cannot write a record down takes nothing more until its
operator has looked, rather than hold in memory what it did not keep.

## 4. Limits

Each device of each list has an allowance that refills with time (a token bucket in whole
milli-requests, with the clock passed in by the caller); a write costs more than a read; a request,
a feed and a range are bounded; a list holds at most a set number of records, and a relay at most a
set number of lists. The defaults suit a household list and an operator sets their own (`Limits`).

A connection with no token is held to its own bounds (ADR 0080 section 5, `LoginPolicy` and the
server's `Bounds`): per source (an IPv4 address, an IPv6 /64, and one source for a transport that
names no address) so many anonymous connections, challenges, logins and joins a minute (30 each by
default), per node of a list so many logins an hour (60), charged only once the signature held, so that a
stranger spends no member's allowance, per list so many joins waiting for an administrator (32); a few
requests (8) and a short idle time (30 s) on one
connection. Each table forgets a key a window after it last spent, and holds a bounded number of
keys. A source behind a proxy is the address its `X-Forwarded-For` names, only on a connection from
a proxy the server is told to trust (`Server::trusting`).

## 5. Storage

A relay's state is the records it was given, in order, and nothing else. A store is an append-only
list of them: `MemoryStore`, and `FileStore`, a file for each list named by the hexadecimal of the
list's name (no name a device sends makes a path), a line for each record, flushed to the disk before
the relay answers. A relay that starts reads the records back **through the checks that took them
in**, so a store that was tampered with does not make the relay hold what members did not sign: the
relay does not start, and says which record failed. A line cut off by a crash was never acknowledged
and is dropped, and is cut from the end of its file before the next record is appended, so that it
never becomes the front of a whole line; a whole line that is not a record, or a record of another
list than its file's, is corruption and is not read past. A list's file is written into its directory
durably before its first record is acknowledged, and a store keeps at most `OPEN_FILES` (64) files
open, so that a relay of many lists does not run out of descriptors (an append that failed for that
reason would stop the relay taking anything).

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
given both takes as the device signing two histories and freezes it for. `Peer::restore_sealed` makes
the trust wait (`Trust::await_own_chain`): a send that would make a new operation is kept with that
reason (below), one the device holds is still `Kept::Held`, and the next round reads the device's own
operations back (step 2 above) and tells them to the machine; the wait ends once the trust holds the
chain as far as the relay's heads say (`Trust::confirm_own_chain`), and the device signs at the number
after its last. The rule is the core's, so that no host forgets it, and it uses only `heads` and
`operations`, which the relay served already. A host that drives a `Courier` and restores its `Trust`
itself calls `await_own_chain` after `restore_from_text`. A host that rebuilds its machine after a
restore gives the core the machine's sends again when it likes, then makes a resync. What the rule
rests on is the relay's word of how far the chain goes: a relay that withholds the device's own
operations from its heads as well as from what it gives back makes the device sign at a used number,
and every host given both holds the conflict as proof, which is the limit ADR 0034 states for a relay
that withholds.

**A send the device may not sign yet is kept, not refused.** A machine sends whenever its person edits,
offline as well, and right after its application starts, which for a device that keeps a sealed state
is a restore. `keep` (and so `Keep` and `Peer::outbound`, through `Trust::keep_sent`) keeps what the
device may not sign yet as `Kept::Waiting`, with why, and every send after it, in the order they were
made: an invited device that has not read the log, or a log that grants its person nothing yet; a list
that keeps its content under an epoch whose key the device has not read; a device waiting for its own
chain. The same send made again waits once, and another content under the stamp of a write that waits
is refused at once. What waits is in the trust's text (`Trust::to_text`), and so in a sealed state. A
round signs it, as far as it may, once the logs are level (step 1 above) and again once the device's
own chain is read back (step 2), before the relay is given what it lacks, so the round that reads what
a send waited for gives it the relay (`Trust::sign_waiting`, `Round::taken`). When the log says the
person never may (a reader, a device taken off, a key the log replaced), every send that waited is
refused with why (`Round::dropped`): the machine took it in as made and no other copy will hold it, so
the round's outcome asks for the machine to be built again (`Outcome::rebuild` in the core). What is
final is refused at once as before: an event that is not signed is not kept, another content under a
signed stamp is refused, and a send of a person the log the device holds does not let write is refused
by the device's own core, since no host would admit it and a number of the device's chain no host
holds would be spent on it. A courier that a host drives over a `Trust` it holds does the same, since
the signing is the round's.

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
- **The operating system's trust store.** A `wss://` client believes the authorities compiled into
  the library or the ones a deployment names (section 9), not the device's store: an authority a user
  or an enterprise installed on the device is not believed, and a root the system distrusts after the
  library was built is distrusted only by a new build of it.
- **Creation of a list by a device with no token of the authority.** A member logs in and the relay
  issues its token, and a device an administrator invited leaves its key package under the
  invitation (section 2, ADR 0080 steps 1 to 3). Open creation and the lapse of a list are not built
  (ADR 0080 step 4): today a list is created with a token of the authority.
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
| `set_token(token)` | the token presented from the next connection on, the empty one for none (a login's, section 2); whether the transport carries one at all (a host's own transport, whose credentials are its host's, says no, the default) |

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
  credentials: the WebSocket binding sends it as `Authorization: Bearer` at the upgrade, and sends no
  header for the empty token, which is the anonymous connection a device logs in on. A transport
  given another token (`set_token`) gives up the connection made with the one before. The core holds
  no token: a host that logs in gives the token the login's outcome holds to its transport.
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

**TLS (decided 2026-11-02).** The relay's WebSocket binding and the Mesh's run TLS as one module
configures it, `sce-rust-mesh`'s `tls` (feature `tls`, which both bindings' `wss` features turn on;
re-exported as `sce_rust_relay::tls`), so that which cryptography runs and which authorities a client
believes are decided in one place.

- **The cryptography is `ring`'s**, which the owner chose over `aws-lc-rs` as the simpler of the two to
  cross-build for Android and iOS. rustls is built with its `ring` feature and no other provider, so
  no `aws-lc-rs` is in the workspace's graph. The provider is passed to rustls in every configuration
  the module makes (`ClientConfig::builder_with_provider`, the same for `ServerConfig`), not installed
  as the process's default: rustls infers a default only when exactly one of its providers is compiled
  in, and an application that links another library asking for `aws-lc-rs` would make the inference
  fail and the first `wss://` exchange panic. So no host installs anything, in Rust or through the C
  ABI.
- **A client believes Mozilla's certificate authorities**, compiled in from `webpki-roots`
  (`WssLink::new`, and the Mesh's client), **or only the authorities a deployment names**, for a relay
  under a private one (`WssLink::believing` with `tls::Roots::Only`, each a DER certificate), **or
  both** (`tls::Roots::PublicAnd`): a deployment that starts under its own authority and later puts its
  relays under a public one, or keeps some under each, ships applications that reach every one of them,
  where `Only` would strand the applications it shipped on the day the certificate changed. A lane
  names the authorities through the C ABI (`sce_peer_link_wss_believing`) as the bytes of a file it
  ships, an Android raw resource or a file of an iOS bundle read whole, in PEM (one or more
  certificates, how an authority is most often handed out) or one certificate in DER; `tls::authorities`
  reads either, so that no lane parses a certificate. A certificate they did not issue fails the
  exchange as the link's failure (`TransportError::Link`, whose reason names the certificate), never a
  panic. TLS 1.3 and 1.2 are offered.
- **The server terminates TLS when it is given a certificate**: `serve::serve_tls` completes each
  connection's handshake with the configuration `tls::server_config` makes from the deployer's
  certificate chain and key before the upgrade, and closes a connection whose handshake fails.
  `serve_connection` still runs on any stream, one the deployer's own TLS acceptor wrapped included.
- **The operating system's store is not read, though it is the long-term choice on a phone**
  (`rustls-platform-verifier`): it would honour the system's root updates and distrust decisions, the
  authorities a user or an enterprise installed, and revocation, with no new build of the library. It
  is not taken now because on Android it needs the host to hand the library its JVM and the
  application's `Context` before the first verification, a JNI call that a lane binding the ABI
  through JNA has no handle for, and the crate's Kotlin component added to the application's Gradle
  build; a verification before that initialisation panics inside the library, the failure this
  decision removes. On iOS it needs no initialisation. Taking it later adds another value of `Roots`
  and, for Android, a call of the ABI the host makes once with those handles.

**The server as it is deployed.** `serve::serve` and `serve_tls` run for as long as their listener
lives and hold a connection to nothing but its messages. A deployed relay runs `serve::Server`: one
bound on the connections open at once over all its listeners (one past it is closed before its
handshake costs anything), a time for each connection's TLS handshake and upgrade, and a time it may
wait for a request; a `Stop` closes the listeners, closes each connection that waits, lets each one
that is answering send its answer, and `Server::drain` waits for them within a grace. `Counts` says how
many connections were taken, refused, never opened and how many requests answered, never who
(`a_deployed_relay_bounds_its_connections_and_stops_when_told`). The program is `sce-relay`
(`backends/rust/relay-server`, `docs/SCE_RELAY_DEPLOY.md`).

`a_relay_is_reached_over_tls` holds an exchange over `wss://` against a relay in the test's process,
with a certificate made for the test and believed by the link, and the refusal of that certificate by
a link that believes only the public authorities; the C ABI's test holds the same refusal through
`sce_peer_link_wss`.

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
