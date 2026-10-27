# ADR 0061 — A codec's length and its window write are two readings of the one encode, and the length is the witness that the write cannot fail

- Status: Accepted (2026-10-28), on the owner's instruction of that day to design it under the rule of 2026-10-26
  and 2026-10-27 (the most long-term correct structure, textbook in its foundations, cost ignored) and to start. The
  consumer's measurements (§7) are not in; they gate the one optional step this ADR names (§6), not the structure.
- Date: 2026-10-28
- Scope: `backends/rust/forge-runtime/src/codec.rs` (the sinks, the VLE length functions, the measured witness);
  the Rust half of `sce-build/src/forge/generator/rust_source.rs` (the impl of the codec's `write` for the witness);
  SCE_FORGE.md §4.6; the golden trees under `tests/forge/expected/` and `backends/rust/mesh/src/generated/`, which
  change only by the added impl. Rust only: the other five backends take it in their idiom later, as ADR 0058 §6.
- Related: ADR 0058 (the source interface and the one encode; this ADR is the measurement its §3 waits for), ADR 0049
  (a list is walked, §3), ADR 0010 decision 2 (every backend writes the same bytes)
- Requested by: watching-zenoh, which writes the plain query reply by hand into its transmit slot because the
  generated encode cannot be handed a window of known size, and which folds a list's extension entries by hand.

## Context

ADR 0058 made the encode one skeleton, `write_source`, generic over the sink it writes to. Its `encode_source` runs
the skeleton twice: into a `CountSink` to learn whether the value is refused (and how long the frame is), then into
the caller's sink. A `SliceSink` checks its capacity on every write. A consumer that owns a transmit slot wants the
other order: learn the exact length first, reserve exactly that many bytes once, write into them. It cannot do so
today without the sink's per-write checks, and it has written the reply frame by hand, which restates wire knowledge
the generator owns. ADR 0058 §3 said the double run stays "until a consumer's measured stack or time says otherwise";
this ADR does not wait for the number to settle the *structure*, only for the step of §6.

The request has a second half: a single pass over a list's entries with early exit. That already exists (§5).

## The textbook structure

Three things are separate in the theory of syntax-directed translation and are kept separate here.

- **The grammar** of the frame, which is the codec document and, in Rust, `write_source`: the one statement of the
  wire rules.
- **The attributes** the grammar defines. The encoded size of a production is a *synthesized* attribute (computed
  from its children, Knuth 1968); the byte offset a field is written at is an *inherited* one. A frame whose total
  size is known before the first byte is written is evaluated left to right in one pass, the class of L-attributed
  grammars.
- **The interpretations** of the grammar. A function of the grammar is a fold, and a fold is chosen by the algebra it
  is given: here, by the sink. This is the tagless-final encoding (Carette, Kiselyov and Shan, *Finally Tagless,
  Partially Evaluated*, JFP 2009): the grammar is written once as a program generic in its interpreter, and
  "how long is it", "write it, checking room" and "write it into room already known" are three interpreters of the
  same program, not three programs. ADR 0058 chose the sink as the algebra for the first two; this ADR adds the third
  and makes the first one a first-class value.

The reason this is the whole design, and not one design among several, is the constraint ADR 0058 §3 stated and this
one keeps: **no second statement of a wire rule.** A hand-written `encoded_len` beside `write_source` is a second
grammar, and the two drift. Here the length *is* the encode run in the counting reading, so `len == bytes written`
(law 1) is not a test to hope for but a consequence of there being one skeleton.

## Decision

### 1. The length is a witness: `Measured`

For each codec `X` that has a source interface and takes no flag input from a parent, `X::measure(&src)` runs the
counting reading and returns `Result<Measured<'_, X, S>, CodecError>`.

- Every refusal ADR 0058 §3 lists (a list over its bound, a gate that disagrees with its presence, a number that does
  not fit its field, a source that disagrees with itself) is raised here, with no byte written anywhere. It is the
  one fallible step.
- `Measured::len()` is a field read: the exact number of bytes the frame is.
- A `Measured` exists only for a value that passed. It is the *parse, don't validate* shape (King, 2019; the typestate
  pattern): the type says "this source is admissible and this long", so the step that needs that fact does not check
  it again and has no failure to report.

`Measured` borrows the source, so a safe caller cannot change it between the two steps.

### 2. The window write: `Measured::write_into`

`Measured::write_into(self, window: &mut [u8])` writes the frame into a window of exactly `len()` bytes, returns
nothing, allocates nothing, and builds no value beyond the source the caller already holds.

- It is `write_source` run into the existing `SliceSink` over the window, which keeps one cursor, and afterwards the
  cursor is checked to stand at `len()`. It is not a second encode and adds no sink type.
