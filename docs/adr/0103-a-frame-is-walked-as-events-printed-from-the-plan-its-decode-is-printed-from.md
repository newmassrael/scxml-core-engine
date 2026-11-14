# ADR 0103 — A frame is walked as events printed from the plan its decode is printed from

- Status: Accepted (decided by the owner 2026-11-14), under the standing rule: the most long-term correct structure,
  textbook in its foundations, cost ignored. Numbered 0103 because 0099 to 0102 and 0104 are being written in
  parallel.
- Date: 2026-11-14
- Amends:
  - ADR 0081 §1, "The plan": the plan of a codec holds a read program beside its write program (§2.1 here), and its
    "Order of work" step 4 gains, for each backend, a decode printed from that program before the backend prints a
    walk;
  - SCE_FORGE.md §4.6, which gains §4.6.5, "The walk of a frame (`generate --walk`)", written with the Rust landing,
    and whose paragraph on `decode_into` says what the destination holds after a refusal (§4);
  - `docs/SCE_CODEGEN_DETERMINISM.md` §1: `--walk` joins `--owned-origin` and `--views` as a flag that changes what is
    written and, off, changes nothing.
- Scope:
  - `sce-build/src/forge/decode_plan.rs` (new): the read program of every codec, positional, CBOR and content-line,
    with the refusal each step can return and the event each step reports;
  - `sce-build/src/forge/walk_plan.rs` (new): the descriptors of a codec's elements, the depth of its paths, and the
    refusal of a codec that has no walk (§8);
  - the printers: `generator/rust_source.rs`, `generator/cpp_source.rs`, `generator/swift_codec.rs`, and the Go,
    Kotlin, Python and C11 printers that ADR 0081 step 4 creates; the decode halves of `render_codec` and
    `generate_decode_expr` (`generator.rs`) and of `tools/codegen/templates/forge/*/codec*.jinja2`, which the printers
    replace;
  - the runtimes, each of which gains a walk module: `backends/rust/forge-runtime/src/walk.rs`,
    `backends/cpp/forge-runtime/include/sce/forge/walk.h`, `backends/c/forge-runtime/include/sce/forge/walk.h`,
    `backends/go/forge-runtime/codec/walk.go`, `Walk.kt` of `com.sce.forge.runtime`, `sce_forge_runtime/walk.py` and
    `backends/swift/forge-runtime/Sources/SceForgeRuntime/Walk.swift`;
  - the option: `generate --walk` (`sce-build/src/bin/sce_codegen.rs`) and `ForgeCompileOptions::walk`
    (`sce-build/src/lib.rs`);
  - the conformance: `tests/forge/conformance/binary_model.py`, `content_line_model.py`, a CBOR reading
    `cbor_model.py` (new), `gen_cases.py`, `numerical_reference.json`, and each backend's harness template under
    `tools/codegen/templates/forge/*/conformance/`.
