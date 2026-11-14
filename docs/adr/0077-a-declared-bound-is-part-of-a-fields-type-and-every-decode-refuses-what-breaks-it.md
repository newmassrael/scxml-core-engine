# ADR 0077 — A declared bound is part of a field's type, and every decode refuses what breaks it

- Status: Accepted (2026-11-01). The owner ordered the work started on the recommendation of that day, under the rule
  of 2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored.
  Amended by the owner on 2026-11-02 (§7): the wire bound and the storage capacity are two facts. Steps 1 to 4 of §7
  have landed on every backend that stores, a heap node's policy limit with them (2026-11-03); step 5 is the
  consumer's.
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

### 7. Amendment of 2026-11-02: the bound is the protocol's, the capacity is the deployment's

The text above gives `sce:max-size` and `max-count` two jobs: §1 and §2 make them part of the language the decode
accepts, and §3 and §5 size storage from them ("the inline profile's capacity is that bound"). They are two facts. The
consumer that reported the defect declared `sce:max-size` as its no-alloc inline capacity, which was the contract at
its pin, and this ADR turned each into a wire ceiling: its decode refuses every Put over 256 bytes on every profile,
heap and router included, where the protocol has no such limit. That is a conformance regression this ADR introduced.

A codec is a correspondence between a language and a set of values. The language is the protocol's, and it does not
depend on the build. What a node can hold is an implementation limit, which the C standard keeps apart from the
language it limits and HTTP keeps apart from a malformed request (413, not 400). This codebase already has the shape on
the other side: a total encode has only the sink's error (ADR 0065 §4), and a capacity is that error's dual on the
decode. So:

- **The bound** is `sce:max-size`, `max-count` and `max-depth`, as §1 reads them: the protocol's, in the document, the
  same on every profile, and absent where the protocol states none. It stays what §2 guards, what
  `Bounded<L, MAX, MIN>` states, and what the encode holds. Past it a frame is malformed (`LengthOutOfRange`,
  `TooManyElements`).
- **The capacity** is what a storing form holds: inline owned storage, a list decoded eagerly, a C11 array, a
  broadcast-log slot. It is the deployment's, so it is not in the codec document. It is read from the deploy input:

  ```yaml
  codecs:
    msg_put:
      capacity:
        payload: 256
        slices: 4
  ```

- **A form that stores nothing needs no capacity.** A borrowed decode and a view hold the receive buffer's bytes, so
  they accept up to the bound, on an MCU too.
- **Past its capacity a storing form refuses with its own error,** `CodecError::CapacityExceeded`. The frame is in the
  language and this node cannot hold it. Today `From<CapacityExceeded>` answers `TooManyElements`, which says the
  frame is malformed; it stops.
- **No capacity stated means the bound,** so a document whose bound is the protocol's changes nothing, and a field
  with neither has no inline storage: its owned form takes growable profiles only, and C11, which has no other, refuses
  it at generation, naming the field. A capacity above the bound, or naming a field that stores nothing, is refused.
- **A heap node's policy limit** is a capacity too, and is spelled and refused the same way.
- **§5's amendment of step 1 is reversed.** A `tail`, `length-ref` or repeat with no `sce:max-size` or `max-count` is
  legal again, meaning the protocol states none. That amendment refused it because "one document [was] a different set
  of frames per target". This keeps its point: the language is one set on every target, the verdict of a frame's
  validity does not vary, and what differs between nodes is which valid frames each can store, reported as that. Which
  frames a node can process does differ, as it does for any node with finite memory.
- **Eager lists take the storage profile.** Today an eager list is `heapless::Vec` on every profile, sized by §5. A list
  decoded eagerly is a storing decision, so it takes its profile's list, and a heap profile gets a growable one. How it
  is spelled is the last amendment of this section: one storing value type, with no number in the tree.
- The original defect stays fixed: a value past what its storage holds is refused, never truncated or overrun.

§6's laws stand for the bound. A new law joins them: **a frame inside the bound and past a form's capacity is refused
by that form with `CapacityExceeded`, and accepted by every form that stores nothing, with the same verdict on every
backend that stores.** A storing decode equals store after parse, so `CapacityExceeded` means the frame is in the
language: Rust stops at the field and then judges the frame by its view, which costs nothing unless it refuses, and
a deployment that states a capacity generates the views. C11 has no view and stops at the field until it reads past
what it cannot store. *Superseded for Rust by the amendment of 2026-11-05 at the end of this section: Rust reads past
what it cannot store as C11 does, and judges the frame by nothing else.*

