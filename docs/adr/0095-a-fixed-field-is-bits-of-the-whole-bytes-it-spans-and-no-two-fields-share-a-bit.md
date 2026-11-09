# ADR 0095 — A fixed field is bits of the whole bytes it spans, and no two fields share a bit

- Status: Accepted (2026-11-09). Decided under the owner's standing rule of 2026-11-09: the most long-term correct
  structure, textbook in its foundations, cost ignored, decided without asking. Of the two repairs the defect below
  allows (lay a field that spans bytes out correctly, or refuse it), the rule takes the one that lifts the limit.
  Numbered 0095 because 0094 is taken on a branch.
- Date: 2026-11-09
- Amends:
  - SCE_FORGE.md §4.6, which gives the place of a field's bits only by example; it gains the rule of "Decision" 1
    and the refusals of "Decision" 3;
  - ADR 0081 §"Decision", whose plan writes a positional layout's fixed fields as `Write::Fixed` (one field at a byte
    offset) and `Write::Packed` (fields that share a byte offset). Both are replaced by the steps of "Decision" 2.
- Scope:
  - `sce-build/src/forge/bit_layout.rs` (new): where each fixed field's bits lie, the steps of a positional frame,
    and the refusals;
  - `sce-build/src/forge/encode_plan.rs`, the printers `generator/rust_source.rs` and `generator/cpp_source.rs`, the
    template encode and decode of `generator.rs` (`generate_encode_exprs`, `generate_decode_expr`) for C++, Kotlin,
    Go, Python and C11, and `generator/swift_codec.rs`;
  - `CodecModel::min_frame_bytes`, `codec_size.rs`, the layout drawing `diagram/kinds/codec.rs`;
  - the parser's codec checks (`parser.rs`);
  - the independent model `tests/forge/conformance/binary_model.py` and the byte cases of
    `tests/forge/conformance/numerical_reference.json`.