- Related:
  - ADR 0010 (a codec reads and writes one content-line component) and ADR 0014, 0015;
  - ADR 0049 (a decoded Rust codec is a view over its wire bytes), whose `--views` is the precedent of §1;
  - ADR 0053 (the Rust runtime's features only add, and a bare-metal crate asks for none);
  - ADR 0058 (presented and derived elements, §1), whose derivation kinds are the roles of §3;
  - ADR 0059 (a C++ consumer compiles SCE's interface at C++17);
  - ADR 0065 (declared totality) and ADR 0077 (a declared bound is part of a field's type, and every decode refuses
    what breaks it);
  - ADR 0078 (a consumer follows SCE one landing at a time, and each API change says how);
  - ADR 0081 (a codec's encode is planned once, and each backend prints the plan);
  - ADR 0095 (a fixed field is bits of the whole bytes it spans), which says where a field's bits lie;
  - ADR 0097 (generated code is a build output): `--walk` is an input of a generation action, as `--views` is;
  - ADR 0098 (a failure is not a value), of which §4 is the codec case;
  - ADR 0100 (positional codec extensions: identity dispatch, refusal, fixed fields, empty chains, lookahead repeat),
    the related grammar ADR, whose forms enter the walk by the rules of §3.

## Context

### What a protocol analyzer needs, and what a generated decode gives

A consumer's protocol analyzer (a dissector) shows a frame as a tree: each element with its name, its value, and the
bytes and bits it occupies, so that selecting an element highlights its bytes and selecting a byte names its element.
For a frame the decoder refuses it must still show what was read up to the refusal, and where the refusal was found,
because a refused frame is the one an analyst most needs to see.

A generated decode gives neither. It returns a value, which holds no positions, or a refusal, which holds no value and
no position. So the consumer keeps a hand-written walker of the frame beside the generated decoder: a second reading
of the codec's grammar, written in the consumer's language, which can disagree with the decoder on any frame. The
disagreement is silent: a dissector that draws a field one byte off, or accepts a frame the decoder refuses, says
nothing until an analyst trusts it and is wrong. This is the defect ADR 0081 removed between backends ("a second
reading of the grammar"), present again between SCE and its consumer.

### What was measured on 2026-11-14

- **The decode is not planned.** `sce-build/src/forge/encode_plan.rs` (1,898 lines) holds the write program of every
  codec and no read program: it does not mention a decode. The decode of Rust, C++, Go, Kotlin, Python and C11 is
  written by `render_codec` and `generate_decode_expr` in `generator.rs` and the `codec*.jinja2` templates of each
  language (the C++ printer contributes how a value is made of what the template read), and Swift's by
  `swift_codec.rs`. Six readings of the grammar decode it, where one plan encodes it. A walk "generated from the decode
  plan" therefore needs the decode plan first; §2.1 decides it.
- **The refusal reaches the caller by four routes.** Rust returns `Result<_, CodecError>`; Go returns an `error`;
  Python raises a `CodecError`; C11 returns a status. C++ returns an empty `std::optional` and Kotlin `null`, each
  recording the refusal on the cursor (`SceCursor::refuse`, `lastRefusal`); their runtimes' own comments say some
  refusals (invalid UTF-8) collapse to the truncation sentinel there.
- **The catalog.** `tests/forge/conformance/numerical_reference.json` holds 146 codec entries: 414 `sources` cases
  over 140 codecs, 473 decoded `cases` over 23, 272 decode `rejects` over 15 (each with its typed `error`, kebab-case,
  such as `too-many-elements` or `length-out-of-range`; 241 of them are content-line), and 21 encode `refused` cases
  over 8. `binary_model.py` writes the positional `sources` and `content_line_model.py` the content-line cases and
  rejects; no model reads CBOR, whose cases are written by hand.
- **A precedent for an option that only adds.** `generate --views` (Rust, ADR 0049, SCE_FORGE.md §4.6) adds a
  `{Codec}View` beside every codec and, off, leaves every generated byte where it was
  (`forge_views_are_not_rendered_unless_asked_for`). It is a property of a set of documents: a parent's view names the
  view of each codec it embeds, lists or dispatches to.
- **The word "walk" is taken twice.** ADR 0058 presents a list "by walking it", and ADR 0049's view reads a list
  through `ListWalk` and a marker type `{Codec}{Field}Walk`. The walk of a frame here is a third, distinct thing, and
  its generated names avoid the `Walk` suffix (§6).
- **The Go modules declare `go 1.22`.** The `iter` package and range over a function arrive in Go 1.23, so a Go walk
  is spelled as the function type `iter.Seq` names, not as `iter.Seq` itself (§6).

## The textbook structure

Dissection is a reading of a frame that reports structure and position, not a value. Three established practices
agree on its shape.

- **Wireshark** dissectors add each item to a protocol tree with its offset and length in the frame's buffer, and
  register each header field once with its type, its byte order, its bitmask within the bytes it spans and the names
  of its values. A field that shares bytes with others is a bitmask over those bytes. A frame that runs out or breaks
  a rule ends the tree where it broke, marked malformed: the tree up to the fault is the dissection of a refused
  frame, and no value of the protocol's type is made of it.
- **Kaitai Struct** generates, from the one description its parser comes from, a debug form that records the start and
  end of every attribute it reads (`--read-pos`, `--debug`); the visualizers highlight bytes from those records. The
  positions and the parse are one reading, not two.
- **DFDL** (OGF GFD.207) parses into an infoset delivered as a stream of events (the start and end of each simple and
  complex element and of each array, as Daffodil's `InfosetOutputter` receives them), and measures position in bits,
  with `dfdl:bitOrder` naming the two ways bits are numbered within a byte.

The stream of events over a document is the older structure of SAX beside DOM: a reader that wants positions, or a
document that may be broken, reads events and builds what it needs; a reader that wants a value takes the tree. And a
reader written once for a grammar, and a printer of that grammar, are two printings of one description (Rendel and
Ostermann, "Invertible Syntax Descriptions", Haskell Symposium 2010); ADR 0081 made the write side such a printing,
and §2 makes the read side one.

## Decision

### 1. An option that only appends: `generate --walk`

`sce-codegen generate --walk` (and `ForgeCompileOptions::walk`) adds the walk of §3 to each codec it generates, on
each backend that prints one (the landing order says when). It is named as `--views` is, by what it adds. It never
changes existing output: with the option, every file written without it is written again either identical or with
text appended after its last byte, and the option may add files of its own. In Go, Kotlin and Swift, whose packages
and modules span files, the walk is a new file of the codec's package or module, so its imports stand at its own top
(Go and Kotlin require them before every declaration); in C and C++, where a header's text is compiled into every unit
that includes it, it is a new header beside the codec's, which only a unit that walks includes; in Rust and Python it
is appended to the codec's module. Off, it writes nothing. A property of the whole corpus holds it (Conformance P1).

Like `--views` it is a property of a set of documents: a parent's walk runs the walk of each codec it holds, so every
codec of the set is generated with it. The generator sees one document at a time; a codec generated without it is
found where the parent's walk names a walk that does not exist, by the target's compiler. Under ADR 0097 the option
is an input of the generation action, so a tree generated with it is cached apart.

### 2. The walk is printed from the read program its decode is printed from

#### 2.1 The read program

`decode_plan::plan(model, roles, imports) -> DecodePlan` is computed once per codec, beside the encode plan, from the
same model, the same roles (`codec_roles.rs`) and the same layout (`bit_layout.rs`). It holds no text of any target
language. It holds the reads in the order the decode makes them: a positional layout's minimum-frame check and its
fixed fields at their spans (`bit_layout::BitSpan`), a streamed layout's fields in turn, each byte string or text with
its length and bound, each body with its scope and the inputs it is handed, each list and chain with how it is counted
or ended, each variant with how its tag is found (a field, a parent's flag, a peeked byte, the caller), each gate group
with how its arm is decided, each law a list is held to, and the CBOR and content-line reads of §7. Each read names
the refusals it can return, as the runtime's `CodecError` names them, and the element it returns them at (§3.5).

The reads and the writes are of the same elements, and a test holds them to it: every element the write program
writes is read by exactly one read, nothing else is consumed but what §3 reports as skipped, and a streamed layout
reads its elements in the order it writes them (`the_reads_visit_the_elements_the_writes_write`). A backend's
decode moves onto the read program one codec at a time, as ADR 0081 §6 moves an encode, and its `unlowered` names
the first read form it lacks. A landing that moves a decode changes no generated byte: that unchanged regeneration is
the proof that the program holds what the decode did, as it was for the write program.

#### 2.2 Each read is an event point

Each read of the program reports one event of §3, or a fixed sequence of them, and each scope it opens or closes
reports an enter or a leave. The events are part of the program, not of a printer: which element a read reports, with
which role, at which path, is decided once.

#### 2.3 One printer, two printings

A backend's decode and its walk are printed by one printer from one program. The spelling of each read (the runtime
call, its checks, the refusal it returns) is written by one function of the printer, which both printings call; only
what surrounds it differs: the decode keeps what it read in the value it makes, and the walk reports it. So the walk
cannot read a frame differently from the decode except by a printing defect, which Conformance P2 and P3 find. A
codec whose decode is not yet printed from the program has no walk on that backend: `--walk` refuses it by name
(§8), and never prints a walk beside a decode the template still writes.

### 3. The events

A walk is a sequence of events over one frame, in the order the decode reads it:

| Event | Carries | Reported when |
|---|---|---|
| `field` | path, span, raw value, descriptor | a presented element is read (ADR 0058 §1) |
| `derived` | path, span, raw value, descriptor, role | a derived element is read |
| `arm` | path of the variant, tag, arm, how the tag was found | a variant's arm is chosen, the default arm included |
| `group` | path of the codec, the gate as the document writes it, the arm's index | a gate group's arm is decided |
| `enter` | path, kind, first bit, the scope's end where a length gives it | a body, an arm, a list, a chain, an entry or a component begins |
| `leave` | path, kind, span | it ends, read whole |
| `skipped` | path, span, why | the decode passes over bits that hold no element |
| `refused` | path, bit position, the typed error | the decode refuses the frame; the last event |
| `end` | span of everything read | the decode accepts the frame; the last event |

The **roles** of a derived element are the derivation kinds of `codec_roles::Derivation` (ADR 0058 §1, ADR 0065 §6),
one each: `length`, `count`, `encoded-length`, `variant-tag`, `parent-tag`, `chain-continuation`, `presence`,
`bound-input`, `arm-flag`; and two the other encodings and ADR 0100 add: `key` (a CBOR entry's key, a content-line
property's or parameter's name) and `fixed` (a field whose value the document states and the decode holds the frame
to). The **kinds** of a scope are `body`, `arm`, `list`, `chain`, `entry` and `component`. A **skip** is `unheld` (bits
of a positional layout no field holds, ADR 0095 §2), `alignment` (the padding before a `sce:dma-burst-align` field),
`unknown` (an entry or a line the codec does not declare, where the decode passes over it) or `framing` (the bytes an
encoding writes around its elements: a content-line's delimiters, line ends and its `BEGIN` and `END` lines). The tag
of an arm is found by `field` (its `variant-tag` derived event precedes it), `peek` (with the span of the peeked bits,
which the arm's own header reports again when it reads them), or `handed` (by the caller, or by the parent whose
`parent-tag` derived event precedes it). Each form ADR 0100 adds enters the walk with the landing that adds it to the
read program, by these rules: a read that does not consume (a lookahead, a peek) is no leaf and reports where it
looked; a refusal the document states is a `refused` event with its typed error; a chain of no entries is an `enter`
and a `leave` with nothing between.

#### 3.1 Paths

A path names the element's place in the decoded value: the field ids of the plan's interface from the codec down,
with an index for an entry of a list or a chain and the alias for the arm of a variant. Its text form is
`$` for the codec itself, `.id` for a field or a flag of a carrier, `[i]` for an entry and `:alias` for an arm, the
default arm written `:#tag` by the tag it holds: `$.header.N`, `$.ext[2].len`, `$.body:open.session`,
`$.body:#9.len`. The walk's paths and the value's elements are therefore in one-to-one correspondence, which is what
Conformance P2 holds. A `derived` event with the role `key`, and the `count` of a CBOR map's or array's head, are
reported at the path of the element they key or count, told apart from that element by their kind and role; every
other path names one presented element, one derived element, one variant or one scope.

The path of an event is bounded: its longest form is the codec's **walk depth**, the most steps any element of the
codec's value lies below its root, counted through every codec it holds (`walk_plan::depth`). A step is a field id or
an arm's alias, each a static string of the generated code, an index, or a default arm's tag. On the heap-free tiers
(Rust, C11) a path is a fixed array of that many steps and a length, held by the walk and never allocated; on the
hosted tiers it is the language's list.

#### 3.2 Spans are in bits from the frame's start

A span is a first bit and a number of bits, counted from the first bit of the frame the walk was handed. Fields share
bytes (a header's flags beside its message id; an extension header's id, its continuation flag and its encoding in
one byte), so a byte offset cannot say which of them a bit belongs to, and the walk cannot report less than a bit.

Bit `k` of the frame is bit `7 - (k mod 8)` of byte `k div 8`, a byte's bits counted from its least significant as
ADR 0095 counts them: the first bit is the most significant bit of the first byte. This is the order of a serial
wire, of a protocol diagram and of an analyzer's bit display, and the order in which a big-endian word's bits are
contiguous. A field of 5 bits at bit offset 0 of byte 0 is `@3+5`; a flag at bit offset 7 of byte 0 is `@0+1`; ADR
0095's `reading`, 12 bits at offset 4 of the big-endian word of bytes 1 and 2, is `@8+12`. The byte range of a span is
`start div 8` up to `(start + len)` rounded up to a byte, the same whichever way bits are numbered, so a byte offset
is derived from a span and never stored.

One case is not contiguous in any numbering that serves both byte orders: a field of a little-endian word that spans
bytes and is not whole. Its span is the smallest interval holding its bits, and its descriptor gives the word (first
byte, bytes, shift, bits and order, ADR 0095 §1, read from `bit_layout::BitSpan`), from which the exact bits follow,
as an analyzer's bitmask over a field's bytes does. Every other element's span holds exactly its bits.

#### 3.3 Raw values and descriptors

The raw value is what the wire says before the element's type reads it: the unsigned number a fixed field's bits make
(ADR 0095 §1), the number a variable-length integer encodes, a CBOR integer's argument (with its major type, so a
negative integer is reported as CBOR states it, `-1 - n`), or, for a byte string, a text, a CBOR string's content or a
content-line value, the span of its bytes in the frame. A span and not a copy: an event then holds no borrow and
allocates nothing, and the consumer reads the bytes from the frame it already holds. A content-line value's span is
its bytes as folded and escaped on the wire; the text the decode makes of it is the runtime's one unfolding and
unescaping of those bytes, which the decode and the walk's consumers both call.

What reads a raw value as a value (a sign, an enum's variant, a quantity's unit and scale, a length's
`sce:length-arith`, UTF-8, an unescape) is the element's type, and the event carries the element's **descriptor**: a
static record the walk appends, one per element, holding its id, its abstract type as the plan's interface names it,
its role and the elements a derived one is derived from, the word of a fixed field (§3.2), the variants of an enum
with their numbers and names, and a quantity's unit and scale. A dissector that shows a value by name reads it there,
and does not keep its own copy of the enum, which could disagree.

#### 3.4 Order and nesting

Events are reported in the order the decode reads, which the read program states; a streamed layout's leaves
therefore rise, and a positional layout's fixed fields come in the order the decode reads them, which a consumer that
draws by position sorts by span. Every `enter` is matched by a `leave` unless a `refused` event comes first. A
carrier's flags are reported after the carrier's own event, each within its span: they refine it. The leaves are the
`field`, `derived` and `skipped` events other than the flags of a carrier; their bits partition the bits the walk
read (Conformance P5).

#### 3.5 A refusal is the walk's last event

Where the decode refuses the frame, the walk reports `refused` with the same typed error, the bit position the read
had reached when it found the refusal, and the path of the element whose type or rule the frame breaks: a bound at
the element it bounds (ADR 0077), an undeclared enum value at the enum's field, a chain past its depth at the chain,
invalid UTF-8 at the text, malformed CBOR at the item, and running out of bytes at the element being read, the
codec's root for a positional layout's minimum-frame check. Where the element was read whole and it is its value that
breaks the rule (a number no variant of a closed enum declares, a count past its list's bound, a fixed field that
differs from the document's value), the element's own event comes first, so the analyzer can show the number it
refuses. Nothing follows: no `leave` for the scopes left open, since a `leave` says its scope was read whole, and no
`end`.

The walk shows what the decode read and no more. A positional layout checks its minimum frame before any field, as
the decode does, so a frame shorter than the layout is refused at bit 0 with no field event; the descriptors hold the
layout's static spans, so an analyzer can draw the fields the frame lacks.

### 4. A refused decode yields no value

No decode returns, raises with, or exposes a partly decoded value of the codec's type. A record half filled would say
of its unread elements whatever they held before, and its type would claim a frame that was never read; ADR 0098
states the rule for every type, and this is its codec case. The walk is how a refused frame is seen: everything the
decode read is in its events, with its position, up to the refusal. `decode_into` keeps its signature; after a refusal
its destination holds no value of the frame and is storage the caller may reuse, not read, which SCE_FORGE.md §4.6
states with the Rust landing.

### 5. A decoded value can say where its elements lay

A consumer that decodes and then wants positions does not walk by hand. With the option, each backend offers a
spanned decode, `decode_spanned`, which returns the value together with the frame and the arguments it was decoded
with, and `span_of(path)` on it, which walks the frame to the event of the element at `path` (a presented element's
`field`, a derived element's `derived`, or a scope's `leave`; not a key or a head's count, which share the path of
what they key or count) and returns that span, or none for a path the frame does not hold. It is derived from the
walk, so it cannot disagree with it, and it stores nothing that grows with the frame, so it serves the heap-free
tiers. A consumer that asks for many spans walks once instead. The path is the text form of §3.1, so a consumer
names an element as the document does.

### 6. The shape in each backend

| Backend | The walk | An event's path | A refusal |
|---|---|---|---|
| Rust | `{Codec}::walk(frame, inputs.., tag?) -> {Codec}Walker<'_>`, an `Iterator` | `Path<D>` by value, `D` the walk depth | `Event::Refused { error: CodecError, .. }` |
| C++ (C++17) | `{Codec}::walk(frame, inputs.., visit)`, `visit` any callable taking `const Walk::Event&` and returning `bool` | a `const Path&` valid during the call | the event, and `walk` returns whether it ran to its last event |
| C11 | `{codec}_walk(frame, len, inputs.., visit, ctx)`, `visit` a function pointer returning `bool` | a pointer to the walk's stack array of `{CODEC}_WALK_DEPTH` steps | the event, with the status the decode returns |
| Go | `Walk{Codec}(frame, inputs..)` returning `func(yield func(codec.WalkEvent) bool)` | a slice valid during `yield` | the event, with the `error` the decode returns |
| Kotlin | `walk(frame, inputs..): Sequence<WalkEvent>`, by `sequence {}` | an immutable list | the event, with the `CodecError` the cursor records |
| Python | `walk(frame, inputs..)`, a generator of `WalkEvent` | a tuple | the event, with the `CodecError` the decode raises |
| Swift | `walk(frame:inputs:) -> {Codec}Walker`, a `Sequence` and `IteratorProtocol` | an array | the event, with the error the decode throws |

A pull iterator is given where the language has a cheap one: Rust's `Iterator`, Kotlin's `Sequence`, Swift's
`Sequence`, Python's generator, and Go's iterator function, whose type is the one `iter.Seq[codec.WalkEvent]` names,
so a Go 1.23 caller ranges over it and a caller of the declared `go 1.22` calls it with a function. C and C++ take a
visitor: a callback that returns whether to go on. The walk never throws or panics: a refusal is an event, and the
iterator ends after it.

Kotlin's `sequence {}`, Python's `yield` and Go's `yield` print the read program straight, with an event at each
point. Rust and Swift have no stable generator, so their printers lower the program into a resumable machine: one
state for each point between two events, the locals the decode keeps (counts, lengths, scope ends, the flags it read,
an entry's index) as fields, and the walker of the codec it is reading held inline, which is the transformation those
languages' own iterators and coroutines are compiled by. The walker of a codec holds the walkers of the codecs it
holds, so its size is fixed by the codec and needs no allocation. C and C++ print the program straight, calling the
visitor.

On the heap-free tiers no event allocates. The Rust walk lives in `sce_forge_runtime::walk`, behind no feature (ADR
0053), uses only `core`, and its event is a value: the path a fixed array, the raw value a number or a span, the
descriptor a `&'static` reference. The C11 walk keeps its path on its own stack and hands the visitor a pointer for the
call's duration. A Rust walker's type does not end in `Walk`, so it cannot meet the view's `{Codec}{Field}Walk`.

### 7. The three encodings walk, each at the level of its own elements

**Positional is first-class.** Every bit is reported, at the bits of §3.2; this is where fields share bytes and where
an analyzer needs the walk most.

**CBOR walks at the level of its items, with byte spans.** A map's head is a `derived` `count` at the map's path, each
entry's key item a `derived` `key` at the entry's path, and each value item a `field` whose span is the whole item, its
head and its content, and whose raw value is the item's argument or the span of its content; an array of records is a
`list` of `entry` scopes, its head a `derived` `count`; an entry of a key the codec does not declare, which the decode
skips, is `skipped` `unknown`. Every span is a whole number of bytes. The walk does not report the bits inside an
item's head (its major type and its additional information): those are the encoding's own framing, the same for every
CBOR document and read from the item's first byte by RFC 8949 §3, so a generic CBOR reading reports them better than
a codec could, and a codec's document says nothing of them. The items tile the map, so the partition of Conformance P5
holds.

**Content-line walks at the level of its properties and parameters, with byte spans.** A component is a `component`
scope from its `BEGIN` line to its `END` line, both reported as `framing`; in each line the property's name is a
`derived` `key`, each parameter's name a `derived` `key` and its value a `field` at the parameter's entry, the
property's value a `field` at its entry (a repeated property's at its index), and the delimiters and the line end
`framing`. A fold that falls inside a value is inside that value's span; the raw value is the span (§3.3). The
partition of Conformance P5 holds over bytes.

Leaving CBOR and content-line out was considered and refused: their decoders refuse too (241 of the catalog's 272
decode rejects are content-line), the consumer's second reading exists for them as for any frame, and the rules above
need nothing that positional does not already need. Walking them at bits was refused for the reason given: below an
item or a token there is no element of the document.

### 8. What has no walk

`--walk` refuses by name, as `generate/unsupported-feature` with the clause naming the reason: a codec on a backend
whose walk has not landed; a codec whose decode on that backend is not yet printed from the read program (the clause
names the first read form it lacks, §2.3); and a codec whose imports close a cycle, which has no walk depth, so no
bounded path. A codec that holds a refused codec is refused at the element that holds it, as ADR 0081 refuses an
encode. Nothing is walked by a reading other than the read program.

## Conformance

Each property below runs on every backend from the landing that gives the backend its walk.

- **P1. The option only appends.** For every codec document of `tests/forge/resources` and every backend that prints
  a walk, generating with `walk` writes each file of the plain generation again, identical or extended after its last
  byte, and adds only the walk's own files; off, the output is the plain output (`forge_walk_only_appends`, over the
  whole corpus, beside `forge_walk_is_not_rendered_unless_asked_for` for the shapes it adds most to).
- **P2. The walk reproduces the decoded value.** For every frame of the catalog a decode accepts (each `sources` case's
  `encoded` and each decoded `cases` entry), the presented elements the decoded value holds and the walk's `field`
  events are in one-to-one correspondence by path, and each event's raw value, read by its descriptor's type, is the
  element; the walk ends with `end`, whose span is the bits the decode consumed. The harness reads each element
  through the value's own accessor, which the generator prints from the plan's interface.
- **P3. The walk refuses as the decode does.** For every `rejects` case, the walk's last event is `refused` with the
  error the decode returns on that backend, which is the case's `error`, and no `end`. On C++ and Kotlin the decode's
  error is the cursor's record; the landing that prints their decode from the read program first records every
  refusal the program names there, so the record is the typed error on every frame.
- **P4. Every backend walks a frame alike.** The independent models write each case's expected walk into the catalog,
  as lines of the text form below, from the page and the document alone (a model walks a frame it did not write, a
  hand-written decoded case or reject, as it walks its own), and each harness compares its walk with them line by
  line. A disagreement is decided against the page, as a byte case's is (ADR 0081 §5).
- **P5. The leaves partition what was read.** For every case, the exact bits of the leaf events of §3.4 (a
  non-contiguous field by its word) are pairwise disjoint and their union is the span of `end`, or the bits before the
  refusal's position for a refused frame; every scope's children lie within its span.
- **P6. `span_of` is the walk.** For every case a decode accepts and every path the walk reports, `span_of(path)` on
  the spanned decode returns the span of that path's event.
- **P7. No allocation on the heap-free tiers.** A `#![no_std]` crate without `alloc` walks every codec of the corpus
  (`walk_compiles_without_alloc`), and the C11 harness runs the walk with no allocator linked.
- **The read program.** `the_reads_visit_the_elements_the_writes_write` over every codec of the resources, and
  `every_refusal_a_decode_returns_is_named_by_its_read` (each refusal kind the decode can return is named by a read of
  its program, and no other).

The text form of an event, which the catalog and the harnesses share, is one line: `field <path> @<start>+<len> =
<raw>`, `derived <path> <role> @<start>+<len> = <raw>`, `arm <path> <tag> <alias|default> <field|peek|handed>`,
`group <path> "<gate>" <index>`, `enter <path> <kind> @<start>[/<scope end>]`, `leave <path> <kind>
@<start>+<len>`, `skipped <path> <why> @<start>+<len>`, `refused <path> @<at> <error>` and `end @0+<len>`; a number is
decimal, a negative CBOR integer `-1-<n>`, and a span of bytes is written `h'..'` with its bytes in hex, after CBOR's
diagnostic notation (RFC 8949 §8). It is a test notation of the catalog, not a wire surface: a consumer reads events,
not their text.

## Landing order per backend

Each step is one or more landings; each landing keeps every lane green and changes no generated byte unless it says
so.

1. **This ADR.**
2. **The read program, and the Rust decode printed from it.** `decode_plan.rs` lifts the decisions of the Rust decode
   one form at a time: a positional layout's fixed and streamed leaves; bodies, lists and chains; variants by every tag
   source, gate groups and laws; then CBOR; then content-line. Each landing regenerates the Rust trees and changes no
   file. Tests: `sce-build/tests/decode_plan.rs` pins what the program says of each shape, with
   `the_reads_visit_the_elements_the_writes_write` and `every_refusal_a_decode_returns_is_named_by_its_read`; the
   census `a_rust_decode_is_printed_from_its_plan_or_refused_by_name` pins the printed codecs.
3. **The walk's plan and the Rust runtime.** The event of each read, the descriptors, the walk depth and the refusals
   of §8 (`walk_plan.rs`); `sce_forge_runtime::walk` with the event, path, span, raw value and descriptor types and the
   text form. Tests: `sce-build/tests/walk_plan.rs` (`a_walk_depth_is_the_longest_path_of_its_value`,
   `a_codec_whose_imports_close_a_cycle_has_no_walk`, `every_derived_element_reports_the_role_of_its_derivation`); the
   runtime's unit tests (`a_span_names_the_bytes_it_touches`, `a_path_is_written_in_the_text_form`).
4. **The models write the walks.** `binary_model.py` and `content_line_model.py` write each case's expected walk, and
   `cbor_model.py`, a reading of RFC 8949 and SCE_FORGE.md §4.6.1, writes the CBOR codecs' walks and holds their
   hand-written cases to itself. `gen_cases.py --check` holds the catalog to the models.
5. **Rust.** The Rust printer prints the walk, `decode_spanned` and `span_of`; the option and its flag; the goldens of
   `expected/walk/`, one per shape; the Rust harness runs P2 to P6 on every case and P1 and P7 hold for Rust.
   SCE_FORGE.md gains §4.6.5 and its `decode_into` sentence (§4), and `docs/SCE_CODEGEN_DETERMINISM.md` §1 its line.
   `docs/SCE_CODEC_CONSUMER_MIGRATION.md` says how a consumer replaces a hand-written walker (ADR 0078); nothing
   existing moves.
6. **C++.** Its decode printed from the read program (no file changes); then every refusal recorded on the cursor (a
   change a consumer sees: a refusal that collapsed to the truncation sentinel now names itself, and the landing says
   how to read it); then the walk, by a visitor, and `sce/forge/walk.h`. That header is an input of the embed manifest,
   so the landing that adds it emits `embed/MANIFEST.json` and `embed/MANIFEST.inputs` with it. The C++ harness runs
   P2 to P6.
7. **Go**, with the Go printer of ADR 0081 step 4: whichever of its decode and its encode lands first creates the
   module. Then the walk, as an iterator function.
8. **Kotlin**: its decode from the read program; every refusal recorded on the cursor, as for C++; then the walk, by
   `sequence {}`.
9. **Python**: its decode from the read program; then the walk, by a generator.
10. **C11**: its decode from the read program; then the walk, by a visitor, its path on the walk's stack, and P7 run
    with no allocator.
11. **Swift**, once its codec lowers the write and read programs (ADR 0081 step 5): the walk by a resumable machine,
    as Rust's.

C++ follows Rust for the reason ADR 0081 gives: it is the engine's language, and a visitor over a template parameter
tests the program against a second idiom early. C11 comes after the hosted backends because a plan four printers have
read is the one its heap-free spelling should meet. Swift comes last because no plan reaches it yet.

## Consequences

- An analyzer reads the generated walk and drops its own walker: the positions it draws and the frames it refuses are
  the decoder's, on every backend, and a frame the decoder refuses is shown up to the refusal with its typed error.
- The decode becomes a printing of one program on every backend, as the encode did under ADR 0081. That is the larger
  part of the work and stands on its own: a decode rule is then written once, and a decode parity defect is a printing
  defect the byte cases locate.
- C++ and Kotlin name every decode refusal, where some collapsed to the truncation sentinel; a consumer that tested
  only for the sentinel is unaffected, and one that reads the cursor's record reads more.
- The catalog gains a walk for every case, which the models write; the harnesses grow by the properties.
- A tree generated with `--walk` is larger: descriptors, and on Rust and Swift a resumable machine per codec. It is
  paid only by a consumer that asks for it, and never by a firmware that does not.
- The walk shows what the decode read and no more: a positional frame shorter than its layout shows no field. This is
  the price of one reading, and the descriptors' static spans let an analyzer draw what is missing.

## Alternatives rejected

- **Leave the walker to the consumer.** It is the defect: a second reading of the grammar, in another language, that
  can disagree with the decoder silently.
- **Print the walk from a plan while the decode stays on its template.** Two readings again, inside SCE: they agree
  only where a case holds them together, and the read program would be a third description of the decode.
- **A table the runtime interprets.** One generic walker per runtime over a table the generator emits would be a
  second reading of every decode rule in seven runtimes, for the reason ADR 0081 refused one encoder in C.
- **The decode as a fold over the walk.** It would make the two one by construction, but it changes every decode
  (against §1), puts a dispatch per element into every decode on the backends whose compilers do not specialize a
  visitor away (Go, Kotlin, Python), and has no place for ADR 0049's view, which stores no events.
- **Positions recorded in every decoded value.** Kaitai's debug form does this, in a build that asks for it; every
  value of that build grows by a record per element, every decode pays for it, and a refused frame is seen only as
  the partly filled value §4 refuses.
- **A partial value on refusal.** A record whose unread elements hold whatever was there before is a value its type
  does not describe (§4, ADR 0098).
- **Byte spans only.** Fields share bytes; an analyzer would draw a header's flags and its id as one item.
- **Bits numbered from the least significant end.** It matches ADR 0095's counting within a word and makes a
  little-endian word contiguous instead of a big-endian one. The frame's order on a serial wire, protocol diagrams and
  analyzers number from the most significant bit, and the byte range is the same either way; the one case left
  non-contiguous carries its word in its descriptor.
- **Events without paths, the structure kept by the consumer from the enters and leaves.** Lighter by a bounded copy
  per event, but each consumer would rebuild the path and could build it wrong, and `span_of` and P2 key on it.
- **A generated analyzer plug-in.** It would serve one tool. The walk and its descriptors are the neutral layer from
  which such a plug-in, if one is ever wanted, is a thin printing.

## Questions left open

None. Decided 2026-11-14; the first step is 2 above. The role `fixed` and the forms of ADR 0100 enter with that ADR's
landings, by the rules of §3.
