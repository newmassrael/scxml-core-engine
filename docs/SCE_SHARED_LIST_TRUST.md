# Trust in a shared list

A shared list has copies on its members' devices and a hub that relays between them. The list
this document secures is the standard shared record list, `sce:std/shared/record_list.scxml`
(docs/adr/0042), and only it: an application that builds its list on that part is secured without
the trust layer learning the application, and no other document's events are signed, admitted or
relayed. This document says what each party can be trusted with, and what the
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
| Trusted | Roles only: the hub's machine drops a write whose author is not a writer on its roster (`record_list.role`), and a person's copy refuses its own member's edit when the member is a reader. |
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
entry already held is taken in once. **Two different entries at one position, both signed by the
administrators, are a fork**: the host that is shown both has proof that the chain was signed two
ways, says where, and takes in no write, note or settled instant (and sends none) until an
administrator resolves it. An entry nobody signed is no fork, however it differs from the one held:
a hub that rewrote an entry and took its signatures off, or invented one, would else freeze every
host that reads it. So a rival entry is judged as the log would have judged it at its place (the
entries before it are replayed, and the administrators and quorum then in force must have signed
it); one that does not hold there is refused for the reason it does not, and the log is as it was.

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
  (`Trust::last_seq`). `Trust::grant_body` writes a grant with it where the log requires it, and
  `Trust::removal_body` the grant that takes a device off at it; `Trust::administer` drafts an entry
  after the head (the genesis of an empty log), signs it with one administrator's key and appends it.
  These are what the peer core's administrator's acts write (`Peer::open_list`, `Peer::grant`,
  `Peer::remove_member`, `docs/SCE_SHARED_LIST_PEER_ABI.md` section 7), and the group's device too
  (`Member::add_member`, `Member::remove_member`).
- A host that relays entries cannot make one: it does not hold an administrator's key.

**What a host held before it knew is dropped.** An entry that takes a device off at a number reaches
hosts in any order relative to the device's operations, so a host that admitted operation 5 of a device
before it was given the entry that ends the chain at 3 would otherwise hold what a host that had the
entry first refuses, and two hosts given the same objects would hold different lists for good. When
such an entry arrives (a grant, a linked grant or an amendment) the host drops every operation it holds
of that device past the number the log now accepts (`Trust::take_evicted` says which), and the same
holds when a device's operations are found to conflict (below): nothing of it from the first conflict
on is told to the machine, and the device signs nothing more on its own host. A machine that was told
those operations holds their effects and cannot take them back, so the host builds the machine again
from what it holds and tells it the list (`Courier::resync`; `Round::evicted` says that it must, and
`Round::restored` says the same when a conflict went away and what it held back is told again). This
was found by checking the rule against a model (`docs/formal`) and a simulation of every order of
delivery; both fail without it.

**A device's operations are a set, and what the machine is told of it is a function of the set.** A
host holds every operation of a device that holds under the log (its author was a writer under the
entry it names, the administrators accepted its number, its signature is the key's). Some of them
conflict: two operations at one number, one that does not follow the one before it, a write whose
time goes back along its chain, two writes sealed under one stamp. The position of a conflict is the
lower number of the two operations in it, and **the machine is told the device's operations below the
lowest position where any two conflict**. The proof (both operations of every conflict and everything
of the device after the first) is kept: it is held, it is given to the relay and to hosts that were not
given both (`Trust::evidence`, `Trust::signed_operations`), it is stored and restored with the rest
(`Trust::to_text`) and it is in the record an auditor takes in (`audit::export`), where `equivocating`
shows it. Because the position is a function of the set and the set does not depend on the order the
operations came in, two hosts that were given the same operations hold the same of the device and
freeze it at the same number. Before this was the rule, a host that refused what came after the first
conflict it saw could lose the very operation that showed a lower one (`chain.rs`). The conflict goes
when the log puts what made it past a cutoff, and what it held back is told again.

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
`receive_grant`), which is the history policy: none unless someone sends it. The peer core sends
them to every device it admits to a list keyed by its group (`Peer::add_member`, section 4).

A hub's machine is told, by `Trust::hub_role_events`, that anyone who could write at some entry
is a writer: the machine has no log to judge by, and must not refuse what its host let through. A
person's machine is told the role they have now (`Trust::role_events`).

## 3. Operations, and the settled instant