*Decided by the owner on 2026-11-02, on review of this section: what a bound guarded stays guarded where the bound is
absent.* §2's check was also the only thing standing between a frame and two failures that have nothing to do with the
language, and §7's own claim about `CapacityExceeded` needs a law to be true. Three laws join it:

- **Memory answers to the frame.** No decode reserves memory from a number the frame only states. An entry takes a byte
  at least, so a list reserves at most the bytes that remain (`entries_backed` on the C++ and Go cursors) and grows past
  that as entries are read. A frame that says a count of `2^64 − 1` and holds none costs what its own bytes cost.
- **A walk moves.** A list with no bound whose entry can take no bytes has no frame that ends it. It is refused at
  generation, naming the field. A chain's entry reads its header, so a chain always moves.
- **A storing decode is a store after a parse.** Past its capacity it stops storing and keeps parsing, and reports
  `CapacityExceeded` only for a frame that parsed. A malformed frame gets the error a view gives it, so
  `CapacityExceeded` always means what this section says it means. A differential test of each storing form against
  the view holds this.

§3's "an unbounded field keeps `&[u8]`" is superseded by one shape: a byte string or text with no bound is
`Bounded<_, { usize::MAX }>`, as a list with no bound is, and a slice converts to it with no check.

This lands in steps, each its own commit and guide section:

1. The split with no change of behaviour: `CodecField` answers what a storing form holds apart from its bound, and
   every storing reader moves to it. The regeneration changes nothing, which is the proof.
2. The deploy input and `CapacityExceeded` on the two backends that store, Rust and C11; the containers of the others
   grow, so they have no such refusal. Then `on-overflow` leaving the document, as a landing of its own.
3. One storing value type with capacity by field: the owned forms move onto field tags, then the eager form joins them.
4. The parser admitting a field with no bound, after the laws of memory and of the walk and after step 3. Until the
   former are on main a bound is the only thing that holds them, and until step 3 a field with neither a bound nor a
   capacity has no eager storage to be read into.
5. The consumer moves each field by what the protocol says.

*Decided by the owner on 2026-11-02: a chain's `max-depth` and a list's `max-count` are the protocol's or they are
nothing.* The upstream codec of the consumer's protocol reads an extension chain with `while has_ext` and no limit,
and a locator list with a `usize` length, so a frame with 9 extensions is one a conforming peer may send. A depth of 8
in the document would call it corrupt, the defect above in another field. A node that refuses past 8 is making a
choice about itself, which is a capacity. There is no third category: a policy limit is reported as this node's
refusal, not as a malformed frame. So:

- `max-depth` and `max-count` become optional, stated only where the protocol bounds the list. A walk stores nothing
  and reads at most its frame, so an unbounded chain costs a view no more than a bounded one.
- A list with no bound is `MAX = usize::MAX`, so `Bounded`, `Chain` and `Aligned` hold it with no second shape.
- `on-overflow` leaves the document with the capacity: what a node does past what it holds is the node's. It defaults
  to refusing.
- `TlvChainOverflow` goes. *Amended while landing:* the first text called it `CapacityExceeded`, which holds for a
  depth that was a node's limit and not for one the document states. A chain past a stated `max-depth` is a frame past
  its bound, and a view, which stores nothing, refuses it as well, so it is the count past its bound it is:
  `TooManyElements` on every backend, recorded on the cursor on C++ and Kotlin. A chain past a deployment's capacity is
  `CapacityExceeded`, as a list's is.
- `truncate` is removed. It drops entries silently, so a re-encode differs from its frame, and dropping a mandatory
  extension breaks the protocol. No document in the tree uses it; one test does.
- The width of a length field stays a bound of the language by §1. Upstream bounds a locator and an encoding schema to
  `u8` and a key suffix to `u16`, so those are 255, 255 and 65535 and not unbounded, whatever a capacity says.

*Decided by the owner on 2026-11-02, on what the consumer measured: one storing value type, and no capacity in the
tree.* The consumer's MCU crates and heap crates share a workspace, so Cargo's feature unification puts both in one
build, and one generated tree serves both. A tree per deployment cannot exist there. A capacity written into the tree as
a number, `S::List<T, CAP>`, states one deployment for the whole build. The consumer also decodes eagerly on its heap
paths and then projects into owned storage, about 38 decodes and 59 projections, so the eager form and the owned form
are one value held two ways. So:

- **One storing value type.** `{Codec}In<S>` replaces the eager `{Codec}<'a>` and `{Codec}Owned<S>`. Both names stay as
  aliases, `{Codec}<'a> = {Codec}In<Lent<'a>>` and `{Codec}Owned<S> = {Codec}In<S>`, so `{Codec}::decode` needs no
  annotation. A decode, a `Source` impl and a move between profiles are each written once, generic in `S`.
- **`Lent<'a, D = Inline>`** is the eager form's profile. Bytes and text borrow the frame, and lists are held as the
  profile `D` holds them. A borrowed field needs no capacity, as §7 says of a form that stores nothing.
- **Capacity is by field and outside the tree.** The tree generates a tag type for each variable field, and a profile
  answers `HoldsList<Tag>`, `HoldsBytes<Tag>` or `HoldsStr<Tag>` with its container. `Heap` answers for every tag.
  `Inline` is generated in the tree from the document's bound, for a field that has one, so `Inline` on a field with no
  bound does not compile: that is "takes growable profiles only". A deployment is a type, generated into its own file
  from the deploy input, and several coexist in one build. `CodecStorage` goes, since its `List<T, const N>` cannot be
  reached from a tag without `generic_const_exprs`, and `OriginStorage` loses its `const N`.
- **Each codec has a storage trait,** whose supertraits are its fields' `Holds*` and its children's storage traits. A
  struct's `where` clause is not implied at a use site and a supertrait is, so every site says `S: {Codec}Storage`.
- **The view stays its own type.** It was proposed as a profile of the one type, and that does not hold: a field that is
  an associated type of `S` makes the type invariant in `S`, so `{Codec}In<Lent<'long>>` does not coerce to
  `{Codec}In<Lent<'short>>` (measured on a prototype). ADR 0049 made the view covariant on purpose, since lending a view
  to a shorter lifetime is what one is for. `Lent` pays this and the view does not. A function that takes a lent value
  takes it at its own lifetime, which is how the generated code already spells it.
- **A storing decode does not go through the view.** Materializing from a view would walk every list twice on the hot
  path. It reads once, from the decode body it shares with the view, and only the path past a capacity reads the frame
  a second time, as store after parse already says. So the view decides a frame's validity and is not what storing is
  built on: the law stands that a form that stores refuses with `CapacityExceeded` only a frame the view accepts.
  *Since the amendment of 2026-11-05 the path past a capacity reads nothing a second time either; the law stands,
  held by the same differential test.*
- **The judgement stands on every codec that takes a profile, in every tree.** A capacity below a bound is not a fact a
  tree can know once `S` is a parameter: a program may name such a profile with no deployment. So the path past a
  capacity is generated wherever a profile is taken. It judges by the view where the tree has views, since a view holds
  no entry, and by the codec's lent form where it has none, since `Lent` holds a list at its bound and cannot run out.
  A body with no view of its own judges by its lent form too. The lent form is the costlier judge, a whole value on the
  stack, which is why the view is taken where it exists and why a deployment's tree has views. *Superseded by the
  amendment of 2026-11-05: no Rust decode judges a frame a second time, by a view or by any other form.*
- **A CBOR codec keeps its two types.** Its `{Codec}<'a>` already is a view: it stores nothing, holds no list, and is
  covariant, and its decode cannot run out. `{Codec}Owned<S>` is what stores, and it is built from that value after the
  whole map has parsed, so store after parse holds by construction. Moving it onto `{Codec}In<S>` would give up the
  covariance and gain nothing. One storing type means one here too.
- **A deployment's file, as landed.** `sce-codegen deployment` and `compile_rust_deployment` take the documents a tree
  is generated from and a deploy input, and write one file: the type, `Copying`, and for each field of each codec the
  set holds or imports, the `Holds*` impl at its capacity or its bound. Its list of fields is the one the tree declares
  tags from, one function for both, so the two cannot name different fields. The file goes beside the tree's modules
  and names them as they name one another, `super::<codec>`. A Rust tree checks a capacity and takes none
  (`Language::tree_holds_capacities`), so its bytes do not depend on the deploy input. C11's tree still takes them.
- **Views are the build's choice, `--views`.** The text above has a deployment that states a capacity generate the
  views. With one tree for every deployment that is the deploy input reaching into the tree, so it stops: a test that
  compares a tree generated with and without a deploy input found it. A tree without views judges a frame past a
  capacity by its lent form, which is correct and costs a whole value on the stack, so a build with a deployment
  generates its tree with `--views`. *Since the amendment of 2026-11-05 a decode past a capacity asks nothing of a
  view, and views are a build's choice for what they are, a list held as a span.*
