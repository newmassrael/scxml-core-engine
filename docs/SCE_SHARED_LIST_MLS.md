# The group key of a shared list: MLS

A list whose members do not trust the host between them seals its content under a key for each epoch
(`docs/SCE_SHARED_LIST_TRUST.md`, section 4). With MLS (RFC 9420) the key is a group's, and the group
is the list's because the membership log says so (`backends/rust/mls`, `sce-rust-mls`, ADR 0034
stage 6). MLS is used for what it is good at, the secrets: a new key on every membership change,
forward secrecy, and post-compromise security. Nothing about who is a member is taken from it.

## 1. What is used

The mandatory suite, MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519, through `openmls` (0.9), and one
implementation of it, in Rust. A device's MLS leaf signs with **the key the log gave it**, so a person
has one key for the log, for operations, and for the group. A leaf's credential names the list and the
node; a key package is valid for ten years (`LIFETIME_SECONDS`), because here the log decides who is a
member and for how long, and a group whose leaves expire by the clock would stop for a reason that is
no one's decision. Handshake messages (commits, Welcomes) are plaintext: the log already says who is
added and removed.

Content is not sent as MLS application messages. The key of an epoch is what the group's exporter
gives for the list (`Device::epoch_key`), and sealing is `sce_rust_trust::secrecy`'s (XChaCha20-
Poly1305, nonce and associated data from the write's stamp), so a relay and a machine see what they
saw before. A section of a list has a key of its own, derived from the epoch's with a label of its
own (`Device::section_key`): a device that is given that key and not the epoch's opens that section and
no other.

## 2. How the log binds the group

| Rule | What it stops |
|---|---|
| The epoch is the log's: `Body::MlsEpoch { epoch, commit }` is an entry the administrators sign, and `commit` is the SHA-256 of the commit message that moves the group to `epoch`. The group's epoch and the log's are the same number. | A relay that reorders commits, makes one up, or hands a member a commit of another group. A host applies a commit the log did not name never. |
| A device is added only if the log holds it: the key package's node must be a member of the log with the key the package is signed by. | A stranger, or a device that signs with another key, in the group. |
| After a commit the group's roster must be the log's members (the nodes and the keys of every grant of role 1 or more). A commit that leaves it otherwise is rolled back (the storage is restored). | An administrator's slip, or a commit that adds or keeps a member the log does not. |
| A device that joins from a Welcome checks it against the log: the commit that made the epoch (named by the log) is a plaintext message with a confirmation tag, and the group the Welcome made must hold that tag. A refused Welcome leaves the device as it was, its key package unused. | A Welcome of another group with the same members, which would give a joiner a key the attacker holds. |
| A list's epochs are all MLS's or all a drawn key's (`Body::Epoch`, which `Trust::rotate_epoch` makes), not both. | Two key schemes in one log. |

An administrator's change is two entries of the log and a commit: the grant (or its removal, with
`final_seq`), the entry that names the commit, and the commit itself, merged after the log holds it.
`Member::add_member` and `Member::remove_member` do it and give back what is for the relay.

## 3. A removed member

The grant that takes a device off (role 0, `final_seq`) comes first, then the commit that removes its
leaf, named by the next entry. The epoch after it is one the device cannot derive: it applies the
commit and finds itself removed (`Applied::Removed`), or never receives it, because the relay lets a
device that is off the log read nothing more; either way `Device::epoch_key` is `Removed` and what is
sealed afterwards does not open for it (`this host holds no key of epoch N`). What it was given before
stays what it was: nothing takes back a key a member held. A new member is given no history by
default (it holds no key of the epochs before it joined, and the operations sealed under them are
parked, not refused); an administrator may give it an older epoch's key (`Trust::grant_key`).

### A person's devices

