# The sealed record of a server's store (`SCE-SEALED-1`)

Status: `pre-release` (registered in `SCE_WIRE_CONTRACTS.md`). Decided by ADR 0109, D8
(`docs/adr/0109-a-server-on-the-public-network-keeps-what-it-owes-bounds-what-its-clients-cost-and-is-upgraded-in-place.md`).

Amended by: ADR 0114 (D8): the event journal, its row in section 3 and its re-seal pass in section 4; (D2, D4): a
session's record bound to its machine as well as its key, and the mark of a reader of the journal kept under the entry
of its name (section 3).
Amended by: ADR 0113 (D4, D5): the members of an open machine, their row in section 3.

Amended by: ADR 0116 (D7): the person a member's incarnation acts for, its row in section 3 and its re-seal pass in
section 4; who a fact of the journal was said for, inside its record (section 3).

A server keeps its sessions (ADR 0107) with its durable clients' outboxes (ADR 0109, D10), its credentials (D3), its
keys (D7), its key-value feature's values (D13), its audit log (D14), its event journal (ADR 0114) and the members of its
open machines with the person each acts for (ADR 0113, ADR 0116) in a store. SCE_MESH.md
section 10.11 asks a host to store a checkpoint encrypted at rest under its own key, since the saved state in it holds
a document's values in the clear. A **sealing store** does that over any store: every value it writes is a sealed
record, and every key it finds a row by is kept as a keyed digest of the key, its **entry**. This document fixes the
bytes, so that a store sealed by one program is opened by another, and the Rust and C++ implementations agree:

- Rust: `sce_rust_crypto::sealed` (`backends/rust/crypto/src/sealed.rs`), whose `Sealed<S>` implements each store
  contract of `sce-rust-session-store` over another implementation of it;
- C++: `SCE::StandardHost::SealedRecord` (`sce/include/host/SealedRecord.h`), the record on libsodium, for an
  embedding that keeps rows of its own (the C++ reference host keeps its keys and tokens in memory and has no store).

## 1. The record