What a device sends is an **operation** of its chain: the event as it travels, the number it has
among the device's operations (`seq`), the id of the one it follows (`prev`), the entry of the
log it was made under (`mh`, and that entry's hash `mhh`), its causal context (`cx`, below), and
the device's signature of all of it (`message::operation_bytes`, domain `sce-trust/operation/2`).
Nothing in it is a time the device claims: what decides whether it holds is the entry it names and
the number it has.

**The causal context** (docs/adr/0085, section 3.1) is, for each other device whose operations the
author's machine had been told when the author signed, that device's node and the highest number
told, lowest node first (`Link::context`; on the wire a list of pairs of decimal texts). The
author's own column is not in it, since the operation's number orders it after the ones before it
in its chain. It is signed, so a relay cannot change the order a host tells its machine
operations in. A context that names its own author, or a device the log had not granted by the
entry the operation names, is refused. The first domain, `sce-trust/operation/1`, signed the same
link without a context; a signature of it verifies under none of the second.

**What a machine is told, and when** is causal delivery (Birman, Schiper and Stephenson, ACM TOCS
9(3), 1991), the layer the list's effect rests on. A host keeps what its machine was told of each
device, the delivered vector (`Trust::delivered`), and tells it an operation only when it is the
device's next (one past the highest told) and every entry of its context was told
(`Trust::deliver`), in an order that is a linear extension of the contexts, whatever order the
operations were taken in. One below the next was told already and is not told again; one past it,
or whose context names what the machine was not told, is held and never skipped, and
`Trust::lacking` names the ranges of devices' chains a held operation waits on, which a round asks
the relay for (`docs/SCE_SHARED_LIST_RELAY.md` section 6). What lies at or past where a device's
chain is frozen, or past the last operation the administrators accepted of a device that may write
no more, is told to no machine, so a context entry that names it is met once the machine was told
the device up to there. An operation sealed under an epoch before the first one the host holds the
key of (a member that joined later is given no history of those) is passed over and never told, so
what its device wrote after it is not held for it. The machine's own column moves when its
person signs, since the machine made what is signed. A machine that lost what it was told is
told everything again (`Trust::forget_delivered`, which a resync calls).

**What an event is signed as** is written from the fields it carries and not from the text of
its data: the domain `sce-trust/event/2`, the list's name, the event's name, then each field as
the text of its value, every part with its length. The events and their fields, in order, are the
standard record list's event schemas, which `message::SIGNED` states and a test of the world crate
(`the_trust_layer_signs_what_the_record_list_declares`) holds to the schemas:

| Event | Signed by | Fields, in order |
|---|---|---|
| `record_list.write` (`record_write`, revision 2) | `nodeId` | `id`, `field`, `value`, `text`, `wallTime`, `counter`, `nodeId`, `parts` |
| `record_list.supersede` (`record_supersede`, revision 2) | `by` | `id`, `field`, `wallTime`, `counter`, `nodeId`, `by`, `parts`, `atWallTime`, `atCounter` |

`parts`, `atWallTime` and `atCounter` are what a change of more than one operation travels with
(docs/adr/0046, section 3), and they are signed: a relay that changed them could hold a change
back from being whole, or make one part look like a change of its own. An event that leaves one
out, as a change of one operation does, is signed as the default its schema gives (1, 0 and 0),
so the two are one set of bytes; every field of the first revision is one every event carries.
What a host keeps a write and a note by is what it is about (its register, its stamp and, for a
write, its content), not the change it travelled in, so a copy that shares an operation alone on
a resync sends on the operation its author signed as a part. The domain was `sce-trust/event/1`
while the layer secured a test fixture's events by other names; a signature does not verify under
two meanings, and there is no path that takes the first.

A host admits an operation when the entry it names is the one the host holds, its author was a
writer under that entry, the number is not past the end the log set for the device, the signature
is true under the key of that entry, and it stands consistently among the device's operations the
host holds, **whatever order they came in** (admitting holds nothing back to wait for a
predecessor; telling the machine does, above):

- two different operations at one number, or one that names a predecessor other than the one
  held, are the device signing two histories. The host says so, holds both as proof, tells its
  machine nothing of the device from that number on (`Trust::equivocating`), and goes on with
  everyone else;
- a write's stamp is its author's clock when it wrote it, and along one chain it does not go
  back: a later write given an earlier stamp is a conflict of the same kind, held as proof by every
  host whatever order the two came in (the device's own host does not sign one);
- **a stamp names one write**: the nonce a write is sealed under is made from the fields of its stamp,
  so two writes of one device under one stamp would be sealed under one nonce, and what a relay knows
  of one gives it the other. The device's own host does not sign a second content under a stamp, and a
  host that is given two holds them as a conflict;
- a write is judged by the **entry it names**: the list keeps content from the hub from the entry that
  begins its first epoch, so a write that names an entry before it is in the clear and holds, whether the
  host was given the entry that began the epoch before it or after, and one that names that entry or a
  later one is sealed under an epoch that entry has begun, or it is refused;
- an operation that names an entry the host has not reached is refused until it has.

What taking an operation in did is `Taken` (`Trust::take_operation`): whose it is, its number,
whether the host held it already, whether the machine is told it, and, for one it did not hold that
conflicts with what it held of its device, `Taken::conflict` (ADR 0083 section 4): the lowest number
of its conflicts, where the device's chain was frozen before and is frozen now (they differ when
this operation froze it), and the operations held that it conflicts with, as they travel. A relay
answers an operation by it (`docs/SCE_SHARED_LIST_RELAY.md` section 3), and a host given those
operations finds the conflict itself.

- **A write** (`record_list.write`) is an operation of the node that stamped it. A hub relays it as
  the operation the member made (`Wire`), with its number and its signature; it cannot make one.
- **A note that a write was written over** (`record_list.supersede`) is an operation of whoever wrote
  over it, numbered in the same chain as the writes and cut with them. The host keeps who wrote
  over what, and when a machine sends the tombstone again as itself (a resync), the host puts
  back the operation of the person who really did.
- **A settled instant** (`record_list.floor`) is the word on which every copy forgets the tombstones
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
sends its `record_list.floor` on with it (`cert`), and every other host checks it alone: each checkpoint
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
keep them from going back along one chain and from being used twice. What decides is the log and the
number. When the administrator replaces a device's key, the old chain ends at the number the entry
gives and the new key's first operation is the number after it, naming nothing before it. A key that was
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

What the hub holds and relays is the **stamp** of each write (which row, which field, when, by
whom) and the tombstones: it orders, merges, forgets and compares digests by them. It learns
that a member changed a field at a time, and not to what. Its own agenda computation is
meaningless over sealed values; the members' copies compute theirs.

A host that holds keys takes no write that is not sealed, and none sealed under an epoch it
holds no key of (it says which; after it is given the key, a `record_list.resync` brings the write).

**When content is sealed is the log's word**: from the entry that begins the list's first epoch,
and a write that names an entry before it is in the clear and holds (section 3). A list is sealed
from its first write only when that entry comes before anyone writes, and once a write is in a
relay's clear no later epoch takes it back. A list keyed by its MLS group names its first epoch when
it is opened (`Peer::open_list`: the genesis, the owner's grant, then the entry that names the
commit that moves the group of one to epoch 1, `docs/SCE_SHARED_LIST_MLS.md` section 2), so the third
entry of its log seals everything after it and its first device's writes are never in the clear. A
host whose device is in the list's group says so (`Trust::keyed_by_a_group`), and then signs no write
of its person's while the log names no epoch: the write waits (`Sent::Waiting`) until an epoch is
begun and its key held, and is refused by `seal_sent`, rather than leave in the clear. A list whose
keys an administrator draws is in the clear until the first `Trust::rotate_epoch`, so an
administrator who wants such a list sealed from its first write rotates before anyone writes; a list
with no group and no drawn key keeps its content in the clear by its log's word.

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
from a member who could write or an administrator, and opens. A relay carries one for a device
(`key_grant`, `docs/SCE_SHARED_LIST_MLS.md` section 4), and the peer core gives a device it admits to a
list keyed by its group the key of every epoch before its own that way (`Peer::add_member`).

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

A list keyed by its MLS group (`docs/SCE_SHARED_LIST_MLS.md`) is taken off by a commit of the group
instead of a drawn key, and the peer core does both halves in one call (`Peer::remove_member`): the
grant that takes the device off, the commit that moves the group to an epoch without it, and the entry
that names the commit. It takes no member off a list whose keys are drawn here, since the grants of
the new key go to the members left by a channel the core does not have, and a removal with no new
epoch would leave what is written after it readable by the device taken off.

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
made or took in, every operation it holds (the writes it relays and the notes of who wrote
over what, as they travel, with their signatures and numbers), and what its machine was told of
each device (the delivered vector), so that a host restored from it tells its machine nothing twice;
a text written before it held the vector restores with none, and the next round tells the machine
every operation held again, which the list takes in once each. It holds no key.
`restore_from_text` verifies each row against today's log and keeps only what is still true,
so that what a host knew to be withheld is still known after a restart. The log itself is kept as its entries (`Trust::entries`, each
`Entry::to_json`), and a host reads it back by appending them in order, which checks the chain again.
Keys and the person's secret are not in that text: `Trust::secrets` gives them (the person's node
and seed, the keys of the epochs held) as the contents of a copy of the device's secrets, which is
the only way the seed leaves the crate, and only sealed. A device's whole state, those secrets with
the text, the log and the rest, is sealed under a data key the host's platform key store wraps
(`backup::seal_state`, below); the peer core keeps a device so (`docs/SCE_SHARED_LIST_PEER_ABI.md`
section 6).

