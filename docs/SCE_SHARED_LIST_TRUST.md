# Trust in a shared list

A shared list (the to-do fixture `todo_peer`) has copies on its members' devices and a hub that
relays between them. This document says what each party can be trusted with, and what the
host of each copy does to hold to it. The code is `backends/rust/trust` (`sce-rust-trust`) and
the two seams it sits behind in Mesh, `Admission` and `Sealer` (`backends/rust/mesh`,
`Endpoint::set_admission` / `set_sealer`), with `Endpoint::forbid_events_from` for what a link
may deliver at all. Where members that are not online together leave what they sign for each other
is the relay (`docs/SCE_SHARED_LIST_RELAY.md`, `backends/rust/relay`).

## 1. Where the rule is kept

A machine of the statically typed data model reads the payload of an event and no other
member of `_event` (SCE Accepted Subset, §2.15), so it cannot tell who sent an event, and it
cannot check a signature. The rule is therefore kept by the **host** of each copy, which is
the one party a member can trust: their own device. The machines and their documents do not
know of it; a signature, the place of an operation in its device's chain and sealed content are
written into an event as it leaves a host and taken out as it arrives at the next, and a machine
sees none of them.

Two modes follow from who runs the hub:

| Hub is | What the hosts of the members do |
|---|---|
| Trusted | Roles only: the hub's machine drops a write whose author is not a writer on its roster (`todo.role`), and a person's copy refuses its own member's edit when the member is a reader. |
| Not trusted | The hosts of the members hold the membership log and check everything: signatures, the log's entries, settled instants and, optionally, keep the content from the hub. |

