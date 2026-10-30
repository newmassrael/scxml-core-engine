# ADR 0065 — A codec says where it is written whether its encode can refuse, and a total encode returns its bytes

- Status: Accepted (2026-10-30). The owner ordered the work started on the recommendation of that day, under the rule
  of 2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored. One
  amendment comes from the measurement in §0 and is marked "Amended" in §3. Totality is a property of the codec's
  abstract syntax, its source trait. It is not a property of the builder alone, as the recommendation assumed.
- Date: 2026-10-30
- Amends: ADR 0058 §2 ("the facade is uniformly fallible") and its consequence "`encode_to_vec` of every Rust codec is
  a `Result`".
- Scope:
  - `sce-build/src/forge/codec_totality.rs` (new; the refusal analysis, backend-neutral);
  - the codec root attribute `sce:encode-may-fail`, with its model field, parser and XSD;
  - two diagnostic codes;
  - the Rust half of `sce-build/src/forge/generator/rust_source.rs`, `tools/codegen/templates/forge/rust/codec.rs.jinja2`
    and `backends/rust/forge-runtime/src/codec.rs`, which gets the sink's own error and the infallible widening;
  - SCE_FORGE.md §4.6;
  - every codec document in the tree, which states its totality once;
  - the goldens under `tests/forge/expected/` and `backends/rust/mesh/src/generated/`.
  The other five backends keep their encode until each takes the source interface (ADR 0058 §6). The declaration is
  a property of the document, so `check` judges it for every backend.
- Related:
  - ADR 0058 (the source interface; its §2 is what this ADR amends);
  - ADR 0061 (`Measured`, the witness that a write cannot fail, which a total codec gets without a check);
  - ADR 0063 (the same principle for an integer operation: a total function returns a plain value, and a value that
    can fail says so where it is written);
  - SCE_FORGE.md §3.4.1 (`may-fail` on an algorithm's signature, and `algorithm/undeclared-integer-failure`).

## Context

ADR 0058 §2 made `encode_to_vec` of every Rust codec return `Result<Vec<u8>, CodecError>`, including codecs that
cannot refuse. Whether a codec can refuse (`codec_source_can_refuse`) is used only to skip a counting pass. Its
argument was stability: adding a refusing child to a codec should never change its consumers' types.

That trades a truth for a convenience. A caller of a codec that cannot refuse writes `?` or `.expect` for an error that
does not exist, and the type no longer separates the codecs that can refuse from those that cannot. The build knows
which is which and discards it. ADR 0063 states the principle this contradicts: a total function returns a plain
value, and one that can fail says so where it is written, checked by the build.

## 0. Measurement (2026-10-30)

Over the 128 Rust codec goldens in `tests/forge/expected/`, the encode-side refusal sites were read off `write_source`
and `encode_source`, the one skeleton every representation runs (ADR 0058 §2). A sink's `BufferOverflow` is not
counted, because it is the sink's refusal and not the source's.

| Refusal | Sites | What the abstract syntax admits that the encoding cannot carry |
|---|---|---|
| `LengthOutOfRange` | 51 | a byte or text length past a field narrower than `usize` (a 4-bit, `u8`, `u16` or `u32` length); an empty value where the wire carries the length minus one (`zid`); a default arm's tag past the tag's bits (`header.mid > 0x1F`) |
| `PresentIfMismatch` | 32 | a gated field whose presence disagrees with a gate the encoder cannot derive: a flag a parent passes in, a disjunction of two flags, a chain's `has(..)` |
| `TlvChainOverflow` | 24 | a chain longer than its declared depth |
| `TooManyElements` | 14 | a list longer than its `max-count` |
| `InconsistentSource` | 4 | two lists sharing one count with different lengths; a stated length of absent content that differs from the derived one |
| `LengthOutOfRange`, vacuous | 22 | `u64::try_from(usize)`, which cannot fail on any target Rust supports |

- 81 codecs have a site, 71 of them a non-vacuous one, and 57 have none or only vacuous ones. A codec that holds a
  refusing child refuses too. The transitive set is the first output of step 1.
- Which representation can reach a site:
  - **Builder** `X<'a>`: lists are bounded (`heapless::Vec<_, N>`), but bytes and text are `&'a [u8]` and `&'a str`
    of any length, a tag is a `u8`, and a flag and a gated `Option` are stored apart. It reaches every kind except
    `TooManyElements`.
  - **Parts** `XParts<'a>` and any hand-written `XSource`: lists are slices or iterators of any length. They reach
    every kind.
  - **Owned mirror**: `Heap` storage is unbounded, so it reaches every kind (SCE_FORGE.md §4.6).
  - **Decoded view**: the decode checked what the encode would. It reaches none, by ADR 0058 §5 laws 1 and 2, held by
    the 40,000-frame sweep and not by a type.
- The split of 41 codecs that can refuse and 78 that cannot, reported on 2026-10-29, counted the builder alone. It is
  superseded by this table.

## The textbook structure

An encoder is a printer. Over its abstract syntax it is total: every value of the abstract syntax has a frame.
Parsing, its inverse, is partial: some byte strings are not frames. Rendel and Ostermann state the pair this way
(*Invertible Syntax Descriptions*, Haskell Symposium 2010). Verified format libraries build their serializers the same
way: EverParse (Ramananandro et al., USENIX Security 2019) and Narcissus (Delaware et al., ICFP 2019) put a format's
constraints into a refinement of the value type, and the serializer is total over that refinement.

The gap the measurement finds is therefore a gap between the abstract syntax and the encodable set. The source trait
admits values that have no frame: a length past its field, a list past its bound, a tag past its bits, a flag that
disagrees with a presence. Each refusal is a refinement `{ s : XSource | admissible(s) }` checked late, at the encode,
on every call. The textbook answer is the one ADR 0061 already applies to a single value, *parse, don't validate*
(King, 2019). The refinement is checked once, where the value is made, by a constructor that returns a witness, and
everything after the witness is total.

That gives three facts, and this ADR keeps them apart:

1. **Totality is a property of the abstract syntax.** A codec is total exactly when its source trait admits no value
   without a frame. Then every representation that implements the trait is total, the builder, the parts, a view or
   a consumer's own type, and `encode_source` is total for every `S`. A property of one representation only (the
   builder) would leave `encode_source` fallible and the facade split by representation, which is the shape this
   ADR removes.