- Related: ADR 0010 decision 2 (byte parity between backends), ADR 0077 (a declared bound is part of a field's
  type), ADR 0081 (a codec's encode is planned once and each backend prints the plan).

## Context

**What was measured.** `tests/forge/resources/codec_optional_field_attributes.scxml` declares `reading`, a `uint16`
of `sce:bit-size="12"` at `sce:byte="1"`, `sce:bit-offset="4"`, big-endian: twelve bits that begin four bits into
byte 1 and so run into byte 2. Every backend got it wrong, each the same way:

- the decode read `(raw[1] >> 4) & 0xFFF`, one byte, so at most four of the twelve bits; C++ and C11 then narrowed
  the result to `uint8_t`;
- the encode wrote one byte, `(reading & 0xFFF) << 4` narrowed to a byte, and nothing for byte 2;
- `CodecModel::min_frame_bytes` and `codec_size.rs` counted the field as one byte;
- Swift and the layout drawing refused it ("bits past the byte").

The fixture had no byte case, so nothing ran it: it existed to carry `sce:bit-offset` and `sce:endian` to the IR, and
nobody read what the backends made of them.

**Why it was not one bug.** The positional encode grouped fixed fields by `sce:byte` alone. A group of one field was
written whole or as one byte; a group of several was one byte, the fields or'ed into it. Nothing asked which bits of
the frame a field held, so three other things were unchecked as well:

- two fields that hold the same bit were accepted, and the encode or'ed them together;
- a byte no field holds was not written, so the fields after it moved down a byte while the decode still read them
  at their declared offsets. `subscription_entry.scxml` (one `uint32` at byte 4) encoded four bytes and decoded from
  byte 4 of an eight-byte frame;
- a field of the streamed layout (written field by field, in turn) took `⌈bit-size / 8⌉` whole bytes and ignored its
  `sce:bit-offset`.

## Decision

### 1. Where a fixed field's bits lie

A field of a fixed `sce:bit-size` `N` at `sce:byte` `B` with `sce:bit-offset` `O` (0 when absent) and byte order `E`
(its `sce:endian`, else the codec's `sce:default-endian`) spans the `W = ⌈(O + N) / 8⌉` bytes `B .. B + W`. Those
bytes, read as one unsigned integer in order `E`, are the field's word, and the field's value is bits `O .. O + N` of
the word, counted from the least significant bit.

- With `W = 1` this is the rule every backend already follows for a field inside one byte (`codec_subbyte.scxml`).
- With `O = 0` and `N = 8W` it is the rule for a whole field of one or more bytes.
- `reading` (`B = 1`, `O = 4`, `N = 12`, big-endian) spans bytes 1 and 2. The value `0xABC` is the word `0xABC0`,
  which is the bytes `AB C0`.
- `native` byte order is read as big, as every backend already reads it.

### 2. The frame of a positional layout

Every bit of byte `p` of the frame belongs to at most one field (3 below). The encode writes the bytes from 0 to the
end of the last fixed field, in order:

- a field that is whole (`O = 0`, `N = 8W`) and shares none of its bytes with another field is written as one value of
  `W` bytes in its byte order, as before;
- every other byte is the or of the bits each field holds in it, and a byte no field holds is zero. The bits field `f`
  holds in byte `p` are a piece: `(value >> from) & mask`, moved `<< to` within the byte, where the piece is where
  the field's bits meet byte `p`'s bits of the word.

The decode reads each field from its word by the rule of 1, and ignores what a byte no field holds contains. The
minimum frame is the end of the last fixed field, and the frame's size counts the bytes no field holds.

The plan of ADR 0081 states these steps as `Write::Whole(field)` and `Write::Byte { at, pieces }`, and every printer
prints them. The template backends read the same steps from `bit_layout.rs`. No backend computes a field's bits
itself.

### 3. What is refused

The parser refuses each of these as `validation/attribute-rule-violated` on the field, with the rule in the message:

- a word wider than 64 bits (`O + N > 64`): no backend has an integer to read it into;
- a field of more bits than its type holds (`N` greater than the width of `uint8` .. `uint64`);
- a field that is not whole and whose type is not an unsigned integer or an enum: its value is an unsigned number of
  `N` bits, and a signed, floating or boolean type would need a sign or a meaning no rule gives it;
- in a positional layout, two fields that hold the same bit of the frame;
- in a streamed layout, a field that is not whole: such a layout writes each field in turn, in bytes of its own, so
  no other field can use the bits it leaves.

### 4. One statement of the layout

`sce-build/src/forge/bit_layout.rs` is the one place that turns `sce:byte`, `sce:bit-offset`, `sce:bit-size` and
`sce:endian` into bits. It gives a field's `BitSpan`, the `Piece` of it in a byte, the steps of a positional frame,
and the refusals of 3. The parser, the plan, every backend, the size of the frame and the drawing read it.

### 5. Held to an independent model

`tests/forge/conformance/binary_model.py` reads the rule of 1 from this page, not from the code, and writes byte
cases for `codec_optional_field_attributes` that every conformance lane runs.

## Consequences

- `codec_optional_field_attributes` encodes and decodes on every backend, and the C++ printer prints it.
- `subscription_entry`'s frame is eight bytes, the first four zero, where the encode wrote four. Its encode and its
  decode now agree.
- A document that declared two fields over one bit, or a partial field in a streamed layout, is refused where it was
  accepted. The tree had none.

## Considered alternatives

- **Refuse a field that leaves its byte.** It would match what Swift and the drawing did, and needs nothing in the
  other backends. It keeps a limit that a protocol of 12-bit numbers (CAN signals, DNS header fields) needs lifted,
  and leaves the gaps and the overlaps above unchecked.
- **Count bits from the most significant end** (as a diagram draws them). Every backend and both documents that use
  `sce:bit-offset` count from the least significant bit, and a change would move the bytes of `codec_subbyte`.
- **Leave a byte no field holds out of the frame.** That is what the encode did, and it is why it disagreed with its
  own decode.

## Order of work

One landing: the rule moves every backend at once, since a frame one backend writes by the old rule is one another
reads by the new. The byte cases hold all of them together.
