# The audit log's entries, chain and signed checkpoints (`sce-audit-1`)

Status: `pre-release` (registered in `SCE_WIRE_CONTRACTS.md`). Decided by ADR 0109, D14
(`docs/adr/0109-a-server-on-the-public-network-keeps-what-it-owes-bounds-what-its-clients-cost-and-is-upgraded-in-place.md`).

A host keeps a durable audit log for its statecharts (docs/SCE_HOST_FEATURES.md, section 15): streams of
entries, append-only, each entry chained on the one before it, and a signature over the chain every so many
entries and at a stop, so that a reader who holds the host's audit public key can prove that an entry was
changed, removed or cut from the end. This document fixes the bytes, so that a log written by one program is
verified by another and the Rust and C++ implementations agree:

- Rust: `sce_rust_crypto::audit` (`backends/rust/crypto/src/audit.rs`), over the store contract
  `sce_rust_session_store::AuditStore`, and the redaction of `sce_rust_host_features::audit`;
- C++: `SCE::StandardHost::audit` and `AuditFeature` (`sce/include/host/AuditHost.h`), on libsodium.

The vectors are `tests/host_features/audit_cross_language.json`, computed with Python's `hashlib`, `hmac` and
the `cryptography` package apart from both hosts; `backends/rust/crypto/tests/audit.rs`,
`backends/rust/host-features/tests/audit.rs` and `tests/integration/AStandardHostAuditTest.cpp` hold both to
every byte.

## 1. A row

