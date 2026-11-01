# ADR 0077 — A declared bound is part of a field's type, and every decode refuses what breaks it

- Status: Accepted (2026-11-01). The owner ordered the work started on the recommendation of that day, under the rule
  of 2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored.
- Date: 2026-11-01
- Amends:
  - ADR 0065 §6 R1: the bound of a bounded byte string or text includes its `sce:max-size`, not only the range of its
    length field;
  - ADR 0058 §5 laws 1 and 3: the laws now hold on every backend's decode, not only on the Rust encode.
- Evidence: a downstream consumer reported the defect from its committed generated code, by reading and not by
  running it. Its timestamp codec declares `zid` as `sce:type="bytes" sce:bit-size="length-ref"
  sce:length-field="zid_len" sce:max-size="16"`, with `zid_len` a VLE. The Rust decode reads `zid_len`, slices that
  many bytes and returns `Ok`, so the frame `[time][zid_len=17][17 bytes]` decodes. Zenoh 1.10.2 refuses that frame
  (`commons/zenoh-codec/src/core/timestamp.rs`, `size > uhlc::ID::MAX_SIZE`), and zenoh-pico 1.10.2 took the same fix.
  The generated documentation of `try_into_owned_in` states that decode enforces the bound. It does not.
- Scope:
  - the codec model (`CodecField`), with one function that answers a field's bound;
  - every decode statement of a `tail` or `length-ref` field and of a `<sce:repeat>`, in
    `sce-build/src/forge/generator.rs`, `generator/rust_view.rs`, `generator/rust_source.rs` and the codec templates
    of six backends;
  - the codec runtimes of Rust, C11, C++, Kotlin, Go and Python, which get the errors of §4 where they lack them;
  - ADR 0065 §6 R1 in the Rust runtime and generator;
  - the codec parser, which requires the bound of every variable field (§5);
  - SCE_FORGE.md §4.6 and the goldens.
  Swift takes the same rule when its codec emitter reaches these fields; its arms are `no_swift_emitter` today.

## Context

A binary codec field of `sce:bit-size="tail"` or `"length-ref"` may declare `sce:max-size`, and a `<sce:repeat>` may
declare `sce:max-count`. What the declaration means is decided in a different place for each backend and each
direction. Each place decides by what that backend's storage needs, not by what the document says:

| Where | `max-size` (bytes, text) | `max-count` (list) |
|---|---|---|
| C11 decode | refused, as `SCE_FORGE_CODEC_NEED_MORE_BYTES` | refused, as `SCE_FORGE_CODEC_NEED_MORE_BYTES` |
| C11 encode | refused (`c11_over_bound_condition`) | refused |
| Rust decode, eager and view | not checked: the field is a borrowed slice | refused by the `heapless::Vec` push, `TooManyElements` |
| Rust encode | refused only where the length field is narrower than the value (`LengthOutOfRange`) | total by type, `BoundedIter` (ADR 0065 R2) |
| Rust owned projection | refused on the inline profile by the profile's capacity, accepted on the growable one | the same |
| C++, Kotlin, Go, Python decode | not checked | not checked: the list is a growable container |
| CBOR and content-line, every backend | refused at decode and encode (`CborOutOfRange`, `LineTooLong`) | refused (`LineTooMany`) |

The binary codecs are the exception. The CBOR and content-line codecs already treat the bound as part of the value
and refuse it at both ends on every backend. The binary codecs treat it as a storage capacity. A capacity only binds
where storage is fixed.

That reading was once written down as the contract. A test in `forge_conformance.rs` stated "`sce:max-size` is the
no-alloc inline capacity, NOT a wire ceiling" and decoded a 100-byte tail against a `max-size` of 32. That was the
answer to a consumer's report of 2026-06-15 that the growable owned form refused a payload the borrowed view had
accepted, and it settled the disagreement in favour of the view. ADR 0065 R2 (2026-10-31) then stated the opposite
for lists: "the bound is a property of the wire, not of where the entries are kept". The same consumer now reports
the view's acceptance as the defect. This ADR settles both in favour of the wire, and the test now pins the refusal.
The growable profile's containers still do not enforce `N`, which is right: the decode has refused a length past the
bound before any value is made.

The same reading gives three more defects in the same decode statements:

1. **The wrong error.** C11 refuses an over-bound length as `NEED_MORE_BYTES`. A streaming caller reads that status as
   "append bytes and try again". On a frame whose length is over its bound, that caller keeps buffering and never
   gets a verdict.
2. **The length is narrowed before it is compared.** The Rust decode computes `zid_len as usize`. On a 32-bit target
   a `u64` length of `2^32 + 5` becomes `5`, and a frame that is not a frame decodes.
