# ADR 0130 — A batch's records are pulled one at a time, a chain's entries are shown as they are read, and a machine's enums hold zero

- Status: Proposed (2026-11-25), under the owner's standing rule: the most long-term correct, textbook structure, cost
  ignored.
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
- A record handed out and not read is skipped by the next `next`, which reads it through the codec's own view and
  drops the value.
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

    pub trait ChainHook {
        fn entry(&mut self, site: ChainSite, entry: &[u8]) -> Result<(), CodecError>;
    }

`ChainSite` names the chain (`codec`, `field`) and `entry` is the bytes the entry occupies. `()` is the hook that
admits every entry, inlined to nothing.

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

`SceSink::write_u8` is a required method: a sink stores a byte itself. The provided VLE writes assemble the encoding
and hand it to `write_bytes` once (forge-vle-fast, 07d17351d20). The generated Rust encoder writes every header, flag
and one-byte field with `write_u8` and every VLE with `write_vle_u*`, never a one-byte slice, and a test holds the
printer to that.

### 4. A machine's enums are numbered from zero in the smallest representation that holds them (R6)

The generated Rust state, history and event enums of a statechart, and the state and event enums of a transition
function, have `#[repr(u8)]` where their variants number at most 256 and `#[repr(u16)]` otherwise, and discriminants
`0..n`, so all-zero bytes are a value of each: a region allocated zeroed holds valid values before it is initialised.

Zero is the first variant. For a statechart's states that is the first state in document order, which is the number
the tables know it by (ADR 0064); the `Default` stays the document's initial state, which a pre-order numbering need
not make the first. An engine region allocated zeroed is a valid value that has not been initialised, and a run starts
from the initial configuration, never from a zeroed field. The runtime's own enums (`EventPayload`'s kind, a state's
kind) are `#[repr(u8)]` already.

### 5. A ZInt is read by one body, the cursor's and the frame reader's alike (R1)

A ZInt of one byte is read inline at each field and every longer one by one cold body (`codec::zint_tail`), which
`SceCursor::read_vle_*` and ADR 0128's `SceReader::read_vle_*` both run: the generated decodes, which read through
the reader, take the fast path the cursor took (forge-vle-fast, b6365060f0a), and the grammar of a ZInt is written once
in the runtime.

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
- **Give the hook the entry's view, typed.** Each chain would need a hook of its entry's type, and a hook threaded
  through every body would need a bound for every entry type a frame reaches. The bytes of the entry are what a
  judgement of a header reads. Refused.
- **Number the initial state zero.** It would break the pre-order numbering the runtime's Appendix D core relies on
  (ADR 0064), to give a zeroed region a meaning a run never reads. Refused.
