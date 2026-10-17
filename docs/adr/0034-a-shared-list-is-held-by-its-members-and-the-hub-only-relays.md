# ADR 0034 - A shared list is held by its members, and the hub only relays

- Status: Proposed (decisions taken 2026-10-17; Accepted when stage 2 lands)
- Date: 2026-10-17
- Scope: the shared to-do list and every list built like it (`todo_peer`), `sce-rust-trust`,
  the Mesh endpoint seams, a relay service, the host features that carry keys
- Related: `docs/SCE_SHARED_LIST_TRUST.md` (what exists today), `docs/SCE_HOST_FEATURES.md`,
  ADR 0005 (bounding and declaring)

## Context

`docs/SCE_SHARED_LIST_TRUST.md` describes what a host does today so that a hub it does not trust
cannot forge a write or read its content: roles, certificates by time, signed writes and notes,
administrator-attested settled instants, sealed content under epoch keys, an authenticated
link. It is sound for what it covers, and its limits fall into three kinds:

1. **Limits a distributed system cannot remove.** An offline member's write cannot be told from a
   write made after they were taken off if both are signed and carry the same claim of time; a
   key that leaked signed valid writes until it was taken off; what a member decrypted they have
   seen. These can be bounded by rule, not removed.
2. **Limits this design chose, which another design does not have.** The hub runs the list's
   machine, so it sees the stamp of every write (who changed which field when); sealed values make
   its own agenda computation meaningless; the time a revocation takes effect is a time, which is
   the author's claim; a settled instant is attested by an administrator who must be reachable;
   tombstones carry no author, so the host keeps a provenance table beside the machine; a
   read-only member receives everything because every copy replicates everything.
3. **Work not done.** Key rotation on removal, a shared implementation for the other lanes, a
   login, multi-device, recovery.

A product that keeps a family's or a team's schedule for years has to hold up against the second
and third kinds as well. This ADR sets the architecture that does, so that what is built next is
built once.

## Decision

**The hub is a relay and a store of sealed objects. The list is held by its members, and every
rule that decides what the list contains is checked by a member's host from objects the members
signed.** The hub is never asked to compute, order, merge, settle or decide; it is only asked to
keep bytes and hand them out. A hub that is trusted is a hub that happens to behave, and the
protocol does not depend on it.

### 1. Objects

Everything that travels is one of four signed, content-addressed objects. A member's host
verifies each from the objects it already holds.

| Object | Written by | Holds |
|---|---|---|
| Membership entry | an administrator | a hash-chained log of the list's membership (below) |
| Operation | a member's device | one change, sealed, in that device's own hash-chained log |
| Checkpoint | every member's device | a signed statement of what it holds up to a point |
| Welcome | a member's device | the keys a new member needs, sealed to that member |

All are canonical (one byte form per value), carry the list's id and a schema version, and are
named by the hash of their bytes.

### 2. The membership log

The list's membership is an append-only, hash-chained log signed by administrators. Entries:
`Grant(device, role, key, serial)`, `Revoke(device, final_head)`, `Epoch(n, welcomes)`,
`Administrator(add or remove)`. A host verifies the chain from the genesis entry, whose hash is
what a person is given when they are invited, so nothing the hub says about who the members are
is believed without the chain.

- A host detects a **fork** (two entries at one position) by comparing log heads with the members
  it meets, and by the heads the checkpoints carry; a fork is reported and the list is frozen
  for writes until an administrator resolves it.
- Roles stay 0, 1, 2 and 3. Administrators are the entries' signers, themselves added by the log.
- This replaces the certificate-by-time of today. A certificate was a statement about a time;
  an entry is a statement about a position in a chain, which nobody can forge or back-date.

### 3. The operation log

Each device keeps its own log of operations: `{list, device, seq, prev_hash, membership_head,
hlc, kind, sealed_payload, signature}`. The payload (the number, the text, the identity of what
was written over) is sealed; everything else is the relay's to read, and nothing else is
needed to relay.

- **Writes over** are part of the operation: an operation says which operations it observed and
  replaced. A tombstone is therefore derived from signed operations, with its author, and the
  provenance table of today disappears, as does the resync that attributes a tombstone to the
  copy that sent it again.
- A host admits an operation when its device was a writer at the membership entry it names, its
  chain extends the device's last admitted operation (`seq` is next, `prev_hash` matches), its
  HLC does not go back along the chain and is within the allowed drift of the receiving clock,
  and the signature verifies. An operation out of order is held until its predecessors arrive.