2. **A total encode returns its bytes.** The only error left is the sink's. A sink that cannot refuse (a `Vec`, a
   counter) has an uninhabited error, so the compiler, not a comment or an `unreachable!`, proves that the frame is
   produced.
3. **A codec states its totality where it is written, and the build checks the statement.** If the statement were
   inferred, a one-line schema edit (a disjunction gate, a narrower length) would silently change a public signature
   that a consumer compiles against. Declared, the same edit is refused at generation with the reason, and its
   author chooses. This is the rule of SCE_FORGE.md §3.4.1 for `may-fail` on an algorithm and of ADR 0063 for an
   integer operation, applied to an encode.

## Decision

### 1. The declaration: `sce:encode-may-fail`

A codec document's root takes `sce:encode-may-fail`, an `xs:boolean`.

- **Absent or `false`: the encode is total.** That is the default.
- **`true`: the encode can refuse a source,** and the declaration is the codec's public statement of it.

It is a property of the document, like `sce:encoding`, so every backend and `check` read the same value.

### 2. The analysis: the refusals of a codec, with their reasons

`sce-build/src/forge/codec_totality.rs` computes `refusals(model, child) -> Vec<Refusal>`. It replaces the boolean
`can_refuse_with`, which over-approximated (any variant, any list). Each `Refusal` names:

- the element: a field id, a variant's tag or a gate;
- the kind: one of the error kinds in the table of §0;
- the bound it can break, for example "a length of `payload` past 15, the width of `carrier.len`";
- for a child, the alias it holds and that child's own refusals.

The analysis reads the codec model and `codec_roles` (ADR 0058 §1), never a backend's output, so it is the one
statement of when an encode can refuse. A codec holds a child that refuses only if it refuses too, so the analysis
is transitive, through the import resolution `codec_source_can_refuse` uses today.

### 3. The check: the declaration agrees with the analysis

The check runs once, before any backend renders, so every backend and `check` refuse the same documents.