### A machine that sends a write again

A machine sends what it wrote again: a copy built again from its application's own log of commands
sends every write and note it makes, as it made them, and a copy that shares what it holds on a resync
sends alone an operation that was signed as a part of a change. The host signs each once
(`Trust::seal_sent`, which the `Sealer` is, and `Trust::keep_sent`, which the relay's `keep` is): it
keeps a write by its register, its stamp and its content, and a note by the write it is about, so the
same one sent again, before a restart or after it, leaves as the operation the host holds, with its
number, its signature and its sealed text, and is reported as not signed (`Sealed::signed`, and
`Sent::Held`; one that waits to be signed waits once). What an operation travels in (`parts`,
and a note's `by`, `atWallTime` and `atCounter`) is not what it is about, and does not make it another.
A write of the host's person under a stamp the device signed with another value or text is refused
before anything is sealed or signed, with that reason: one stamp names one write, and a second content
under it would be the device signing two things for one write. So a device's chain does not grow by a
replay, and it never signs two things at one number.

### A state that came back from a store

What `restore_from_text` gives back may be older than the device's last operation: a backup, a copy of
the host's storage, a save the device ran past before it was stopped. A host that signed at the next
number it holds would then sign at a number it used, and every host given both would freeze the device.
A host whose state came back calls `Trust::await_own_chain`: it signs nothing new, and still gives what
it holds as it was, until it is given heads (a relay's) that the chain it holds of its person reaches
(`Trust::confirm_own_chain`); `Trust::own_chain_behind` says which of its own operations to read back.
The peer core does this on every restore, and its round reads the chain back from the relay
(`docs/SCE_SHARED_LIST_RELAY.md`, section 6). `restore_from_text` alone does not wait, since a host
with no relay to read from (a hub's, or a test's) has nothing to wait for. A device rebuilt from a copy
of its secrets with no state at all is in the same case, and its host calls `await_own_chain` too.

### A send the person may not sign yet

`Trust::seal_sent` answers at once, which is what a `Sealer` behind a Mesh endpoint needs: what it
cannot sign now it refuses. A host whose sends a courier carries to a relay keeps them instead
(`Trust::keep_sent`, which the relay's `keep` is): a send of the host's person that the person may
not sign yet waits, with every send after it, in the order they were made, and `Trust::sign_waiting`
signs them in that order once the person may (`docs/SCE_SHARED_LIST_RELAY.md`, section 6). What may
not be signed yet: a log not begun, a log that grants the person nothing yet (or grants nothing to the
key the device signs with), a list that keeps its content under an epoch whose key the host does not
hold, and a host waiting for its own chain. What never may, under the log the host holds: a person
whose latest grant does not let them write, a key the log replaced, a frozen chain, a forked log, a
host with no person. A send of the second kind is refused at once, and one that waited is refused when
the log comes to say so (`Waited::dropped`); a send of another content under the stamp of a write that
waits, or that the device signed, is refused at once; the same send made again waits once. What
waits is in `Trust::to_text`, in its order, and comes back with `restore_from_text`; it is judged
when it is signed, as any send is.

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

### A device's state under its platform's key

The same contents are what a device keeps of itself between runs, sealed under a key and not a
passphrase (`backup::seal_state`, `backup::open_state`): envelope encryption, the pattern of a
platform key store whose keys are not exportable. A `DataKey` of 32 bytes, drawn from the operating
system for one state, seals it; the host wraps that key with its platform's key and keeps the wrapped
key beside the state. The cipher is the same XChaCha20-Poly1305. Its key and the key the nonce is
made with are drawn from the data key by HKDF-Expand (RFC 5869, section 2.3: the data key is uniformly
random, so it is the pseudorandom key and needs no extraction) under labels of their own, and the
nonce is HMAC-SHA-256 under the second of what is sealed, cut to 24 bytes: the synthetic nonce of
deterministic authenticated encryption (SIV). The same key and contents give the same bytes, and a key
used twice gives away only that two states were the same. The list, the node and the caller's name
of the form are associated data, so a state opens only for the device it was sealed for; another key,
a changed byte, a cut and another device's state all fail as `Corrupt`, and bytes that are no sealed
state as `Malformed`. The data key, the keys drawn from it and the plain contents are zeroed once
used, and a person's key zeroes its seed when it is dropped.

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

- The hub sees stamps: which row, which field, when, by whom. Hiding them is another design
  (a hub that does not run the machine).
- A reader receives everything that is replicated: a read-only role stops writes, not reading.
  Content a member may not read goes in another list, or under a key that member is not given.
- The Kotlin Mesh has the same three seams (`forbidEventsFrom`, `EventAdmission`, `Sealer`) and
  no implementation behind them: the signatures, log and sealing of `sce-rust-trust`
  are Rust, so a Kotlin host that must check what arrives implements `EventAdmission` itself
  and has to agree with the byte formats of `message.rs`, `membership.rs` and `secrecy.rs`.
- Login itself (who a person is, and the token a host issues for it) is the application's.