- The machine on each device is unchanged in what it computes (the registers, the observed-remove
  sets, the digest). The host turns an admitted operation into the events the machine takes, and
  the machine's own sends into operations. What leaves a machine is no longer an event a relay
  must understand.

### 4. Revocation is a position, not a time

`Revoke(device, final_head)` names the last operation of that device's log the administrators
accept. An operation of that device beyond it is refused by every host, whatever time it claims,
and one at or before it is part of the list. The administrator takes `final_head` from what
the relay and the members' checkpoints have seen of that device's log.

The cost is stated and accepted: operations the removed device wrote offline and had not yet
published are lost. That is the same loss every offline-first group system takes, and it is a
smaller one than leaving a removed member's key valid for a window of time that only they can
measure. A leaked key is revoked by the same entry; what it signed before `final_head` stays,
and is corrected by an operation.

### 5. Settling is the members' proof

A checkpoint is `{list, device, membership_head, before, digest_root, op_heads}` signed by the
device: what it holds up to `before`, and the head of each device's log it has admitted. A set of
checkpoints from every device that is a writer at the membership head, agreeing on `digest_root`,
is a **settle certificate**: any host verifies it alone, and the tombstones up to `before` are
forgotten on it. An administrator is not asked for anything, a relay cannot make one, and a member
who disagrees (stale, or forked) is named by the checkpoints that differ.

Checkpoints also detect a relay that **withholds** operations: a member whose `op_heads` name an
operation a host does not hold has proof that it is missing, and fetches it from another relay or
a peer.

### 6. Keys: MLS, once, in Rust

Content keys come from a group key agreement, not from keys this project invents: **Messaging
Layer Security (RFC 9420)**, with the mandatory cipher suite (X25519, AES-128-GCM, SHA-256,
Ed25519). It gives what the epoch keys of today were built to approximate and cannot: a new key on
every membership change, forward secrecy, post-compromise security, and authenticated adds and
removes.

- The credential of an MLS member is the administrators' membership entry for that device; a
  member whose credential is not in the log is not accepted into a group.
