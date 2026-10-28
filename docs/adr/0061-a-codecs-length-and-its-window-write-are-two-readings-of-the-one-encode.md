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

The consumer measured again against that fix: `encode_source` 47.65 us (-0.45), `measure` and `write_into` 47.83 us
(-1.97), the hand-written writer 42.00 us. The fix helped, less than the disassembly alone suggested, and the two
generated arms are level within the kit's noise at about 5.7 us over the hand writer (about 2,000 cycles). So the
hypothesis that the sink was the main cost was incomplete, and `measure` plus `write_into` buys nothing over the
double run here: both walk the skeleton twice.

To find the rest without a second guess, the Reply path was run under an emulated Cortex-M7 (QEMU `mps2-an500`,
one translated instruction per step) and the executed instructions were counted by function, on the consumer's own
generated codec at the pinned revision and its plain Reply (a 57-byte frame: local key with a suffix, Put with an
encoding and a 32-byte payload). An executed-instruction count is a proxy: it has no pipeline, no flash wait states
and no cache, and the kit's time is not a linear function of it. It does rank where work is done, and it is exact and
repeatable, which the kit's layout noise is not.

| step | instructions per arm | what moved |
|---|---|---|
| after the sink's copy-call fix | 1,362 | `memcpy` 319; counting pass about 520, of which the VLE count 315; `SliceSink` VLE 158 |
| a VLE of one byte stored or counted in line, the width as a constant | 986 (-28%) | the VLE count 315 -> 0 in the pass that only counts; `write_vle_long` 62 remains |

`vle_len` is now always inlined with a one-byte fast path, `CountSink` states each width's cap as a constant, and
`SliceSink` stores a one-byte VLE in line and calls out for a longer one. Linked text for the probe grew 504 bytes for
it (5.7 to 6.3 KB, the whole probe image), which is the price of inlining at this optimisation level.

What the count shows is left, for 986 instructions: `memcpy` 319 (a third; the copy of the 16-byte key and 32-byte
payload, which the hand-written writer also makes), and about 650 across the two passes of the skeleton, the pass that
counts being about 200 of them. The consumer's two arms are level because each runs the skeleton twice; the
candidates left, in the order the count ranks them, are: the counting pass being cheaper than a run of the skeleton
(the count of a frame is arithmetic over the same derived numbers), `Measured` keeping the derived values so
`write_into` does not derive them again, and `extensions()` being asked three times in one `write_source_in`. They are
not done yet, since each makes `Measured` larger or the skeleton less plain, and the kit has not yet said what the
step above bought.

## Where the rest went, a staged design that did not pay, and a one-pass window (2026-10-28)

The consumer's third measurement (`ef50d044ad`): hand-written writer 41.38 us, `encode_source` 46.36, `measure` with
`write_into` 47.28, so +4.98 and +5.90 us over the hand writer, about 1,740 and 2,070 cycles. The instruction trace
of the previous section counts 986 executed instructions for the generated path against 442 for a hand-staged
writer of the same 57 bytes (byte-identical, held by the trace harness), of which 319 are the `memcpy` both pay.
544 instructions cost 2,000 cycles on the kit, about 3.7 cycles each, which a Cortex-M7 running generated code that
touches some twenty small functions in flash can explain by instruction-cache misses and wait states and a trace
cannot show. So the trace ranks, and the kit decides.

**A staged design, built and measured, and not kept.** The textbook answer to "the second pass re-derives what the
first found" is to keep what was found: each codec gets `plan` (evaluate the source once, refuse what it cannot
carry, count the bytes arithmetically from the same decision that generates each write) and `emit` (write from the
plan), `Measured` holds the plan, and `write_source` is the two in sequence. It was built through the generator for
every Rust codec (one `Prog` of paired statements so that the count and the write cannot drift, plans filled in
place so that nested plans are not moved, `Default` plans), passed the whole conformance matrix, and was measured on
the consumer's own generated codec: 1,833 instructions for `encode_source` and 2,442 for `measure` with `write_into`,
against 986. A plan is as large as its deepest arm of every variant it nests (some 200 bytes for a Response), so
returning it, zeroing it and moving it into `Measured` costs more than the derivation it saves: the derivation was
about 200 instructions a pass, and `memclr` and `memcpy` of the plan about 650. This is the finding of ADR 0049 again,
a record that carries its capacity is paid for at every move. The work is kept in the repository's stash for whoever
wants to try a plan that is small by construction (a flat tape of the derived numbers in the order they are read,
with the plans of list entries recomputed); it is not part of this change, and no interface here depends on it.

**What was kept, each measured on the same trace:**

| change | `encode_source` | one pass |
|---|---|---|
| after the one-byte VLE in line (`ef50d044ad`) | 986 | 764 |
| a VLE of two bytes stored in line | 936 | 714 |

A word-wise copy for short text and payload was tried and dropped: LLVM turns the loop back into the same `memcpy`
call, and the copy is a cost the hand writer pays equally, so it is not part of the gap. Forcing the whole generated
tree to inline took 986 to 900 instructions for 8.8 KB more text, which does not justify an option on the evidence
of the trace alone; whether it helps the kit, where the number of functions touched may matter more than the number
of instructions, is for the consumer to measure by adding `#[inline(always)]` to the `write_source` functions of its
generated codecs, before the generator grows a switch for it.

**Three smaller ideas, measured and not kept**, each on the same trace of the consumer's Reply:
evaluating each gate once at the head of the prologue (936 to 971, worse: the values stay live across the writes and
are spilled), an inline hint on the thin `write_source` wrapper (no change), and a word-wise copy (above). The
`Result` checks after each write are about a tenth of what is left; they are the contract of a fallible sink and are
not removed.

**`generate --inline-encode`.** The forced-inline experiment is now an option, `ForgeCompileOptions::inline_encode`,
that marks the function which writes each codec `#[inline(always)]` and changes nothing else (a test holds that the
two outputs differ in no other line, and that the marked set compiles under the same lints). Generated for the
consumer's Reply it reproduces the hand-edited experiment: `encode_source` 936 to 876 instructions and the one-pass
window 714 to 693, byte for byte the same frame, for about 5 KB more text in the measured image. That is a modest
saving in instructions; the reason to have the option is that the 544 instructions by which the generated path
exceeds the hand-written one cost the consumer's kit about 2,000 cycles (3.7 each), more than instructions alone
explain, and a frame written by one function instead of nine may close part of that. Whether it does is not known: it
is off by default and meant to be measured on the target before it is kept, and unlike `--views` it is a property of
one document.

**A one-pass window.** The count is a pass of its own, 222 of the 936 instructions. A caller that can reserve more
than the frame needs and give back what it did not use (a transmit slot of the largest frame, committed at the
length written) does not need it. `write_window::<C, S>(src, &mut [u8]) -> Result<usize, CodecError>` and the
generated `X::write_window(src, window)` write the frame in one pass into such a window and return its length. The
price is the contract: on an error the window holds a prefix, which a slot that is committed only on success
discards. A caller that must leave its buffer untouched, or reserve exactly the frame, keeps `measure` and
`write_into`. The conformance matrix holds, for every codec, the same bytes and count as `encode_source`, the same
refusal, and an overflow in a window a byte short.

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