- A codec declared total with a refusal is refused with `codec/undeclared-encode-refusal` at the row of the first
  element the encode refuses at, with the reason, as every other validation of a document reports its first
  refusal. `sce-build/tests/codec_totality.rs` lists every place of every codec at once. Two repairs are right and the record proposes neither: declare
  `sce:encode-may-fail="true"`, or change the grammar so the refusal is gone (a wider length field, a bound the
  value's type already keeps).
- A codec declared `may-fail` with no refusal is refused with `codec/encode-may-fail-without-refusal`. A declaration
  that is not true is the uniform `Result` of ADR 0058 §2 in one document: it tells every consumer to handle an
  error that cannot occur.

*Amended.* The recommendation of 2026-10-30 assumed that most codecs would be total once their own builder was. §0
shows that every list codec, every codec whose length field is narrower than `usize`, and every codec that holds such
a codec can refuse at its source trait. Step 1 therefore declares `may-fail` on more than half the tree, and each step
of §6 removes declarations. That ordering is the reason the check refuses an untrue declaration: a declaration that
outlives its last refusal would hide the progress of §6 from every consumer.

### 4. The interface: a total codec's encode has only the sink's error

`SceSink` gains `type Error`. `VecSink` and `CountSink` take `core::convert::Infallible`. `SliceSink`, `WindowSink`
and a consumer's own sink take `CodecError`, and `From<Infallible> for CodecError` lets `?` cross from one to the
other.

| | total codec | `may-fail` codec |
|---|---|---|
| `write_source`, `encode_source`, `encode` | `Result<(), K::Error>` | `Result<(), CodecError>`, `where CodecError: From<K::Error>` |
| `encode_to_vec` | `Vec<u8>` | `Result<Vec<u8>, CodecError>` |
| `measure` | `Measured<'_, X, S>` | `Result<Measured<'_, X, S>, CodecError>` |
| `write_window` | `Result<usize, CodecError>` (the window's own overflow) | the same |

- In a total codec's `encode_to_vec`, `let Ok(()) = …;` is an irrefutable pattern, because the error type is
  uninhabited (Rust 1.82). The proof that a frame is produced is the type checker's.
- A refusal site emitted into a total codec's body does not compile: `CodecError` does not convert into a generic
  `K::Error`. The Rust compiler is therefore a second checker of the analysis on every golden, and a refusal that the
  analysis misses breaks the golden build rather than a consumer.
- `SourceWrite` (ADR 0061) states the error of its codec through a generic associated type: `type Error<E>`, which
  is `E` for a total codec and `CodecError` for a `may-fail` one. `measure` and `Measured::write_into` are written
  once over it.
- A `may-fail` parent holds a total child through `?`, and a total parent never holds a `may-fail` child (§2).
- A `<send>` payload of a total codec in a procedure has no failure to report, so the `Event::Fail` path of ADR 0058
  §2 is emitted only for a `may-fail` codec.

### 5. A conversion that cannot fail is not written as one that can

The 22 vacuous sites (`u64::try_from(usize)`) are emitted as `sce_forge_runtime::codec::widen_usize(n)`, a `const fn`
next to a compile-time assertion that `usize::BITS <= 64`. A conversion that cannot fail does not make a codec
`may-fail`, and the analysis does not count it.

### 6. The refinement program: each refusal leaves the abstract syntax

The end state is a tree in which no codec declares `may-fail`, because no source trait admits a value without a frame.
Each kind of refusal is removed by a type that cannot hold the refused value. The check runs at the type's
constructor, where the value is made, and that is the one place left that returns a `CodecError`. A decode builds the
same types through the same constructors, and their error becomes the decode's. A step lands one kind across the
tree, and the declarations it makes untrue are removed in the same commit, as §3 requires.

- **R1, bounded bytes and text.** `Bytes<'a, MIN, MAX>` and `Text<'a, MIN, MAX>` hold a borrowed slice whose length
  lies in `MIN..=MAX`, checked by `new`. `From<&[u8; N]>` is total when `N` lies in range, so a fixed-size key costs
  nothing. The bounds are those of the length field: `0..=15` for a 4-bit length, `1..=16` for a length written as
  itself minus one. A field whose length is 64 bits or a VLE of 64 bits is unbounded and keeps `&[u8]`. The CBOR
  codecs' exact lengths and maximum sizes take the same types.
- **R2, bounded lists.** A list item of the trait is `Bounded<I, MAX>`, an iterator whose length was counted once,
  against `max-count` or the chain's depth, by its constructor. A builder's `heapless::Vec<_, N>` converts to it
  totally when `N <= MAX`, which the generator asserts at compile time. A slice or an iterator converts through the
  checked constructor. A list that is shorter or longer on its second walk is the impure source of ADR 0061 §3 and
  panics; it is not refused.
- **R3, the default arm's tag.** A codec emits `XUnknownTag`, a newtype whose constructor admits a value inside the
  tag's bits and outside every known arm's tag. That also closes the case of a default arm that carries a known tag,
  which a decode would read back as the known arm, a break of ADR 0058 §5 law 1.
  *Landed 2026-10-31, before R1.* A codec that holds a codec that can refuse counts its source before it writes, so
  the caller's sink is untouched when the held codec refuses, and that count is a second copy of the write in the
  program. A consumer measured it on a Zenoh `put`, whose one held codec that could refuse was `ext_entry`, and
  `ext_entry`'s one refusal was this tag. With R3 a codec whose held codecs no longer refuse loses the count. The
  constructor is `new(tag) -> Option<Self>`, as `char::from_u32` is: a tag an arm names is not a
  length, and no `CodecError` names it. The decode does not call it: the arms before its catch-all take every tag the
  document names (an arm left out of a build refusing its tag, ADR 0072) and the tag is read from its bits, so the
  catch-all builds the value directly. An arm under a build feature is among the known tags in every build.
- **R4, gate normal forms.** A gate the encoder cannot derive becomes a sum type over the cases it allows:
  - a disjunction `a.X || b.Y` becomes one arm for each satisfying assignment of the flags it names;
  - a flag a parent passes in that gates fields here is derived in the other direction, so the parent's bit is
    computed from the arm the child presents, as ADR 0058 §1 derives a one-flag gate;
  - a stated length of absent content and the content become one sum, which replaces two `Option`s.
- **R5, a shared count.** Lists that share one count are presented as one list whose entries carry each list's item,
  the second one optional when its list is gated.
- **R6, a gate on a chain's content** (`chain.has(id)`, SCE_FORGE.md §4.6.2). Where the entry the gate names carries
  no content of its own, the entry is derived from the field's presence. Where it does carry content, the relation
  is a dependent pair, and the codec keeps `may-fail` until this ADR's open question is answered.

A source made of refined types cannot be refused, but the refinement leaves the trait shape alone. A consumer that
implements `XSource` on its own type returns refined values from the same accessors.

## Laws

For every codec `X`:

1. **Declaration.** `X` is declared total exactly when the analysis finds no refusal (§3).
2. **Soundness.** For a total `X`, `encode_source` returns `Ok` over every sink whose error is uninhabited. Two things
   hold it: the type of §4, and the conformance sweep, which runs every source shape ADR 0058 §5 lists, the shapes
   that break each bound included, against every total codec.
3. **Completeness.** For a `may-fail` `X`, each refusal the analysis names has a conformance source that the encode
   refuses with that kind.
4. **Conservation.** The bytes of every accepted source are those before this ADR. The 40,000-frame sweep and the
   golden bytes do not move.
5. **Refinement** (each step of §6). A refined constructor refuses exactly what the encode refused before the step,
   with the same kind, and a decode of a frame refused by the new type gives the error it gave before.

The mutation round for step 1 kills five mutants:

- an analysis that drops a kind;
- a check that accepts an untrue declaration in either direction;
- a facade that keeps `Result` on a total codec;
- a widening that narrows;
- a transitive refusal that is not propagated.

## Consequences

- A caller of a total codec writes `let bytes = X::encode_to_vec(&v);`. A caller of a `may-fail` codec writes `?` and
  knows it is not a formality.
- Adding a refusal to a total codec is refused where it is written, with the reason. A consumer's types change only
  when an author declares the change.
- `SceSink` implementors outside the tree add `type Error = CodecError;`. In watching-zenoh that is one impl
  (`TxSink`).
- The codecs of watching-zenoh change in step 1 by their own totality. A total codec drops one `?` at each call site,
  and a `may-fail` codec is unchanged. Each step of §6 then removes `?` from the codecs it makes total.
- The `may-fail` codecs of step 1 are the list of what §6 has left to do, read off the documents.

## Considered alternatives

- **The uniform `Result` of ADR 0058 §2, kept.** Refused, because it discards what the build knows and contradicts
  ADR 0063.
- **The signature inferred from the analysis, with no declaration.** It is sound, and a schema edit then changes a
  public signature with nothing at the author's end to say so. Refused for the reason of §3 in "The textbook
  structure".
- **Totality per representation**, a plain `encode_to_vec` on the builder or the view of a codec whose trait admits
  more. It splits one codec's surface by representation, leaves `encode_source` fallible, and rests the view's case on
  a tested law, not a type. Refused. A decoded view becomes total when its codec does.
- **An `unreachable!()` in a total `encode_to_vec`** in place of the sink's error type. It is shorter, and the proof
  sits in a comment where the compiler cannot check it, so a refusal the analysis misses becomes a panic in a
  consumer rather than a compile error in the golden build. Refused.
- **A `may-fail` declaration kept as a reservation** for a codec that might gain a refusal. Refused, because it is an
  untrue statement that every consumer pays for. A codec that gains a refusal changes its declaration in the same
  commit, and its consumers see the change in their types.
- **The refinement of §6 first, with no declaration in between.** It reaches the same end, and the codecs it has not
  reached yet stay without a stated totality for as long as it takes. The declaration makes the remaining work a
  visible property of each document and lets the total codecs return their bytes now.

## Order of work

1. The analysis, the declaration and its check with the two codes, the sink's error type, the signatures of §4, the
   widening of §5, and the declarations across the tree. The goldens are regenerated once. The conformance sweep
   holds laws 1 to 4, and the mutation round holds the five mutants above.
2. R1. Then R2, R3, R4 and R5, each one step, each removing the declarations it makes untrue.
3. R6, after the open question.
4. The other five backends, as each takes the source interface (ADR 0058 §6), with the same signatures in their
   idiom: a total encode returns its bytes, and a `may-fail` one returns the backend's failure.

## Questions left open

- Whether every `chain.has(id)` gate in the tree names an entry with no content of its own (R6). It is answered by
  reading the documents when R6 starts.