- An epoch's content key for the list is exported from the MLS epoch secret (the MLS exporter),
  and sealing is what `secrecy.rs` does today under that key (XChaCha20-Poly1305, nonce and
  associated data from the operation's position); an operation carries the epoch it is sealed
  under, and a device that holds an epoch can open it.
- Commits are ordered by the relay as MLS expects of its delivery service; because membership
  changes are administrator entries, the ordering is the log's, and a relay that reorders is
  detected by the chain.
- **A removed member's epoch is closed:** the epoch after the removal is one they cannot derive,
  and an operation of a device sealed under an older epoch after the epoch change is outside its
  `final_head` in any case.
- History for a new member is a decision of the administrator that adds them (a Welcome that
  carries the older epochs' keys or does not); both are supported.
- There is **one implementation**, in Rust (the audited MLS library, `openmls`, behind a small
  surface), exposed to the other lanes through a C ABI and generated bindings. A second
  implementation in each language is a second place for the same flaw, and the project's rule
  against duplication applies to cryptography first.

### 7. Who may read what

A list may be divided into **sections** (a family's chores and its finances), each with its own
key derived from the epoch's by a labelled KDF, and the Welcome of a member carries the keys of the
sections they may read. A reader of one section holds ciphertext of the others and no key: the
read-only role stops being "writes are refused" and becomes "this is all you can open". A writer
needs the key of the section they write to.

### 8. What the relay learns, and what it is held to

The relay sees: the list's id, each device's pseudonymous id for that list (not an account),
each operation's size and arrival time, and the membership log. Sizes are padded to a few
classes; devices use a different id in each list; operations may be batched. It does not see
who changed which field, what the values are, or what was settled and forgotten. The relay
verifies a signature and the writer's role at the membership head it holds (it holds the log, which
is public to members) so that it can refuse spam and the removed without reading anything, and
it rate-limits by device. It never decides anything the members' hosts would then trust.

Several relays may serve one list; a host reads from any and writes to the ones it
chooses, and the checkpoints make a lagging relay visible.

### 9. People, devices and login

A person is a set of devices; each device has its own key and its own entry in the log (a node
number is a device's). A new device is linked by an existing one, which signs for it
(cross-signing), and an administrator's entry makes the link a member. **Login** (OIDC or any
other) authenticates a person to the relay, so that it can bind a token to a device; it is not
the key of anything. Recovery is the administrator's: a lost device is revoked at `final_head` and
a new one granted. An optional passphrase-encrypted backup of the device key is the application's.

### 10. Two profiles, one protocol

A list that trusts its hub (a household with a home server) runs the same objects with the hub
holding the keys of a member: it keeps the machine, computes the agenda, and may read. Nothing in
the objects changes; what changes is that one device is the hub's. The profile is a fact in the
membership log, so a host knows which it is in and nobody is surprised by it.

## Alternatives considered

- **Keep what exists and add rotation.** Smallest change; leaves the time-based revocation, the
  hub's view of every stamp, the administrator-attested settle and the provenance table.
  Rejected as the end state, and kept as the first stage (below).
- **A blind hub with the epoch keys of today (no MLS).** Closes the stamp but keeps a key scheme
  this project would have to defend alone. Rejected: group key agreement is a field where
  inventing is the risk.
- **A trusted hub, hardened.** A hub that must not misbehave is a single point whose compromise
  breaks every list. Rejected for lists whose members do not run the hub.
- **Matrix-style (Megolm) or Signal-style (sender keys) group keys.** They give forward secrecy
  but not post-compromise security on removal without re-keying work MLS already specifies, and
  they carry no authenticated membership. Rejected in favour of the standard.
- **Per-field encryption chosen by the machine's document.** Would put key policy in the documents
  and break the rule that a machine reads the payload only. Rejected: keys stay in the host.

## Consequences

- The relay becomes a small, generic, audited service (store, fetch, verify-and-forward) that
  is the same for every kind of list. The to-do machine runs on devices only.
- The sync protocol changes from digest-walking to log exchange (per-device heads, ranges),
  with the digest kept as an integrity check of the machine's state.
- `todo.write` and `todo.supersede` stop being what travels; operations are. The documents are
  unchanged: the host maps between them. The wider `text` of a write (96) is no longer needed for
  sealing and returns to the person's bound; this is a document change and a regeneration of the
  committed trees.
- A removal costs a re-key and a settle; unpublished offline work of the removed device is lost.
- Cryptography is one Rust implementation behind a C ABI, so every lane's tests include the same
  known-answer vectors, and a flaw is fixed once.

## Plan

Each stage ships and is useful alone; a stage's exit is a test that fails without it.

1. **Close what is cheap on the present design.** Automate key rotation on removal (new epoch,
   Welcome-style grants to the members that remain, the removed member's epoch closed); document
   the limits as the ADR states them; test vectors for the byte formats. *Exit:* a removed member
   cannot open anything written after the removal in a world test.
2. **The membership log** with positions instead of times: Grant, Revoke(`final_head`), Epoch,
   Administrator; fork detection; invitation by genesis hash. *Exit:* a world test where a removed
   device's late operations beyond `final_head` are refused by every host and its earlier ones stay.
3. **The operation log and the host's mapping** from machine events to operations (with writes
   over carried in the operation, provenance table removed), hash chains, out-of-order holding,
   monotonic HLC. *Exit:* the world tests of today pass on operations, and a relay that drops,
   reorders or repeats operations does not change what members hold.
4. **Checkpoints and settle certificates.** Signed checkpoints, the settle certificate, withholding
   detection, several relays. *Exit:* a settle with no administrator online, and a withholding relay
   detected by a member.
5. **The relay** as a service: verify-and-store, ranges, rate limits, padding, pseudonymous ids,
   login-bound tokens; the trusted-hub profile. *Exit:* the same lists run under both profiles.
6. **MLS** behind the Rust surface with the C ABI and bindings for the other lanes; the exporter
   as the epoch key; sections. *Exit:* known-answer vectors agree in every lane; a removed member
   is cut off at the next epoch.
7. **Multi-device, recovery, history policy,** and the operational pieces (key backup, audit
   export, crypto-shredding by key destruction).
8. **Assurance** before it is called production: a model of the membership log, revocation and
   settle (TLA+ or equivalent) checked for the safety properties below; property tests and
   fuzzing of every parser; mutation rounds over the verification code; an independent
   cryptographic review.

Safety properties the model and tests state:

- No host admits an operation of a device beyond that device's `final_head`.
- Two hosts that hold the same objects hold the same list, in any order of arrival.
- A relay that holds no key and no administrator key cannot cause a host to hold a value no
  member wrote, or to forget one a settle certificate does not cover.
- A member removed at epoch *n* cannot open an operation sealed under an epoch after *n*.
- A fork of the membership log is detected by any two members that exchange heads.

## Decisions

The owner asked on 2026-10-17 for the design that is longest-lived and correct for production,
cost set aside, and left the choices below to be made. They are made here, each for the reason
given, and each can be revisited by a later ADR that names the reason.

1. **Cipher suite and library:** the MLS mandatory suite (X25519, AES-128-GCM, SHA-256,
   Ed25519) through `openmls`, in Rust, with other lanes through a C ABI and generated bindings.
   The standard suite is the one every audited implementation interoperates on, and a second
   implementation can be checked against it.
2. **The relay's home:** a reference relay in this repository (small, generic, with the
   verification and the limits of stage 5) that anyone may deploy, and no hosted service
   as part of this project. The protocol must not depend on who runs a relay, and a reference
   that is tested against it is what keeps that true.
3. **History for a new member:** none by default; an administrator may add the older epochs'
   keys to a Welcome. A key that is not given cannot leak, and the choice that costs a member
   history is the one that can be corrected later by giving it.
4. **Loss on revocation:** an administrator may issue an `Amend(device, final_head)` entry that
   raises a `final_head` when the removed device's own log proves, with the signatures of
   members' checkpoints that name those operations, that they were seen before the removal. It is a log
   entry, so every host sees it and none can be talked into it.
5. **The trusted-hub profile is shipped,** as a fact in the membership log (section 10). A
   household with one server is the common case, and refusing it would push it to a weaker
   product of someone else's.

## Status of the plan

Stage 1 and the membership log of stage 2 have landed. Stage 2 as first landed removed a member
by a time (`from`), which is not what section 4 says; it was corrected with stage 3, where the
number of an operation that section 4 needs exists. Stages 3 and 4 have landed with the
refinements below. This ADR is Accepted with stage 3, since the log and the chains are what make
the rest binding.

## Revisions made in building it

- **Removal is a position, as section 4 says.** A grant carries no time. It carries `final_seq`
  when it takes writing away or changes the key; an operation numbered past it is refused whatever
  entry it names. A replaced key begins a new chain at the number after the end, naming nothing
  before it. (Section 2's `Revoke` is the grant with role 0 and a `final_seq`.)
- **No operation is held back for a predecessor** (section 3 said out-of-order operations are
  held). The machine's merge is commutative, so an operation that holds by itself (entry, role,
  cutoff, signature, consistency with what the host holds) is admitted in whatever order it comes,
  which is stronger than ordering and needs no queue. What holding was meant to catch, an operation
  missing from a chain, is detected by the members' checkpoints (stage 4), which name each device's
  head.
- **A device that signs two things at one number is frozen** on every host that sees both, from
  that number, until an administrator removes it; the host reports it (`Trust::equivocating`).
- **A note is an operation** in its author's chain, numbered with the writes, and the host's table
  of who wrote over what is the store of those operations, not a separate record.
- **No time is read to admit an operation.** The one place a stamp is looked at is that a device's
  writes do not go back along one chain.
- **Sealing refuses instead of falling back.** A host that cannot sign or seal as the list
  requires sends nothing, and says why (`RouterError::EventNotSealed`), where it used to send what
  it could.
- **The settlement replaces the administrator's attestation** (stage 4). The attestation of an
  administrator was the weaker rule: one stolen key could settle an instant a slow member had not
  reached, and an administrator had to be online. A `todo.floor` now carries the checkpoints of
  every writer under one entry of the log, all for its instant and digest, and the attestation
  (`attest_floor`, `hold_floor_signature`) is gone. Every device that is a writer under the entry
  is required, so a lost writer stops a settlement until an administrator takes it off.
- **A head is a contiguous head.** A checkpoint names, for each device, the highest number held
  with every number before it, and the id of that operation; so a host that holds 5 and 7 and not
  6 names 4 or fewer, and a member that names 7 shows the gap to everyone who takes it in. An
  operation is not held back for a gap (above), the gap is only made visible.
- **Checkpoints are carried by the host, not by the machines.** The settle protocol of the
  machines is unchanged: the host reads the machine's digest, signs it, and moves the checkpoints
  between hosts by any channel (the relay of stage 5 will store them), so that no machine or
  document changes and no relay has to understand one.
- **Operations can be asked of any host** (`Trust::operations`) and are taken in by the same
  admission as anything else, so a relay that withholds is routed around by asking another relay
  or a member, whichever is reachable.
- **The persisted text holds every operation**, not only the ones keyed by their content, so a
  restarted host can still give a range it held; the text and its two functions are `to_text` and
  `restore_from_text` (`signatures_to_text` named what it no longer is).