| Bytes | Field | Value |
|---|---|---|
| 12 | magic | the ASCII text `SCE-SEALED-1`, the version of the form |
| 1 | id length | the byte length `n` of the sealing key's id, 1 to 255 |
| `n` | key id | the UTF-8 id of the key that sealed the record |
| 24 | nonce | 192 bits drawn from the operating system's generator for each record |
| rest | ciphertext and tag | the XChaCha20-Poly1305 encryption of the plaintext (`crypto_aead_xchacha20poly1305_ietf` of libsodium, RustCrypto's `XChaCha20Poly1305`), followed by its 16-byte tag |

The **associated data** the cipher authenticates is the record's header (magic, id length, key id), then the store's
name after its byte length in one byte, then the row's entry, which runs to the end:

    associated data = magic || n || key id || m || store name (m bytes) || entry

So a record copied to another row of the store, or to another store, does not open; neither does one changed by a
byte anywhere, nor one opened under another key of the same id. A reader picks the key by the id the record names
and refuses a record that names a key it does not hold.

A random 192-bit nonce sets no bound on how many records one key seals that a server could reach: a 96-bit random
nonce is bounded at 2^32 records per key (NIST SP 800-38D section 8.3), which a server that writes a checkpoint per
input reaches.

## 2. Entries

A row is found by its **entry**, never by its key:

- a key that is text (a session's key, a client's name, a key's id, a generation's identity) is entered as the
  lowercase hexadecimal of the HMAC-SHA-256 of its UTF-8 bytes under the store's **index key**: 64 characters;
- a token's digest (the SHA-256 of its text, D3) is entered as the 32 bytes of its HMAC-SHA-256 under the index key.

A key that names a person, or is a bearer value, is then never in the clear on the medium; the key itself is inside
the sealed value. The entry of a key is the same in every process that holds the index key, and a store lists its
rows (a session store's `due_before`) by their entries.

## 3. What each store seals

Each value is a deterministic CBOR item (RFC 8949 section 4.2.1), the plaintext of the record of its row, sealed for
the store named in the second column with the row's entry:

| Contract | Store name | Entry | Plaintext | Kept in the clear by the wrapped store |
|---|---|---|---|---|
| Sessions (`SessionStore`, ADR 0107) | `sessions` | the entry of the session's key; for a handle of a named machine (ADR 0114, D2), the byte length of the machine's name as an unsigned 64-bit big-endian integer, the name in UTF-8 and then that entry, so a record moved to the equal key of another machine does not open | `[key: text, state: bytes]` | the machine, the version, the earliest due, the lease (owner, fence, expiry) |
| Pins (`PinStore`, D3) | `credential_pins` | the entry of the client's name | `[name: text, pin: bytes]` | nothing else |
| Tokens (`TokenStore`, D3) | `credential_tokens` | the keyed digest of the token's digest | `[digest: bytes, name: text, issued_ms: uint, expires_ms: uint, sealed: bytes]` | the revocation; the row's name is the entry of the name, which a revocation matches; its moments are 0 |
| Keys (`KeyStore`, D7) | `keys` | the entry of the key's id | `[id: text, label: text, wrapped: bytes]` | the use, the algorithm, the creation, the state; the label is empty |
| The naming key of a key store | `key_naming` | empty | the wrapped naming key's bytes | nothing else |
| Outboxes of a durable client (`SessionStore`, D10) | `outbox` | the entry of the client's name | `[client: text, envelope: bytes]`, or `[client: text, envelope: bytes, sender: text]` for an envelope a session sent, the sender its key | the number, the moment and the sender's entry; the outbox's incarnation and next number. An expiry's note holds the record of the entry it was written for, under the same entry, and names the session it is for by that session's entry |
| Values of the key-value feature (`KvStore`, D13) | `kv_values` | the entry of the value's key | `[key: text, value: bytes]` | the version; a first result of a put-if is kept under its identity's session and invoke, each as its entry, its outcome and version in the clear |
| Rows of an audit log (`AuditStore`, D14) | `audit_rows` | the entry of the stream's name, a colon and the row's sequence in decimal | `[stream: text, entry: bytes]` | the sequence, the moment, the chain (docs/SCE_AUDIT_LOG.md), which a reader verifies without the sealing keys; the head; an append's identity as a first result's is |
| Checkpoints of an audit log (`AuditStore`, D14) | `audit_checkpoints` | as the row's | `[stream: text, signature: bytes]` | the sequence and the chain |
| Members of an open machine (`MembershipStore`, ADR 0113) | none: no record | the entry of the member's name, by which a write made under its incarnation names it too, and by which a member that ran out is answered (`member_entry`) | none | the incarnation, the low-water mark, the lease's expiry and length and whether a write kept it; an offer's incarnation, expiry and the nonce of the join it answers, a value the instance drew |
| Entries of the event journal (`JournalStore`, ADR 0114) | `journal` | for a session's stream, the entry of its machine's name, a colon, the entry of its key, a colon and the position in decimal; for an observed stream, the entry of its name, a colon and the position | `[kind: uint, first: text, second: text, schema: text, revision: uint, data: bytes]`, and `valid_ms: int` as a seventh item for a fact with a valid time; for a fact said for a person (ADR 0116, D7) eight items, the seventh `valid_ms` or `null` and the eighth `[instance: text, issuer: text, subject: text, proved_ms: uint, method: text]`; `kind` is 0 for a session's entry, whose `first` and `second` are its machine and its key, and 1 for an observed one, whose are the stream's name and the origin id | the position and the recorded moment, the stream's entries and the entry of an origin id, and the horizon; the wrapped store's schema is empty, its revision 0, and it holds no valid time and no one a fact was said for; a reader's mark (`keep_read`, ADR 0114 D4) is kept under the entry of the reader's name, its position in the clear |
| The person of a member's incarnation (`MembershipStore`, ADR 0116 D7) | `member_delegations` | the entry of the member's name, a colon and the incarnation in decimal | `[issuer: text, subject: text, proved_ms: uint, method: text, sealed: bytes]` | the entry of the member's name, its incarnation, mark and lease, the delegation's `until`; the person's subject is the entry of the issuer's length in decimal, a colon, the issuer and the subject, which a revocation and a renewal match; its issuer and method are empty and its moment 0 |

A journal entry's position is given inside the transaction that writes it, so its record is made there. A store that
holds no valid time cannot order by one: an answer as of a moment over a sealed journal opens every entry of the
stream at or below the position asked, for each page (`backends/rust/crypto/src/sealed/journal.rs`).

A generation of a key is kept under the entry of its identity, naming the entry of its key. What the wrapped store
keeps in the clear is what it compares or sets: a write's version and fence, the index of dues, a revocation, a key's
state.

## 4. The keys

The sealing store's keys are keys of the host's key ring (D7), kept in the store's own key store and wrapped under
the **key-encryption key**, which the program holds apart from the store:

- `index:sealed`, the index key, a MAC key of 32 bytes; it is never rotated, since every entry is made under it;
- `sealing:` and 16 random bytes in hexadecimal, the sealing keys, each 32 bytes for XChaCha20-Poly1305.

These rows are the ones a sealing key store keeps as they are given: a key cannot be sealed under itself, and the
key-encryption key protects them. `SealingKeys::from_keyring` draws the index key and a first sealing key at the
store's first use; of the active sealing keys, the latest made seals what is written (the greatest id among two made
at one moment).

**Rotation.** `SealingKeys::rotate` makes a new sealing key and retires every other active one; a retired key still
opens what it sealed. A store given the new keys writes under the new one, and a **re-seal pass** of each contract
writes every row sealed under any other key again through the store's ordinary write: a session's versioned and
fenced put, under a lease the pass takes and gives back (a session another process holds is left for a later pass;
`Sealed::reseal_sessions`); a pin's, a token's, a value's, an outbox envelope's, a note's, an audit entry's, a
checkpoint signature's, a journal entry's or a person's replacement from the bytes it read, every other column kept, so
a row another process changed in between is left (`reseal_credentials`, `reseal_values`, `reseal_outboxes`,
`reseal_audit`, `reseal_journal`, `reseal_members`); a key's rewrap (`reseal_keys`). An audit row keeps its chain, which is over the entry the record holds and not over the record, so a
re-sealed stream verifies as before. Once every pass leaves nothing and refuses nothing, no row names the retired
key, and it may be destroyed.

## 5. What it does not give

Freshness against someone who can write the medium: a row replaced by an earlier sealing of the same row opens. The
lease's fence and the version stop a process that is wrong, not a medium written by hand. Nor does it hide how many
rows a store holds, when each was written, or which session is due when.

## 6. Test vectors

`tests/host_features/sealed_cross_language.json` holds four records (each a key, its id, a nonce, a store's name, an
entry and a plaintext, and the record they seal to), the entries of four keys under one index key, and a token's
entry. libsodium computed them; `backends/rust/crypto/tests/sealed.rs` and `tests/integration/ASealedRecordTest.cpp`
seal each plaintext to its record and open it back, byte for byte, so the two implementations agree.

The fourth, written out: the key `000102...1f` (32 bytes counting up from 0), named
`sealing:000102030405060708090a0b0c0d0e0f`, seals the empty plaintext with the nonce `c0c1...d7` (24 bytes counting
up from `c0`) for the store `key_naming` and the empty entry. Its associated data is

    5343452d5345414c45442d3128 7365616c696e673a3030303130323033303430353036303730383039306130623063306430653066
    0a 6b65795f6e616d696e67

and its record, the header, the nonce and the tag alone, is

    5343452d5345414c45442d3128 7365616c696e673a3030303130323033303430353036303730383039306130623063306430653066
    c0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7 cfed5fbb0d7a1159339f296d91269fe1

(spaces and line breaks separate the fields).
