# Trust in a shared list

A shared list (the to-do fixture `todo_peer`) has copies on its members' devices and a hub that
relays between them. This document says what each party can be trusted with, and what the
host of each copy does to hold to it. The code is `backends/rust/trust` (`sce-rust-trust`) and
the two seams it sits behind in Mesh, `Admission` and `Sealer` (`backends/rust/mesh`,
`Endpoint::set_admission` / `set_sealer`), with `Endpoint::forbid_events_from` for what a link
may deliver at all.

## 1. Where the rule is kept

A machine of the statically typed data model reads the payload of an event and no other
member of `_event` (SCE Accepted Subset, §2.15), so it cannot tell who sent an event, and it
cannot check a signature. The rule is therefore kept by the **host** of each copy, which is
the one party a member can trust: their own device. The machines and their documents do not
know of it; a signature, the time of a note and sealed content are written into an event as it
leaves a host and taken out as it arrives at the next, and a machine sees none of them.

Two modes follow from who runs the hub:

| Hub is | What the hosts of the members do |
|---|---|
| Trusted | Roles only: the hub's machine drops a write whose author is not a writer on its roster (`todo.role`), and a person's copy refuses its own member's edit when the member is a reader. |
| Not trusted | The hosts of the members hold the administrators' keys and check everything: signatures, certificates, settled instants and, optionally, keep the content from the hub. |

The two are one mechanism with different settings, so a list can start in the first and move to
the second.

**Who is on a link** is not decided by a name. The name a client writes in the upgrade path is
its own word, so a hub's host that only checks a name table lets in anyone who knows a machine's
name. A host that authenticates its links uses `wss::accept_authenticated`, which gives its
callback the name and the bearer token the client sent (`wss::connect_authenticated`), and runs
the link only when the host says yes: the token is the host's, issued after its own login. What
a link may deliver once it is run is `Endpoint::forbid_events_from`, and what the members' hosts
hold to whatever the link is, is this document.

## 2. Members, roles and certificates

A role is 0 (not a member), 1 (may read), 2 (may also write) or 3 (may also do what an owner
does). An administrator holds a key the hosts are told to trust when they are set up, and signs
a **certificate** for each member: the list, the member's node, the role, the key their writes
are signed by, a `serial` and a time `from` (in the clock the machines stamp with).

Certificates of one node make its history. A write or a note made at a time *t* is judged by the
certificate with the highest serial among those whose `from` is at or before *t*. So:

- A member taken off (role 0 from a time) still wrote what they wrote before it; what they sign
  from that time on is refused.
- A replaced key (a lost device) judges only what is made from its time; the old key still
  judges what came before.
- The same set of certificates gives the same answer in whatever order a host received them,
  and a host that relays them cannot make one.

A hub's machine is told, by `Trust::hub_role_events`, that anyone who could write at some time
is a writer: it has no time to judge by, and must not refuse what its host let through. A
person's machine is told the role they have now (`Trust::role_events`).

## 3. Writes, notes and settled instants

- **A write** (`todo.write`) is signed by the node that stamped it. A host admits it when its
  author was a writer at the time of the stamp and the signature is true under that era's key.
  A hub relays a write with the signature it arrived with; it cannot make one.
- **A note that a write was written over** (`todo.supersede`) is signed by whoever wrote over
  it, with the time they made it at, which the host adds (`at`, from `Trust::set_clock`, the
  clock the host gives its machine as an edit's `now`). The host keeps who wrote over what, and
  when a machine sends the tombstone again as itself (a resync), the host puts back the real
  author, the time and the signature.
- **A settled instant** (`todo.floor`) is the word on which every copy forgets the tombstones
  up to an instant. It is accepted only with the signature of an administrator over the instant
  and the hash (`Trust::attest_floor`, `hold_floor_signature`), because a hub that could settle
  an instant its copies have not could bring back, with an old signed write, what was deleted.
  A host that trusts its hub allows it unsigned (`Trust::allow_unsigned`).

### Limits of the time

The time of a write or a note is its author's claim. What a member taken off signs with an
earlier time than the one they were taken off from, after it was made, is accepted until the
instant they were taken off from is settled; after that every copy refuses writes at or before
the settled instant. An administrator who takes a member off therefore settles promptly.

A key that was leaked made valid writes until it was taken off, and they stay valid: undoing
them is a write of its own.

## 4. Keeping the content from the hub

A host that holds keys seals the number and the text of every write under the key of the latest
**epoch** it holds (XChaCha20-Poly1305). The nonce is made from the write's stamp, so a host
needs no source of randomness and one write is one set of bytes; the stamp, the list and the
epoch are the associated data, so sealed content does not move to another write, list or epoch.
A sealed text is `~` and the epoch and ciphertext in the URL alphabet, at most 81 bytes, which
is why a write's `text` holds 96 (a person still types 32, held by the machine's `typed`).

What the hub holds and relays is the **stamp** of each write (which to-do, which field, when, by
whom) and the tombstones: it orders, merges, forgets and compares digests by them. It learns
that a member changed a field at a time, and not to what. Its own agenda computation is
meaningless over sealed values; the members' copies compute theirs.

A host that holds keys takes no write that is not sealed, and none sealed under an epoch it
holds no key of (it says which; after it is given the key, a `todo.resync` brings the write).

### Keys

`Trust::set_epoch_key` takes a key the host drew from its own randomness; the crate draws none.
A key is given to a member with `Trust::grant_key(epoch, member_key)`: sealed under what the
giver's signing key and the member's agree on (X25519 over the same keys), so it travels over
any channel, the hub's included. `receive_grant` takes it when it is for this list and person,
from a member who could write or an administrator, and opens.

### Taking a member off

A member taken off still holds the keys they were given, and reads what was sealed under them.
To keep what is written from then on, an owner makes a new epoch key and gives it to the members
that remain. This is the host's flow and is not automated here. A lost key means lost content:
the hub cannot recover it.

## 5. What a host keeps across a restart

`Trust::signatures_to_text` is one text for the host's store (the key-value feature): the
attested instants, the writes it relays (as they travel, with their signatures) and who wrote
over what. It holds no key. `restore_signatures_from_text` verifies each row against today's
certificates and keeps only what is still true. Keys and the person's secret are kept where the
host keeps secrets.

## 6. What is not here

- The hub sees stamps: which to-do, which field, when, by whom. Hiding them is another design
  (a hub that does not run the machine).
- A reader receives everything that is replicated: a read-only role stops writes, not reading.
  Content a member may not read goes in another list, or under a key that member is not given.
- The Kotlin Mesh does not have the `Admission` and `Sealer` seams yet.
- Login itself (who a person is, and the token a host issues for it) is the application's.