- The precondition `window.len() == self.len()` is checked once, on entry; a violation panics, as
  `copy_from_slice` does with a wrong length. There is no `Result`, because there is nothing left to report.
- The write stays safe Rust. Each write is a slice copy whose bounds check cannot fail for an admitted source, and the
  check is the one cost this ADR does not remove (§6). The reason is soundness, below.

### 3. Purity of a source is a law, and a violation panics

`XSource` accessors are reads of a value. Rust cannot forbid interior mutability, so a source whose accessors answer
differently the second time is possible. Then the frame the second run writes can differ in length from the one the
first run counted. `write_into` therefore never trusts the count: a write past the window, or a window left unfilled,
is a panic stating that the source changed, never an out-of-bounds write. This is why the window write is checked, and
why a `Measured` does not offer an unchecked write in safe code. The law is added to the trait's documentation and to
the conformance suite (a source that lies is refused with a panic, not with memory unsafety).

### 4. The VLE length is a function, and the counting reading uses it

`sce_forge_runtime::codec::vle_len_u16/u32/u64(v) -> usize` are `const fn`, computed from the position of the highest
set bit (a leading-zeros count, a multiply and a shift, no loop), in the canonical Zenoh form of ADR 0058's runtime
(seven bits per byte, the ninth byte a full eight). `CountSink` overrides `write_vle_inner` to add that number, so the
counting reading of a VLE field is one add and not a loop of byte writes. This is the building block the consumer
asked for, and it is *used* by the length, not only offered beside it.

### 5. A list is folded by its iterator; no second API

`ListView::iter()` (ADR 0049 §3) is a lazy `Iterator`: each `next()` decodes one entry from the cursor and stops. A
`for` with `break`, or `try_fold`, is the single-pass fold with early exit: it visits the entries once in wire order
with their decoded headers and bodies, and it never reads past the entry it stopped at. The mandatory bit is a flag on
the entry header, and a codec does not refuse an unknown mandatory extension; the consumer decides, as it did.

A `fold` method that forwards to `try_fold` adds nothing a reader of the iterator does not have, so none is added. What
the conformance suite gains is the property that was only implied: **early stop decodes exactly the entries it
visited** (law 4). The one thing an iterator cannot offer is skipping the body of an entry whose header the consumer
has already refused to care about; that is §6.

### 6. What is deferred, and on what number

Two steps wait on the consumer's kit, because each trades a guarantee for a cost that has not been measured.

- **The per-write bounds check of `SliceSink` in `write_into`.** If the measured frame time or text shows it, the option is an
  `unsafe fn Measured::write_into_unchecked`, whose contract is the purity law of §3 and whose violation is memory
  unsafety. It is not offered before the number, because the safe sink is the answer unless a measurement says
  otherwise.
- **A header-only entry walk** that yields an entry's header and the span of its body without decoding the body. It
  would be a second reading of a list entry's grammar, so it is generated from the same entry skeleton, by the same
  rule as §1. It is added only if the consumer's one-`for` fold, against its hand fold, is slower for the body decode.

## Laws, and the tests that hold them

For every admissible source `s` of a codec `X` that has `measure`:

1. **Length is the write.** `measure(&s).len()` equals the bytes `write_into` fills, and equals the bytes
   `encode_source` writes into a `VecSink`.
2. **Window equals encode.** The window holds the bytes `encode_source` writes, over the 40,000-frame sweep of ADR 0058
   §5 and over the consumer's matrix (rid, key, payload length, encoding, put or delete, QoS present or absent).
3. **Refusal is at `measure`.** A source `encode_source` refuses is refused by `measure` with the same error, before the
   first byte of any window, and no `Measured` exists for it.
4. **A walk stops where it stops.** After a `break`, the iterator has decoded the visited entries and no more.
5. **A lying source panics.** A source whose second run differs in length from its first never writes out of its
   window.

The mutation round for the change kills: a length that omits a derived field, a `vle_len` off by one at the 7-bit
boundaries (127, 128, 16383, 16384 and the ninth byte), `write_into` that writes a field twice, and the entry-count
bound of law 4.

## Consequences

- The consumer computes the length, reserves that many bytes once, and writes. The Reply, Response and Err bodies and
  their extensions are encoded by the generated code; the hand-written writer and the test that guards it against the
  codec are deleted from the consumer.
- Generated code grows by one small impl per codec (the witness's link to the codec's `write_source`); the skeleton
  does not grow, so the byte-exact `.text` gate sees an added instantiation only where a consumer calls it.
- `encode_source` and its double run are unchanged. A consumer that wants the pre-pass folded away uses
  `measure` then `write_into`; the checked path stays for sinks of other kinds.