A person's devices are separate leaves of the group, each with its own key and node. A new one is
linked by a device the person has (`Member::add_linked_member`: the log records who vouched, then the
group adds it), and is given no history; the device that vouched sends the keys of the epochs it holds
(`Member::history_grants`). A lost device is taken off with its chain cut where the administrator saw it
(and raised by an amendment if other writers' checkpoints show more), the group moves to an epoch it
cannot derive, and the person's other devices go on; taking the whole person off is a removal for each
device (`Member::remove_person`), each an epoch of its own. A device that is restored from a copy of its
secrets (`sce_rust_trust::backup`, with `Device::to_bytes`) is the same device, in the epoch it was in,
and applies the commits it missed in order.

## 4. What a relay carries

A relay keeps three more kinds of record and reads none of them: a key package a device leaves for an
administrator to add it with (any session may leave its own, a bounded number of them), the commit the
log names for an epoch (kept only if its hash is the one the log names), and a Welcome for a device
(kept for an epoch the log has begun). A host's courier hands them to its `Member`
(`Member::absorb`), which does what the log allows in order (`Member::advance`): a device in no group
joins from a Welcome addressed to it, a device in the group applies the commit the log names for the
next epoch, and each gives its `Trust` the key of the epoch it reaches. A commit the log does not name
yet waits. A relay that is not trusted can give any number of commits for an epoch, and a false one
first: the `Member` keeps them by their hash (a bounded number while the log names none), uses the one
the log names whichever came first, and drops the rest once the log names it. The bound has a cost: a
relay that gives more false commits than it, before the log names one, can make the host drop the true
commit, and a relay's feed does not give a record twice. The host is told this (`Member::commits_to_fetch`
lists the epochs whose commit the log names and it does not hold) and reads the relay again from the start
(`Courier::resync`): the log names the commit by then, so the true one is kept whatever came before it.
A host that does not ask stays at its epoch, which is a loss of availability and never of
confidentiality or integrity.

An operation sealed under an epoch whose key a host does not hold is **parked**: the courier tries it
again each round, since the key may come (the commit reaches the host), and goes on past it. It is not
refused, because it holds, and not an error, because for an epoch before the host joined it never
opens.

## 5. What a device keeps

`Device::to_bytes` is the device's MLS state (the group, its secrets, the private halves of its key
packages), secret, kept where the host keeps its person's secret; `Device::from_bytes` reads it back.

## 6. The other lanes: the C ABI

There is one implementation of the group, in Rust, and the lanes that are not Rust call it through
a C ABI (`backends/rust/mls-capi`, `sce-mls-capi`, header `include/sce_mls.h`, a shared and a static
library): a second implementation in each language would be a second place for the same flaw. A lane
holds a device (`SceMlsDevice`) and tells it **the membership log as data** (`SceMlsLog`: the members
with their keys, the commit hash the log names for each epoch, the epoch the log is in). The lane's own
verification of the log is what that data must be; the group applies a commit only when the data names
it and refuses a roster that is not the data's, as it does for a Rust host that gives it a `Trust`
(`LogView`, which `Trust` and `Roster` implement).

Every function returns 0 or a status (`SCE_MLS_ERR_*`) and says why in `sce_mls_last_error()`; the
buffers the library returns are given back with `sce_mls_buf_free`; a null pointer is a status, a
pointer is read for exactly the length it comes with, and a panic is caught at the boundary. This is the
one crate of the group that holds `unsafe` code. The header is written by hand and a test keeps it equal
to the library's exports and its status numbers.

MLS is randomised, so a known-answer vector cannot be computed from fixed inputs: it is a **recorded
scenario** (`backends/rust/mls/tests/fixtures/join_and_remove.json`). One device is kept as it was
before it joined (its state holds the private half of a key package), with the messages it is then
given: the Welcome and the commit that made its epoch, the commit that adds a third member, the commit
that removes it, each with the log as data and the keys it must derive (and a section's). A lane loads
the device, joins, applies, and the keys are the recorded ones until the device is removed, after which
it derives none. **Rust, C, Python (ctypes) and Go (cgo) replay the same file** and agree
(`a_c_abi_is_what_the_lanes_call_and_replays_the_recorded_scenario`). The scenario is recorded again
with `SCE_MLS_SCENARIO=write`; the leaves last ten years so that it does not go stale on a clock. The
Kotlin and C++ lanes use the same header (JNA or the Panama API; a plain include) and have no replay
yet.

## 7. What is not here

- The Kotlin and C++ replays of the scenario, and a membership log in any lane but Rust: a lane that
  is not Rust builds the log data from its own verification, and none of them has one.
- The machine's documents do not know of sections: a section's key is derived, and no write is yet
  in a section.
- External joins, proposals by members other than the committer, and application messages are not
  used.
- A member who is not online when the administrator removes someone applies commits in order when it
  returns; one that was offline for several epochs is brought up by the relay's commits in sequence.