The two are one mechanism with different settings, and **which one a list is in is a fact of
its membership log** (`Profile::Untrusted` or `Profile::TrustedHub`, set by the first entry and
changed by an entry that needs the administrators' quorum), so that every host knows it and a
list can move from one to the other by an entry every host sees.

**Who is on a link** is not decided by a name. The name a client writes in the upgrade path is
its own word, so a hub's host that only checks a name table lets in anyone who knows a machine's
name. A host that authenticates its links uses `wss::accept_authenticated`, which gives its
callback the name and the bearer token the client sent (`wss::connect_authenticated`), and runs
the link only when the host says yes: the token is the host's, issued after its own login. What
a link may deliver once it is run is `Endpoint::forbid_events_from`, and what the members' hosts
hold to whatever the link is, is this document.

## 2. The membership log

A role is 0 (not a member), 1 (may read), 2 (may also write) or 3 (may also do what an owner
does). Who the members are, what each may do, who the administrators are, which epoch the
content is sealed under and whether the hub is trusted is one **log** (`sce-rust-trust`,
`membership`): an append-only chain of entries, each naming the hash of the one before it and
signed by administrators. A person is invited with the hash of the first entry (`Trust::invited`),
which is all they have to be told in a channel they trust; a host takes that genesis and no other,
and checks every entry from there, so nothing a hub says about who the members are is believed on
its word.

| Entry | Does | Signatures it needs |
|---|---|---|
| Genesis | names the administrators, the quorum and the profile | one of the administrators it names |
| Grant | from this entry, the device of a node is a member with a role and signs with a key (role 0 takes it off); one that takes writing away or changes the key carries `final_seq`, where the device's chain ends | one administrator |
| LinkedGrant | a new device of a person is a member, vouched for by one of their devices that is a member (both signed) | one administrator |
| Amend | the last operation accepted of a device that was taken off is raised, on other writers' checkpoints made before the removal | one administrator |
| Epoch | the content key of the list changes | one administrator |
| MlsEpoch | the same, and the key is an MLS group's: names the commit by hash | one administrator |
| Shred | the keys of an earlier epoch are destroyed | the quorum |
| Administrators | add and remove administrators, set the quorum | the quorum |
| Profile | the hub becomes trusted or not | the quorum |

A change to who the administrators are needs the quorum so that one stolen administrator key
cannot make another or take the list over. An entry is taken in only when it is the next position
and names the head (`LogError::Gap` otherwise, so entries that arrive out of order wait), and an
entry already held is taken in once. **Two different entries at one position are a fork**: the
host that is shown both has proof that the chain was signed two ways, says where, and takes in no
write, note or settled instant (and sends none) until an administrator resolves it.

The grants of one node make its history, and **a removal is a position, not a time**. An
operation names the entry of the log it was made under, and is judged by the latest grant at or
before that entry. A grant that takes writing away from a device that could write (a role below
2, or another key) must carry `final_seq`, the number of the last operation of that device the
administrators accept, and the log refuses the entry without it. Every host then refuses an
operation of the device numbered past it, whatever entry it names, so a device cannot be removed
by a time it alone can claim, and cannot go back to an entry before its removal to write on. So:

- A device taken off keeps what it made up to `final_seq`, which reaches any host in any order;
  what it made past it is refused by every host. The cost is stated and accepted: operations the
  device made and had not yet published when it was removed are lost.
- A replaced key (a lost device) begins a new chain at the number after the end the grant set:
  the old key's operations up to it stay, and the new key's are numbered past it and name nothing
  before them.
- The administrator reads `final_seq` from what the hosts have seen of the device's chain
  (`Trust::last_seq`).
- A host that relays entries cannot make one: it does not hold an administrator's key.

**What a host held before it knew is dropped.** An entry that takes a device off at a number reaches
hosts in any order relative to the device's operations, so a host that admitted operation 5 of a device
before it was given the entry that ends the chain at 3 would otherwise hold what a host that had the
entry first refuses, and two hosts given the same objects would hold different lists for good. When
such an entry arrives (a grant, a linked grant or an amendment) the host drops every operation it holds
of that device past the number the log now accepts (`Trust::take_evicted` says which), and the same
holds when a device is found to have signed two operations at one number: nothing of it from that number
is held, and the device signs nothing more on its own host. A machine that was told those operations
holds their effects and cannot take them back, so the host builds the machine again from what it holds
and tells it the list (`Courier::resync`; `Round::evicted` says that it must). This was found by checking
the rule against a model (`docs/formal`) and a simulation of every order of delivery; both fail
without it.

**An amendment** (`Body::Amend`) takes back part of that cost when the loss was not real: other
writers' checkpoints, made under an entry before the removal, may show that the device's chain was
whole to a later number than the administrator's host had seen. The entry raises the last accepted
to that number and the log checks the evidence (a writer other than the device made it, before the
removal, and it names the device's chain to at least the number); one checkpoint that holds is enough
and the administrator who signs answers for choosing it. It raises and never lowers, and only for a
device that was taken off and has no later grant. A host that refused those operations before reads
the relay again (a courier does it when an amendment arrives), and the operations are taken in.

### Devices

A person has several devices, each with its own key and its own node (a node is a device's). A new
device is linked by a device the person has already, which signs for it and which the new key signs
with too (`Body::LinkedGrant`, over `link_bytes`: the list, the node that vouches, the node and the
key): the log then says which devices are one person's (`Trust::person_of`, `devices_of`), an
administrator who is shown a device by a relay holds proof from a member and not the relay's word,
and a linked device has no more than its parent's role and is a node the log has not held before. A
removal is for a device: **taking a person off is a removal for each of their devices**, which the
log does not do by itself (`Member::remove_person` in `sce-rust-mls` does it), and a device that
is lost is taken off while the person's others go on. A new device holds no key of the epochs before
it joined; a device of the same person that holds them sends them (`Member::history_grants`, then
`receive_grant`), which is the history policy: none unless someone sends it.

A hub's machine is told, by `Trust::hub_role_events`, that anyone who could write at some entry
is a writer: the machine has no log to judge by, and must not refuse what its host let through. A
person's machine is told the role they have now (`Trust::role_events`).

## 3. Operations, and the settled instant

What a device sends is an **operation** of its chain: the event as it travels, the number it has
among the device's operations (`seq`), the id of the one it follows (`prev`), the entry of the
log it was made under (`mh`, and that entry's hash `mhh`), and the device's signature of all of
it (`message::operation_bytes`). Nothing in it is a time the device claims: what decides whether
it holds is the entry it names and the number it has.

A host admits an operation when the entry it names is the one the host holds, its author was a
writer under that entry, the number is not past the end the log set for the device, the signature
is true under the key of that entry, and it stands consistently among the device's operations the
host holds, **whatever order they came in** (the machine's merge is commutative, so the host holds
nothing back to wait for a predecessor):

- two different operations at one number, or one that names a predecessor other than the one
  held, are the device signing two histories. The host says so, takes nothing of the device from
  that number on (`Trust::equivocating`), and goes on with everyone else;
- a write's stamp is its author's clock when it wrote it, and along one chain it does not go
  back: a later write given an earlier stamp is refused;
- an operation that names an entry the host has not reached is refused until it has.

- **A write** (`todo.write`) is an operation of the node that stamped it. A hub relays it as the
  operation the member made (`Wire`), with its number and its signature; it cannot make one.
- **A note that a write was written over** (`todo.supersede`) is an operation of whoever wrote
  over it, numbered in the same chain as the writes and cut with them. The host keeps who wrote
  over what, and when a machine sends the tombstone again as itself (a resync), the host puts
  back the operation of the person who really did.
- **A settled instant** (`todo.floor`) is the word on which every copy forgets the tombstones
  up to an instant. It is accepted only with the members' settlement (below), because a hub that
  could settle an instant its copies have not could bring back, with an old signed write, what
  was deleted. A host that trusts its hub allows it without (`Trust::allow_unsigned`).

### Checkpoints, settlement and withholding

A **checkpoint** is one device's signed statement (`sce-rust-trust`, `checkpoint`): the entry of
the log it speaks under, the instant `before` and the `digest` its copy holds of everything up to
it (the machine's own `frozenHash` of the settle), and the **head** of every device's chain it
holds in full (the highest number `n` for which it holds every operation from 1 to `n`, and the id
of the nth). Only a device that was a writer under the entry it names can make one
(`Trust::checkpoint`), and a host takes one in when the key the log gave that device then signed
it (`Trust::take_checkpoint`). The checkpoints travel by any channel, a relay's included: a relay
cannot make or change one, and one it keeps back only delays the settlement.

**The settlement** of an instant is the checkpoints of every device that was a writer under one
entry of the log, all for that instant and that digest (`Trust::settlement`). The hub's host
sends its `todo.floor` on with it (`cert`), and every other host checks it alone: each checkpoint
holds, they are for this instant and this hash, they speak under one entry the host holds, they
are from exactly the writers under it (none missing, none twice, no reader), and the heads they
name agree with each other and with the operations the host holds. No administrator is asked, and
a hub holds none of the devices' keys. A device that was taken off is not a writer under the
later entry and is not asked; a settlement made under an earlier entry is a settlement of that
entry's writers still. A writer that is lost stops a settlement until an administrator takes it
off, which is the price of every copy agreeing before any forgets.

**Withholding** is what the heads show. A host that holds the chain of a device only to 5 and
takes in a checkpoint that names its head at 7 knows that 6 and 7 exist and were not given to it
(`Trust::divergences`, `Divergence::Missing`); one that holds another operation than the one a
checkpoint names at a number (`Divergence::Differs`) knows that the device signed two histories,
or that the member lies about it, and settles on nothing that carries it. The host asks any other
host that holds the operations (`Trust::operations`: a relay, or another member) and takes them in
as it takes in anything (`Admission::admit`), so the host asked need not be trusted. A host that
is connected to several relays has one `Trust`; what one of them did not pass on another gives.

### Limits

A device's stamps and the time they say are its own claim, and nothing here reads them except to
keep them from going back along one chain. What decides is the log and the number. A key that was
leaked made valid operations until it was taken off, and the ones up to `final_seq` stay valid:
undoing them is an operation of its own.

An operation a device made and never published is not in its chain as far as the hosts know. A
host that holds operations 5 and 7 and never 6 shows it in its own checkpoint (its head is 4),
and a member whose checkpoint names a later head shows it to every host that takes it in; a
member that is never heard from, or a relay that withholds every checkpoint as well as the
operations, is a delay that no host can tell from a quiet list until some checkpoint arrives by
another route.

## 4. Keeping the content from the hub

A host that holds keys seals the number and the text of every write under the key of the latest
**epoch** it holds (XChaCha20-Poly1305). The nonce is made from the write's stamp, so a host
needs no source of randomness and one write is one set of bytes; the stamp, the list and the
epoch are the associated data, so sealed content does not move to another write, list or epoch.
A sealed text is `~` and the epoch and ciphertext in the URL alphabet, 84 characters for any
text a person types (what is sealed is the number, the length, the text and zeros up to 32 bytes,
so the size a relay sees says nothing of whether a write holds a number, a word or a line), which
is why a write's `text` holds 96 (a person still types 32, held by the machine's `typed`).

What the hub holds and relays is the **stamp** of each write (which to-do, which field, when, by
whom) and the tombstones: it orders, merges, forgets and compares digests by them. It learns
that a member changed a field at a time, and not to what. Its own agenda computation is
meaningless over sealed values; the members' copies compute theirs.

A host that holds keys takes no write that is not sealed, and none sealed under an epoch it
holds no key of (it says which; after it is given the key, a `todo.resync` brings the write).

### Keys

Where the key of an epoch comes from is one of two things, and a list has one of them: a key an
administrator draws and gives to the members (this section and the next), or the key of an MLS
group the membership log names (`docs/SCE_SHARED_LIST_MLS.md`, `Body::MlsEpoch`), which gives
forward secrecy and a new key that a removed member cannot derive on every change. Either way a
host is given the key with `Trust::set_epoch_key`, and an operation sealed under an epoch whose key
a host does not hold is not refused: it is `TakeError::NoKey`, which a courier parks and tries again.

`Trust::set_epoch_key` takes a key the host drew from its own randomness; the crate draws none.
A key is given to a member with `Trust::grant_key(epoch, member_key)`: sealed under what the
giver's signing key and the member's agree on (X25519 over the same keys), so it travels over
any channel, the hub's included. `receive_grant` takes it when it is for this list and person,
from a member who could write or an administrator, and opens.

### Taking a member off

A member taken off still holds the keys they were given, and reads what was sealed under them;
no scheme can take that back. What can be done is that nothing written afterwards opens for them.
The administrator's host does it in one call: `Trust::rotate_epoch(administrator, recipients)`
draws a new key from the operating system (never from the old one, never from the host's own
source), takes the next epoch number, and gives back a grant for each recipient, which is
`Trust::current_members()` after the removal's entry was appended. The call also makes the entry
of the log that begins the epoch, appends it to the administrator's own log and returns it
(`Rotation::entry`) for every other host to append: the log, and not a host's keys, says which
epoch the list is in, so a host that holds the entry and not yet the key writes nothing, and a
host does not take a write sealed under an epoch its log has not begun. A hub that holds no key
checks that content is sealed under a begun epoch and hands it on as it is. The members that are left
take the grant (`receive_grant`) and seal under the new epoch from then on; the removed member is
given nothing and finds `no key of epoch N` on every write that follows. A host does not replace
the key it holds for an epoch with another (`GrantError::EpochTaken`), so an administrator's host
must hold the keys of the epochs that exist, or it would begin a number already used. Only an
administrator the host was told to trust begins an epoch. A lost key means lost content: the hub
cannot recover it.

### Destroying an epoch's keys

A list may have the keys of an earlier epoch destroyed (`Body::Shred`, an epoch before the one the
list is in; it needs the quorum, since it cannot be undone). Every host that sees the entry zeroes
what it holds of those keys (`zeroize`) and takes no more (`set_epoch_key` is `false`,
`receive_grant` is `GrantError::Shredded`, a copy of a device's secrets keeps none and puts none back),
and what is sealed under the epoch is refused for good, not parked, since no key will come. A hub that
holds no key carries it still, as ciphertext. What this is: the content is unreadable to every host
that keeps to the list. What it is not: a member that copied a key before keeps what it could read,
and nothing a list does reaches that.

The bytes that are signed and sealed are pinned by known-answer vectors
(`backends/rust/trust/tests/a_byte_format_does_not_drift.rs`): a change to any of them is a
decision that makes every existing signature and sealed text stop verifying, and the vectors are
where it shows.

## 5. What a host keeps across a restart

`Trust::to_text` is one text for the host's store (the key-value feature): the checkpoints it
made or took in, and every operation it holds (the writes it relays and the notes of who wrote
over what, as they travel, with their signatures and numbers). It holds no key.
`restore_from_text` verifies each row against today's log and keeps only what is still true,
so that what a host knew to be withheld is still known after a restart. The log itself is kept as its entries (`Trust::entries`, each
`Entry::to_json`), and a host reads it back by appending them in order, which checks the chain again.
Keys and the person's secret are kept where the host keeps secrets.

### A copy of a device's secrets

What a person can keep for themselves against a lost device is a copy of its secrets under a
passphrase (`sce_rust_trust::backup`): the seed of its key, named extras (its MLS state,
`Device::to_bytes`) and the keys of the epochs it holds (none that the list had destroyed). The key
is made from the passphrase by Argon2id (64 MiB, 3 passes by default, a salt drawn from the
operating system) and the copy is sealed with XChaCha20-Poly1305 under a nonce drawn the same way,
the header authenticated with the rest. A copy that asks for more than 1 GiB, 16 passes or 16 lanes
is refused before it is paid for. A wrong passphrase and a changed copy look the same. The bytes are
pinned by a known-answer vector. A device made from a copy still needs the administrator: a lost key is
taken off and a new device is granted, and the copy is for the person who did not lose their
passphrase and wants the same device back, with its MLS state and its epoch keys.

### An audit of the list

`sce_rust_trust::audit::export` writes the record of a list as one text (the log, every operation
as it travelled, the checkpoints), and `audit::verify` checks it with the hash of the genesis and no
key, by the same code a host uses: each entry signed by the administrators it needs, each operation
made by a device that was a writer under the entry it names, within what the log accepted of the
device, and signed. Content stays sealed. The report says who the administrators and members were
and from which entry (a device taken off with where its chain was cut, an amendment, the person each
device belongs to), the length of each chain and how far it is whole, the epochs destroyed, and what
in the text does not hold. A record that was changed fails where it was changed: a changed entry
stops the log there, a changed or removed operation is listed or leaves a gap the report shows.

## 6. What is not here

- The hub sees stamps: which to-do, which field, when, by whom. Hiding them is another design
  (a hub that does not run the machine).
- A reader receives everything that is replicated: a read-only role stops writes, not reading.
  Content a member may not read goes in another list, or under a key that member is not given.
- The Kotlin Mesh has the same three seams (`forbidEventsFrom`, `EventAdmission`, `Sealer`) and
  no implementation behind them: the signatures, log and sealing of `sce-rust-trust`
  are Rust, so a Kotlin host that must check what arrives implements `EventAdmission` itself
  and has to agree with the byte formats of `message.rs`, `membership.rs` and `secrecy.rs`.
- Login itself (who a person is, and the token a host issues for it) is the application's.