A stream is named by a UTF-8 text of 1 to 64 bytes. Its rows are numbered 1, 2, 3 and on, one past the last
(the stream's **head**, which outlives retention), and each holds:

| Column | Value |
|---|---|
| `sequence` | its number in the stream, from 1 |
| `at_ms` | when it was appended, milliseconds since the Unix epoch by the host's clock |
| `entry` | the redacted entry in its canonical form (section 2), UTF-8 |
| `chain` | 32 bytes (section 3) |

A row is never changed. Retention deletes rows only by whole segments (section 6).

## 2. The entry

The statechart gives a record of the event schema its `sce:entry` names, as the record's JSON payload, and the
marks the schema declares on its fields, which the build writes (`sensitivity`, SCE Accepted Subset §2.12):
each mark a JSON pointer to a field and the mark, joined by `;`, as `/password=secret;/email=personal`. A
segment `*` names every element of a list.

**Redaction.** For each mark, in order, the member it names is replaced:

| Mark | What is written in its place |
|---|---|
| `secret` | the text `sce-secret`, whatever the value; also for an absent one |
| `personal` | the text `sce-personal:` and the lowercase hexadecimal of the HMAC-SHA-256 (RFC 2104) of the value's UTF-8 bytes under the log's pseudonymisation key; an absent value (`null`) stays `null`; in a stream that keeps personal values in the clear, the value as it is |

A mark that names a member the entry lacks, a `personal` mark on a value that is neither a text nor `null`, and
an entry that is not a JSON object are refused (`invalid_argument`): the build writes none of them, so only a
request made without the generated interface carries one. The pseudonymisation key is 32 bytes a host draws
from its random source at its first use and keeps (Rust: a MAC key `audit:pseudonym` of the key ring).

**The canonical form** the entry is written in, after redaction: JSON with no whitespace, an object's members in
the order of their names' UTF-8 bytes at every depth, a string with `"` and `\` escaped, `\n`, `\r` and `\t` as
those two characters, every other character below U+0020 as `\u00XX` in lowercase hexadecimal, and every other
character as itself; `true`, `false` and `null` as written; an integer as its decimal digits. A non-integer
number is written as each host's JSON library writes the shortest text that reads back to it, which the
libraries spell apart (`1e+20` and `1e20`): an entry two hosts must agree on byte for byte holds no such
number, and a record of SCE's payload form writes a 64-bit integer as a text, which is.

**Who an append was made for** (ADR 0116, D7). An append a host-run invocation makes in a run under a delegation,
whose invoker is lent the person the run acts for (Rust: `EngineState::register_invoker_for`), is written with one
member more, `sce:principal`, which no record a document declares can name: the object `{"instance", "issuer", "method",
"proved_ms", "subject"}` of the instance that acts for the person and the person as the server's store holds the
delegation, `proved_ms` as its decimal digits and `subject` written as a `personal` value is (its keyed digest
with the prefix `sce-personal:`, or as it is in a stream that keeps personal values in the clear). The host writes
it before the canonical form is taken, so the chain covers it. An entry received with a member of that name is
refused (`invalid_argument`): a document neither writes who an append was made for nor leaves it out. An append made
for nobody, and every append of a host that serves no person (the C++ host), carries no such member.

## 3. The chain

    chain_0 = 32 zero bytes
    chain_n = SHA-256(chain_(n-1) || E_n)
    E_n     = the deterministic CBOR (RFC 8949 section 4.2.1) array [stream, n, at_ms, entry]
              of a text string, an unsigned integer, an unsigned integer and a byte string

A row changed, removed, moved to another number or another stream, or given another moment changes the chain
of every row after it.

## 4. The checkpoint

A checkpoint of a stream at row `n` is the tagged `COSE_Sign1` (RFC 9052 section 4.2; tag 18) of the payload

    the deterministic CBOR array ["sce-audit-1", stream, n, chain_n]
    of a text string, a text string, an unsigned integer and a byte string of 32 bytes

with the protected header `{1: -8, 4: kid}` (EdDSA, and the bytes of the key's id, `audit:sign`), an empty
unprotected header, the payload attached, no external data, and the Ed25519 signature (RFC 8032) of its
`Sig_structure` under the host's **audit key**: a signing key a host draws at its first checkpoint and keeps
(Rust: the key `audit:sign` of the key ring, ADR 0109 D7; its public key exported as any signing key's is).
`sce-audit-1` is the version of the form.

A host writes a checkpoint with the row whose number is a multiple of the stream's `checkpoint_every`, in the
transaction that appends it, and one of the head of every stream at its orderly stop.

## 5. Verifying a stream

A reader holds the audit public key, reads every checkpoint of the stream and every row, and:

1. refuses the stream when a checkpoint's signature does not verify over its own stream, sequence and chain
   (**forged**);
2. anchors at the predecessor of the first row it holds: the genesis (`0`, `chain_0`) when that row is 1, the
   checkpoint a retention left at its cut otherwise; with no such checkpoint the stream is **unanchored**;
3. recomputes the chain of every row forward from the anchor, refusing a number that does not follow
   (**missing**) and a row whose stored chain, or whose checkpoint's chain, is not the one recomputed
   (**changed**);
4. refuses a checkpoint, or a head, past the last row it holds (**truncated**): rows were cut from the end.

A truncation after the last checkpoint is told only by a later checkpoint, which is why a host signs its head
at a stop. A reader verifies from any signed checkpoint forward the same way, anchoring there.

## 6. Retention

A stream keeps its entries for its `retain_days`. A retention pass at a moment deletes, of each stream, the
segment ending at the latest checkpoint before the first row appended at or after `retain_days` before the
moment: every row through that checkpoint and every checkpoint before it. The checkpoint at the cut is kept,
and anchors what remains (section 5, step 2). A stream whose rows are all older keeps its last checkpoint and
the rows after it.

## 7. At rest

Under a sealing store (docs/SCE_SEALED_RECORD.md) an entry and a checkpoint's signature are sealed records and a
stream's name is its entry; the sequence, the moment and the chain are kept as they are, so a reader verifies
the chain without the sealing keys. The chain is the digest of what a row holds: an entry of few possible
values can be found again from it by trying them, and a log of such entries keeps a value that is not, a
request's id, in each.

## 8. Vectors

`tests/host_features/audit_cross_language.json` holds the audit seed and its public key, the pseudonymisation
key, the stream `sign-ins` and the marks `/account=personal;/password=secret`; for each of three rows the entry
as received, as written, `E_n` and the chain; and the checkpoint of the third row, its payload and its
`COSE_Sign1`.
