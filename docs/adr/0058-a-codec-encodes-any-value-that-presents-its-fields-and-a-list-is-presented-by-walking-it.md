# ADR 0058 — A codec encodes any value that presents its fields, and a list is presented by walking it

- Status: Accepted (2026-10-27), on the owner's instruction of that day to decide it under the rule of 2026-10-26
  and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored. Requested by
  watching-zenoh, which measured the cost below. Decided with three amendments to the proposal, each marked
  "Amended" below: what a source presents is only what the value says and not what the encoding determines (§1),
  one measuring fold checks the source and computes the lengths before anything is written (§3), and the laws are
  stated for every admissible source (§5)
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

Those readings share a contract that is written only in generator comments: the encoder writes **whatever the struct
holds**. A count field is stored beside its list, a variant's tag beside its body, and "the author keeps them
consistent" (`repeat_streaming_encode_block`; the variant prologue says the same of the tag). A builder can therefore
hold a value that encodes to a frame no decoder accepts, and nothing says so. An interface that presents every stored
field would carry the contract into the consumer's types, where it is harder to see than in a struct a consumer
fills by hand. This ADR does not carry it over (§1).

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

*Amended: presented and derived.* An element of a codec is either **presented**, information the value carries that
nothing else on the wire determines, or **derived**, determined by what is presented. A count field is the length of
its list; a length field the encoded length of what it bounds; the tag of a known variant arm is that arm's; a carrier
bit is what the parent-tag derivation makes of the arm; the flag a field is gated on is set exactly when the field is
present; the flag a TLV chain ends on is set on every entry but the last. A derived element is not an item of the
trait: the encoder computes it, as the encoding rules of ASN.1 (X.690) compute a length and a tag and never take them
from the value (and as the presence bitmap of an ASN.1 `OPTIONAL` is the encoder's, not a field of the value). The
decoder already reads every one of them to walk the frame, so the generator knows which is which.

The rule is by declaration and not by appearance. The declarations are the ones the decoder reads: `sce:length-field`,
`<sce:repeat count=>`, `sce:length-from`, an own-field `<sce:variant>`'s tag, `<sce:variant-dispatch>` on an import,
`terminate-on="entry-flag"`, and `sce:present-if` in its one-flag form (`carrier.flag` or `!carrier.flag`). Every other
element is presented, a header flag bit among it, even where the values consumers choose for it usually follow from
the content: a flag that says something the content does not (a header bit that marks an interest as a subscription
to tokens or not) is a thing a value can say. A tag the grammar does not determine (the default arm of a variant,
which keeps the tag it could not name) is presented, by that arm. Several declarations of one kind that name one
element are one derivation with several sources, which must agree in size (zenoh's LinkState: one count for `links`
and for the gated `weights`); declarations of different kinds that name one element are ambiguous, and no codec in
the tree has one. A gate of any other form (a disjunction `a.X || b.Y`, a bare flag-input name, a chain's
`ext.has(2)`) names no one flag this codec could set, so its relation of flag to presence is a *check* of §3 and the
flags it names are presented.

*Landed (step 1).* `sce-build/src/forge/codec_roles.rs` is the one place this is written, and
`sce-codegen codec-roles <codec.scxml>...` prints it as a table, element by element, so that a consumer can see which
of the values it sets by hand the encoding will take over before any output changes. Over every codec in
`tests/forge/resources` the classification agrees with a second reader that sees only the document's XML and never the
model (`sce-build/tests/codec_roles.rs`), and no element is derived by two kinds. Agreement with what the decoder
*computes*, as opposed to what it declares, is law 1 of §5, which a round trip holds when step 2 lands.

The consequence is the property the proposal lacked: every source that passes §3 encodes to a frame the decoder
accepts.

For each codec `X` the generator emits `trait XSource`, with one item per presented element of the grammar, in
declaration order:

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

The wire is unchanged for every value a decode produces: every golden byte stays where it is. That is the test of the
change's correctness (law 2 of §5), and the reason the generated tree is regenerated only once, by the step that lands
it. The builders' own `encode` keeps its name and signature and calls `encode_source`. A consumer that fills a builder
consistently changes nothing; one that fills it inconsistently is the case §1 closes, stated under law 2 of §5.

### 3. One measuring fold checks the source and computes what the encoding determines, before anything is written

*Amended.* A builder cannot hold a value that breaks a bound its types state: a `heapless::Vec` of `N` entries, a
`heapless::String` of its maximum. A source can. A builder can hold a flag that disagrees with the presence it
announces (a flag byte stored as given beside an `Option` the encode writes only when it is `Some`), and today it
encodes the frame that disagreement describes, which no decoder reads back as the value; §1 derives that flag, and
the same disagreement between a source's gate and its presence, where the gate is not of the one-flag form, is a
check below. And a derived element that precedes its content on the wire (a count, a length) needs what follows it
before the first of it is written, which a streaming sink cannot be patched for. Both are answered by a second algebra over the same grammar,
generated beside the encode: `measure_source`, a fold that visits the source once and returns what the encode needs of
it.

- It **checks** every bound the builder's types state by construction (`max-count`, the maximum size of a string or
  bytes), every relation of §1 that has several sources (a count two lists share, a length two fields share) and every
  gate that is not of the one-flag form (`codec-roles` lists them as `check` lines), and refuses a violation with the
  error a decode of that frame gives (`TooManyElements` and its siblings), before the first byte.
