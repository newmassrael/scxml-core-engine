# ADR 0128 — A frame's reads are total and its verdict is read once

- Status: Accepted (2026-11-25), under the owner's standing rule: the most long-term correct, textbook structure, cost
  ignored.
- Amends: ADR 0049 (a decoded Rust codec is a view over its wire bytes): a view is built once as a value, and a walk
  reads its entries with the scan's total read. ADR 0077 §7 (a value past a capacity): the capacity is noted in the
  frame's reader, not in a local of each decode.
- Related: ADR 0081 (a codec is planned once and each backend prints the plan), ADR 0099 (every runtime answers the
  same kinds of refusal), ADR 0065 §6 (what a decode refuses).
- Amended by: ADR 0137 (2026-11-26): `SceCursor::read_frame` answers the value the read returns, its verdict is one
  function of the runtime, a view's `decode` is the read of its frame (sections 2 and 4), and a refusal is recorded
  by one out-of-line call (section 1).
- Requested by: a downstream bare-metal consumer, 2026-11-25.

## Context

A Rust codec's decode read each field through a call that answers `Result`: `cursor.peek_array::<N>()?`,
`cursor.read_vle_u16()?`, `Body::decode_into(cursor, ..)?`, `.ok_or(CodecError::..)?`. Each `?` is a test and an
early return, and every nested decode returned its own `Result` to its parent, which tested it again. The consumer
profiled its image: the decode of one extension entry of a view (`CodecZenohExtEntryView::decode_into`) is 117
instructions a record, most of them the tests and the returns, and a walk of a `ListView`
(`backends/rust/forge-runtime/src/view.rs`, `ListIter::next`) decodes each entry again through the same fallible
decode, so a walk the scan already verified pays every test again. ADR 0049 chose to hold a list as its bytes and read
it again on each walk, which keeps a view's size independent of its lists; that choice stays.