- **A module's `cfg` is the build's, and the caller says it.** A consumer may compile a codec's module under a feature
  of its own, and a file that names a module the build leaves out does not compile. Neither the documents nor the deploy
  input know a build's layout, so the gate is an input of the file's generation, `--gate <codec>=<predicate>`, and a gate
  for a codec the set does not hold is refused.
- **Step 4 for lists, as landed.** `max-count` and `max-depth` are optional, and nothing stands in for an absent one:
  `CodecField::storage_entries` and `list_bound` answer `None`, where a default of 16 entries used to answer. A walk
  with no bound loops until the chain or the frame ends, with no refusal past a depth and no reservation. What cannot
  hold such a list says so by name: `Inline` has no impl for its tag, a deployment that gives it no capacity is
  refused, C11 is refused without a capacity, and a Rust tree without views is refused, since its decode past a
  capacity would judge the frame by the lent form, which holds lists as `Inline` does. *The last refusal is lifted by
  the amendment of 2026-11-05: with no judge, a tree without views holds such a list as any profile it names does.* A gated C11 chain wrote past its
  array when a deployment's capacity was below the depth; it now refuses, as the ungated chain did.
- **Step 4 for positional byte strings and text, as landed.** A `tail` or `length-ref` takes no `sce:max-size`, and
  its bound is `CodecField::bound`: what the document states met with what the field that carries the length can say,
  as §1 and the upstream widths above require, so a `u8` length with none is 255 and a `tail` with none is unbounded.
  Lists read the same seam, so a list counted by a `u8` is 255 entries where the first landing for lists called it
  unbounded. A bound past `u32::MAX` is held as none, since no type or array states it on every target. Nothing stands
  in for an absent bound: `storage_bytes` answers `None` where a default of 256 bytes used to answer. One shape holds
  every byte string, `Bytes<'a, { usize::MAX }>` where the language states no bound, and the bare `&[u8]` and `&str`
  of §3 are gone. The lent form borrows every byte string and text from the frame, so it asks the profile `D` only for
  its lists: `{Codec}LentStorage` names those, and the lent methods bound `D` by it, so the lent form at `Inline` holds
  a codec whose byte string has no bound and a Rust tree without views takes one. `Inline` has no impl for such a tag, a
  deployment with no capacity for it is refused, and C11 is refused without a capacity, naming the field.
- **An embedded body is counted at its own largest frame.** The default of 256 bytes stood in for every
  `<sce:embed>` in a codec's largest frame, whatever the body held, so `MAX_ENCODED_BYTES` and C11's `_MAX_BYTES` were
  a guess: 2710 for a codec whose largest frame is 3296, a buffer sized from it too small for a frame of the language.
  The body's size is the imported codec's, added as a list's entries and a variant's arms are.
- **A CBOR map's text and byte string with no bound, as landed.** A map has an owned mirror wherever it holds a text
  or a byte string, bounded or not, each held in what the profile answers for its tag; a string with no bound is
  `Text<'a, { usize::MAX }>`, and the bare `&str` and `&[u8]` are gone here too. An entry takes a capacity from the
  deployment, as a positional field does. C11 refuses a past-capacity string apart from a past-bound one:
  `sce_forge_cbor_read_text` and `sce_forge_cbor_read_bytes` take the bound, `SIZE_MAX` where the protocol states
  none, and the capacity, and answer `CBOR_OUT_OF_RANGE` past the first and `CAPACITY_EXCEEDED` past the second.
- **A codec `Inline` does not hold has no default profile.** On the heap-free tier the default profile is `Inline`,
  and a signature or a struct's default argument that names `{Codec}Owned` at it is checked there whether or not it
  is called: the first landing for lists left every codec with a list of no bound failing to compile with no
  allocator, through `try_into_owned()`, and no test built one there. `CodecModel::inline_holds_with` answers whether
  `Inline` holds a codec and every codec it holds, read through the imports as whether a codec borrows is; where it
  does not, `try_into_owned()` is not written and `{Codec}Owned<S>` has no default argument, so the codec is named at
  a profile on every target. A bound written as `where DefaultStorage: {Codec}From<'a>` does not do it: a predicate
  naming no type parameter is checked where it is written. A no-allocator build of every kind of field with no bound
  holds this.
