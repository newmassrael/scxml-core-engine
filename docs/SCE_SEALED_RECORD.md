# The sealed record of a server's store (`SCE-SEALED-1`)

Status: `pre-release` (registered in `SCE_WIRE_CONTRACTS.md`). Decided by ADR 0109, D8
(`docs/adr/0109-a-server-on-the-public-network-keeps-what-it-owes-bounds-what-its-clients-cost-and-is-upgraded-in-place.md`).

A server keeps its sessions (ADR 0107), its credentials (ADR 0109, D3) and its keys (D7) in a store. SCE_MESH.md
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
| Sessions (`SessionStore`, ADR 0107) | `sessions` | the entry of the session's key | `[key: text, state: bytes]` | the version, the earliest due, the lease (owner, fence, expiry) |
| Pins (`PinStore`, D3) | `credential_pins` | the entry of the client's name | `[name: text, pin: bytes]` | nothing else |
| Tokens (`TokenStore`, D3) | `credential_tokens` | the keyed digest of the token's digest | `[digest: bytes, name: text, issued_ms: uint, expires_ms: uint, sealed: bytes]` | the revocation; the row's name is the entry of the name, which a revocation matches; its moments are 0 |
| Keys (`KeyStore`, D7) | `keys` | the entry of the key's id | `[id: text, label: text, wrapped: bytes]` | the use, the algorithm, the creation, the state; the label is empty |
| The naming key of a key store | `key_naming` | empty | the wrapped naming key's bytes | nothing else |

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
opens what it sealed. A store given the new keys writes under the new one, and a **re-seal pass** writes every row
sealed under any other key again through the store's ordinary write: a session's versioned and fenced put, under a
lease the pass takes and gives back (a session another process holds is left for a later pass); a pin's or a token's
replacement from the bytes it read; a key's rewrap. Once a pass leaves nothing and refuses nothing, no row names the
retired key, and it may be destroyed.

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