The disassembly showed two writes more than the frame asks for. A variant arm of a decode was installed holding its
body's default (`_out.body = Variant(X::default())`) and then read into in place, so every byte of the body was written
twice; and an optional list was made present with `_out.extensions.insert(Default::default())`, which copies a constant
of the whole list (287 bytes in the consumer's lent `Push` and `Declare` decodes) for each record, over a list the
destination may already hold.

## Decision

### 1. A frame is read through one reader, every read total, the first refusal sticky

The forge runtime gains `SceReader<'a>` (`backends/rust/forge-runtime/src/reader.rs`, re-exported from `codec`): the
bytes a frame is read from, how far it has read, the first refusal recorded and whether a value was past a capacity.
Every read is total: `take_array::<N>()` returns `N` bytes, `take_slice(n)` `n`, `peek_u8()` a byte, `read_vle_u16()`
a number; where the frame cannot give them it returns the zero of the type (`N` zero bytes for an array, so an index
of a constant below `N` still has no bounds check) and records the refusal. The first refusal recorded is the frame's;
recording one ends the input, so every later read is a no-op that returns zero and consumes nothing.

This is the textbook sticky error of a decoder's state, as upb's parse state and Go's `bufio.Scanner` keep it. A
decode is one straight line of reads with no test after each, and it answers exactly what a decode that returns at its
first refusal answers: `Ok` for the same frames, the same first error for every other, and the same bytes consumed on
success.

A step that can refuse and is not a read (`Bounded::new`, an enum's `from_underlying`, a UTF-8 check, a narrowing of
a length) is settled into the reader (`settle`, `settle_with`): its value, or its type's zero with the refusal
recorded. A refusal a decode states itself (a count past its bound, a chain cut short, an arm the build leaves out) is
recorded (`fail`); the failure table's statement for Rust is that call (`CodecFailure::raise_stmt`).

### 2. The verdict is read once

A codec's `decode_into(cursor, out, ..)` is `cursor.read_frame(|cursor| Self::read_into(cursor, out, ..))`:
`SceCursor::read_frame` makes a reader where the cursor stands, runs the read, and answers the reader's verdict, with
the value the read returns (ADR 0137: a view's `decode` is `cursor.read_frame(|cursor| Self::read(cursor))`, and the
verdict is one function, `end_frame`, for every codec). The
body is `read_into` (`read` for a codec built as a value), which returns nothing. A codec that embeds or dispatches
to another reads it through its own reader (`Body::read_into(cursor, ..)`), so a frame has one verdict however deep it
nests, and no parent tests a child's answer.

The cursor moves past a frame read whole: on `Ok`, and on `CapacityExceeded`, whose frame is well formed and read to
its end. On any other refusal it does not move, which is the contract `decode` always stated (a caller that receives
`NeedMoreBytes` reads again from the same place once more bytes have arrived) and which a decode that returned
mid-frame did not keep.

### 3. Loops test the failure at their head; a capacity is noted, not refused

A loop whose count the frame says (a repeat's count, a chain's depth, a scan) leaves where the reader has failed
(`if cursor.failed() { break; }`), and one to the end of the frame reads `remaining()`, which is zero once the frame has
failed. A count read before a refusal therefore makes no long loop of reads that read nothing: the cost of a refused
frame is bounded by the code, not by what the frame says.

A value past what a storage profile holds is not a refusal of the frame (ADR 0077 §7). The reader notes it
(`note_capacity`, `hold`, `hold_with`) and the reads go on; its verdict is `CapacityExceeded` only where nothing was
refused. A list's fill answers its capacity to the list it fills (`Bounded::refill`, `decode_chain_in`), as before, and
the reader holds that answer. A group whose construction relies on what the gates the reads found (`Chain::new(..)`,
`Aligned::new(..)`) is built only on a frame read with no refusal and no capacity (`cursor.clean()`), as it was built
only with no capacity before: a total read gives such a construction no input its type does not describe. The
construction is still a `Result`, settled in the reader, and a branch the gates leave no frame to take (an arm that does
not hold what they read, a chain read again that lacks the kinds it was read with) records `PresentIfMismatch`
(`fail`), so no panic path is added and no `?` either.

### 4. A view is built once as a value, and its walk reads with the same total read

A view is a value of a few words (ADR 0049). Its `read` reads each field into a local and builds the view once, as a
struct literal of what its reads returned; a variant arm is `Arm(BodyView::read(cursor, ..))`, so no arm is made
holding a default body before its body is read and no byte of a view is written twice.

`ListWalk::read` replaces `ListWalk::decode`: an entry is read with the entry view's total read, which the generated
walk inlines (`#[inline(always)]`). The scan verified the entries, so a walk tests no refusal field by field; it tests
the reader once an entry, and ends where it failed (a span a caller built by hand need not be one a scan read), never
yielding the entry a refused read made and never panicking.

### 5. The eager form writes no whole default over what its destination holds

The eager form builds its lists in place (ADR 0049: a `heapless::Vec` moved copies its whole capacity), so it keeps
reading a body where it lives. It installs an arm holding its body's default only where the destination holds another
arm (`if !matches!(_out.body, Arm(_))`), and makes an optional list or body present only where it holds none
(`get_or_insert_with`); the fill then clears the list, and the read writes every field of the body. A destination a
caller reads frame after frame into, which `decode_into` exists for, is never overwritten by a default. Safe Rust
cannot make an enum hold an arm, or an `Option` hold a value, without writing the payload, and a read that built the
body as a value and moved it in would copy its lists' whole capacity; so the one write left is the payload of an arm
or an optional the destination did not hold, and a caller that reads into a fresh destination each time can take the
view, which writes nothing twice.

### 6. Scope

The Rust printer's decode of a layout codec, eager and view alike (ADR 0081 prints one plan), and the forge runtime's
reader, list walk and cursor. The cursor's fallible readers stay for hand-written readers of a stream, and `decode`,
`decode_into` and their signatures do not change. The CBOR and content-line codecs read self-describing items through
readers of their own and keep them. Other backends are unchanged. No unsafe code is added, and no panic path.

## Consequences

- A decode body has no `?` and no `return Err`; a nested body returns nothing. The per-field branch and early return,
  and the test of each nested answer, are gone.
- `decode_into` leaves the cursor where it was on a refusal other than `CapacityExceeded`.
- `codec::note_held` is removed: a storing decode holds a value through `SceReader::hold`.
- Every Rust golden under `tests/forge/expected/` and `tests/forge/expected/views/`, and every committed tree of
  generated Rust codecs, is generated again.

## Alternatives considered

- **Verify the frame, then read it unchecked** (the FlatBuffers verifier). A layout of fixed offsets can be verified
  once and read by index. A frame of ZInts, length-prefixed fields and chains has no extents until it is parsed: the
  verifier is a second full parse, and the reads after it still need bounds to stay free of panics. Refused.
- **Cache the entries a scan read.** A view would then grow with its lists, which ADR 0049 refused: its size depends
  on neither the bound nor the entry. Refused.
- **Keep the sticky state in `SceCursor`.** The cursor is the caller's place in its input, and resumable: a refused
  frame must leave it where it was. A per-frame reader whose state is dropped with the frame keeps the two apart, and
  leaves the cursor's fallible readers for hand-written code. Refused.
- **Read every arm and optional body as a value and move it in.** It writes each byte of a view once, which the view
  does; on the eager form it moves bodies that hold lists inline, copying their whole capacity, which ADR 0049
  measured and removed. Refused for the eager form.

## Amendment, 2026-11-27: a length reaches the reader in its field's own type

A body bounded by a length was read by `within(len as usize, ..)`: on a target whose `usize` is 32 bits a 64-bit
length was cut to its low bits, and a negative one of a signed field wrapped, so the frame read a different body
than the wire named. `SceReader::within` takes the length in its field's own integer type through `WireLength`
(the byte count it names, or none for a negative one), and the reader refuses a length the target cannot hold as
`LengthOutOfRange`. Where `usize` is 64 bits wide and the field unsigned, the test folds away; the generated read
is `cursor.within(len, ..)` with no conversion.