- **Content-line text and lists with no bound, decided as follows and landed in the steps below.**
  - *A repeated property says so apart from its bound.* ADR 0014 made `sce:max-count` both the mark of a repeated
    property and its bound, which was sound while every list had one; a property the protocol repeats without a bound
    (an iCalendar `ATTENDEE`) then has no spelling. `sce:repeated="true"` marks it, and `sce:max-count` is taken only
    beside it, as `sce:max-values` is taken only beside `sce:separator`. `sce:max-values` and `sce:max-size` become
    optional. A document that writes `sce:max-count` alone is refused, naming `sce:repeated`, and the guide says so.
  - *The Rust value takes the storage profile.* It is `{Codec}In<S>`, with a tag for each text, parameter text, value
    list and line list, and `Inline` answers for those with a bound. A content line is unescaped and unfolded as it is
    read, so no form borrows the frame, and the value has no lent form; the runtime's readers write into what `S`
    holds through the one container trait the positional and CBOR forms store through, and answer past the bound
    with the line's own error and past the capacity with `CapacityExceeded`.
  - *The other backends* take "no bound" in their readers, and C11 takes a deployment's capacity for a content-line
    entry as it does for a CBOR one, and refuses an entry with neither.
  - *Landed first:* the parser takes a text, a line of values and a repeated property with no bound. Each runtime
    names "no bound" once (`codec.Unbounded`, `SCE::Forge::ContentLine::kUnbounded`, `ContentLine.UNBOUNDED`,
    `content_line.UNBOUNDED`, and `usize::MAX` to the Rust writer, `None` to its reader), the generator writes that
    name where the protocol states no number, and a count is checked only where there is one.
    `ContentLineEntry::unbounded` is the one answer of which declaration an entry lacks: C11 refuses the codec by it,
    the largest encoded size is `None` by it, and a broadcast log's element is refused by it. C11's deployment
    capacity for a content-line entry landed after it (below).
  - *Held on every backend:* the conformance fixture `codec_content_line_open` states no bound of a text, a line of
    values, a repeated property and a parameter, beside two it does state, and its vectors go past every number a
    default once stood in for (256 octets, 64 lines, 8 values), which a test of the vectors holds. Rust, C++, Go,
    Kotlin and Python read and write it; C11 is left out by its refusal. Its first run found two readers that
    allocated a value's bound up front, Go's list of values and Kotlin's text buffer: with no bound that is the
    largest integer, so they failed. A bound is a refusal threshold and never an allocation; both now grow as the line
    is read.
  - *C11 at a deployment's capacity:* each container is held at the capacity a deployment gives it, by the id its Rust
    tag is named by, or at its bound; one with neither is refused, naming it. The runtime's readers take a bound and a
    capacity, as the CBOR ones do, and hold a text as Rust's `TextInto` does: the bound judged at each byte, UTF-8 at
    each character, a character held whole or not at all, the rest read on and not held. A decode reads a frame past
    a capacity whole, a line of a full list into its last row, and answers `CAPACITY_EXCEEDED` after every other
    refusal, so C11 and Rust give one frame one answer. `..._MAX_BYTES` is measured at what is held (`Measured::Held`),
    and a broadcast log's element at the bound (`Measured::Bound`).
  - *C11 CBOR, a store after a parse:* a text or byte string past its capacity is judged in place, a text for UTF-8,
    stepped over and not held, and the map answers `CAPACITY_EXCEEDED` after its own refusals (a key given twice, a
    required key absent), as Rust's projection answers after its decode. `sce_forge_codec_note_capacity` is the one
    helper both encodings note a capacity through.
  - *C11 positional, a store after a parse:* a list, a chain and a byte string or text past the capacity a
    deployment gives them are read whole. A list's and a chain's entries past it are decoded into the last row, the
    bound and the depth counted on the entries read; bytes past it are judged (a text for UTF-8) and stepped over,
    held as empty (`sce_forge_codec_hold`); a body a codec holds is noted when it answers a capacity, and the decode
    answers `CAPACITY_EXCEEDED` just before it would succeed. A decode that can answer one is told by
    `c11_may_exceed_capacity`, over the codec's own capacities and `ImportContext::codec_may_exceed_capacity` of what
    it holds; one that cannot is written exactly as it was, so every tree generated with no capacity is unchanged.
    C11 now gives a frame the answer Rust's view gives it on every encoding.