- The frame size of the encode is still a number to take on `thumbv7em-none-eabihf` at `opt-level = "s"`; this ADR
  states the gate (the encode's frame is no larger than `encode_source`'s) and does not claim the number.

## First measurement (2026-10-28) and what it changed

The consumer measured the plain Reply on a Cortex-M7 at 350 MHz (`opt-level = "s"`, fat LTO), from `recvfrom` to
`sendto`: its hand-written window writer 42.15 us; `XParts` with `encode_source` 48.10 us (+5.95); `XParts` with
`measure` and `write_into` 49.80 us (+7.65). Bytes equal in both generated arms over its matrix, and with a transmit
buffer of one byte too small `encode_source` leaves a prefix and fails while `measure` refuses before writing, which
the consumer needs. Layout noise of the kit is about 0.6 us. The consumer's reading was that neither the counting
pass nor the per-write checks is what costs, since the second arm has neither.

That reading rested on a premise that was false: `write_into` writes through `SliceSink`, whose every write was
checked, and every write was a call. The disassembly of the Reply path built for `thumbv7em-none-eabihf` at the same
settings shows where the cycles went:

- `SceSink::write_u8` is provided as `write_bytes(&[b])`, and `SliceSink::write_bytes` ended in a call to
  `compiler_builtins::mem::memcpy` (a general routine of some 660 bytes) whatever the length: a header byte, a
  flag byte and each byte of a VLE each cost a call, a capacity test and a `memcpy`. About twenty such writes a
  frame, in the pass that writes.
- `write_vle_inner` recomputed `ceil((max_bits - 1) / 7)` with a multiply-high on every call, and wrote one byte
  per iteration through the call above.
- A slice copy of a length the compiler knows is still a call at this optimisation level; an array store is not.

So the loss was in the sink and not in the structure: the double pass of `write_source` over a source is not shown
to cost anything yet. `SliceSink` now stores a byte or a fixed-width integer as an array store behind one capacity
test, writes a VLE as one capacity test and the bytes in place (a VLE that does not fit writes nothing), and calls
`memcpy` only in `write_bytes`, for text and payload. The bytes are those of the provided methods (a test holds that
at every bit boundary of the three widths). The change is in the runtime alone: no generated file moves.

Not measured: the effect of that on the kit. The consumer measures again; the order of the remaining candidates, if a
gap is left, is by what the disassembly shows each costs: `Measured::write_into` repeating the derivation the count
pass made (the consumer's suggestion, a `Measured` that keeps the derived values), then `extensions()` being asked
three times in one `write_source_in`, then the non-inlined call at each VLE. None is done before a number says it is
worth its cost, since each trades a smaller `Measured` and a plainer skeleton for speed.

## Considered alternatives

- **A hand-written `encoded_len` beside `write_source`.** Two statements of the grammar; refused (the same ground as
  ADR 0058's second measure fold).
- **A `skip_checks` mode of the skeleton, selected by the sink.** It would need every guard in the prologue to be
  skippable, but a narrowing (`u16::try_from(len)`) is the derivation and not a guard that can be dropped, and a
  mode that skips the others buys back a handful of compares for a second code path per codec. Rejected until a
  measurement asks for it.
- **A size tree cached by `measure`**, as protocol buffers' cached sizes do, to avoid re-deriving a nested body's
  length during the write. It needs storage proportional to the nesting, which the no-alloc tier does not have; the
  recomputation is `O(d·n)` for `d` the nesting depth, a constant of the grammar, as ADR 0058 §3 stated.
- **Writing the frame back to front**, as DER encoders and FlatBuffers do, so that a length follows its content and
  no measure is needed. It yields a different API (the caller does not know the slot's start) and cannot re-encode a
  stream whose consumer reserves a slot first.
- **An `unchecked` window write now.** Unsound for an impure source (§3), and not asked for by a measurement.

## Order of work

1. The runtime: `vle_len_*`, the counting override, the `Measured` witness and the trait that links a
   codec's `write_source` to it.
2. The generator: the impl of that trait for every Rust codec with a source and no flag input; `X::measure`. The
   goldens are regenerated once, by the step that lands it, and change only by the added impl.
3. The conformance cases of laws 1 to 5, with the consumer's matrix and a lying source, and the mutation round.
4. The consumer replaces its writer. Gate: its test of the hand-written writer against the codec is deleted, and the
   byte-for-byte test is the generated codec against itself.
5. The consumer's two measurements (text, stack frame and time of this against its writer; the one-`for` fold against
   its hand fold) decide the two steps of §6.
6. The interface on the other five backends, as ADR 0058 §6, held to byte parity.

## Questions left open

The two of §6, each with the number that settles it. Nothing else.
