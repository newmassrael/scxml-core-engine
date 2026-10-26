# ADR 0058 — A codec encodes any value that presents its fields, and a list is presented by walking it

- Status: Proposed (2026-10-27), designed under the owner's rule of 2026-10-26 and 2026-10-27: the most long-term
  correct structure, textbook in its foundations, cost ignored. Requested by watching-zenoh, which measured the cost
  below; the decision is the owner's
- Date: 2026-10-27
- Scope: `tools/codegen/templates/forge/rust/codec.rs.jinja2`, `tools/codegen/templates/_macros/forge_rust_codec_encode_body.jinja2`
  and the Rust half of `sce-build/src/forge/generator.rs` (the encode blocks of fields, lists, embedded bodies and
  variants); `backends/rust/forge-runtime/src/codec.rs` (the bound check a walked list is held to); SCE_FORGE.md §4.6;
  the golden trees under `tests/forge/expected/` and `backends/rust/mesh/src/generated/`, which must not move. The
  other five backends take the same interface in their own idiom in a later step (see "Order of work")
- Related: ADR 0049 (a decoded value is a view; its §4, "writing stays on the builder form", is what this ADR
  revises), ADR 0010 decision 2 (the wire rules are written once, and every backend writes the same bytes), ADR 0053
  (the runtime's features only add)

## Context

A Rust codec is written through its builder `X<'a>`, whose lists are inline `heapless::Vec<Entry<'a>, N>` with `N`
the declared `sce:max-count`. A value to be sent is built into that struct and then encoded. Measured by
watching-zenoh on a `thumbv7em-none-eabihf` client (`-Zprint-type-sizes`, LLVM IR allocas): answering a query puts a
2,096-byte frame on the stack, 1,104 bytes of it one `Response<'_>` builder, of which the body alone is 776 bytes
because each extension chain is 272 bytes of inline capacity, nested inside the body. That frame is on the path of the
measured stack peak, 6,480 bytes, against zenoh-pico's 5,984 for its main and task together. The consumer already
holds the parts (key, payload, QoS, responder) in its own types; it copies them into the builder only because the
builder is the one thing `encode` reads.

ADR 0049 removed the same cost from the receive side: a decoded value is a view whose lists are walked, not stored.
It left writing on the builder (§4), on the ground that an owned value encodes straight from its own fields. The owned
mirror needs `alloc`, so on the no-alloc tier the builder is still the only way to send.

Today `encode` is one skeleton rendered three times per codec: for the builder, for the view and for the owned
mirror, each reading the fields off its own struct through flavour-specific blocks (`encode_view`, `owned_encode`).
Three readings of one grammar, and none of them is the consumer's.

## The textbook structure

The separation this ADR makes is the oldest one in data encoding: an **abstract syntax** — what a value of the type
says — kept apart from its **representations**, and from the **encoding rules** that turn the abstract value into
bytes. It is the split between ASN.1's abstract notation and its encoding rules (ITU-T X.680 and X.690), and the one
Wadler's *views* make between a type's interface and its representation ("Views: a way for pattern matching to
cohabit with data abstraction", POPL 1987). Encoding is then a function of the abstract value alone: a fold over what
a representation *presents* (in the vocabulary of Meijer, Fokkinga and Paterson, "Functional Programming with
Bananas, Lenses, Envelopes and Barbed Wire", FPCA 1991, a catamorphism over the codec's grammar). Serde's
`Serialize` is the same idea in Rust: the value presents itself, the format writes.

So a codec's grammar becomes an **interface**, every representation **implements** it, and there is **one** encode,
written against the interface.

## Decision

### 1. Each codec has a source trait, its abstract syntax as an interface

For each codec `X` the generator emits `trait XSource`, with one item per element of the grammar, in declaration
order:

- a scalar, a flag or an enum field: `fn field(&self) -> T`, by value;
- a string or bytes field: `fn field(&self) -> &str` or `&[u8]`;
- a field gated by `present_if`: `Option<…>` of the above;
- an embedded body of codec `Y`: `type Field: YSource; fn field(&self) -> &Self::Field`;
- a `<sce:repeat>` or `<sce:tlv-chain>` of entries of codec `E`:
  `type FieldEntries<'s>: Iterator<Item = Self::FieldEntry<'s>> + Clone where Self: 's;`
  `type FieldEntry<'s>: ESource where Self: 's;`
  `fn field(&self) -> Self::FieldEntries<'_>`;
- a variant: `fn body(&self) -> XSourceVariant<'_, Self>`, an enum whose arms carry the arms' sources through
  associated types.

The interface reads; it never stores. A list is presented by an iterator the implementor makes fresh on each call,
so its entries may come from a slice, from a ring, from another value's view, or be computed.

### 2. There is one encode, and it reads the interface

`fn encode_source<S: XSource + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError>` is the encode
skeleton rendered once, its reads made through the trait. The builder, the view (ADR 0049) and the owned mirror each
implement `XSource`, and their `encode` methods become calls of `encode_source(self, w)`. The three flavour blocks of
the skeleton collapse into one, so a wire rule written there reaches every representation by construction.

The wire is unchanged: every golden byte stays where it is. That is the test of the refactor's correctness, and the
reason the first step is the refactor alone.

### 3. A walked list is held to its bound before anything is written

A builder's list cannot exceed `N`, because `heapless::Vec` cannot. A walked list can, so `encode_source` counts each
list through a clone of its iterator before the first byte of the value is written, and refuses an excess with the
error a decode of the same frame would give (`TooManyElements`). An encode therefore writes the whole value or
nothing of it, whatever the source. Where a length or count field precedes its content on the wire, the encode reads
it from the source as it does today, so no second pass over content is introduced.

### 4. A ready-made source for borrowed slices

For the common shape — parts already in hand, lists in slices — the generator also emits `struct XParts<'a>`, whose
lists are `&'a [EParts<'a>]` and whose bodies are `&'a YParts<'a>`, implementing `XSource`. A consumer either fills
`XParts` from what it has or implements `XSource` on its own type. Encoding needs the stack of the entries the caller
holds, not `N` slots per list.

### 5. Laws, and the tests that hold them

For every source `s` of codec `X`:

1. `encode_source(s)` is byte-for-byte the encode of the builder filled from `s` — so a hand-written source cannot
   reach a byte the builder could not.
2. `decode(encode_source(s))`, read through the view, presents the same fields and entries as `s` — the round trip.
3. On every backend that implements the interface, the bytes are those of every other (ADR 0010 decision 2).

The conformance suite holds law 1 and 2 with sources of every shape the trait admits (a slice, an iterator that
computes, a view re-encoded as a relay does), and law 3 in the existing cross-backend byte-parity matrix.

### 6. Other backends

The interface is the codec's grammar, which every backend already reads; each takes it in its idiom in a later step:
a C++ concept, a Go and Kotlin interface, a Python protocol, and in C a table of functions over a context pointer,
with lists presented by a begin/next pair. Until then those backends keep their current encode, and the matrix holds
their bytes equal.

## Consequences

- A no-alloc consumer sends from what it holds: the stack an encode needs is the entries it presents, not the declared
  capacity of every list.
- One encode skeleton instead of three; a fourth representation is an implementation, not a template branch.
- A relay re-encodes a received view through the same interface it decoded into (ADR 0049), with no builder between.
- Generated code grows by one trait and one parts struct per codec. The encode is generic, so a binary pays for the
  instantiations it uses; the byte-exact `.text` gate sees the difference and the step that lands the interface
  measures it before it is called done.

## Considered alternatives

- **A builder variant whose lists are borrowed slices** (the consumer's first sketch). It solves the measured frame,
  but it is a fourth representation with a fourth template branch, and the caller still materialises every entry as a
  builder value in a slice. Kept as the ready-made source of §4, not as a branch of the encode.
- **A direct-to-wire builder** in the manner of FlatBuffers and Cap'n Proto: the caller writes fields straight into the
  output in order, its ordering enforced by typestate. It removes the intermediate value too, but it is a second encode
  with its own ordering rules, it cannot re-encode a view or an owned value, and a length that precedes its content
  needs back-patching, which a streaming sink cannot do.
- **Lowering `sce:max-count`**: a protocol decision, not a generator one, and it moves the cost rather than removing it.
- **The heap**: forbidden on the tier that measured the cost.

## Order of work

1. The refactor: `XSource`, `encode_source`, and the three representations as implementations; no golden moves.
2. `XParts`, the bound check of §3, and the conformance cases of §5 with every source shape.
3. The interface on the other five backends, in their idioms, held to byte parity.

## Questions left open

None of the design. The decision is the owner's.