- **No generated item stands on the lent form at `Inline`.** The first run of a list with no bound found every static
  of a borrowed codec, and its lent projections, on `{Codec}<'a>`, which is `Lent<'a, Inline>`: such a codec did not
  compile anywhere, an MCU build with a deployment included. A codec's wire functions are on `{Codec}Wire`, a unit
  type of each codec that takes no profile, and a parent, a view, a broadcast log and every form write through it. The
  lent form's methods are written for `{Codec}In<Lent<'a, D>>` with `D: {Codec}LentStorage`, bounding `D` and not the
  lent profile, since a bound on `Lent<'a, D>` in a `where` clause shadows its impls and its fields' types stop
  normalizing. An owned value re-borrows to `{Codec}In<Lent<'_, S>>`, its lists as `S` holds them; it was re-borrowed
  into `Inline`'s lists, which refused one past its bound. `{Codec}<'a>` stays the eager form at `Inline` for a codec
  that has one, and only using it on a codec with a list of no bound fails to compile, as this section says of
  `Inline`. *Settled when the tree's own documents moved to the protocol's bounds (2026-11-03):* a codec that has
  none, because a list of it or of a codec it holds states no bound and is given no capacity, is generated with no
  `{Codec}<'a>` and no `{Codec}Variant<'a>`, which no program could use. Its eager form is named at a profile that
  holds its lists, `{Codec}In<Lent<'a, D>>`. The conformance build and harness already named it so; the generator
  emitted the name regardless, which no fixture showed until one had such a list.
- **A heap node, as landed.** The deploy input's `codec_storage:` says which kind of node a deployment is, `inline`
  by default or `heap` (`NodeStorage`). A heap node's deployment holds each field on the heap to what it holds, the
  deployment's capacity (its policy limit) or else the field's bound, in containers that grow and refuse past their
  number with `CapacityExceeded`: `CappedVec<T, N>`, `CappedStr<N>`, `CappedBytes<N>`, apart from `HeapStr<N>` and
  `HeapBytes<N>`, whose `N` the algorithm kind relies on being advisory. A field the protocol bounds by nothing and
  the deployment limits by nothing grows there (`GrowVec`, `GrowStr`, `GrowBytes`), where an inline deployment is
  refused for it. A storing decode on either kind of node reads a frame past what it holds whole and answers the
  capacity last, so the two kinds differ in where a field is held and not in what a frame is answered.
- **The tree's own Zenoh documents, as landed (2026-11-03).** The fixtures that model the consumer's protocol
  (`tests/forge/resources/codec_zenoh_*.scxml`) stated the sizes a node held as bounds, the defect this section
  names, which the consumer found against its own documents. Each now states what upstream states: a locator and an
  encoding schema 255 (a `u8` length), a key suffix and an INIT cookie 65535 (`u16`), a zbuf and its slices
  4294967295 (`u32`), and no bound for an OPEN cookie, a HELLO's locators, query parameters, a slice count or any
  extension chain's depth. The sizes they held moved to the conformance run's deploy input,
  `tests/forge/conformance/deploy.yaml`, as capacities: the C11 conformance build and the C11 goldens read it, so the
  storage of the run is what it was. A depth the document states is still refused past with `TooManyElements`; that
  shape is held by fixtures of its own (`codec_chain_depth_plain`, `codec_chain_depth_gated`) and the reject vectors
  moved with it. A Rust tree of a set that holds a list of no bound is generated with views, as the generator
  required until the amendment of 2026-11-05. The goldens of `tests/forge/expected/` are one crate and so one tree:
  the landing generated each as a tree of its own document, so a HELLO's golden named the view of a locator whose
  golden had none, which only the build of that crate saw. A golden is now generated with views where its codec, or a
  codec of the directory that holds it, has a list of no bound of its own or in a codec it holds, and a test reads
  every name a golden takes from one beside it.

*Amendment of 2026-11-05, decided by the owner: a Rust storing decode is a store after a parse in one pass, by the
rule C11 already followed.* A Rust decode that stopped at the field past its capacity and then judged the frame
again, by its view or by its lent form, kept that second form's value in the decode's call chain: a bare-metal
client measured a put's `decode_into` frame at 1,416 bytes, and once the lists of a group were built where they
live (ADR 0065 §6, amendment of the same day), what was left past one inline list was that judge (288, 296 and 280
bytes on the three probe codecs against lists of 136 and 132). There is no judge any more:

- **The rule, shared.** A field past what the profile holds is read on and not held, the capacity is noted, and the
  decode answers `CapacityExceeded` just before it would succeed, once the frame is read whole and every refusal of
  the frame's own has had its turn. Rust notes it through `note_capacity` and `note_held` (`codec.rs` of the Rust
  runtime) in a decode's `_over`, as C11 notes it through `sce_forge_codec_note_capacity`
  (`backends/c/forge-runtime/include/sce/forge/codec.h`) in its `_over` and reads a list past its array in
  `C11ListLoop` and `c11_chain_entry_lines`; the Rust content-line codec already did (`ContentLineReader`, with
  the same helper). So the Rust and C11 backends give one frame one answer by one rule, and the law above, that a
  form that stores refuses with `CapacityExceeded` only a frame a form that stores nothing accepts, holds without
  a second reading.