- It **computes** each derived length that precedes its content and each count, so that `encode_source` writes them
  from the measure and never reads them from the source.

The cost is time and not memory: a measure keeps a few integers and allocates nothing, which is what the no-alloc tier
asks. A body whose length precedes it is measured once by its parent and again by its own encode, so the work is
`O(d·n)` for `d` the nesting depth of length-prefixed bodies, a constant of the grammar fixed when the codec is
generated; no second pass is made over a codec whose grammar has no such length.

What is guaranteed is exactly this: **a source that fails the measure writes nothing**, and one that passes writes a
frame the decoder accepts. A sink that runs out of room can still refuse partway, as it can today; that refusal is the
sink's and is not a property of the source.

### 4. A ready-made source for borrowed slices

For the common shape — parts already in hand, lists in slices — the generator also emits `struct XParts<'a>`, whose
lists are `&'a [EParts<'a>]` and whose bodies are `&'a YParts<'a>`, implementing `XSource`. A consumer either fills
`XParts` from what it has or implements `XSource` on its own type. Encoding needs the stack of the entries the caller
holds, not `N` slots per list.

### 5. Laws, and the tests that hold them

*Amended: stated for every admissible source, a source being admissible when `measure_source` accepts it.* For every
admissible source `s` of codec `X`:

1. **Totality.** `encode_source(s)` is a frame the decoder accepts, and `decode(encode_source(s))`, read through the
   view, presents the same elements and entries as `s`. This is the round trip, and it holds of every source, not of
   those a careful author wrote.
2. **Conservation.** For a value a decode produced, `encode_source` writes the bytes the encode of the builder wrote
   before this ADR. Where a builder is filled by hand to hold what a decode would not (a count that is not its list's,
   a tag that is not its arm's, a gate flag that disagrees with the presence it announces), the new encode writes what
   the value says and the old one wrote what the struct held;
   that is the defect of §1 closing, and it is stated in the step that lands it.
3. **Refusal.** A source that breaks a bound is refused before the first byte, with the error a decode of that frame
   gives.
4. **Parity.** On every backend that implements the interface, the bytes are those of every other (ADR 0010
   decision 2).

The conformance suite holds them with sources of every shape the trait admits (a slice, an iterator that computes,
a view re-encoded as a relay does, a source that breaks each kind of bound). Law 2 is held against the committed
golden bytes, which must not move, and against the retained decode: over 40,000 frames from a fixed seed, in the
manner of ADR 0049's `view_generated`, a decoded frame re-encoded through the builder, the view and the owned mirror
must give one frame, and that frame decoded again must present what the first did. Law 4 is held in the existing
cross-backend byte-parity matrix.

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
- Generated code grows by one trait, one measure fold and one parts struct per codec. The encode is generic, so a
  binary pays for the instantiations it uses; the byte-exact `.text` gate sees the difference and the step that lands
  the interface measures it before it is called done.
- A source is total over the grammar: a field added to a codec is a new required item of its trait, so a hand-written
  source cannot silently omit it, which a builder with a default can. That is the price of the interface being the
  grammar, and it is paid at compile time.
- A builder filled by hand with a count, length or tag that disagrees with its content no longer encodes the
  disagreement; it encodes what the content says (§1, §5 law 2).

## Considered alternatives

- **A builder variant whose lists are borrowed slices** (the consumer's first sketch). It solves the measured frame,
  but it is a fourth representation with a fourth template branch, and the caller still materialises every entry as a
  builder value in a slice. Kept as the ready-made source of §4, not as a branch of the encode.
- **A direct-to-wire builder** in the manner of FlatBuffers and Cap'n Proto: the caller writes fields straight into the
  output in order, its ordering enforced by typestate. It removes the intermediate value too, but it is a second encode
  with its own ordering rules, it cannot re-encode a view or an owned value, and a length that precedes its content
  needs back-patching, which a streaming sink cannot do.
- **Presenting every stored field, derived ones included** (the proposal as first written). It keeps the trust contract
  of §1: a source could describe a count that is not its list's, and the interface would be the place a consumer is
  trusted to get it right. Refused: an interface is a grammar, and a grammar says what a value is, not what the
  encoding of it must agree with.
- **Back-patching a length after its content** instead of a measure: impossible on a streaming sink, which cannot seek,
  and the sinks this tier uses do not.
- **Lowering `sce:max-count`**: a protocol decision, not a generator one, and it moves the cost rather than removing it.
- **The heap**: forbidden on the tier that measured the cost.

## Order of work

1. The classification of §1 made explicit in the generator (which element of each codec is presented and which
   derived), the table of it for a consumer, and a test that it agrees with the declarations in the documents. No
   output changes. **Landed 2026-10-27** (`codec_roles.rs`, `sce-codegen codec-roles`); its agreement with what the
   decoder computes is held by step 2.
2. `XSource`, `measure_source`, `encode_source`, and the three representations as implementations; no golden moves;
   the conformance cases of §5 laws 1 to 3 with a hand-written source of each shape.
3. `XParts`, and the consumer's first use of it (watching-zenoh's send stack), measured against the 6,480-byte peak.
4. The interface on the other five backends, in their idioms, held to byte parity (law 4).

## Questions left open

None. Decided 2026-10-27; the first step is 1 above.
