# ADR 0130 — A batch's records are pulled one at a time, a chain's entries are shown as they are read, and a machine's enums hold zero

- Status: Accepted (2026-11-25), under the owner's standing rule: the most long-term correct, textbook structure, cost
  ignored. Sections 1, 2 and 4 are this record's decisions; section 3 records the runtime half that landed with ADR
  0128's integration (a3b833ee19b) and adds only the printer's half, and section 5 finishes the single ZInt read that
  landed there (25af0b8b61e) by giving the frame reader the same body.
- Amended (2026-11-25): a chain hook's refusal is a kind of its own (section 2), and a record handed out and not read
  is skipped as the arm it was handed out as, not as the codec's view of every arm (section 1).
- Amended by ADR 0137 (2026-11-26): the reader runs the longer body through one out-of-line function of its own,
  `SceReader::read_vle_long`, so no field holds the body's answer and record, and `codec::zint_tail` reads the chain
  by one loop (section 5).
- Amended by ADR 0138 (2026-11-26): a ZInt of one byte is no longer read in place at each field; every ZInt is one
  call of its reader's one out-of-line read, which runs the one body, `codec::read_zint` (was `zint_tail`), no longer
  cold (section 5). The provided VLE write takes its encoding from one out-of-line function, `codec::encode_vle`
  (section 3).
- Amends: ADR 0049 (a decoded Rust codec is a view over its wire bytes): a view's read takes a chain hook. ADR 0128 (a
  frame's reads are total and its verdict is read once): a sequence of frames is read through one reader, whose first
  refusal ends the sequence.