3. **The length arithmetic wraps.** `sce:length-arith="-1"` on a length of zero computes `wrapping_sub(1)`, which is
   `usize::MAX`. That reaches `peek_slice` and is refused as `NeedMoreBytes`, with the defect of (1).

A document's language is one set of frames. A frame that C11 refuses and Python accepts means the six backends do not
implement one codec. Law 1 of ADR 0058 fails as well: the Rust encode of a view with a 17-byte `zid` writes a frame
that a C11 decoder refuses.

## The textbook structure

ADR 0065 states the foundation, and this ADR applies it to one more kind of constraint. A codec is a pair: a total
printer over a value type V and a partial parser that is its inverse (Rendel and Ostermann, *Invertible Syntax
Descriptions*, 2010). V carries the format's constraints as a refinement of the value type (EverParse, Ramananandro
et al. 2019; Narcissus, Delaware et al. 2019). The parser accepts exactly the frames of V, and the printer is total on
V (*parse, don't validate*, King 2019).

`sce:max-size="16"` is such a refinement: V's field is a byte string of length 0 to 16. A length of 17 is not a
value of V. It follows that:

- **The parser refuses it on every backend, before it slices or allocates.** A parser that accepts it returns a
  value outside its own type. The check also comes before any allocation, so an over-bound count cannot reserve
  memory: a `reserve(_n)` of a wire-chosen `_n` is a denial of service.
- **The printer cannot be handed it.** The value is refused where it is made, by a constructor that returns a
  witness. This is R1 of ADR 0065 §6, extended to the declared bound.
- **The bound belongs to the document, not to a backend's storage.** Storage is sized from the bound. A backend whose
  storage needs a finite bound refuses a document that states none. It does not invent a bound of its own.

## Decision

### 1. A field's bound is one function of the model

`CodecField::length_bound()` returns the lengths a field's value may have, `MIN..=MAX`:

- **`length-ref`:** the intersection of two ranges. The first is the range the length field carries after its
  `sce:length-arith`: for a length field of `w` bits and an offset `a`, `max(0, a)..=(2^w − 1 + a)`, with a VLE of
  `u<N>` read as `N` bits and a dotted-path flag as its width. The second is `0..=max-size` when `sce:max-size` is
  declared.
- **`tail`:** `0..=max-size` when declared.
- **`<sce:repeat count=>`:** the intersection of the count field's range and `0..=max-count` when declared.
- **`<sce:repeat>` to the end of the frame:** `0..=max-count` when declared.
- **`<sce:tlv-chain>`:** `0..=max-depth`, as today. `list_bound` becomes this function's list case.
- **Anything else:** `MAX` is unbounded. "Unbounded" means wider than any `usize`: a 64-bit length field with no
  `max-size`, or a `tail` with none.

Every place that today reads `resolve_bytes_max(field.max_size)` or `resolve_max_count(field.max_count)` for a codec
field reads this function instead. That covers C11 storage, the Rust inline profile's capacity, the largest encoded
size (`codec_size.rs`), the broadcast-log slot and the encode checks. `BYTES_DEFAULT_MAX` and
`REPEAT_DEFAULT_MAX_COUNT` stay for the procedure and schema fields they already serve (ADR 0005), and no codec field
reads them.

### 2. Every decode refuses a length outside its bound, in the length field's own type

Each decode statement of a `tail` field, a `length-ref` field (bytes or text, gated or not, positional or streaming,
eager or view) and a `<sce:repeat count=>` compares the length with `MIN..=MAX`. The comparison:

- **runs before** the length is narrowed to `usize`, before the slice is cut and before anything is allocated;
- **is done in the length field's type**, with `length-arith` applied by checked arithmetic. A result below zero is
  outside the bound, so it is refused as over-bound, not wrapped;
- **for a `tail`, compares the bytes remaining in the frame.** A frame with more than `MAX` remaining is refused. It
  is not truncated.

One generator function renders the guard for each language. Every decode path calls it, so no path can be added
without it. An unbounded `MAX` renders no comparison, and a bound that the field's type already implies renders none
either (a `u8` length with `max-size="255"`), so a codec pays only for a check that can fail.

A `<sce:repeat>` read until the end of the frame counts its entries and refuses the entry past `MAX` before
decoding it.

### 3. The encode cannot be handed a value outside the bound

- **Rust:** R1 of ADR 0065 §6, with the bound of §1. `Bytes<'a, MIN, MAX>` and `Text<'a, MIN, MAX>` hold a borrowed
  slice whose length lies in the range, checked by `new`. They are the item type of the source trait, the field type
  of the borrowed struct, of the view and of the parts. The decode builds them through the same `new`, so the error of
  §4 is the decode's error, by construction and not by a second check. An unbounded field keeps `&[u8]` and `&str`.
  The refusals this removes leave `codec_totality`, and every `sce:encode-may-fail` that becomes untrue is removed in
  the same commit (ADR 0065 §3).

  *Amended at step 2.* `Bytes` and `Text` are names for R2's `Bounded`, not types beside it:
  `Bytes<'a, MAX, MIN = 0>` is `Bounded<&'a [u8], MAX, MIN>` and `Text<'a, MAX, MIN = 0>` is
  `Bounded<&'a str, MAX, MIN>`, a text's length counted in the UTF-8 bytes the wire carries. A bound is one fact
  whether it counts entries or bytes, so it has one type, one constructor and one compile-time check of an array's
  length. The order of the bounds is `Bounded`'s, `MIN` last and none by default. `MIN` is 1 only under
  `sce:length-arith="+1"`, so a bounded field's default is empty, or one zero byte where `MIN` is 1.
  - The owned form holds `Bounded<S::Bytes<N>, MAX, MIN>` (and the text counterpart), as it holds a list since R2,
    so its projection to the source is total.
  - The decode keeps the guard of §2, which refuses before it slices and in the length's own type; the value is then
    made through `new`, the type's only door, and its `None` is the same `LengthOutOfRange`.
  - This step reaches the binary fields §1 bounds, `length-ref` and `tail`. The exact lengths and maximum sizes of the
    CBOR codecs, the rest of ADR 0065 R1, take the same types in a landing of their own. A content-line codec is not
    part of R1: a value it refuses for its size it also refuses for a control character, which no length type rules
    out, so a bound in its type would leave its refusal where it is.
  - On C++, Go, Kotlin and Python the refusal joins the checks an encode makes before its first byte, and the
    convenience facade of a codec that has one returns an optional or an error, as it does for a gate the encode
    cannot derive. C++ and Kotlin name no chain overflow, so they refuse a chain past its depth as the count past
    its bound it is (`TooManyElements`). The guard and the error each backend names are one table
    (`codec_failure.rs`, `encode_refusal_stmt`, `over_bound`), which C11's own check reads too.
- **The other five backends** do not have the source interface yet (ADR 0058 §6). Until they do, their encode
  refuses an over-bound field before the first byte, with the error of §4, as C11 already does. When a backend takes
  the source interface, the refusal moves into its bounded type, as in Rust.

The owned projection then has nothing left to refuse on account of a bound. The value it copies already lies within
the bound, and the inline profile's capacity is that bound. The documentation of `try_into_owned_in` is rewritten to
say this.

### 4. One error per fact, named the same on every backend

- **A byte or text length outside its bound** is `LengthOutOfRange`. It is the error the Rust encode already returns
  for a length its field cannot carry, which is the same fact seen from the encoder, and ADR 0058 law 3 requires the
  encode and the decode of one frame to give one error. It exists in Rust, Go, Python and C11 today. C++ and Kotlin
  get it.
- **A list count past `max-count`** is `TooManyElements`, as ADR 0058 §3 already names it. C11, C++, Kotlin, Go and
  Python get it.
- **C++** records the error on the cursor (`SceCursor::refuse`, read by `last_refusal`, ADR 0072) and returns
  `std::nullopt`. **Kotlin** records it where it records a CBOR or content-line refusal and returns `null`. C11 stops
  answering `NEED_MORE_BYTES` for a bound.

`NeedMoreBytes` keeps exactly one meaning: the frame is shorter than its own lengths say, so more bytes could still
complete it.

### 5. The document states the bound of every variable field

C11 storage, the Rust inline profile, a broadcast-log slot and the Rust eager list (`heapless::Vec` on every profile)
size their storage from a field's bound, and the decode of every backend refuses past it. Both need the bound the
document states. A length field's width alone is not used to size storage: a `u8` count would otherwise give a list
storage for 255 bodies that nobody asked for.

*Amended while landing step 1.* The first text refused an unbounded field only where a target that stores it was
generated, under a new code. That made one document a different set of frames per target, the defect of the Context
section in another form. The parser now refuses it for every target, as `validation/missing-attribute`: a `tail` or
`length-ref` field without `sce:max-size`, and a `<sce:repeat>` without `max-count`. This is the rule a content-line
codec already holds every string to ("the bound is the codec's to state"), and it needs no new code. `check` judges
it for every backend, as it judges `sce:encode-may-fail`. C11 no longer sizes such a byte field at 256, and Rust and
C11 no longer size such a list at 16: the defaults (`BYTES_DEFAULT_MAX`, `REPEAT_DEFAULT_MAX_COUNT`) are no longer
reached for a codec field. The decode guard of §2 still uses the whole of §1, because the width of a length field
bounds what a frame can say even where storage is not involved.

The broadcast-log refusal `broadcast-log/element-size-unbounded` keeps its code and reads §1. Every `tail` and
`length-ref` field in the shipped fixture documents declares `sce:max-size` (82 of 82), and every `<sce:repeat>`
declares `max-count`; so do every codec document of the consumer that reported the defect (its own count: 22
`length-ref`, 2 `tail`, 7 repeats). The fields without one are nine, in inline documents of four files
(`codec_size.rs`, `forge_conformance.rs`, `broadcast_log_kind.rs`, `a_codec_field_refusal_stands_at_its_field.rs`),
several of them tests of the unbounded case itself. Each either bounds its field or names the refusal in its
assertion.

### 6. Laws, and the tests that hold them

For every codec X with a bounded field f, on every backend:

1. **Acceptance at the bound.** A frame whose f has length `MAX` (or count `MAX`) decodes, and re-encodes to the same
   bytes.
2. **Refusal past it.** A frame whose f says `MAX + 1`, with that many bytes present, is refused with the error of §4
   and not with `NeedMoreBytes`. A frame whose f says `MAX + 1` with fewer bytes present is refused the same way: the
   bound is judged before the frame's length.
3. **No narrowing.** On a length field of 64 bits, a length of `2^32 + MAX'` is refused for every `MAX' ≤ MAX`.
4. **No wrapping.** With `length-arith="-1"`, a length field of zero is refused with the error of §4.
5. **One verdict.** The six backends give the same verdict, accepted or which error, on each frame of laws 1 to 4.

`forge-conformance` holds laws 1 to 5 with one boundary fixture set read by every backend, and the cross-backend
matrix holds law 5. A mutation case removes the guard of §2 from each path. One of the tests above must kill each
mutant: the positional and the streaming decode, gated and not, bytes and text, eager and view, and the repeat by count
and to the end of the frame.

## Alternatives

- **The view decode does not check, and the owned projection does.** This is the reading the consumer offered as the
  other possible contract. It keeps a value outside V in the type that the zero-copy path hands on, so the relay path
  (view re-encoded) writes frames that no conforming decoder accepts. Refused.
- **Check at encode, and declare `sce:encode-may-fail` on each codec with a bounded field.** This is ADR 0065's step-1
  method, applied here. It would add declarations that R1 removes in the next step. ADR 0065 kept that method
  because R1 to R6 each reach the tree in turns. Here a single step reaches every bounded byte and text field in
  Rust, so there is no interval in which a declaration says anything. Refused.
- **Keep `BYTES_DEFAULT_MAX` as the bound of an undeclared codec field everywhere.** This makes the six backends agree
  by giving every document a bound its author did not write, and a consumer with a larger payload would fail at
  decode with no build-time sign. Refused for codec fields. The default stays where ADR 0005 put it.

## Order of work

1. **§1, §2, §4 and §5 on all six backends, with the laws of §6.** The decode is fixed in one step, so the six agree
   from the commit that lands it. The goldens are regenerated once.
2. **§3: R1 in Rust with the bound of §1, and the encode refusal on the other five.** Rust's total encode and the
   decode's constructor land together. The declarations R1 makes untrue are removed in the same commit.
3. **The consumer is told:** each step's commit, and the regeneration that picks it up.

Steps 1 and 2 may land as one commit if step 2's diff is ready when step 1's is. They are separate only so that the
decode, which is the defect a consumer reported, does not wait on an API change.

## Consequences

- A frame that was accepted and lies outside its document's bound is now refused, on every backend. A consumer that
  relied on reading such a frame was relying on a defect.
- In Rust, a value with a bounded byte or text field is built through `Bytes::new` / `Text::new`, which can fail, or
  through `From<&[u8; N]>`, which is total when `N` is in range. A consumer that builds parts or a source from a
  slice adds one checked construction where the slice enters the value, and its encode call keeps or loses its `?` as
  the codec's declaration says.
- A codec field without a stated bound is refused by the parser, for every backend, with a diagnostic that names the
  field and the attribute. Before, it had a bound the author did not write: 256 bytes on C11 and 16 entries in Rust
  and C11. A consumer's documents that relied on those defaults state them.
- C11 callers that treated `NEED_MORE_BYTES` as "wait" stop waiting on a frame that can never decode.

## Questions left open

- Whether a `tail` field's bound should also bound the frame a parent hands it (an embedded body whose length a parent
  states). This is answered when §2 reaches the embed path, by reading the documents that embed a codec with a `tail`.