- **Each site.** A list reads every entry its count or its chain names, holds each where the list has room
  (`push_keeping`) and counts its bound on the entries read; its fill answers `CapacityExceeded` once the list is read
  whole, and a list so read keeps the entries it held (`Bounded::refill`), which are inside its bounds since every
  profile holds at least a list's fewest. A body, embedded or an arm of a variant, reads its own frame whole before
  it says so, and the decode that holds it notes it and goes on where that frame ends. Bytes or text past what a
  copying profile holds leave the field at its default. Two lists of one count answer it where their held counts
  differ, which only a capacity makes them do.
- **The gates on a chain's entries.** A chain notes the kinds its gates ask for of every entry it reads, held or
  not, so the fields after it are read as the frame says: Rust in its fill, C11 in the loop that reads an entry past
  its array into the last row, whose array alone cannot say which kinds the frame held. A full list keeps an entry
  of each kind it read in place of one whose kinds another entry holds, so the arm the kinds name is one the value
  can hold while the list has room for one entry of each kind; the door the arm is installed through keeps it and
  answers that it was past the capacity. A chain whose gated kinds outnumber what the profile holds of it names an
  arm the value cannot hold, and the door leaves the group at its default. Where the group builds nothing after the
  chain, every field after it is read into a local by the kinds the fill noted, so the decode notes the capacity and
  reads on, a frame cut or malformed after the chain is refused for that, and `CapacityExceeded` is said only of a
  frame read whole.
- **What the value holds.** On `CapacityExceeded`, as on every `Err`, `decode_into` leaves `_out` not to be read, and
  every field of it holds what its type says: a list inside its bounds, a chain its arm's kinds (or the group's
  default, where no arm the value can hold is the frame's), two lists one count, a group built by value left as it
  was.
- **What is not read whole, stated.** Two frames are answered `CapacityExceeded` before the frame is read whole, and a
  frame malformed after that point gets that answer rather than its own: a chain whose gated kinds outnumber what the
  profile holds of it, in a group whose arms hold a list or a body read after the chain, which has no place to be read
  into but the arm the value cannot hold; and a list held to its law (ADR 0065 §6 R8) whose entries break the law past
  its capacity, since the law is held to the entries the list keeps. A deployment that gives such a chain fewer
  entries than it has gated kinds, or a lawful list fewer entries than a frame breaks its law at, is the only one that
  meets them. No codec of the tree has the first shape: the one chain with two gated kinds
  (`codec_chain_has_tagged`) has only scalars after it.
- **What it frees.** No form is generated to judge a frame: `store_into`, `stored` and `past_capacity` are gone, and
  a Rust tree with a list of no bound is no longer refused without views. The MCU frame of a lent decode is held by
  `scripts/codec_frame_probe.sh` (`backends/rust/probes/nostd-codec-frame`), which refuses a frame on the decode's
  chain that is not shorter than one inline list of its codec.

*Note of 2026-11-05 on a lent reply decode and an encode.* A consumer measured its thumbv7em kit once the amendment
above had landed: its worst chain of direct calls fell from 4,280 to 2,960 bytes, and the kit's stack peak from 4,472 to
3,592, while the static bound of its stack fell only 120 bytes, from 4,816 to 4,696. The bound's worst path joins a
lent reply decode, whose own frame is about 1,472 bytes, with a generic encode of about 1,488, through a call by
register. The probe read neither: it decoded the three shapes of a group, at 296, 296 and 136 bytes of worst-case
stack.

- **What a lent decode builds outside where it lives.** It builds lists, bodies and chains in place; the one value
  it makes elsewhere is a default it installs: a variant's arm holding its body's default
  (`tools/codegen/templates/_macros/forge_rust_codec_decode_body.jinja2`, each arm and the default arm), a gated
  body's `Some` (the embed statement), a chain's fewest entries before its `refill`, and the entry a chain reads each
  entry into. A default called out of line would return its whole value through a slot of the caller's frame, so these
  were the suspects.
- **What was measured, and what does not change.** The probe below read the reply decode at 304 bytes of worst-case
  stack and the reply encode at 232, against one of the response's lists at 520, with the generated code as it is: the
  defaults are already built in line at the probe's opt-level, and no frame of SCE's decode or encode holds a reply.
  Marking those defaults `#[inline(always)]` was written and not landed, because it bought no measured byte and costs
  flash at every install site. If a later toolchain or profile keeps a default out of line, the probe's refusal says
  so, and that change is the remedy to land then.
