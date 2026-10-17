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

The relay is specific to the events whose rules `sce-rust-trust` knows (the to-do list's writes and
notes). It is generic over where it keeps records (`Store`) and how it is reached (`Link`).

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

A request and its response are JSON texts with a version, every 64-bit number written as text. The
transport carries them one for one (`serve`, a WebSocket binding, or `DirectLink`, in process).

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
   checkpoints the host holds, so that nothing needs a queue that could be lost in a crash;
3. reads what the relay holds after its cursor and takes each record in as it would from anyone
   (`Trust::take_operation`). It returns the events the machine is to be told: the roster when the
   log moved, then each operation the host did not hold. A record that names an entry of the log the
   host lacks stops the round after one more try at the log, with the cursor before it.

`Courier::resync` reads from the start and tells the machine every operation, for a machine that lost
its state. `publish_checkpoint(before, digest)` signs what the machine froze for an instant
(`todo.propose` of the machine, which a hub would send) and gives it to the relay;
`floor_event(before, digest)` is the word `todo.floor` for the machine once the checkpoints the host
holds make a settlement. No machine of a hub and no administrator takes part.

A host connected to several relays has one `Trust` and a courier for each: a relay that withholds an
operation is seen by the heads the members' checkpoints name (`Trust::divergences`) and routed around
by the next relay's round.

## 7. Two profiles, one relay

The relay does not know which profile a list is in and does not need to: the profile is a fact in the
membership log that every host reads. Under `Untrusted` the members seal their content (a write is the
same length whether it holds a number, a word or a line, so the size says nothing) and the relay holds
sealed text; under `TrustedHub` nothing is sealed and a member device that is the hub's holds the keys
of a member. The same relay serves both (`a_shared_list_runs_through_a_relay_with_no_hub_machine`).

## 8. What the relay learns, and what is not built

The relay sees the list's name, each device's node number in that list (a number the log gave it, not
an account), the stamp of each write (which to-do, which field, when, by whom), each operation's size
(constant for a text a person types) and the membership log. It does not see what was written, or what
was settled and forgotten. It does not record arrival times, and it does not batch.

Not built, and not claimed:

- **Compaction.** The relay keeps every record of a list up to its bound.
- **Hiding stamps from the relay.** That is another design (a relay that does not run the order of the
  merge); today the stamp is in the clear because the machines order and compare by it.
- **TLS.** The binding runs on any stream the host hands it, one its TLS acceptor wrapped included;
  the deployer provides the certificate.
- **Login.** Who a person is, and when to issue them a token, is the application's.
- **Other lists' rules.** The relay verifies the to-do list's events.