- Related: ADR 0100 (a codec dispatches on a masked tag), ADR 0103 (a frame is walked as events), ADR 0061 (a sink's
  writes), ADR 0064 (a document's tables number its states).
- Requested by: a downstream bare-metal consumer, 2026-11-25, whose receive path reads a batch of messages per datagram
  and whose session region is allocated zeroed.

## Context

The consumer measured its receive path against a C implementation of the same protocol and found two copies of the
grammar in its own code. It walks a batch by a hand-written dispatch on the message tag that calls the generated view
of each message through a chain of callbacks, and it judges the extension chain of a message by a hand-written reader,
because the generated view reads the chain once to check it and the consumer then had to walk it again to decide
whether to admit the message. Both copies exist because the generated code had no shape for them: no reader of a
sequence of frames, and no point at which a consumer sees a chain's entry as it is read.

The read of a batch has a cost of size as well as of time: the image the consumer ships has 116 bytes of stack to
spare under the call that hands it a message, and a value of every message kind at once (the eager `Declare` alone is
about 2 KB) cannot be returned there.

Two smaller facts came with the request. A sink's one-byte write was a write of a one-byte slice, a general copy for
each header byte and each byte of a VLE. And the consumer allocates its session region zeroed, so every value in it
must be one whose all-zero bytes are a value.

## Decision

### 1. A sequence of frames is read by a pull reader of records (R4)

For a variant codec that dispatches on a peeked byte and has no field of its own (the shape of a network message
envelope, ADR 0100), the Rust view form generates `{Codec}Records<'f>`, a reader of the frames of that codec laid one
after the other in `&'f [u8]`:

- `new(bytes)`, `next(&mut self) -> Option<{Codec}Record<'_, 'f>>`, `consumed()` (the bytes of the records read
  whole) and `verdict()`.
- `{Codec}Record<'r, 'f>` is an enum with one variant per arm the document names, each holding a handle of the
  runtime's `Record<'r, 'f, W>`: a borrow of the reader and nothing else. Its `read()` reads that arm's view, by
  value, from where the record starts, and `read_with(hook)` does so with a chain hook (section 2). The handle is what
  `next` returns; the body of a record is read only when the consumer asks for it, where it asks, so the stack under
  the consumer's call holds the view of the one kind it reads and not the largest of them.
- The size of the record is an invariant: `size_of::<{Codec}Record>()` is at most 64 bytes on every target, and the
  generated module asserts it at compile time. A handle is one pointer, so the record is two words.
- A record handed out and not read is skipped by the next `next` as the arm it was handed out as. `record::<W>()`
  records in the sequence the skip of its kind `W` (the runtime's `skip_record::<W>`, out of line, held as a function
  pointer, so the handle stays one pointer), and the next head runs it: it reads that arm's view from where the record
  starts and drops it. The skip reads the record's own extent and nothing else, so the stack under it holds that arm's
  view and never the codec's view of every arm, which is the cost this section exists to avoid. A skipped record that
  is malformed refuses the sequence as a record read does, and `consumed` stays before it.
- The sequence ends at the end of the bytes, at a tag no arm names, and at the first refusal. A tag no arm names is the
  start of something else (a transport header after a batch's network messages), so the sequence ends before it and
  `consumed` says where; a codec's default arm takes such a tag when one frame is decoded, and is not a member of a
  sequence.
- The reader is ADR 0128's `SceReader`, one for the whole sequence: the first refusal of any record is the
  sequence's verdict, `next` answers `None` from then on, and `consumed` stays at the end of the last record read
  whole.
- A tag of an arm the build leaves out (ADR 0072) refuses the sequence with `VariantArmNotBuilt`, as one frame's
  decode refuses it: the tag is one the document names, so the frame is not something else. A codec every arm of
  which a build may leave out has no records, since a record enum with no variant in some build is no type.

The rest of the codec does not change: `{Codec}View::decode` still reads one frame as the enum of every arm.

### 2. A chain's entries are shown to the consumer as they are read (R3)

The runtime gains `ChainHook` (`backends/rust/forge-runtime/src/records.rs`, re-exported from `codec`):

    pub struct EntryRefused;

    pub trait ChainHook {
        fn entry(&mut self, site: ChainSite, entry: &[u8]) -> Result<(), EntryRefused>;
    }

`ChainSite` names the chain (`codec`, `field`) and `entry` is the bytes the entry occupies. `()` is the hook that
admits every entry, inlined to nothing.

A hook's refusal is a kind of its own. A hook answers admit or `EntryRefused` and nothing else, so it cannot say a
frame is short or malformed when the frame is neither; the frame's reader records the refusal as
`CodecError::ChainEntryRefused`, a variant of the one error type every verdict is (ADR 0128), and a consumer tells its
own refusal from the frame's by that variant. Why the hook refused is the hook's own state, which the consumer holds
after the read. The kind is the Rust runtime's alone: only a Rust view reads with a hook, and the document's grammar
never decides it, so it is no kind of the decode vocabulary every runtime declares (ADR 0099) and no reject vector
names it. `CodecError` is `#[non_exhaustive]`, and no code of the repository maps its kinds one by one to another
name (the bare-metal probe answers every refusal as one status), so the variant is total wherever a verdict is read.

Every Rust view gains `read_with(cursor, .., hook)` and `decode_with`; `read` is `read_with` with `()`. A view
hands the hook to every body it reads (an embedded body, an arm, a list's entry), so one hook sees every chain of a
frame however deep it nests. A `<sce:tlv-chain>` scan calls the hook once per entry, right after the entry is read and
before the next is, and records what the hook refuses in the frame's reader: the hook can end the frame at the entry it
refuses (an unknown entry the protocol marks mandatory), and an entry it admits is skipped or kept as the view keeps it
(a view keeps no entry; its walk reads them again). A walk of a list does not call the hook: the scan did.

The hook is generic, so the code of a view read with `()` is the code of a view read without it. It is not ADR 0103's
walk: a walk event names a position in a tree for an analyser, and this hook is on the message path, with no event
value and no record of where it is.

### 3. A one-byte write is the sink's own store, and a VLE is one write (R5)

The runtime half is decided and landed already (a3b833ee19b): `SceSink::write_u8` is a required method, so a sink
stores a byte itself, and the provided VLE writes assemble the encoding and hand it to `write_bytes` once. This record
does not decide it again. What it adds is the printer's half: the generated Rust encoder writes every header, flag and
one-byte field with `write_u8` and every VLE with `write_vle_u*`, never a slice it built for the write, and a test
sweeps every codec resource to hold the printer to that
(`sce-build/tests/a_rust_encoder_writes_a_byte_and_a_vle_each_in_one_write.rs`).

### 4. A machine's enums are numbered from zero in the smallest representation that holds them (R6)

The generated Rust state, history and event enums of a statechart, and the state and event enums of a transition
function, have `#[repr(u8)]` where their variants number at most 256 and `#[repr(u16)]` otherwise, and discriminants
`0..n`, so all-zero bytes are a value of each: a region allocated zeroed holds valid values before it is initialised.
The count is that of the list the variants are made from: a statechart's events are `tables.events` (the one list the
event numbers and the selection rows are made from, ADR 0127) and the null event after them. An event enum of a
transition function that names no event has no variant and so no representation.

Zero is the first variant. For a statechart's states that is the first state in document order, which is the number
the tables know it by (ADR 0064); the `Default` stays the document's initial state, which a pre-order numbering need
not make the first. An engine region allocated zeroed is a valid value that has not been initialised, and a run starts
from the initial configuration, never from a zeroed field. The runtime's own enums (`EventPayload`'s kind, a state's
kind) are `#[repr(u8)]` already.

### 5. A ZInt is read by one body, the cursor's and the frame reader's alike (R1)

The cursor's one-byte inline read and one cold body of every longer ZInt landed before this record (25af0b8b61e), but
ADR 0128's `SceReader` kept a byte loop of its own, and the generated decodes read through the reader: the grammar of
a ZInt was written twice, and the fast path was not the one the generated code took. Both are now one: a ZInt of one
byte is read inline at each field (`codec::zint_byte`) and every longer one by one cold body (`codec::zint_tail`),
which `SceCursor::read_vle_*` and `SceReader::read_vle_*` both run, the reader recording the body's refusal through
`SceReader::fail` as every other refusal of a frame. (ADR 0137: the reader calls the body through
`SceReader::read_vle_long`, one function out of line, and the body is one loop.)

## Consequences

- Every Rust codec golden under `tests/forge/expected/` and `tests/forge/expected/views/`, every committed tree of
  generated Rust codecs, and every committed Rust statechart and transition function is generated again.
- A consumer reads a batch with `while let Some(record) = records.next()` and a `match`, with no callback and no
  `dyn`.
- A consumer that judged a chain by walking it after the decode judges it in the decode's one read.

## Alternatives considered

- **Return the view of every arm as one enum from `next`.** It is the shape of one frame's decode, and it puts the
  largest arm's view on the stack of every call, whatever the record is. Refused for the sequence; kept for one frame.
- **Find a record's extent in `next` and hand the consumer its bytes.** A frame of ZInts and chains has no extent
  until it is read, so the record would be read twice. Refused.
- **Let a hook refuse with any `CodecError`.** A hook could then answer `TooManyElements` or `NeedMoreBytes` for a
  frame that is neither, and a consumer reading the verdict could not tell its own refusal from the frame's. Refused.
- **Give the hook an error type of its own (`type Error`).** The verdict of a frame read with a hook would be
  `CodecError` or `H::Error`, so `SceReader`, `Records`, `decode_with` and every view's read would carry the hook's
  type, the one verdict type of ADR 0128 would become one per hook, and every mapping of a verdict to a status a caller
  of another language reads would be one per hook as well. The hook's reason is its state and needs no channel through
  the verdict. Refused.
- **Skip a record not read as the codec's view.** It is a read of the enum of every arm, whose largest arm's view the
  skip then puts on the stack, in exactly the case a record exists to avoid it. Refused: a record is skipped as the
  kind it was handed out as.
- **Give the hook the entry's view, typed.** Each chain would need a hook of its entry's type, and a hook threaded
  through every body would need a bound for every entry type a frame reaches. The bytes of the entry are what a
  judgement of a header reads. Refused.
- **Number the initial state zero.** It would break the pre-order numbering the runtime's Appendix D core relies on
  (ADR 0064), to give a zeroed region a meaning a run never reads. Refused.