- **The encode.** It reads its value by reference through its source and writes each field into the sink
  (`{Codec}Wire::write_source_in`); the only arrays on its path are the eight bytes or fewer of a fixed-width integer
  or a VLE that `SliceSink` stores in place. Nothing on it grows with the codec, so nothing in it changes; the probe
  now holds that.
- **The probe.** It decodes a response (`codec_zenoh_response`, a variant whose arm is a reply) at a deployment of its
  own, each extension chain held at 16 entries, into a value the caller holds, and encodes that value into a
  `SliceSink`; it refuses a frame on either chain that is not shorter than one of the response's lists, which a reply's
  arm holds one of.
- **What it cannot see.** The value a lent decode fills and the value an encode reads live where the caller puts
  them. A function that holds a decoded reply on its own stack to lend it holds the whole value, and so does one that
  builds a message on its stack to encode it. Where the probe's two reply frames are below a list, the two frames of
  about 1.5 KB are those values, and the remedy is the caller's: decode into a value that does not live on that stack,
  or into a view, whose lists are spans of the frame, and encode from a source over what the caller already holds
  (`{Codec}Parts`, or its own `{Codec}Source`, docs/adr/0058) rather than from a value built to be encoded.

*Amendment of 2026-11-15: an origin profile holds each field in a container that builds itself from the origin, so
the profile chooses share or copy tag by tag.* A profile answers `HoldsBytes<Tag>` and `HoldsStr<Tag>` field by field,
as this section decided, but the origin projection (`--owned-origin`) built every container through one generic
method of `OriginStorage`, `bytes_from<Tag>` and `str_from<Tag>`. That body had to build
`<Self as HoldsBytes<Tag>>::Bytes` for every `Tag` knowing only `Self: HoldsBytes<Tag>`, and stable Rust gives it no
way to tell a sharing container
from a copying one there (no specialization, no `'static` on the container for `Any`, no `SceByteBuf` bound to copy
through). An origin profile could therefore hold every byte field in one container: a consumer whose receive profile
shares a frame-sized payload paid that container's size for a 255-byte schema beside it on every message it
forwarded, where a copy of the schema would have cost nothing.

- **What a container is decides share or copy.** A container builds itself from a slice and the origin the slice was
  decoded from: `FromOrigin<O>` for bytes and `FromOriginStr<O>` for text
  (`backends/rust/forge-runtime/src/storage.rs`).
  A sharing container refers back to the origin; a copying one copies, and says so in one line with
  `copy_from_origin` or `copy_str_from_origin`. `OriginStorage` keeps only `type Origin`.
- **The profile chooses the container per tag.** What the projection asks for the field `Tag` is `OriginBytes<Tag>` or
  `OriginStr<Tag>`, shaped as `StoresBytes<'f, Tag>` is for a copying read, and answered by a blanket impl for every
  profile whose container for `Tag` builds itself from the profile's origin. One profile then shares a payload and
  copies a short field beside it, by answering the two tags with two containers.
- **The generated bound is one trait per codec.** `{Codec}Origin` asks for `{Codec}Storage`, `OriginStorage`, the
  codec's `OriginBytes<Tag>` and `OriginStr<Tag>`, and each embedded, repeated or dispatched codec's `{Inner}Origin`,
  as `{Codec}From<'f>` bundles its children, a child under a build feature through a trait of its own. It is declared
  with the projection, so a tree generated without the option is unchanged. The call site is
  `<S as OriginBytes<Tag>>::bytes_from(origin, v)`, the tag the trait's argument and no turbofish.
- **Still no copying default.** No impl gives every `SceByteBuf` a `FromOrigin`: a sharing container is one too, and
  a profile that forgot to choose would copy without a word, which is the failure `OriginStorage` was written to
  rule out. A field whose container has no constructor from the profile's origin is a compile error at the
  projection that reaches it (`an_origin_profile_whose_container_has_no_constructor_from_its_origin_does_not_compile`
  in `sce-build/tests/forge_conformance.rs`); a profile that shares one field and copies another is run beside the
  one that shares every field (`an_origin_profile_shares_the_frame_it_was_decoded_from`). A container of the runtime
  gets a constructor from a profile's origin only where the origin type is the consumer's own, by the orphan rule.
- **No compatibility layer.** `OriginStorage` had two implementers, a test profile of this tree and one consumer's
  receive profile; both move to the containers in the landing (`docs/SCE_CODEC_CONSUMER_MIGRATION.md`).

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
