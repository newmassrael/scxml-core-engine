# ADR 0138 — A VLE is encoded and a ZInt read by one function out of line, and only the read of a frame is generic

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure,
  decided by measurement. Its figures are the consumer's measurement of the pin `65288b0696a` against
  `b616325c651` (section "Context").
- Amends: ADR 0130 (R1, a ZInt is read by one body): a ZInt of one byte is no longer read in place at each field;
  every ZInt, of every length, is a call of one function of its reader, which runs the one body. ADR 0130 (R5, a VLE
  is one write) and the provided VLE write of `07d17351d20`: the encoding the provided write hands to `write_bytes`
  is made by one function out of line, not assembled in each sink and at each call site. ADR 0137 (section 3, the
  one-byte read kept at the field; section 4, the reader's start inlined at each decode): both are withdrawn, and the
  start of a frame is one function of the runtime, as its verdict is.
- Related: ADR 0061 (a sink's VLE of one or two bytes stored in line, and `CountSink`'s length), ADR 0128 (a frame's
  reads are total and its verdict is read once), ADR 0136 (a chain is read by one routine, dispatched statically).
- Requested by: a downstream bare-metal consumer, 2026-11-26.

## Context

The consumer measured its release no_std image (fat LTO, `opt-level = 3`) at the pin `65288b0696a` against
`b616325c651`, attributing each byte of code by its DWARF line to the function of the forge runtime whose line it
is, wherever that function was inlined. The runtime's bytes grew from 4,020 to 8,052 (+4,032). The lines that moved:

| Lines of | Before | After |
|---|---:|---:|
| `codec.rs` `fill_vle` | 0 | 1,486 |
| `codec.rs` `vle_len` | 24 | 334 |
| `reader.rs` `read_vle_inner` | 0 | 826 |
| `codec.rs` `read_vle_inner` | 37 | 779 |
| `codec.rs` `read_zint_byte` | none | 87 |
| `reader.rs` `new` | 0 | 404 |
| `reader.rs` `read_frame` | 0 | 141 |
| `reader.rs` `settle` | 0 | 178 |
| `codec.rs` `note_capacity` | | -528 |

The reader's `take_array`, `take_slice`, `advance`, `remaining` and `rest` together come to about what the old
cursor's primitives were, and are not a cause.

The receive cost of an 8-byte put, per message, went from 4,843.0 to 4,848.4 instructions (+5.4): the code the
inlining added bought no speed.

Three shapes are behind the growth.

**W1: every sink and every call site held the VLE encoding.** The provided `SceSink::write_vle_inner` (`07d17351d20`)
built the encoding in a local array with `fill_vle` and `vle_len`, both `#[inline(always)]`, and handed it to
`write_bytes`. A provided method is compiled for each sink that uses it, and the consumer's own sinks use the
defaults, so the loop and the length were in each sink's VLE write, and, where that write was inlined, at each field
that writes a VLE.

**W2: every ZInt field held the one-byte read.** ADR 0130 R1 (`b6365060f0a`) read a ZInt of one byte in place at each
field, `#[inline(always)]`, a test of the flag, the store of the position and the value, and called the cold body
for a longer one. The cursor's and the reader's reads of every field of every codec held it: about 1.6 KB of the
image, for no instruction saved on the measured receive.

**W3: every decode held the reader's start.** `SceCursor::read_frame` is generic over the read it is handed and
inlined at each decode, and its start, `SceReader::new` over the bytes from where the cursor stands (the bounds of the
tail and the reader's four fields), was written out at each decode. ADR 0137 had moved the verdict out of line
(`end_frame`) and kept the start in line; the start is no more generic than the verdict.

`settle` (178) was examined and is not a cause of the same kind: its `Ok` arm is the value, its `Err` arm is a call
of the one out-of-line `fail` and the zero of the value's type; what is left at each site is the test of the
`Result`, whose layout is the value's type, and that zero, which no function generic over nothing can hold.

## Decision

### 1. A VLE is encoded by one function, out of line, for every sink

The encoding of a VLE is one function of the runtime, not generic and `#[inline(never)]`:

    pub(crate) fn encode_vle(value: u64, max_bits: u32, out: &mut [u8; VLE_MAX_BYTES]) -> &[u8];

It writes the VLE of `value` at `max_bits` into `out` and answers the bytes it occupies there, `vle_len` of them: each
byte but the last carries seven data bits with the continuation flag set, and the last what is left, a full eight in
a ZInt's ninth. It answers the bytes and not their count, so the slice is cut once, inside, under the bound the
function itself states, and no caller holds a bounds check of a length it was handed.

- The provided `SceSink::write_vle_inner` is a local array, a call of `encode_vle`, and one `write_bytes` of what it
  answers: a sink still sees one write of the exact length, and a sink of bounded room that refuses the write writes
  none of the VLE (ADR 0130 R5).
- `SliceSink` and `WindowSink` keep their VLE of one and two bytes stored in line (ADR 0061); a longer one is their
  `write_vle_long`, out of line, which is the same call of `encode_vle` and one `write_bytes` with its one capacity
  test. No sink has a copy of the loop.
- `fill_vle` is withdrawn: its loop is `encode_vle`'s. `vle_len` stays `#[inline(always)]` for the length a
  `CountSink` adds with a width the caller states as a constant (ADR 0061 section 4) and for the bounds a
  `BoundedWrite` states; the encoding reads it in one place.

### 2. A ZInt is read by one call of its reader's one read, which runs the one body

- The body of every ZInt, of every length and width, is one function, `codec::read_zint` (it was `zint_tail`):
  `(buf, pos, max_bits) -> (Result<u64, CodecError>, pos)`, not generic, `#[inline(never)]`, and no longer
  `#[cold]`, since every ZInt of every frame runs it. The cursor's and the frame reader's reads run it alike, so
  the grammar of a ZInt is written once (ADR 0130 R1).
- Each reader has one read of a ZInt, out of line: `SceCursor::read_vle_inner` stores the position the body answers
  and returns its `Result`; `SceReader::read_vle_inner` (which absorbs `read_vle_long`) consumes the ZInt or records
  its refusal through `fail`, and returns the value. A field of either reader, `read_vle_u16`, `read_vle_u32` or
  `read_vle_u64`, is a call of it and a cast.
- The one-byte read in place (`zint_byte`, `SceCursor::read_zint_byte`) is withdrawn. Measured: about +1.6 KB of
  image for +0 instructions on the receive.

The reader's read is a function of its own, and not the body called from each field, because the body answers a pair
through memory and the reader records the refusal: written at each field, that is ADR 0137's S3 again. The reader's
read answers a value in registers, and the field is the call.

### 3. Only the read of a frame is generic

`SceCursor::read_frame<T>` keeps generic the one thing that is: the call of the read it is handed.

    pub fn read_frame<T>(&mut self, read: impl FnOnce(&mut SceReader<'a>) -> T) -> Result<T, CodecError> {
        let mut reader = self.begin_frame();
        let value = read(&mut reader);
        self.end_frame(&reader).map(|()| value)
    }

`begin_frame`, the reader over the bytes from where the cursor stands, and `end_frame`, the verdict and the move of
the cursor, are each one function of the runtime, `#[inline(never)]` and generic over nothing a decode reads.
`SceReader::within`, the frame of the bytes a length says a body occupies, is bracketed the same way: `begin_within`
takes the bytes and starts the inner reader (failed, its input ended, where the outer one has failed), and
`end_within` makes the inner reader's refusal and capacity the outer one's.

`settle`, `hold` and their `_with` forms stay `#[inline]`: the `Ok` arm is the value, the `Err` arm a call of the
one cold `fail` or `refuse_held` and the zero of the type (section "Context").

## Consequences

- No byte written or read changes: every VLE is the encoding it was, every ZInt reads to the value, the refusal and
  the position it did, at every width, and every frame's verdict is ADR 0128's. The runtime's tests hold the encoding
  and each reader to a byte-by-byte reference at every boundary value, every length and every cut.
- ADR 0128's invariants hold: every read is total, a frame has one verdict, a read body has no `?` and no
  `return Err`, and no panic path is added. ADR 0136's chains keep their static dispatch. No indirect call is added.
- The generated code calls the same methods by the same names (`read_vle_u*`, `write_vle_u*`, `read_frame`,
  `within`), so no golden and no committed tree of generated code changes.
- No public declaration changes: `encode_vle` and `read_zint` are the crate's own; `SceSink`'s provided methods keep
  their signatures, so a consumer's sink compiles and writes as before.
- A ZInt read costs one call more than the read in place did for a ZInt of one byte, and the frame reader's two
  (its read and the body): a few instructions per ZInt field, against the code the in-place read cost at every field.

## Alternatives considered

- **Keep the one-byte read in place (ADR 0130 R1, ADR 0137 section 3).** Measured at about +1.6 KB for +0
  instructions. Refused.
- **Inline the body into each reader's read.** Two copies of the loop in a program that reads through both readers,
  and the grammar compiled twice. Refused: one body, as ADR 0130 decided.
- **Call the shared body from each field and record there.** The pair the body answers, matched and recorded at each
  field, is ADR 0137's S3. Refused: the field is a call of its reader's read.
- **Keep the bounded sinks' longer VLE encoded in place, with a loop of their own.** A copy of the loop for each sink
  type, which section 1 removes. Refused: the copy of at most ten bytes after the one encoding is paid only by a VLE of
  three bytes or more.
- **`encode_vle` answering a length.** Each caller would cut the slice, a bounds check at each sink. Refused.
- **The provided VLE as a loop of `write_u8`.** A test and a possible refusal at each byte, and a VLE half written
  where room runs out. Refused: a VLE is one write (ADR 0130 R5).
- **`settle` made non-generic, as a `Result<(), CodecError>` beside the value.** It would change the shape of every
  generated read for no byte saved: the test of the `Result` and the zero of the type remain at the site. Refused.
- **`read_frame` taking the read as a function pointer**, so one copy serves every codec. An indirect call per
  decode, which a stack bound cannot follow (ADR 0136). Refused, as ADR 0137 refused it.

## Measured

To be measured by the consumer at the landing, on the same image, by the same attribution. Expected direction:

- Section 1: `fill_vle` 1,486 and `vle_len` 334 fall to one `encode_vle` (the size of one loop and one length) and
  `vle_len`'s `CountSink` uses (about the 24 of the earlier pin); each sink's VLE write is a call and a `write_bytes`.
- Section 2: `reader.rs` `read_vle_inner` 826 and `codec.rs` `read_vle_inner` 779 fall to one small function each,
  `read_zint_byte` 87 to none, and one `read_zint` (the body ADR 0137 measured at 371 as `zint_tail`) stays; each
  ZInt field is a call. Receive instructions up by a few per ZInt field of the message.
- Section 3: `reader.rs` `new` 404 and `read_frame` 141 fall to one `begin_frame` and a call at each decode;
  `within`'s bracket likewise; `settle` unchanged.
- The runtime's bytes: back toward the 4,020 of `b616325c651`.

## Order of the changes

1. This ADR, with the amendment lines of ADRs 0130 and 0137.
2. The runtime: `read_zint` and each reader's one read; `encode_vle` and the sinks' VLE writes; `begin_frame`,
   `begin_within` and `end_within`.
3. The main session runs the runtime's tests, the Rust golden tests and conformance lane, and the codec clippy check;
   no golden is regenerated.
4. The consumer measures its image at the landing.

## Acceptance

Accepted on all of these: the runtime's unit tests pass, the encoding and every ZInt read held to the reference at
every boundary; the Rust goldens are unchanged and the conformance lane passes with every frame's bytes and verdict
as before; `scripts/check_clippy_codec.sh` passes; the nostd-mcu gate's worst-case stack finds no indirect call; the
consumer measures its runtime bytes at the landing.

## Amendment, 2026-11-26: a ZInt of one byte is the read's first branch

The consumer measured this ADR's landing (bedc527d480) against its baseline (b616325c651) with its corrected
harness: the codec area at +1,295 B (from +5,728 B), the whole image smaller, and the receive path of an 8-byte put
at 4,932.2 instructions a message against 4,855.9 (+76.3, +1.6 %), `codec::read_zint` taking 103 of them. Every
ZInt, a one-byte one included, made two calls: the reader's `read_vle_inner` and the shared `read_zint`.

Each reader's one out-of-line ZInt read now answers a byte below 0x80 in its first branch and calls `read_zint`
only for a longer ZInt. A one-byte field is one call, as the baseline's out-of-line cursor read was; the shared body
stays one copy, and no read site holds the test in place, so the size this ADR recovered is kept. The two readers'
reads each hold the same four-instruction test, which is the price of one call where there were two.

## Amendment, 2026-11-26 (second): a ZInt of two bytes is the read's second branch

Measured at 540f688ea2e with the same harness: the whole image still smaller (-571 B over the consumer's areas, the
codec area +1,353 B), the receive path of an 8-byte put at 4,887.5 instructions a message against the baseline's
4,856 (+31), with `codec::read_zint` at 65 of its own a message where the baseline's out-of-line cursor read took
57. The ZInts that still reached the shared body were the two-byte ones, a sequence number past 127 first among
them.

Each reader's ZInt read answers two bytes in its second branch as well, with no width test: fourteen bits are below
every width a field is read at (`vle_u16` the narrowest). Only a ZInt of three bytes or more calls `read_zint`. The
size cost is a few instructions in each of the two reads; no read site holds anything in place.
