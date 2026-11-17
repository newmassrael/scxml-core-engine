# ADR 0081 — A codec's encode is planned once, and each backend prints the plan

- Status: Accepted (2026-11-03). The owner ordered the work started on the recommendation of that day, under the rule
  of 2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored.
- Date: 2026-11-03
- Amends:
  - ADR 0058 §6, which has each other backend take the source interface "in its idiom" in a later step. The idiom is
    kept. It becomes the spelling of one plan, not a sixth reading of the grammar.
  - ADR 0065, Order of work 4, which reaches the other five backends one at a time with the same signatures. The
    refinements of its §6 reach each backend through the plan, not as a second run of R1 to R8.
- Scope:
  - `sce-build/src/forge/encode_plan.rs` (new) and its submodules: the plan, built from `CodecModel`, `CodecRoles`
    (`codec_roles.rs`) and `Totality` (`codec_totality.rs`);
  - `sce-build/src/forge/generator/rust_source.rs` and its submodules, which become the Rust printer of the plan;
  - one printer module per backend, and the encode halves of `render_codec` (`generator.rs`) and of
    `tools/codegen/templates/forge/{cpp,c,go,kotlin,python}/codec*.jinja2`, which the printers replace;
  - the forge runtimes of those backends, which gain what the plan names (a counting sink, the bounded and lawful
    types, the walk);
  - `tests/forge/conformance/numerical_reference.json` and `fixtures.json`, which gain byte cases for every shape the
    plan has.
- Related:
  - ADR 0010 decision 2 (byte parity between backends);
  - ADR 0058 (the source interface, the one encode, its laws 1 to 5);
  - ADR 0059 (a C++ consumer compiles the generated interface at C++17);
  - ADR 0061 (`Measured`, the witness that a write cannot fail);
  - ADR 0065 (declared totality and the refinement program);
  - ADR 0077 (a declared bound is part of a field's type);
  - ADR 0078 (a consumer follows SCE one landing at a time, and each API change says how).

## Context

ADR 0058 gave the Rust backend a source trait, one `encode_source`, and the rule that the encoder derives what the
encoding determines. ADR 0065 then made the Rust encode total over refinements of that trait. The other five
backends still encode their builder struct. A survey of the tree on 2026-11-03 found the following.

- **The Rust encode has no plan between analysis and text.** The neutral decisions are made in `codec_roles::classify`
  and `codec_totality::analyse`, and `rust_source.rs` (4,764 lines) turns them straight into Rust strings. The order
  of writes (`write_body`), the dispatch over the nine `Derivation` kinds, the emission of each check where
  `Totality::refuses` names it, the gate groups and their arms, the chain membership keys, the list laws and the
  threading of inputs from parent to child are all decided in the same functions that spell `Result`, `?`,
  `CountSink` and `Lawful<..>`. Its `Derived` records hold Rust local names. `Totality::require` returns the Rust
  text `CodecError::{variant}`.
- **The other backends write derived elements from the struct.** A count, a length, a tag and a chain's continuation
  flag are read off `self` (the C++ golden `codec_length_ref.h` writes `w.write_u8(len)`). What they do derive is a
  parent-tag carrier bit, the inputs a chain hands the entry after a marker, and the `chain.has` locals.
- **The other backends decide refusal by their own route.** `render_codec` computes
  `encode_can_refuse = refuses_mismatch || !bound_lines.is_empty()`, checks every gate in `roles.checks` (not
  `source_checks`), and emits bound guards of its own (`encode_bound_guard_lines`). None of the five reads
  `codec_totality::may_fail`, so a codec ADR 0065 has made total still returns a failure on every backend but Rust.
- **Parity is held for few shapes.** Of 124 codec fixtures, 22 carry byte cases in the cross-backend oracle. They are
  all positional frames, CBOR or content-line. No gate, gate group, chain, chain membership, variant default arm,
  derived flag or list law is compared by bytes between backends. The golden test compares each backend's text with
  itself.
- **None of the five runtimes has a counting sink, a bounded list or text type, or a walk.** All of them have an
  abstract sink.

Porting `rust_source.rs` to each backend would write the grammar's encoding rules five more times, each in the
functions that spell one language. ADR 0058 §2 removed exactly that shape for three representations of one backend
("a wire rule written there reaches every representation by construction"). The same reasoning applies to backends.

## The textbook structure

A codec generator is a compiler. Its source language is the codec document. Its front end is the parser and the
semantic analysis: the model, the roles of §1 of ADR 0058, and the totality of ADR 0065. Its targets are the six
backends. A compiler for several targets lowers the analysed program once, into an intermediate representation that
holds every decision the targets share and none of their spelling, and each target is a printer of that
representation. This is the textbook arrangement (Appel, *Modern Compiler Implementation*, ch. 7, "Translation to
Intermediate Code"), and the one LLVM is built on (Lattner and Adve, CGO 2004). Data-description languages that emit
several languages are built the same way: PADS (Fisher and Gruber, PLDI 2005) compiles one description through one
internal representation into its parsers and printers, and ASN.1 compilers lower a module once and print it for each
target language.

The refinements of ADR 0065 cross languages in the same way. Where a type system can carry the refinement (Rust's
const generics, C++ template parameters), the refinement is a type. Where it cannot (Go, Kotlin, Python, C), the
refinement is an abstract type: its constructor is the only way to make a value, so the check runs once, where the
value is made, and everything after is total (Liskov and Zilles, "Programming with Abstract Data Types", 1974; King,
"Parse, don't validate", 2019). In both cases the encode over the refinement is total. Only where the refusal is
checked differs, and the plan says which refusal each type takes over.

## Decision

### 1. The plan

`encode_plan::plan(model, roles, totality, imports) -> EncodePlan` is computed once per codec. It holds no text of any
target language, and the module imports nothing that spells one (`LangCtx` included). It holds:

1. **The interface.** The presented elements of ADR 0058 §1, in declaration order, each with an abstract type: an
   integer of a width and signedness, a flag, an enum, bytes, text, an embedded body of a codec, a list or a chain of
   entries of a codec, a variant with its arms and its default arm, a gate group with its arms (ADR 0065 R4), and the
   `Option` of a gated element. A length or count stated while its content is absent (ADR 0058 §1) is an element of
   its own kind.
2. **The refinements.** For each element, the refinement ADR 0065 §6 gives it: bounded bytes and text (R1), a bounded
   list (R2), the default arm's tag (R3), the gate sum (R4), the shared count (R5), the chain membership pair (R6),
   the non-empty chain (R7), and the list law with its automaton (R8). Each refinement names the refusal it takes
   over from the encode.
3. **The derivations.** For each derived element, an expression over presented elements: the length or count of an
   element, the encoded length of a body, an arm's tag, a parent-tag carrier bit, a presence, a list's start
   (R8), and a chain's continuation. The expression language is closed: a printer spells each form and decides none.
4. **The checks.** Exactly the refusals `Totality` keeps, each as the refusal kind, the element and the predicate. A
   printer emits these checks and no other. A refusal that a refinement takes over is not a check.
5. **The inputs.** The flag inputs a caller supplies (a parameter of the encode), the inputs the value says (ADR 0065
   R4e), and for each child the arguments the parent hands it.
6. **The program.** The writes in wire order: the fixed fields packed at their offsets, an integer, bytes, text, a
   child with its inputs and overrides, a walk of a list or chain with the inputs each entry is handed and its
   continuation, a match over a variant's arms, and a gated step. Where a child can refuse, the program runs over a
   counting sink first (ADR 0058 §3).
7. **The totality.** Whether the encode can refuse, and, when it can, the refusals it can return.

A backend that computes any of these from the model is a defect. The test of the plan is that every printer reads the
model only through the plan.

### 2. A backend is a printer of the plan

Each backend has one printer module that spells the plan in its idiom:

| Backend | Interface | A list | A refinement | A refusal |
|---|---|---|---|---|
| Rust | a trait | an iterator, `Clone` | a type (`Bounded`, `Lawful`, ...) | `Result` |
| C++ (C++17, ADR 0059) | a template parameter, its requirements checked by a generated trait and `static_assert` | a begin and end pair | a class template | `std::optional<CodecError>` |
| Go | an interface | `iter.Seq` | a type with an unexported field and one constructor | `error` |
| Kotlin | an interface | `Sequence` | a class with a private constructor and a factory | `CodecError?` |
| Python | a `Protocol` | an `Iterable` | a class whose constructor checks | an exception |
| C11 | a table of functions over a context pointer | a begin and next pair | a struct made by one function that returns a status | a status code |

A total codec's encode carries no refusal in any backend: it returns the bytes, or only the sink's failure where the
sink can fail (ADR 0065 §4). A codec that can refuse returns the backend's refusal, as the table says.

Swift's codec (`swift_codec.rs`) moves onto the plan when it lowers what the plan holds. Until then it keeps the
refusal by name it has today.

### 3. Every backend derives what the encoding determines

As a backend's printer lands, its builder loses the stored derived elements: a count, a length, a known arm's tag, a
continuation flag, and a gate flag of the one-flag form. The encoder computes them, as Rust's has since ADR 0058. This
is a change a consumer of that backend sees, and the landing says how to move, per ADR 0078.

### 4. One analysis decides refusal

The backend-specific routes are removed as each backend moves: `encode_can_refuse`, `encode_bound_guard_lines`, and
the mismatch checks over every `roles.checks`. The Rust spelling in `Totality::require` moves into the Rust printer,
and `Totality` keeps the refusal kind.

### 5. Parity is held by bytes, for every shape

ADR 0058 §5 law 4 holds only where there are byte cases. The oracle gains hand-computed byte cases for every shape the
plan has: a gate of each form, a gate group, a chain with and without membership, a variant's default arm, each
derived flag, and a list law. Each case is written from the document and not taken from any backend's output, so the
oracle stays independent of the printers. A shape enters a printer together with its cases. A printer runs every case
of every shape it prints.

*Designed at step 3 (2026-11-03): the notation, the model and the harness.*

- **A case states a value as the plan's interface presents it.** Beside `encoded`, a case of a positional codec carries
  `source`: one member for each element the value presents, named by its field id. A derived element (a count, a
  length, a known arm's tag, a derived flag) is not a member, since no value states it. A scalar is a number (an
  enum's underlying number, a flag set's whole carrier), bytes are an array of numbers, text is a string, an absent
  element is `null`, an embedded body is an object of the body's codec's members, a list or chain is an array of such
  objects, and the variant is `{"arm": <the arm codec's alias>, "body": {...}}`, or `{"tag": n, "body": {...}}` for
  the default arm. A gate group is an object of its members with `"arm"`, the index of the arm the value holds. The
  catalog's hand-written field lists (`fixtures.json` `fields`) state a builder's struct; `source` states the abstract
  syntax, and the plan, not the catalog, says which elements it has.
- **The bytes are computed by an independent model.** `tests/forge/conformance/binary_model.py` is a reading of the
  positional rules of SCE_FORGE.md, written from the page and the fixture's document alone, as
  `content_line_model.py` is for content lines: it derives every derived element from the `source` and writes the
  frame, and it decodes the frame back to the `source`, so each case holds against the model before it is written.
  A backend that disagrees with a case reads the page differently from the model, and which is wrong is decided
  against the page.
- **A printer's harness writes each case as a literal of its ready-made source.** The conformance generator reads the
  cases at generation time, as it already does for rejects, and a printer that prints the plan writes each `source`
  as a literal of its idiom's ready-made source (Rust `XParts`, docs/adr/0058 §4), so no harness parses a nested
  value at run time. The harness encodes it and compares the bytes (law 4), then decodes the bytes and compares what
  the view presents with the literal (law 1).
- **A backend that has not taken the plan keeps its `decoded` cases.** Its builder stores derived elements, which a
  `source` does not state, so it runs the cases of its catalog fields and is compile-only for the rest until its
  printer lands (§6).

*Landed (step 3, first part, 2026-11-03).* `binary_model.py` reads fixed-width fields, a flag set presented whole,
fields that share a byte, and a field gated on one flag, whose flag it derives. It writes the `sources` of
`codec_present_if_basic` and `codec_flags_basic`, nine cases, each held by the model; `gen_cases.py` runs it. The
generator folds each `source` into the Rust harness as a `{Codec}Parts` literal, which the harness encodes, compares
with the bytes, decodes and compares element by element. A body, a list, a variant, a gate group and an enum are
refused by name by the literal and by the model until their parts land.

*Landed (step 3, second part, 2026-11-03).* The model reads the leaves: a VLE (Appendix B of the synthesis RFC), a
flag set written as a VLE, bytes and text of a `length-ref` (a sibling or a flag of a carrier, less its
`sce:length-arith`) or a `tail`, each within `sce:max-size`. A length gated with its content is derived with it; a
length stated where its gated content is absent (ADR 0058 §1) belongs to a group with that content (§4.6, R4b), so it
waits for the group part. The model refuses by name any attribute, root attribute or child it does not read, so a
rule a document states is never left out of a case. It writes the `sources` of every codec of the catalog that
imports nothing, takes no flag input and holds no group (45 codecs, 120 cases), adding the entry to
`numerical_reference.json` where it had none; the bytes it writes agree with the examples the documents give by hand.
The harness also holds that the decode reads the frame to its end. The literal names its types by the label the
harness generated the codec under.

*Landed (step 3, third part, 2026-11-03).* The model reads the codecs a codec imports, each from its own document: an
embedded body (its length said by a field before it where `sce:length-from` names one), a list counted by a field
(`count=`) or read to the end of its scope (`until-eof`), and a chain that ends where its scope does. A count and a
body's length are derived as a byte length is, and the decode reads within the scope its holder gives it. It writes the
`sources` of ten more codecs (21 cases): a body is stated as its codec's value and a list as an array of them. The
literal reads each held codec's plan from its document (`encode_plan_file_with_imports`), binds each body and array of
entries before the value that refers to them, and the harness compares a decode with the literal element by element
down into each body and entry, since a held codec is presented by an interface and not compared whole. A gated list, a
list held to a law, a chain that ends on an entry's flag, a codec that takes a flag input, a variant and a group are
refused by name until their parts land.

*Landed (step 3, fourth part, 2026-11-04): gate groups.* A gate group is the member of `source` keyed by its gate as
the document writes it (`"flags.has_payload"`, `"flags.wants_a || flags.wants_b"`), whose value is `{"arm": i, ..}`
with the arm's members, or `null` where the value may hold no arm. The page names the arms and not their order, so
the notation fixes it: for one flag, the side where it holds and then the side where it does not, each where it holds
an element; for a disjunction, the assignments of its flags that satisfy it, those with fewer flags set first and among
them the one setting the earlier flags the gate names. The model forms a group as §4.6 R4b and R4c do (two elements a
flag decides, a stated length on the side without its content; a disjunction of the codec's own flags no other gate
reads) and derives every flag of it from the arm. Three codecs gain 13 cases. The literal holds the arm as the plan's
group type and arm name, and the harness compares a group whole. A group of a flag input or of a chain's membership,
one holding a body or a list, and one aligning two lists of one count are refused by name.

*Landed (step 3, fifth part, 2026-11-04): flag inputs.* A case of a codec that declares `<sce:flag-inputs>` states
`inputs`, beside `source`: the value of each input its parent hands it, `{"name": 0 | 1}`. The model reads a gate on an
input as a clause the parent's value decides, and a group on one (R4b) as on a flag, keyed by the input's name. The
harness hands the inputs to the decode in the order the codec declares them, and holds the value to saying each one it
says (`{Codec}Wire::input_x`, docs/adr/0065 §6 R4a), so a case whose inputs the value would contradict fails. Eleven
codecs gain 25 cases. An input the value does not say, which the encode takes as an argument, and a held codec whose
inputs its parent binds are refused by name.

*Landed (step 3, sixth part, 2026-11-04): variants.* The model reads a `<sce:variant>` whose tag is a field or a flag
of a carrier before it: the tag is derived from the arm, a known arm's the document's and the default arm's the one it
holds, which must be no arm's and fit the tag's bits; the decode reads the tag and the arm's body within its scope.
The variant is the source's member `body`, `{"arm": alias, "body": ..}` or `{"tag": n, "body": ..}`. The literal holds
`{Codec}PartsBody`, the default arm's tag as `{Codec}UnknownTag::new(n)`, and the harness compares the arm, its body
down into its elements and the default arm's tag. Four codecs gain 13 cases. A variant whose caller supplies its tag
or a peeked byte decides it, an arm under a build feature, and an arm's codec that takes flag inputs are refused by
name.

*Landed (step 3, seventh part, 2026-11-04): chains that end on an entry's flag, and gated lists.* The model writes a
chain with `terminate-on="entry-flag"` with the flag set on every entry but the last, derived as ADR 0058 §1 says, and
reads entries until one has it clear; such a chain holds one entry or more. A list or a chain gated by `sce:present-if`
is present where its gate holds, its gate flag and a count gated with it following it. A case states each entry's
flag as the chain writes it, so a case and its decode agree. A flag's `value` names the builder's default and is read
as nothing of the wire. Sixteen codecs gain 32 cases, the Zenoh put, del, err, reply, OAM, query, final response,
hello and push among them, each frame with every element it derives written by the model. Two lists of one count (R5)
are refused by name.

*Landed (step 3, eighth part, 2026-11-04): flag binds.* The model reads an import's `<sce:flag-bind>`: the parent hands
the held codec each input it binds, its own input passed on or a flag of a carrier before the body. A flag of a carrier
so bound is derived (docs/adr/0065 §6 R4a): where the body is held and says the input, the flag is what it says; where
the body is absent or says nothing, the flag is the value's own. A case states the carrier as the frame writes it, so
the case and its decode agree. Five codecs gain 12 cases: the Zenoh key-expression, subscriber, token and queryable
declarations, each handing its `N` to the key expression it embeds, and the interest body, whose flags byte's `N` the
embedded key expression says. A held codec whose inputs a list's entries or an arm's codec take stays refused by name.

*Landed (step 3, ninth part, 2026-11-04): arms that take flag inputs.* An arm of a variant, the default arm among them,
is handed each input its import binds, as an embedded body is; a body and an arm share one reading of the binds and
one handing on. A flag of a carrier bound to an input of the arm the value holds is what that arm says, and the
value's own where the arm binds it not or does not say it (docs/adr/0065 §6 R4a, the plan's `ArmSays`). Four codecs
gain 17 cases: the Zenoh declaration, whose arms are handed the header's N and Z, the declare message that embeds
it, the transport envelope, whose arms are handed A and S, and the init-syn envelope, whose default arm takes S too.
A header that contradicts the arm it holds does not read back as written, so the model refuses it as a case.

*Landed (step 3, tenth part, 2026-11-04): groups that hold a body or a list.* A body or a list a gate group holds is
stated in its arm as an element is, and the literal binds it first, as it does an element's. The harness compares a
group arm by arm: the same arm on both sides, or none, each member compared as an element is, a body by its elements
and a list by its entries in order; an element and a group's member share the one comparison. Three codecs gain 10
cases: the Zenoh zbuf value, whose `after_shm` side holds its slices, the value entry whose arm holds it, and the
interest message, whose body `CURRENT || FUTURE` decides.

*Landed (step 3, eleventh part, 2026-11-04): variants a peeked byte decides.* The model reads a `<sce:peek-byte>`: the
tag is a flag of the arm's own first byte, which the arm's value states and the variant does not write; the decode
reads the flag there without moving past it and hands the byte to the arm. A case states each arm's header as the arm
writes it, so a body whose header names another arm reads back as that arm, and the model refuses it; the default
arm's tag is the one its body's header states. The literal and the comparison are a variant's, unchanged. Four codecs
gain 18 cases: a two-arm variant, and the Zenoh request, response and network message, whose seven arms are the
interest, final response, response, request, push, declare and OAM. A variant whose caller supplies its tag stays
refused by name.

*Landed (step 3, twelfth part, 2026-11-04): groups a chain's kinds decide.* The model reads a chain's `entry-id` and
`entry-id-except` and a gate `chain.has(k)`, and builds the group of docs/adr/0065 §6 R6 as the plan does: the atoms
closed under the gates that read one, the chain's kinds first and then the flags joined with them, and an arm for
every assignment, the ones setting more first. The group is keyed by its atoms joined with ` || `, each kind as
`chain.has(0xK)`. Every arm holds the chain, stated as its entries, `null` where it is gated and the arm holds no kind;
where a bare entry is the group's one atom, the arm that holds it states the chain as `{"rest": [..], "place": n}`, and
the model derives the entry. A chain whose entries disagree with the kinds the arm sets and clears, or a place that is
not the first of its kind, is refused. The literal holds the chain as the runtime's `Chain::new` or `Placed::new`, and
the harness compares the place and the entries. Three codecs gain 13 cases: an entry with content, a bare marker
first, among others and alone, and two kinds and a flag over eight arms. A chain whose entries take flag inputs (a
law) stays refused by name.

*Landed (step 3, thirteenth part, 2026-11-04): lists held to a law.* The model reads a list whose entries take flag
inputs (docs/adr/0065 §6 R8): an input the list's import binds, which every entry is handed from the list's start, and
one a chain supplies (`<sce:entry-flag-bind previous-entry-id>`), which each entry is handed as whether the entry
before is of that kind, the first clear. Such a list is stated as `{"start": {input: 0|1}, "entries": [..]}`, the
start giving each input the list binds; a flag bound to it is derived from the start where the list is there and is
the value's own where it is not, and a codec's own input bound to it is said by it. An entry that says other than it
is handed is refused, as a start that contradicts the input the codec is handed. The literal holds the list as the
runtime's `Lawful::new`, the start a bool for each input the list binds and `()` for each a chain supplies, a tuple
where the law runs several, and the harness compares the start and the entries. Nine codecs gain 20 cases, the Zenoh
query value among them, whose shared-memory marker hands the entry after it `after_shm`.

*Landed (step 3, fourteenth part, 2026-11-04): two lists of one count.* The model reads a count that counts two lists,
one always there and one gated on a flag of a carrier (docs/adr/0065 §6 R5), as the plan does: the flag's group holds
the first list on both sides, so the arm where the flag holds holds both and the other the first alone. A case states
each list by its id; the count is derived from them and refuses two of different lengths. The literal holds the pair
as the runtime's `Aligned::new`, under the name the arm gives it, and the harness compares both lists. With this every
gate group is spelled. One codec gains 3 cases. Three lists of one count, two that are both gated and two that are both
always there stay refused by name, as the plan keeps them checks.

*Landed (step 3, fifteenth part, 2026-11-04): a field aligned to a DMA burst, and step 3 over the fixtures.* The model
reads `sce:dma-burst-align` (RFC §synth-5-B B3): the field starts at its declared byte, a multiple of the burst; the
encode writes zeros up to it and the decode skips to it unread, as the generated decode does. One codec gains 3 cases,
the first the document's own oracle. With it the model reads every positional codec of the conformance fixtures, and
every one has byte cases (120 codecs, 326 cases). The shapes no fixture holds stay refused by name in the model and the
literal alike: a variant whose caller supplies its tag, an arm under a build feature, and a flag input the value does
not say, which the encode takes as an argument. CBOR and content-line codecs keep their own plans and models.

### 6. A backend moves one codec at a time, and the old encode goes when the last codec moves

A printer lowers the plan's forms one at a time. A codec whose plan uses only lowered forms is encoded by the printer.
The others keep the old encode, and the backend's `unlowered` names the first form missing, as `swift_codec::unlowered`
does. A landing never moves part of a codec, so no codec has two encodes. When no codec of a backend is left on the
old encode, the old encode and its templates are removed in the same landing.

## Order of work

1. This ADR.
2. The plan, and the Rust backend printing it. The decisions of `rust_source.rs` move into `encode_plan`, one part per
   landing: the interface, the refinements, the derivations, the checks and inputs, the program. Each landing
   regenerates the tree and changes no generated file. That unchanged regeneration is the proof that the plan holds
   what the Rust encode decided.
   *The interface landed 2026-11-03* (`sce-build/src/forge/encode_plan.rs`): the elements a value presents alone,
   each with its abstract type, its bound (R1, R2, R7) and its law (R8), and whether it may be absent. The Rust
   printer spells them, and the bounds' Rust spelling left `codec_totality` for the printer. The regeneration
   changed no file. `sce-build/tests/encode_plan.rs` pins what the plan says of each shape.
   *The derived numbers landed 2026-11-03:* each length, count and encoded length as what it measures (the bytes of a
   field with its `length-arith` offset, the entries of a list, the encoded length of a body), whether each measure
   may be absent, and how the number goes into its element (checked, widened or cast, and what the field holds where
   nothing it measures is there). The regeneration changed no file.
   *Every derivation landed 2026-11-03:* the plan holds the variant's arms, and a flag bound to an input the held
   codecs say is a number whose measures are what each body says, each list's start and what the held arm says. The
   variant's tag, a parent tag (each embedded body's arms and their masked tags), a presence flag, a disjunction's arm
   flag and a chain's continuation are kinds of their own. No printer reads a derivation off the roles. The
   regeneration changed no file.
   *The checks and inputs landed 2026-11-03:* the inputs the encode is handed and those the value says, what each
   held codec is handed for each input it takes, each input check (the held codec, where it is held, and what it is
   held to), each gate check, the clauses of a gate (a flag, an input, or a chain's membership with the group that
   holds it), and how each chain identifies its entries. The regeneration changed no file. A total codec plans no
   check, over every codec of the resources.
   *The program landed 2026-11-03:* the writes in wire order (a positional layout's packed bytes, fixed fields and
   tails; a streamed layout's fields, each with when it is written and how a list is walked; the variant's arm) and
   the flags a parent may set. The regeneration changed no file. The plan of a positional codec is then whole: the
   Rust printer decides nothing of it.
   *Step 2 closed 2026-11-03* with the other two encodings: `CodecPlan` is the plan of a codec by the encoding it
   declares, positional, CBOR (each entry's presence and value kind, the key order it is written in, the signed
   enum carrier its one refusal) or content-line (each property's line shape, its parameters, what a line holds,
   and the refusals that precede a line). Each regeneration changed no file.
3. The oracle's byte cases for every shape (§5), held first by Rust.
4. C++, then Go, Kotlin, Python and C11, each by §6, with its runtime's counting sink, refinements and walk. C++ comes
   first because it is the engine's own language and its type system carries a refinement as a type, as Rust's
   does. That tests the plan against a second interface idiom where the refinements stay static. C11 comes last
   because its interface, a table of functions over a context pointer, is the furthest from the others, and it
   then meets a plan four printers have already read.
   *The C++ interface, decided by the owner 2026-11-04.* A C++ value presents its elements through accessor
   functions, as a Rust value does through its trait (ADR 0058 §1), and not through public data members: every form
   of a codec, the value that holds its elements, one that borrows them and a view that reads them from the frame's
   bytes on demand, is then one interface, and a view has no member to hold. A generated value is made only by its
   constructor, which takes the presented elements in declaration order, so `T{a, b}` still reads as ADR 0059 §5's
   positional initializer while each refinement is checked where the value is made and holds after (Liskov and
   Zilles, 1974). The encode reads each element through a customization point, as `std::ranges::begin` reads a range
   (Niebler, N4381): the member function where the type has one, and otherwise a function of that name the type's
   own namespace declares, found by argument-dependent lookup, so a consumer's type joins without opening an SCE
   namespace. The requirement is a C++17 trait checked by `static_assert` (ADR 0059 §3). Considered and refused: a
   traits class the consumer specializes, which every SCE form would then need beside the type it adapts, and
   public data members alone, which no view can present and which let any value be written past its refinement.
   The cost is consumer-visible and was accepted: an element read as `frame.msgId` is read as `frame.msgId()`, and
   a derived element leaves the value (§3). Each landing says how to move (ADR 0078).
   *The first C++ landing, 2026-11-04:* the codecs whose elements are all integers or flag sets, each always present
   and none derived, with no flag input, no check and a total encode, written by packed bytes and fixed fields of a
   width the sink writes whole (`cpp_source::unlowered` names the first form any other codec uses). Each is printed
   from its plan: the readers of its elements, the trait `is_source`, the encode over any source (`Wire::encode_source`),
   the constructor and the accessors. Its `encode` is `Wire::encode_source(*this, w)`. A document that imports a printed
   codec reads its elements through the accessors (a chain's entry identifier, a validator's or a procedure's
   expression), and a procedure's `<assign>` to one element makes the value again by its constructor, the other
   elements read from their accessors. The numerical harness makes each
   printed value by its constructor and reads it by its accessors; `codec_source_generated_test` holds a consumer's
   types, presenting the elements by member functions, by functions of their namespace, or by both, to the generated
   value's bytes.

   *The second C++ landing, 2026-11-05:* byte strings of a stated bound, lengths derived from one byte string,
   variable-length integers, a tail padded to its offset, and a streamed write that is always made. A byte string is
   presented as `::SCE::Forge::Bytes<MAX>` (`sce/forge/bounded.h`): the accessor returns it, the constructor takes
   it, and a source's element converts to it. The value holds its bytes as `Bounded<std::vector<std::uint8_t>, MAX>`,
   and `make` is the only way to hold a list, so no value holds bytes past its bound and the encode is total:
   `encode_to_vec` returns the bytes, not an optional. A derived length is no longer an element (§3): the encode
   counts it from the bytes it writes, and the decode reads it and makes no member of it. The printed set is 47
   codecs of the resources, which `a_cpp_codec_is_printed_from_its_plan_once_it_lowers_every_form` pins by name.
   Text, bodies, lists, gated elements and lengths counted in encoded bytes still keep a codec on the template's
   encode.

   *The third C++ landing, 2026-11-05:* an element one gate decides, and the flags the encode derives. Such an element
   is presented as `std::optional` of its type (ADR 0055): the accessor returns it, the constructor takes it, and a
   source's element converts to it. The encode writes it where it is present and folds into its carrier the flag that
   says so, replacing the bits the value holds; a length held in a flag is folded the same way, keeping the value's
   bits where a gate leaves the bytes out, and a length field gated beside its bytes is written where its gate holds.
   The value no longer has to set a presence flag to agree with the element it gates. The printed set is 55 codecs.
   `cpp_source::unlowered_forms` names every form that keeps each other codec, which the census test prints; text
   waits for the C++ readers of a text element to read it through its accessor.

   *The fourth C++ landing, 2026-11-05:* text of a stated bound, presented as `::SCE::Forge::Text<MAX>`, a view of a
   `std::string_view` (`sce/forge/bounded.h`). A list of `char` is a text, and `make` holds it to well-formed UTF-8
   as well as to its bound (`sce/forge/utf8.h`), so no value holds a text the decode would refuse; the value keeps its
   own `Bounded<std::string, MAX>`. A collection of a printed element reads its fields through the element's
   accessor: the `<sce:index-by>` field the collection finds by, and an algorithm's `entry.<field>` over the
   collection, which the document set threads as `element_type_cpp_printed` beside the element schemas. The printed
   set is 60 codecs.

   *The fifth C++ landing, 2026-11-05:* a body always there, of a codec the printer prints, and the flag inputs a
   caller hands the encode. The value holds the body's value and presents it by reference; a source presents it as
   any source of the body's codec (that codec's `is_source`), and the encode writes it by that codec's
   `Wire::encode_source`, handing it the inputs the parent binds. A length of the bytes a body encodes to is counted
   by the same encode over a `CountingSink` (`sce/forge/codec.h`) before it is written. The inputs a caller hands
   are parameters of `Wire::encode_source` after the sink, as the template's `encode` has them, and a gate may test
   them. Whether a codec a C++ codec holds is printed is now read with its imports resolved as the C++ tree resolves
   them (`read_codec_file_as`), as the generator reads it. The printed set is 64 codecs. An input the value says
   (docs/adr/0065 §6 R4a) waits for its own landing with the wire expression it gates.

   *The sixth C++ landing, 2026-11-05:* a flag input the value says (docs/adr/0065 §6 R4a), by the field gated on it
   alone or by the body always there that says it. As the Rust encode, a printed C++ codec's `encode`,
   `encode_to_vec` and `Wire::encode_source` are not handed such an input; the decode still is. The encode reads
   it off the value, and each one is a function of the codec's namespace, `input_<name>(src)`, which a parent
   derives the flag it binds to it from and which a declaration forwarding it calls on its body. A codec the
   template still writes hands a printed codec it holds only the inputs that codec does not say
   (`embed_flag_bind_thread_args_supplying`). The printed set is 69 codecs.

   *The seventh C++ landing, 2026-11-06:* a list always there, of a stated bound and no law, of entries of a printed
   codec, and the count of its entries. The value holds `Bounded<std::vector<Entry>, MAX>` and presents
   `::SCE::Forge::List<Entry, MAX>`, a view of it; a source presents the list as any `Bounded` of bounds within its
   own, of any type of entry sources (`is_<list>_list`), and the encode walks it, writing each entry by the entry
   codec's `Wire`. `Bounded` reads a list that is not held in a row (its `data` and indexing only where the list
   has them). A chain whose entries end on a flag it sets, are handed an input from the entry before, or are held to
   a law, and a list a gate decides, wait for their landings. The printed set is 75 codecs.

   *The eighth C++ landing, 2026-11-06:* a variant whose tag a field or a flag of the codec's own holds, of arms of
   printed codecs under no build feature. The value holds the arm in the `{Codec}Variant` the template defined (each
   arm's value in turn, the catch-all `{Codec}Default{tag, body}` last), privately, and presents it as `body()`; a
   value made by default holds the arm marked `default="true"`. A source presents the body as any `std::variant` of
   a source of each arm's codec in turn (`is_body`), the catch-all holding its `tag` and its `body`; the encode derives
   the tag from the arm the body holds, the catch-all's masked to the tag's bits, and writes the arm by its codec's
   `Wire`. A codec the template still writes reads an embedded printed variant through `body()`. A variant whose
   caller hands its tag, whose arm's own header holds it, or with an arm under a feature, waits. The printed set is
   80 codecs.

   *The ninth C++ landing, 2026-11-06:* a chain whose entries end on a flag it sets. As Rust's `{Codec}Overrides`, a
   printed codec a parent may set a flag of has a `struct Overrides` of `std::optional<bool>` members, one for each
   such flag, and `Wire::encode_source` takes one last, empty by default; what it holds is folded into the carrier
   after every derived flag. A chain walks its entries setting on each but the last the flag they end on. The
   printed set is 81 codecs. A list of no stated bound and a list a gate decides wait for their own landing, which
   the extension chains of the Zenoh messages need.

   *The tenth C++ landing, 2026-11-06:* a body a gate decides. The value holds it as a `std::optional` of the body's
   value; a source presents it as a `std::optional` of any source of the body's codec (the trait
   `is_<element>_body`), from which the encode derives the flag it is gated on and writes it where it is present.
   A flag input such a body forwards still waits, since an absent body says nothing of it. The printed set is 86
   codecs. Two defects of a template parent of a printed codec were found and closed with it: a C++ broadcast log
   handed its element's encode the flags the element says, and a C++ decode of a codec whose last field is a
   length-ref consumed the rest of the cursor, so that a codec holding it as a body could not decode on after it
   (the Rust decode already consumed only what it read).

   *The eleventh C++ landing, 2026-11-06:* a list a gate decides. The value holds it as a `std::optional` of the
   bounded list; a source presents it as a `std::optional` of any list the list's own trait accepts (the trait
   `is_<element>_present_list`), from which the encode derives the flag it is gated on and walks the entries where
   it is present, a chain setting the flag its entries end on as before. A count of its entries is derived where it
   is present. The printed set is 88 codecs.

   *The twelfth C++ landing, 2026-11-06:* a list, a byte string or a text the protocol states no bound of
   (docs/adr/0077 §7), held as `Bounded<_, kUnbounded>`. `SCE::Forge::kUnbounded` is the runtime's one such bound,
   which `ContentLine::kUnbounded` names. The printed set is 97 codecs. A defect of the template parents of printed
   codecs, open since the fifth landing, was closed with it: a printed codec is not handed an input its value says,
   and a parent on the template's encode had then written its own flag with no test against the value, where the
   codec had refused the two disagreeing (`PresentIfMismatch`) before it was printed. Such a parent now holds the
   flag it binds against `input_<name>` of the arm or the body it writes, and refuses as the codec did; one whose
   list holds entries that say an input is refused at generation, since each entry would say it anew.

   *The thirteenth C++ landing, 2026-11-06:* a group of fields one gate decides together, of one side
   (docs/adr/0065 §6 R4b). The value holds it as a `std::optional` of the class `{Codec}{Gate}`, made by its
   constructor and read by its accessors as the value is; a source presents it as a `std::optional` of any type that
   presents each of its fields by name (the traits `is_<group>_members` and `is_<group>_group`). The encode binds
   each field to an optional local from the group where it is there, so each statement that writes a field is the
   one that writes an optional element, and derives the gate from whether the group is there: a flag of the codec,
   or an input the value says. The printed set is 101 codecs. A group of two sides, of a chain's memberships, of
   lists that share a count, or one holding more than integers, still waits.

   *The fourteenth C++ landing, 2026-11-07:* a flag bound to an input the arm the value holds says, or a body a gate
   decides says (docs/adr/0065 §6 R4a). The encode reads the input through the arm's codec, by a switch over the arm
   the value holds (the catch-all's, off its body), or through the body's codec where the value holds the body; where
   the value holds an arm that does not say it, or no body, the flag keeps the bit the value presents. A flag every
   arm says leaves the value's word out of the wire. The printed set is 106 codecs, the declaration and its declare
   among them, so no resource keeps a template parent whose printed arm says an input: the check the twelfth landing
   gave such a parent is held by its generated text alone.

   *The fifteenth C++ landing, 2026-11-07:* a count of a list a gate decides. The encode counts the list where the
   value holds it, and the count field, gated beside the list, is written only there; where a flag holds the count,
   it keeps the bits the value presents. The printed set is 108 codecs, the Zenoh hello among them.

   *The sixteenth C++ landing, 2026-11-07:* fields on each side of a gate, and a list held to the law its entries
   say their inputs by (docs/adr/0065 §6 R4c, R8). A group of more than one arm is a `std::variant` of a class for
   each arm, in the plan's order, and the encode derives each gate from the arm the value holds. A list whose
   entries say the inputs it hands them is a `SCE::Forge::Lawful` of the list and its law (`sce/forge/law.h`, the
   C++ form of the Rust runtime's `Lawful`: `Uniform` for a flag bound to every entry, `AfterKey` for an input a
   chain supplies from the entry before, `Laws` for several), checked once by `make`; the flag the parent binds is
   the list's start, and the decode holds what it read to the law from the start the frame gave. These forms move
   together: a codec that says its own input, printed while its parent keeps the template's encode, is refused in
   that parent's list (the twelfth landing), so the chain entries move with every carrier of them. An input a
   gated list says is read as a `std::optional`, empty where the list is absent, and a parent on the template's
   encode holds such an input against its flag only where it is said. The printed set is 118 codecs.

   *The seventeenth C++ landing, 2026-11-07:* a field typed by an imported enum. The value presents it as the
   enum's type; the encode writes the enum's carrier, through the enum's own `to_underlying`; a value made by
   default holds the first variant the enum's document declares, since a closed set need not declare its
   carrier's zero. The decode already read the carrier into the type, and a closed set's refusal of a value it
   does not declare is now recorded on the cursor as `CodecError::UndeclaredEnumValue`, on the fixed and the
   streaming path alike, where before it was read as running out of bytes; the C and Rust runtimes already named
   it. The printed set is 120 codecs.

   *The eighteenth C++ landing, 2026-11-07:* a byte string or a text on one side of a gate, and the length that
   counts it (docs/adr/0065 §6 R4b). An arm that holds bytes holds its own copy and presents a view, so it is
   made as a value holding bytes is, neither `constexpr` nor `noexcept`, and moved where it is handed on. The
   length is derived from the bytes on the side that holds them, and is what the other side states where they are
   not, zero where the value holds neither (`Absent::Group`, as the Rust encode reads it). The encode keeps the
   view of what it reads off the arm until it writes it, so a source must present such a member as something that
   lives on past the read, a reference or a view, which the runtime's `Detail::Lasting` asks of the read: the
   lifetime the Rust source states by its borrow. The printed set is 122 codecs.

   *The nineteenth C++ landing, 2026-11-07:* a list or a body on one side of a gate. Unlike a byte string, which
   any source presents as something that converts to the one view the value holds, a list or a body is
   presented as whatever type the source holds it in, so the encode cannot convert it into a local of one type
   before it knows the arm. It reads it once, off the one arm that holds it, and keeps what the read returns
   (`Detail::Kept`): what a reference names, by its address, or what a read by value returns, whole, so no view
   outlives what it views; the count of the list is derived from what it kept, zero where the arm is not there.
   The decode holds the list it read to its bound before the arm is made of it, and moves a body in. A list or a
   body more than one arm holds would need one type across the arms the source presents, and a list a group
   holds held to a law, the law's start in each arm; both stay unlowered. The printed set is 123 codecs.

   *The twentieth C++ landing, 2026-11-07:* the flags of a disjunction that decides a group (docs/adr/0065 §6
   R4c), and a body or a list more than one arm holds. A gate `a || b` makes an arm for each way the flags can
   hold it, so the value says which flags hold by the arm it holds, and the encode folds each flag into its
   carrier from that arm, clear while the value holds none; the carrier's other bits stay the value's. The fields
   such a gate decides are in every arm, so a body or a list among them is held by several arms. The Rust group
   holds such a member as one type parameter in all of its arms; the C++ group asks the same of a source, that
   every arm presenting the member presents it as one type, which the encode then keeps once whichever arm the
   value holds. The landing's own forbidding of a member several arms hold is lifted with it. The printed set is
   125 codecs.

   *The twenty-first C++ landing, 2026-11-08:* a flag input the arms of a variant say (docs/adr/0065 §6 R4d,
   R4e). The value holds one arm, so the arm says the input, through its codec's `input_<name>`, and a parent
   reads it off the body as the value holds it: a `std::uint8_t` where every arm says it, a
   `std::optional<std::uint8_t>` where some arm does not read it and so says nothing. A body that holds no arm
   says nothing either, as its write writes nothing. No gate of the codec reads such an input, so the encode
   binds a said input only where a gate reads it (`Said::read`), as the Rust encode does; before, it bound each
   one and discarded those no gate read. A chain of such entries moves with it, as the template's encode cannot
   hold a printed entry that says its input against the flag it binds: the list's law reads what each entry
   says as the runtime's automata take it, nothing from an entry that says nothing, which follows any state
   (§6 R8b). The printed set is 127 codecs.

   *The twenty-second C++ landing, 2026-11-08:* a variant whose arm's own header holds its tag (a peek byte).
   The decode peeks the byte and leaves it for the arm, which reads it as its header; the arm's header holds
   the tag, which an arm made by default holds as its codec declares it, so the encode writes the arm the value
   holds and nothing of its own, as the Rust encode does, and the plan derives no tag. The catch-all arm holds the tag it was read with, as for any variant,
   and writes its body. The envelope whose arms are two such variants moves with them. The printed set is 132
   codecs.

   *The twenty-third C++ landing, 2026-11-08:* a flag of a parent that dispatches the variant of a body it
   embeds (`<sce:variant-dispatch>`). The encode derives the flag from the arm the body holds, each arm's tag
   masked to the flag's bits, the catch-all's the tag it holds, and nothing where a gate leaves the body out;
   the bits of the carrier no such flag names stay the value's, as for any derived flag. Such a parent is no
   conformance fixture, so its C++ round trip is judged beside the variant it embeds. The printed set is 133
   codecs.

   *The twenty-fourth C++ landing, 2026-11-08:* a field that holds a physical quantity (`sce:quantity`). The
   wire holds the raw value, which the plan already writes; what the quantity adds is the pair of accessors
   that read and set the value in its unit, and they read and write the raw value where the printed value
   holds it, its private member, as the flag accessors do. The C++ value of such a codec had been compiled
   nowhere, so its round trip is judged beside the other non-fixture codecs. The printed set is 134 codecs.

   *The twenty-fifth C++ landing, 2026-11-08:* a group of two lists that share one count (docs/adr/0065 §6
   R5). The arm that holds both holds them as one member, a `SCE::Forge::Aligned` of the two, which the
   runtime makes only of two lists of one count, as the Rust runtime's `Aligned` is; the arm that holds the
   first alone holds it as a list. The encode keeps the two as the arm presents them and reads each list off
   what it kept, so the count it derives from the first is the count of both; a source presents the first
   list as one type wherever an arm holds it. The decode makes the two of the lists it read by the one
   count. The printed set is 135 codecs.

   *The twenty-sixth C++ landing, 2026-11-08:* a group the gates on a chain's entries decide, whose entries
   have content (docs/adr/0065 §6 R6b). The runtime's `sce/forge/membership.h` gives the chain each arm
   holds its type, as the Rust runtime's `membership.rs` does: `SCE::Forge::Chain<L, Kinds<...>, Kinds<...>,
   Max, Min>`, made only of a chain within its bounds that holds an entry of each kind of the first set and
   of none of the second, and `Placed`, for the bare entry a later landing holds. The printer gives each kind
   a key, which reads an entry's identifier through the entry codec's readers; each arm holds the chain as a
   `Chain` of the kinds it says; the encode keeps the list the chain holds, one type in every arm, and
   writes it as any chain; the decode picks the arm by the kinds the chain it read holds. A chain whose arm
   holds an entry of some kind has no value made by default, so a value made by default holds the arm where
   no gate holds. The printed set is 136 codecs.

   *The twenty-seventh C++ landing, 2026-11-08:* such a group whose chain is gated itself (R6b). The arm
   where the chain holds none of the kinds holds it as a `std::optional` of its `Chain`, as the Rust arm holds
   an `Option`: an absent chain holds no entry of any kind either. The encode keeps the list where the arm
   holds the chain, and derives the chain's presence flag from that; a field gated on a membership, such as
   a count or a length the encode derives, is written where the arm the value holds says the chain has the
   kind, as the Rust printer tests it. The decode reads an absent chain as holding no kind. The printed set
   is 137 codecs.

   *The twenty-eighth C++ landing, 2026-11-08:* the one gate on a bare entry (R6a), an entry whose
   identifier is all of its value. The arm where the gate holds holds a `SCE::Forge::Placed` of the other
   entries and the named entry's place, as the Rust arm does; the kind's key makes the named entry
   (`named()`), its carrier the identifier and its variant the arm the identifier's tag selects. The encode
   walks the chain as slots, writing the named entry where the value holds its place and each other in
   order, the flag its entries end on set on each but the last; the decode takes the first entry of the
   kind, which the gate found, out of the chain it read as the place. The printed set is 138 codecs.

   *The twenty-ninth C++ landing, 2026-11-08:* a CBOR map, printed from its own plan (`cpp_source::cbor`).
   Every entry is presented, as the Rust interface presents it: one accessor each, an optional for one the map
   may lack, a text or a byte string as its bound's `Text` or `Bytes`, so the encode over any source refuses
   nothing and writes the map head and each entry present in ascending key order, as the Rust printer does.
   The runtime's CBOR writers take views. An enum of a signed carrier, whose negative number the encode
   refuses, is not printed yet. The engine's mesh envelope (`sce:std/mesh/envelope.scxml`) is such a map: the
   engine's adapter makes it by its constructor, each bounded entry by its `make`, which is where an entry
   past its bound is refused, and reads it by its accessors. The printed set is 139 codecs.

   *The thirtieth C++ landing, 2026-11-09:* a content-line component, printed from its own plan
   (`cpp_source::content_line`), and the first C++ encode that refuses. Every entry is presented, as the Rust
   interface presents it: an accessor for a property's value and one for each of its parameters, an optional
   for one that may be absent, a range (a begin and end pair) for a repeated property and for the values of one
   line, and a range of line records for a repeated property with parameters or values, each record a source
   of its own class. A text is a view: a line refuses a value by its content as well as its size, so the
   plan's checks stay checks and no bound refines them. The encode writes over a counting sink first and then
   over the caller's (§1), so a refusal leaves the caller's sink untouched; it returns the refusal as a
   `CodecError`. The runtime's line writer takes views and any range of texts, and throws nothing. The
   template's struct is gone: every content-line plan is printed. The printed set is 144 codecs.

   *The thirty-first C++ landing, 2026-11-09:* a fixed field whose bits run past its first byte, the last form
   the printer lacked. ADR 0095 says where such a field's bits lie: bits of the word its bytes make in its byte
   order. The plan writes a whole field that shares none of its bytes as one value of its bytes
   (`Write::Whole`), and every other byte of the fixed part as the or of the pieces the fields hold in it
   (`Write::Byte`); the C++ printer prints both, and the template's encode and decode, Swift and the diagram
   read the same spans (`forge::bit_layout`). The printed set is 145 codecs, every codec of the resources, so
   the template's C++ encode prints none of them.

   *The thirty-second C++ landing, 2026-11-09:* a variant whose caller hands its tag. The template's encode
   still printed codecs outside the resources, so the census now reads every codec document of the repository
   and pins the ones the printer keeps. Such a variant writes its arm alone; the value answers `tag()` by the
   arm it holds, a switch over the arm's place, the tag the decode was handed for an arm that stands for every
   other tag. A C++ test holds the caller-tag fixtures of the Rust conformance arm to the Rust arm's frames.

   *The thirty-third C++ landing, 2026-11-09:* a variant with an arm a build feature decides (ADR 0057 §10).
   An arm's place in the `std::variant` is a constant of the build's macros, the arms before it plus each gated
   arm before it that the build holds, and every line that names a gated arm sits inside the guard of its
   macro: the traits, the tag switch, the cases of the encode and of a parent that derives the tag. A C++ test
   runs a codec that holds such a variant in both builds, the arm written back where the feature is on and
   refused as an arm not built where it is off.

   *The thirty-fourth C++ landing, 2026-11-09:* a positional encode that refuses a value. Each refusal the
   totality analysis names is one the printed encode writes (`assert_all_written`): an input a chain of slots
   holds an entry to, a gate the value contradicts, a length out of range. The checks run before the first
   byte; where a held codec can refuse, the encode runs over a counting sink first and then over the caller's
   (§1), so a refusal leaves the caller's sink untouched. A printed codec's `encode_to_vec` answers an optional
   only where its plan is not total: nineteen goldens whose plan is total now answer the bytes. One codec of the
   repository is kept, a CBOR record list.

   *The thirty-fifth C++ landing, 2026-11-10:* a C++ machine holds every list, text and byte string in the
   forge runtime's owned bounded type of it (`OwnedList`, `OwnedText`, `OwnedBytes`), its bound in its type, as
   ADR 0086 §7.4 decided and as the Rust machine does. A value past the bound is no value of the type, so a
   record's text reaches the CBOR encode already within its bound, and the analysis's refusal for it is one
   the type keeps rather than one the encode writes. The C++ printer prints a record with a text field, and the
   CBOR record list is printed; a record's byte-string field is not printed yet. No codec of the repository is
   kept: the template's C++ encode prints none, and it can be removed.

   *The thirty-sixth C++ landing, 2026-11-10:* a CBOR map whose encode refuses, and a record with a byte-string
   field. The printed encode writes each refusal the totality analysis names, as the Rust printer writes it:
   `CborOutOfRange` for a negative value of an enum whose carrier is signed, for a list of fewer records than
   its lower count, and for a record whose byte string, which the schema's unit holds in a `std::vector` with
   no bound, is past its bound. The checks run before the map head, so a refusal leaves the sink untouched, and
   the value's `encode_to_vec` answers an optional. A list is viewed within the most records it holds and not
   its lower count, as Rust's `BoundedList` holds it, so the two languages refuse the same values. Every CBOR
   plan is printed. A C++ test holds three CBOR fixtures, one for each refusal, to the bytes and the refusals
   of the Rust encode.

   *The thirty-seventh C++ landing, 2026-11-10:* the C++ harness runs every `sources` case (§5), as the Rust
   harness does. The generator folds each `source` into the C++ harness as the printed value its constructor
   makes (`cpp_source::source_case`). An element that holds a bound or a law is made by `make` of the type the
   class's own accessor presents it as, read with `decltype`, so the case names no bound, law or kind the
   printer would have to agree with; a byte string and the entries of a list or a chain are made over a
   `std::vector` bound first, since the views `make` takes own nothing, and a body, a group's arm and a
   variant's arm by value, an arm at its place among the alternatives. The harness encodes the value through
   `Wire::encode_source` over a sink and compares the bytes (law 4), then decodes the bytes and compares what
   the decode presents with the value, element by element, each body, list, group and variant down into the
   elements of the codecs it holds (law 1). The notation is read once for both printers: the Rust case and the
   C++ case share the readers of a value, a group's arm, a variant's arm, a law's start, a bare entry's place
   and the inputs, so one case states one value in both. The shapes no case states stay refused by name, as
   in the Rust case; a codec the C++ backend kept on the template's encode would be refused by name too, and
   none is. Every codec with `sources` that the C++ harness holds runs them; the one it does not hold,
   `codec_dma_aligned_basic`, is an MCU-only codec no C++ tree generates.

   *The thirty-eighth C++ landing, 2026-11-10:* the catalog gains refused cases (§5), which the Rust and
   the C++ harness run as they run `sources`. Until now no positional codec of the catalog refused a value:
   each declared its encode total, and the model refused by name every shape whose encode refuses. A
   refused case states a value as the interface presents it and the refusal its encode answers
   (`{"source", "inputs"?, "error", "note"}` under the fixture's `refused`), and `binary_model.py` holds the
   encode to raising exactly that refusal (`EncodeRefused`). The model raises it only where the page says the
   encode refuses a value of the interface (ADR 0065 §2): a gate it cannot derive that the value contradicts
   (`present-if-mismatch`), two things the value says of one number that disagree (`inconsistent-source`),
   a body's length past the field that carries it (`length-out-of-range`). A value no type of the interface
   holds (a byte string past its bound, two lists of one count that differ) is no value of the codec, and a
   case that states one is written wrong. The model reads one shape more for it: a disjunction one of whose
   flags another gate reads is a check and not a group (R4c), its field presented alone. The new resource
   `codec_present_if_checked` is that shape, its five sources covering each flag and each side of both
   gates, and its four refused cases the trailer given where neither flag is set and left out where either
   is, the value's own has_seq bit set or clear against the one seq derives. The generator reads the refusal
   as `forge::encode_failure::EncodeFailure`, which names the runtime's symbol of each backend, and holds
   it to the refusals the codec's totality names; a refusal a backend's runtime does not name (the C++
   runtime has no inconsistent source yet) is refused by name. A refused case is made into the value as a
   source is, by the same reading, and the harness holds the encode to answering that refusal and to
   leaving the sink empty. The inconsistent source and the length out of range enter the catalog with the
   landing that prints them in C++, since a codec that refuses them is one the C++ backend does not print
   yet, and a harness runs every case of every shape it prints.

   *The thirty-ninth C++ landing, 2026-11-11:* the shapes the C++ printer gained since enter the catalog
   with their cases (§5), and the C++ test that held them to hand-derived frames keeps only what no case can
   state. The seven codecs are a length two byte strings share (`codec_length_shared`), a length the value
   states where the two gated byte strings it measures are absent (`codec_length_stated`), the encoded length
   of a body that refuses, in a 16-bit field (`codec_encoded_length_checked`), a closed enum in a group
   (`codec_group_enum`, with its `enum_session_state`), a gate on a chain's kinds joined with a flag input
   (`codec_chain_has_input`), and a chain that hands each entry an input the entry does not say
   (`codec_chain_handed`, with its entry `codec_chain_handed_entry`). `binary_model.py` reads each as the
   page does: a field of an enum is written as its carrier and stated by its variant's name, and the decode
   of a number a closed enum does not declare fails; a gate that reads a flag input, or a chain's kinds
   joined with one, is no group but a check the value is held to (ADR 0065 §6 R4c, R6); the entries of a
   list handed an input they do not say follow no law and are each handed it as the walk writes them (R8);
   and a codec handed an input its value does not say takes it at its encode as at its decode, which a case
   states in its `inputs`. A stated length joins a side of a group only where all of its content is on one
   side of one gate (R4b). The length of `codec_length_stated` measures byte strings under two gates, so it
   joins no group and is presented alone, as `codec_roles` and the printer held it: the model had joined it
   to both gates and is corrected, and SCE_FORGE.md §4.6, which said less than R4b, now says it. The
   catalog gains 33 sources and 15 refused cases, the inconsistent source among them; no value of these
   codecs reaches a length out of range, since each byte string is held within its one-byte length by its
   type and a body of two of them is at most 511 bytes, so the refusal the analysis names for the 16-bit
   field is one no case can state. The case printers spell what the cases hold: a value of an enum as the
   variant of the type each harness names it by, read from the codec's import (`HeldCodecs::held_enum`); an
   input the value does not say, handed to `encode_source` as to the decode; and in C++ a number the value
   states while its content is absent, as a `std::optional`. The C++ runtime now names the inconsistent
   source (`CodecError::InconsistentSource`), so `EncodeFailure` names every refusal for both harnesses.
   `positional_forms_generated_test` keeps a length the value states beside content that agrees and none
   with no content, each written as the Rust encode writes it and decoded to another value, so no source
   states it; a value a closed enum does not declare, which no decode refusal of the catalog names; and the
   input `codec_some_arms_say_forward` hands on to a body that says it in some of its values. That fixture
   stays out of the catalog: its body is a variant whose caller hands its tag, one of the Rust arm's
   caller-tag fixtures (`forge_source_round_trip`) that neither the model nor a case reads, and the family
   enters the catalog together with that reading.

   *The fortieth C++ landing, 2026-11-11:* the C++ backend prints every codec from its plan or refuses it by name,
   and the template's encode is removed (§6). No codec of the repository uses a form the printer lacks, so the
   landing moves none; what is left is refused as Swift refuses what it does not lower, as
   `generate/unsupported-feature` with the clause `cpp_source::unlowered` names (`codec 'x': field 'f': ...`): a
   float or a bool, which no C++ value has a type for yet; a list on one side of a gate held to the law its entries
   say their inputs by, whose start no arm of a C++ group holds; a chain of slots whose entries are held to an
   input, which the Rust printer does not determine either; and a body, a list or an arm of a codec the C++
   backend refuses, by that codec's own clause. The import carries the clause (`ImportContext::cpp_unlowered`,
   read as the codec is generated, its own imports resolved), so a codec that holds a refused codec is refused
   at the field that holds it, and a document of another kind that imports one is refused for it. The template's
   C++ encode, its value's public members and its `tag()`, the CBOR template's struct, the encode blocks the
   generator wrote for C++ (whose helpers now refuse C++ by name, as they refuse Rust), and the checks a template
   parent held a printed codec's said input to are gone; a C++ encode refuses no bound in a prologue, since its
   value holds each bound in a type. The census is renamed
   `a_cpp_codec_is_printed_from_its_plan_or_refused_by_name`: it pins the printed codecs of the resources and
   each refused codec of the repository with its clause (none), and holds the refusal of a float, of a codec that
   embeds one, and of a validator that imports one. Two tests that still read the template's C++ text of a parent
   that derives a variant's tag, and of a variant with a gated arm, read the printed text: the switch over the
   alternative the body holds, each case at its index counted per build.

   *The forty-first C++ landing, 2026-11-12:* the caller-tag family enters the catalog together with its
   reading (§5), as the thirty-ninth landing said it would. Its ten documents move from the Rust arm's fixtures
   (`backends/rust/forge-conformance/fixtures`) to the resources: a variant with no tag of its own
   (`codec_source_leaf`) and a parent that holds its tag in a flag (`codec_source_parent_tag`); a variant every
   arm of which says the flag input it is handed, one arm at the other polarity (`codec_every_arm_says`, its arms
   `codec_every_arm_says_near` and `codec_every_arm_says_far`, its parent `codec_every_arm_says_parent`); one that
   hands the input to one arm of two (`codec_some_arms_say`, its arm `codec_some_arms_say_plain` that reads no
   input, its parent `codec_some_arms_say_parent`); and a codec that hands its own input on to such a body
   (`codec_some_arms_say_forward`). `binary_model.py` reads them as the page does. The value presents the arm and
   no tag; the caller hands the decode the arm's tag, a byte, after the flag inputs, as it hands a flag input, and
   a case states it as `tag`, the tag of the arm the source holds. A parent names the flag that holds the tag
   (`<sce:variant-dispatch>`), derives it from the arm the body holds masked to the flag's bits, the value's own
   where a gate leaves the body out, and hands the decode what it read there. A flag a parent binds to an input
   every arm says (docs/adr/0065 §6 R4d) is derived from the arm, as a body's is; one bound to an input some arm is
   not handed, or says in some values only (R4e), is the value's own, held to the arm where the arm says it, and
   the encode refuses a value whose flag says otherwise (`present-if-mismatch`), as R4e's landing left it.
   SCE_FORGE.md said a parent derives such a flag and did not describe `<sce:variant-dispatch>`; it now says both.
   Such a variant held as an arm or as the entries of a list, or embedded with no flag to read its tag from, is
   refused by name. The catalog gains 39 sources and 2 refused cases. Both case printers read the tag in one place
   (`rust_source::case_tag`), which refuses a tag that is not the one of the arm the source holds and a tag stated
   for a codec whose variant takes none: the decode is handed it after the flag inputs, and the value made and the
   value decoded both answer it (Rust `{Codec}Wire::tag_of`, C++ `tag()`); a caller-tag variant is no longer
   refused by name by the literal. Kotlin, Go, Python and C11 compile the ten as they compile every codec whose
   cases are `sources` alone, their encodes not printing the plan, and Swift where its generator writes them.
   The hand tests the cases now state are gone: `caller_tag_variant_generated_test`, which held the frames, the
   leaf's tag and the refusal; the forwarded input of `positional_forms_generated_test`, which the cases of
   `codec_some_arms_say_forward` hand both ways in the arm that reads none, so that arm's answer can only be
   none; and of `forge_source_round_trip`, the frames, inputs and refusal of `some_arms_say` and
   `every_arm_says`. What stays is a parent writing `N` and `M` from the arm whatever its value states of them: a
   case states a value as its decode presents it, so no case states a header that says otherwise than the arm.

   *The forty-second C++ landing, 2026-11-13:* a list on one side of a gate held to the law its entries say their
   inputs by (docs/adr/0065 §6 R4b, R8), which the fortieth landing left refused by name, enters the printer
   together with its cases (§5). No document used it, so the new resource `codec_repeat_flag_group` is that shape:
   the elements of a repeat handed a flag of the codec's own carrier (`head.W`) as the input they say, on the side
   of `head.L` that also holds `level`, the other side holding `note`. Each arm that holds such a list holds it as
   the value holds one alone, a `SCE::Forge::Lawful` of its law, and the law is defined before the arms' classes.
   The encode derives the bound flag from the start of the list it kept off the arm the value holds, and keeps the
   bit the value presents where the value holds the other arm; the decode holds the list it read to the law from
   the flag the frame gave, before it makes the arm. What the printer does not read off an arm stays refused by
   name, narrower than before (`cpp_source::grouped_law_forms`): a list whose law is over an input of the codec's
   own, which the value says by the list's start and which the printer reads off a source through the list's own
   accessor, as the Rust printer does, while a group's member is read off its arm; and a list held to a law that
   shares its count with another (R5), which an arm holds as one `Aligned` of the two. No document uses either,
   and the census holds both refusals. `binary_model.py` already read the shape, a lawful list stated in its arm
   as `{"start", "entries"}`; its page now says where the bound flag is derived and where it is the value's own.
   The catalog gains 7 sources and no refused case: the encode is total, the count held within its field by the
   list's bound. Kotlin, Go, Python and C11 compile the codec as they compile every codec whose cases are `sources`
   alone, and Swift where its generator writes it; the codec joins the set the Rust, C++, Kotlin, Go, Python and
   C11 backends each compile (`entries_that_take_a_flag_compile_on_every_backend`).

   *The encoded length of a gated body, 2026-11-14:* the plan read the encoded length of a body a gate decides
   (`sce:length-from` on a gated `<sce:embed>`) as always there, zero where the body is absent, where ADR 0058 §1
   has a length of gated content stated by the value while the content is absent and `binary_model.py` read it
   so: a frame that carries a number beside an absent body would have been written back with zero. The measure is
   now gated as the body is (`encode_plan`, `codec_totality::measured`), so the number is derived from the body's
   frame where the body is there and stated where it is not, held on the side of the gate's group the body is not
   on (docs/adr/0065 §6 R4b), as a length of bytes is; the Rust and C++ printers test the body's presence on the
   source. No document used the shape, so the new resource `codec_present_if_length_from` is it, a timestamp gated
   on a flag and read from as many bytes as a VLE size before it says. The catalog gains 4 sources and no refused
   case: the encode is total. Kotlin, Go, Python and C11 compile the codec, and Swift where its generator writes
   it; each writes the size as the value holds it, as it writes every length, until it prints the plan (§3).

   *The forty-third C++ landing, 2026-11-13:* the first of the two forms the forty-second landing left refused by
   name enters the printer together with its document and its cases (§5): a list on one side of a gate whose law
   is over an input of the codec's own (docs/adr/0065 §6 R4b, R8). The new resource
   `codec_repeat_flag_forward_group` is the hand-over of `codec_chain_flag_forward` in a group: the codec's own
   `wide` bound to every element of a repeat on the side of `head.L` that holds it, the other side holding `note`.
   The value says `wide` by the list's start where it holds the arm that holds the list, and nothing of it in the
   other arm (R4e). Reading it found a defect of the Rust printer, which no document had reached: its
   `input_<name>` read the start through an accessor of the list's own, which the source trait has not for a
   member of a group, so the codec did not compile. Both printers now read the start off the group the source
   presents: Rust matches the group's accessor on the arm that holds the list, `None` in any other arm; C++ reads
   the group once in `input_<name>` and answers the start where the arm the value holds holds the list, and the
   encode reads what it kept of that arm. `binary_model.py` read a lawful list's start only where the value
   states the list at its top, so it missed the input a grouped list says; it now reads it in the arm, and holds
   each source to saying every input it says as the case hands it, as each harness holds the value
   (`{Codec}Wire::input_<name>`), which every case of the catalog already met. The catalog gains 8 sources and no
   refused case. The census holds the codec printed, and a holder of the shape that it used to refuse printed
   whole. Kotlin, Go, Python and C11 compile the codec as they compile every codec whose cases are `sources` alone,
   and Swift where its generator writes it; the codec joins `entries_that_take_a_flag_compile_on_every_backend`.

   *The forty-fourth C++ landing, 2026-11-13:* the second form enters with its document and cases (§5): a list a
   gate decides, held to its law, that shares its count with a list always there (docs/adr/0065 §6 R5, R8). The
   new resource `codec_repeat_flag_aligned_group` is the shape of `codec_repeat_unconditional_count` whose gated
   `weights` are handed `head.W` as the input they say. The arm where `head.H` holds holds the two as one
   `Aligned`, its second list a `Lawful` of the law, as the value holds such a list alone; the bound flag is the
   start of the list the encode kept off that arm, and the bit the value presents in the arm of `links` alone. The
   C++ printer already held each list of a pair as the arms present it and made the pair by `make` of the two the
   decode read, a `Lawful` having a size, so the refusal named nothing the printer lacked. The Rust printer's pair
   did not compile: `Aligned::new` takes two lists that count their entries (`Counted`), which `Lawful` did not;
   the runtime now counts a `Lawful` as the list it holds. Its eager decode pairs the two by value, not through the
   pair's door of the 2026-11-05 amendment of ADR 0065 §6: `refill_first` gives the second list as many default
   entries as the first, which need not follow its law, so the door takes no second list held to one. The list
   always there of such two, held to a law itself, stays refused by name, narrower than before
   (`cpp_source::grouped_law_forms`): no document uses it, no case states it, and the Rust printer, whose encode
   reads the start of a list no gate decides as always there while the group binds it as an `Option`, does not
   write it either; the census holds the refusal, and a holder of the form it used to refuse printed whole.
   `binary_model.py` already read the shape, a lawful list stated by its id in the arm beside the list it shares
   its count with. The catalog gains 8 sources and no refused case. Kotlin, Go, Python and C11 compile the codec
   as they compile every codec whose cases are `sources` alone, and Swift where its generator writes it; the codec
   and its `codec_repeat_elem` join `entries_that_take_a_flag_compile_on_every_backend`.

   *The forty-fifth C++ landing, 2026-11-16:* the last form the forty-second landing left refused by name enters
   with its document and cases (§5), and with it the Rust printer writes the form: a list always there, held to
   the law its entries say their inputs by, that shares its count with a list a gate decides (docs/adr/0065 §6
   R5, R8). The new resource `codec_repeat_flag_aligned_always` is `codec_repeat_flag_aligned_group` turned over:
   its `links`, always on the wire, are handed `head.W` as the input they say, and its `weights`, there where
   `head.H` holds, are plain. Every arm of the group of `head.H` holds the links, as the first list of the
   `Aligned` where the arm holds both and alone in the other, so `head.W` is the start of the links whichever arm
   the value holds. Writing the Rust side found the two defects the forty-fourth landing named and did not fix,
   which no document had reached. The encode read the start of the list as if the value held it alone
   (`start(&read)`), while the group binds each of its members off the arm the value holds as an `Option`, the
   list always there among them; it now reads it through the `Option` wherever a group holds the list, as the
   list's count was already read, and the measure is there exactly where a gate of the list's own leaves it in,
   as the plan says (`rust_source::read_optional`, which the two checks a list is read for share). The eager
   decode of a group by value declared a list it fills in place unassigned unless the statement called `refill`:
   a list held to its law calls `refill_from`, so it was filled before it was assigned; it now starts from the
   list's default either way (`generator::decode_into_local`). The pair's in-place door takes no list held to a
   law in either place, as the forty-fourth landing found for the second, so such a group is decoded by value.
   The C++ printer read the list off each arm as it reads any member of a group, so the refusal named nothing
   it lacked, and `cpp_source::grouped_law_forms` is gone with it: the C++ printer refuses no form of a list held
   to a law. `binary_model.py` already read the shape. The catalog gains 8 sources and no refused case: the
   encode is total, the count held within its field by the lists' bound. The census holds the codec printed, and
   a codec that embeds it printed whole. Kotlin, Go, Python and C11 compile the codec as they compile every codec
   whose cases are `sources` alone, and Swift where its generator writes it; the codec joins
   `entries_that_take_a_flag_compile_on_every_backend`.

   *Amendment of 2026-11-17: what a procedure or a validator reads of a printed C++ value, and how it makes one
   again.* The first landing gave a procedure's `<assign>` to one element a rebuild of the value by its constructor,
   and every later landing narrowed what the constructor takes without the rebuild following: it still handed the
   constructor every field of the codec, each read by an accessor of its name, so a length, a count or an encoded
   length the encode derives (§3), and each member of a group, was read by an accessor the value does not have, and
   a byte string or a text was handed as the expression gave it where the constructor takes its bounded view. A read
   of a derived number called the same missing accessor. No document of the repository reached either, so nothing
   failed. The C++ reader is now written from the value's interface, through one record of what a reader knows of a
   printed codec (`cpp_source::CppPrinted`, held by the import as `ImportContext::cpp_printed`):
   - the rebuild hands the constructor exactly what it takes, in its order: each element the value presents alone
     by its accessor, each group and the arm of the variant whole. A byte string or a text assigned is held first and
     made by `make` of the type the accessor presents it as, read with `decltype`, which views it; a refusal of the
     bound or of UTF-8 raises `error.execution`, as a value over a bytes slot's cap does, before the value is made
     again. An assign to a number the encode derives is refused (`InvalidLvalue`), as no value holds it, and so is an
     assign to a body or a list, which no expression gives;
   - each number the encode derives into a field is answered by a function of the codec's namespace,
     `wire_<field>(src)`, over any source as `input_<name>(src)` is: it reads what the number measures as the encode
     reads it and combines the measures as the encode combines them (the same helpers print both), answering the
     first measure there for a value the encode would refuse, what the value states or what the group holds where
     nothing it measures is there, and none where the field is gated beside its content. `wire_` names it apart
     from a length the value states while its content is absent, which keeps the accessor of its own name. A number
     whose body is handed an input the encode is handed, one the value says, or bits of a carrier the encode derives
     is not answered, as the source alone does not say it; no codec of the repository has one;
   - a member of a group, presented only through its group, and a number no function answers are no members of the
     import a C++ expression names: the expression is refused as naming no member, rather than generating a call of
     an accessor the value lacks.
   The decision was between a function over any source and a member function of the value. The first was chosen
   because the encode is already a function over any source and its readers are the encode's own, so the number a
   reader gets is the one the encode writes by construction, for the value and for any consumer type alike, as the
   inputs a value says already are. `printed_codec_reader_generated_test` compiles and runs a procedure and a
   validator over `codec_origin_envelope`, whose `note_len` and `m` are derived and whose `optional` body is gated.

   *Amendment of 2026-11-16: Python before Go.* The order of the four after C++ is not structural, since each
   printer reads the same plan. Go, Kotlin, Python and C11 all took every derived element from the value, which is
   the defect this step closes, so the first of them to land is the one whose printer moves the derived elements
   while moving least else. A Python value already presents its elements in the idiom §2 gives its interface:
   attributes, which a `typing.Protocol` names, so a printed Python codec changes no read of an element it presents.
   A Go interface is a set of methods, and a Go value presents its elements as exported fields of the same names, so
   the first Go landing moves every read of every Go codec it prints, as the first C++ landing did; it lands next,
   by itself. Kotlin and C11 follow.

   *The first Python landing, 2026-11-16:* the positional codecs whose elements are integers, flag sets, enums and
   byte strings, each always there, whose every derived element is the length of one byte string always there, held
   in a field of its own that the length fits, its `sce:length-arith` never taking it below zero, and whose encode
   refuses nothing (`python_source::unlowered` names the first form any other codec uses; such a codec keeps the
   template's encode whole, §6). Each is printed from its plan (`sce-build/src/forge/generator/python_source.rs`):
   - the source, a `typing.Protocol` `{Codec}Source` with a read-only property for each element the value presents;
     the dataclass is one, and so is any object with those attributes;
   - the refinement of a byte string, a class `{Codec}{Element}Bytes` that names its bound, a subclass of the
     runtime's `BoundedBytes` (`sce_forge_runtime/bounded.py`), a `bytes` made only by a constructor that checks
     the bound (§2: a class whose constructor checks). A `bytes` cannot change after it is made, so a value of the
     class holds its bound for as long as it lives. The dataclass holds the element in it, and its `__setattr__`
     makes the class of whatever bytes it is handed, where the value is made and wherever the element is set
     after, so the dataclass, which Python lets a caller assign to, holds its bound too. The decision was between
     this and a frozen dataclass checked in `__post_init__`, which the payload classes of ADR 0086 are; a codec's
     value is assigned to in place (its flag setters, a procedure's `<assign>`), so a frozen one would move every
     such site, and a `__post_init__` alone would leave an assignment unchecked;
   - the encode over any source, `encode_source(src, w)`, a function of the codec's module, which is the codec's
     namespace as a C++ codec's namespace is, and not a method of the value's class, whose attributes an author's
     field names would share; it derives each length as
     the length of the bytes the source presents, less or more its arithmetic, and writes the plan's program: the
     fixed bytes by the same byte expressions the template's encode wrote (`encode_whole_field_read_as`,
     `encode_byte_read_by`, which now read each value as the caller spells it), a VLE by the runtime's writer, a
     byte string whole. Its `encode` is `encode_source(self, w)`, and it refuses no value: the bound the
     template's encode refused is the class's (§4). The value holds no derived length (§3): the dataclass declares
     none, the decode reads it to cut the bytes and makes the value without it.

   The harness runs each printed codec's `cases`, made and compared without the elements the encode derives, as the
   C++ harness did until its thirty-seventh landing: the fixture's `printed` (renamed from `cpp_printed`, as it now
   says whether the harness language prints the codec, C++ or Python) says so. The Python case printer
   (`python_source::source_case`) writes each `source` of a printed codec as its dataclass made by its constructor,
   reading the notation where the Rust and C++ cases read it; the harness encodes it with the module's
   `encode_source`, compares the bytes, decodes them and compares each element (§5). It states only the shapes the
   Python printer prints and refuses any other by name, and a refused case waits for the first Python encode that
   refuses, whose runtime then names each refusal (`EncodeFailure::symbol`). `a_python_codec_is_printed_from_its_plan_once_it_lowers_every_form` holds a
   printed codec's module, that every codec of the resources is printed exactly where the printer lowers every form
   of it, and that the others keep the template's encode.

   *The second Python landing, 2026-11-16:* text of a stated bound, an element one flag of the codec's own decides,
   and the flags the encode derives from it. A text is a value of `{Codec}{Element}Text`, a subclass of the runtime's
   `BoundedText` (a `str` held to its bound in the UTF-8 bytes every backend counts, refused as `InvalidUtf8` where no
   UTF-8 holds it), and a length of it is derived from those bytes. A gated element is `None` where it is absent; the
   encode folds the flag that gates it into its carrier from whether it is there, in place of the bit the value
   presents, and writes it only where it is there; a length gated beside it is derived as zero where it is absent
   and written where its gate holds, tested on the carrier as the encode writes it. A length the value states where
   its content is absent (§4.6 R4b), a group, and a gate on an input or a chain's entries stay unlowered.

   *The third Python landing, 2026-11-16:* a body of a codec Python prints, handed no flag input, and its encoded
   length. Whether an import is printed is read as the import is generated (`ImportContext::python_printed`), as
   C++ reads its own. The printed parent imports the body's module whole under `sce_<snake>` and writes the body by
   that module's `encode_source`, so a source presents the body as any source of its codec (`{Body}Source`); the
   value's field defaults to the body's codec's value, made by its class. A length of the body's frame is measured
   by encoding the body once into a buffer, which is then what is written where the body is: the Python runtime has
   no counting sink, and the one encode both counts and writes. A body handed flag inputs, or whose encode can
   refuse, stays unlowered, and so does a codec that holds one.

   *The fourth Python landing, 2026-11-16:* a list, and a chain that ends with its scope, of entries of a codec
   Python prints, handed no flag input and held to no law, and the count of its entries. The refinement of R2 is a
   class `{Codec}{Element}List`, a subclass of the runtime's `BoundedTuple`: a tuple, so a list made within its bound
   cannot grow past it, which a Python `list` could. The encode derives the count from how many entries the source's
   list holds and walks it, each entry written by its codec's module's `encode_source`. A chain that sets its
   entries' flag, hands an entry what the one before it was, holds a bare entry's place or is held to a law (R6, R7,
   R8) stays unlowered.

   *The fifth Python landing, 2026-11-16:* an encode that refuses. Each refusal the totality analysis names is one
   the printed encode raises (`assert_all_written`), as the runtime's exception class of its name: a gate the encode
   cannot derive, held to the field it gates on the carrier as the encode writes it (`PresentIfMismatch`); a number
   several elements measure, every one there saying the first's (`InconsistentSource`, which the Python runtime
   gains); a number past its field (`LengthOutOfRange`). The checks run before the first byte. Where a codec it holds
   can refuse, the encode writes its frame whole into a buffer first and hands the buffer to the caller's sink once it
   is written, the Python form of the count over a sink that keeps nothing (§1), since the runtime has no counting
   sink and one pass both measures and writes. `EncodeFailure::symbol` names each refusal for Python, so the harness
   runs the refused cases of a printed codec, holding the encode to raising that class and leaving the sink empty. An
   input a held codec says, checked against what the codec is handed, waits for the flag inputs.

   *The sixth Python landing, 2026-11-16:* a length the value states while all of the content it measures is absent
   (ADR 0058 §1), where that content lies under more than one gate and so joins no group (docs/adr/0065 §6 R4b). The
   value presents it as `Optional[int]`, `None` by default. The encode derives it from the content that is there, the
   first one there fixing it and each other one there saying the same, and reads what the value states where none is;
   a value that states it beside content that says otherwise is refused (`InconsistentSource`). The decode gives it
   back `None` where it read any of the content and the number it read where it read none, which the template's
   decode takes from the printer (`PythonSource::decoded`), so a decode and a case state one value. A length a group
   holds on the side without its content waits for the groups.

   *The seventh Python landing, 2026-11-16:* a length or a count held in a flag of a carrier. The encode folds it
   into the carrier as it folds a presence flag, in place of the bits the value presents, where what it measures is
   there, and leaves the value's bits where the number is free and none is; where the grammar does not bound the
   number within the flag's bits, a number past them is refused (`LengthOutOfRange`). The value keeps the carrier
   whole, as every printer presents a flag set.

   *The eighth Python landing, 2026-11-16:* the flag inputs a value does not say. `encode_source` takes each after
   the sink, in the order the codec declares them, as the decode takes them after the cursor, and the value's
   `encode` hands on what it is handed. A gate tests such an input as it tests a flag, a check among them. A body or
   the entries of a list are handed each input their codec takes and does not say: the bits of a carrier as the
   encode writes it, or an input of the codec's own passed on; the encodes that measure a body's frame run once the
   carriers are folded, so a body is handed the bits the frame carries. The case printer hands a case's `inputs` to
   the decode and the encode. An input the value says, which a parent reads off the value, waits for its landing.

   *The ninth Python landing, 2026-11-16:* the flag inputs a value says (docs/adr/0065 §6 R4a), by the field gated on
   one alone or by the body always there it is handed on to. Each is read off any source by a function of the
   codec's module, `input_<name>(src)`, set exactly where the field is there or what the body's own `input_<name>`
   answers (`None` where that body can say nothing of it, R4e); the encode and the value's `encode` are not handed
   it, and the decode is handed every input. A gate or a check that reads such an input reads it off the source
   first, under the name a given input has. A flag of a carrier bound to an input a body says is that body's
   `input_<name>`, folded as any derived flag is. An input a body or the entries of a list say beside what the codec
   hands them from a carrier or an input of its own is held to it before the first byte (`PresentIfMismatch`); the
   checks of a codec's own now precede the encode of a body whose length it measures, as they do in C++. A parent
   the template writes hands a printed codec's encode no input the codec says (`ImportContext::python_printed`). The
   case printer holds the value to saying each input the case hands it that the value says. An input a group, the
   arms of a variant or the start of a list says waits for that form.

   *The tenth Python landing, 2026-11-16:* gate groups of a codec's own flags or of its flag inputs (docs/adr/0065 §6
   R4b, R4c). A group is one field of the value, named by its gates as every printer names it (`group_accessor`):
   each arm is a dataclass of the members present while the gates hold the arm's values, held as an element alone is
   but always there, and a group of several arms is the `typing.Union` of their classes; it is `Optional` where the
   gates allow no arm. The decision was between a dataclass per arm and one dataclass with a tag member: a tag is a
   second statement of which arm the value holds, which a value could make disagree with the members it holds, while
   the class of the arm is that statement alone, and `isinstance` reads it. The encode binds each member to a local
   off the arm the value holds, `None` where that arm does not hold it, so every statement that reads a member reads
   it as it reads an element a gate decides; each flag of a disjunction is derived from the arm (`ArmFlag`), a
   presence flag of members held together from the first, a length on the side without its content is read off that
   arm (`Absent::Group`), and an input a group says is what its arm gives it. The decode makes the arm the gates it
   read select, from the locals it read each member into, and its members are no fields of the value of their own.
   The case printer states a group as the arm's class made of its members, compared whole. A group of a chain's
   memberships (R6) and two lists of one count (R5) wait for their landings.

   *The eleventh Python landing, 2026-11-16:* two lists of one count (docs/adr/0065 §6 R5). The arm that holds both
   holds them as one member, named as every printer names it (`aligned_name`), a value of the runtime's `Aligned`
   (`sce_forge_runtime/bounded.py`): made only by a constructor that compares the two counts and raises
   `InconsistentSource` where they differ, with no way to set either list after, as `Aligned` is in Rust and C++.
   Each list in it is held in the class of its bound, made where the arm's member is set. The encode binds each list
   to its local off the pair where the arm holds both and off the arm where it holds the first alone, so the count is
   derived from the first as before and the second is no measure of it; the decode pairs the two lists it read. The
   case printer states the pair as `Aligned` of the two lists the source states by their own ids.

   *The twelfth Python landing, 2026-11-16:* a variant. The value's `body` holds the arm itself: a value of a named
   arm's codec's class, or a `{Codec}Default` of the tag the default arm keeps and its body, and its type
   `{Codec}Variant` is the `typing.Union` of those classes, as the C++ value holds a `std::variant` of a source of
   each arm's codec and the catch-all struct. The decision was between this and a dataclass for every arm, which a
   group's arms are: the class of a named arm's body already says which arm it is, so a wrapper would state it twice,
   and the catch-all needs one only because its body may be of a named arm's class. Two named arms whose codecs are
   one Python class could not be told apart, and `python_source::unlowered` names that form; no codec of the
   resources has it. The default arm holds its tag in `{Codec}UnknownTag`, a subclass of the runtime's `UnknownTag`
   (R3): an `int` made only by a constructor that refuses a tag an arm names or one past the tag's bits, with
   `ValueError`, as `chr` refuses a number no character is, and the default arm's `__setattr__` holds it wherever it
   is set. The encode refuses a `body` that holds no arm with `TypeError`, before the first byte. It derives the tag
   from the arm into the field or the flag that holds it (`Derive::VariantTag`), writes the arm by its codec's own
   `encode_source`, handed what that codec takes, and holds the input an arm says to what the arm is handed
   (`CheckSite::Arm`). A flag bound to an input the arms say is what the arm the value holds says
   (`Measured::ArmSays`), and an input the arms say (`EveryArm`, `SomeArms`) is read off the arm by the module's
   `input_<name>`. A variant whose caller hands its tag has the module's `tag(src)`, the tag of the arm any source
   holds, and the value's `tag()` answers it; a parent derives the bits a held body's arm fills
   (`Derive::ParentTag`) through that body's codec's `tag`, a printed parent and a parent the template writes alike.
   The decode makes the arm it read, the default arm of the tag it read and the body. A peek byte is the arm's own
   header and needs nothing of the parent. The case printer states the arm as the arm's codec's value, or the
   default arm's class of the tag and the body, hands a caller's tag to the decode after the inputs, and holds the
   value and its decode to answering it.

   *The thirteenth Python landing, 2026-11-16:* the walk of a chain that ends on its entries' flag or hands each
   entry what the one before it was. Each one-bit flag a value presents of a fixed carrier is one a parent may set
   (`EncodePlan::overridable`): the encode takes it as a keyword after the inputs, `override_<carrier>_<flag>`,
   `None` by default, and folds it into the carrier last, so what the parent says is what the wire holds, as the C++
   encode takes its `Overrides`. The walk sets the flag its entries end on, 1 on every entry but the last, and keeps
   for each input an entry takes from the one before (`HandedFrom` none) whether the entry it wrote last was of the
   kind the input waits for, read off the entry by the chain's `entry-id`, 0 before the first. A chain of a least
   past zero (R7) starts with that many entries made by default, a factory so no two values share one. A chain that
   holds a bare entry's place (R6a), is held to a law (R8), or whose entries say the input they are handed from the
   entry before waits for its landing.

   *The fourteenth Python landing, 2026-11-16:* a list held to the law its entries say their inputs by (docs/adr/0065
   §6 R8). The value holds it as a `{Codec}{List}Lawful`, a subclass of the runtime's `Lawful`
   (`sce_forge_runtime/law.py`): the start the value gives and the entries in the class of their bound, made only by
   a constructor that runs the law over them and raises `PresentIfMismatch` where an entry says otherwise, with no
   way to change either after, as `Lawful` is in Rust and C++. The law is `{Codec}{List}Law`, named as every printer
   names it, a tuple of the runtime's automata, one for each input: `Uniform` of a flag or an input bound to every
   entry, `AfterKey` of an input the chain supplies from the entry before, each reading the input off an entry
   through the entry codec's own `input_<name>`, and the kind `AfterKey` waits for a class `{Codec}{Chain}{Kind}Key`
   whose `matches` reads the chain's `entry-id`. The start is a `bool` of one input, `None` where the entry before
   decides it, a tuple of each one's for several. A `Lawful` reads as its entries, so the walk and the count read it
   as they read a list. A flag bound to every entry is derived from the start (`Measured::Start`), and an input the
   start says is the module's `input_<name>`, `None` where a gate leaves the list out, read clear by a gate. The
   decode holds the list it read to the law from the carrier's bit or the input it is handed, and makes the value
   inside the `try:` that answers a refused frame with `None`, so a frame whose entries break the law decodes to
   nothing. As in C++, a list on one side of a gate held to a law over the codec's own input, or sharing its count
   with another (R5), stays unlowered. The case printer states the list as its `Lawful` of the start and the entries
   the source states.

   *The fifteenth Python landing, 2026-11-16:* the gates on a chain's entries (docs/adr/0065 §6 R6). A group of a
   chain's memberships holds the chain in each arm as a class of the arm's own, `{Arm}Chain`, a subclass of the
   runtime's `Chain` (`sce_forge_runtime/membership.py`): a tuple of the entries where they stand, made only where it
   holds an entry of each kind of `HOLDS` and of none of `LACKS`, `Optional` where a gate of the chain's own may leave
   it out and the arm holds no kind (R6b). Where the one gate on a bare entry holds, the arm holds `{Arm}Placed`, a
   subclass of the runtime's `Placed`: the other entries and the place of the first entry of the kind among them,
   which reads as the chain whole, the named entry made by the kind's `named()` at its place, so the walk that writes
   a chain and the count that measures it read it as they read a list (R6a). The decision was between this and a walk
   of slots in the encode, as C++ and Rust write it: a `Placed` that reads as its entries keeps the slots in the one
   class that holds the place, and the encode's walk, count and kinds read every chain alike. Each kind is a class
   `{Codec}{Chain}{Kind}Key`, named as every printer names it, whose `matches` reads the chain's `entry-id` and, for a
   bare entry's kind, whose `named()` makes the entry of its identifier, its arm the one its tag bits select through
   the entry codec's `ARM_CLASSES`, which every printed module with a variant now defines: the class of the arm each
   tag names. A gate on a membership a group holds is what the arm the value holds says; any other is whether the
   chain the value holds has an entry of the kind, read as the chain's `entry-id` says, and the decode makes the arm
   its read chain's kinds select. A value made by default holds the arm present while no gate holds, as in C++,
   which for a group of a chain's memberships is the arm whose chain must hold no kind. An input a chain's entry says
   that the chain supplies from the entry before, where the memberships hold the chain to no law, is held entry by
   entry to whether the entry before was of the kind (`Against::PreviousEntry`, R8b). As in C++, a chain of slots
   whose entries are held to an input stays unlowered. The case printer states the chain as the arm's `Chain` of its
   entries, or its `Placed` of `{"rest", "place"}`.

   *Amendment of 2026-11-17: an integer is held to its field's width.* A Python `int` has no width, so a printed
   value held any number in an integer element and its encode wrote the bits the field holds of it (`& 0xFF`),
   losing the rest without a word: the encode was total over values no Rust or C++ type holds. Decided: a narrow
   integer is refined by the runtime's `BoundedInt` (`sce_forge_runtime/bounded.py`), a class of its width and
   signedness, applied alike to the positional and the CBOR printers, so a number past its field's width is
   refused where the value is made, never masked. The width is the type's, narrowed to the bits a fixed field has
   on the wire, as the wire holds no more; a VLE's type bounds its value alone. The refusal is `OverflowError`, as
   `int.to_bytes` answers a number its bytes cannot hold, and not a `CodecError`: no frame holds such a number,
   since a decode reads exactly the field's bits. A signed field's bits are read as their two's complement
   (`BoundedInt.from_wire`), which the template's decode had read as an unsigned number. The decision was between
   a class per element, which names its element in the refusal and is spelled as the class of a byte string's bound
   is, and one runtime class per width shared by every codec: the first was taken, for the one spelling of every
   refinement of a printed module. Each element is `{Codec}{Element}Int`, held by the value's `__setattr__` as a
   byte string is, a group's member and a length the value states among them.

   *The sixteenth Python landing, 2026-11-17:* a CBOR map, printed from its own plan (`python_source::cbor`), as
   the C++ printer prints it. Every entry is presented, a member of `{Codec}Source` each, `Optional` for one the map
   may lack: an unsigned integer as the class of its width (`BoundedInt`), a text or a byte string as the class of
   its bound (`BoundedText`, `BoundedBytes`, an exact length a least equal to its most, so the encode refuses no
   `CborWrongLength`), a list of records as the class of its most (`BoundedTuple`, its least 0, as Rust's
   `BoundedList` holds it). The value's `__setattr__` makes each class wherever an entry is set. What no class
   holds the encode refuses before the first byte with `CborOutOfRange`: the refusals the totality names (a negative
   value of an enum whose carrier is signed, a list of fewer records than its lower count, a record whose byte
   string is past its bound) and, since the payload type of a Python event schema's unit holds a record's text and
   integer in a bare `str` and `int`, a record's text past its bound and its integer past its width, a refusal no
   Rust or C++ value reaches. The encode then writes the map head, counting the optional entries present, and each
   entry present in ascending key order (`by_key`), a list of records as an array of one map per record, each
   written by the module's `_encode_record_<alias>`. The template's encode of a map, its checks and the record writes
   of its class are gone: every CBOR plan is printed.

   *The seventeenth Python landing, 2026-11-17:* a content-line component, printed from its own plan
   (`python_source::content_line`), as the C++ printer prints it. Every entry is presented, a member of
   `{Codec}Source` each: a property's value and each parameter declared for it, `Optional` for one that may be
   absent, a `Sequence` for a repeated property and for the values of one line, and a `Sequence` of line records
   for a repeated property that declares a parameter or whose line holds a list of values, each record a source of
   its own `{Codec}{Entry}LineSource`. A line refuses a text by its content as well as its size, so no bound refines
   a text or a list, as in C++: the plan's checks stay checks. An integer held alone, a property's value or a line
   record's, is refined by the class of its width (`BoundedInt`), as the amendment of 2026-11-17 has every printed
   integer; one in a list is held to its width as its line is written (`LineBadValue`), since a value appends to
   its list after it is made and no class sees the append. The encode writes each property's lines in declaration
   order, the judgements of the plan before a line (a required list empty, a list past its `sce:max-count`, a
   parameter given without its property), into a buffer first where a line can refuse the value, so a refusal
   leaves the caller's sink untouched, as the positional encode stages a frame (§1). The template's encode is
   gone: every content-line plan is printed.

   *Amendment of 2026-11-17: what the Python printer refuses, and why.* Of the forms the Python printer named
   unlowered beside the C++ printer's, two are invariants of the analysis and no form: a VLE of eight bits, which
   the parser refuses ("vle requires sce:type in {uint16, uint32, uint64}"), and a flag the presence of several
   fields decides that are not held together, which `codec_roles::gate_groups` makes the sides of one group. The
   printer asserts both where it writes them, as the C++ printer does. Two stay refused by name. Two named arms
   whose codecs are one Python class cannot be told apart by the arm's class, which is how a printed value says
   its arm (the twelfth landing). A streamed field of part of its bytes, which no codec of the repository has,
   waits until a byte case (§5) holds its write against its decode. A field of a float or a bool has no wire form
   any printer writes: the page states none, the C++ and Swift printers refuse it by name, and the templates'
   encodes shift or cast its value as if it were an integer, never its bits, so lifting it is a decision of
   the page (its bits, its byte order, a bool's values) taken for every printer together with its byte cases (§5),
   and not a Python landing.

   *The eighteenth Python landing, 2026-11-17:* the Python backend prints every codec from its plan or refuses it
   by name, and the template's encode is removed (§6), as the fortieth C++ landing removed C++'s. No codec of the
   repository uses a form the printer lacks, so the landing moves none. What is left is refused as
   `generate/unsupported-feature` with the clause `python_source::unlowered` names (`codec 'x': field 'f': ...`):
   a float or a bool, two named arms of one class, a streamed field of part of its bytes, and a body, a list or an
   arm of a codec the Python backend refuses, by that codec's own clause, which the import carries
   (`ImportContext::python_unlowered`, read as the codec is generated, its own imports resolved); a document of
   another kind that imports a refused codec is refused for it. `render_codec` asks no Python codec for the
   template's per-field encode blocks, its encode expressions or its prologue, and the template's encode of a
   value, its variant class of a `kind` and a member for each arm, and its `tag()` over that class are gone. The
   census pins the printed codecs of the resources (`PRINTED`) and each refused codec of the resources and of the
   rest of the repository with its clause (`REFUSED`, `REFUSED_ELSEWHERE`, both empty), and holds the refusal of a
   float, of a codec that embeds one, and of a validator that imports one.

   *Amendment of 2026-11-17: the template's encode helpers refuse Python by name.* The helpers that wrote the
   template's encode of a field one by one (the streamed list, chain and body blocks, gated or not, the gated
   field's writes of each width, the derivation of a parent's tag and its carrier write, and the prologue's
   presence checks) refuse Python by name (`PYTHON_ENCODE_IS_PRINTED`), as they refuse Rust and C++, so a call
   site that lets a Python codec through fails where it is made and not in the text it writes. The helpers only
   Python's template called are gone: the term a parent read a dispatcher's arm by, and the byte writes of a gated
   fixed field. A printed Python value holds each bound in a class, so the template's check of a bound at the
   encode is written for Kotlin, Go and Swift alone (`encode_bound_guard_lines`). The helpers the printer calls,
   a whole field's bytes, a byte of pieces, a multibyte field's bytes and the VLE writer, keep their Python arms.
   The carriers a parent's tag fills, and the local each gathers its bits in, are formed only where the template
   writes the encode (`field_encode_blocks`), so their naming refuses Rust, C++ and Python as the writes do.

   *Amendment of 2026-11-17: byte cases of a CBOR map and of a content-line component (§5).* The conformance
   generator refused `sources` of any codec but a positional one, so the CBOR and content-line encodes every printer
   prints were held only by the `cases` of the builder's fields the catalog lists by hand. Decided: a map and a
   component derive nothing a value says, so a case's `source` is the value whole, each entry keyed by its id as the
   plan presents it: a boolean, an integer within its type, a text within its bound in UTF-8 bytes, a byte string of
   a length its entry admits, `null` for an entry the value may lack, and an array for the values of a repeated
   property, `[]` for none. It is the notation the `decoded` objects of these codecs already write, so one object
   states one value in both. The notation is read once, against the plan (`generator::source_notation`), into the
   elements in the order the value presents them, a property's parameters after it, which is the order the C++
   constructor takes them; each printer spells the reading in the module that prints the encoding
   (`{rust,cpp,python}_source::{cbor,content_line}::source_case`), since the literal must agree with the value that
   module declares. Rust makes the map by its fields, a text and a byte string through `Bounded::new`, and the
   component by `new` and the entries set, a text by `SceStr::from_view` and each value of a list by
   `SceList::try_push`; C++ hands the constructor each element, a map's text and byte string by `make` of the view
   its accessor presents and a list as a vector bound first; Python makes the dataclass. A decode is compared with
   the value element by element, and structurally where the value holds a sequence: in C++ a span by its size and
   each value, in Python a list as the `list` of its values, whatever sequence class a decode builds. A value of an
   enum, a list of records, a line of a list of values and lines that are records are refused by name until a case
   needs them, and so are `inputs` and a `tag`, which neither codec takes. The bytes are the models': for a map, a
   reading of §4.6.1 and RFC 8949's deterministic encoding in `gen_cases.py`, held first to the fixture's
   hand-written `cases`, which writes six sources of `codec_cbor_map`; for a component, the content-line model, whose
   twelve hand-chosen values become the sources of `codec_content_line_event`. Each is read back by its model before
   it is written.

   *Amendment of 2026-11-17: a streamed field of part of its bytes is no form.* The amendment on what the Python
   printer refuses, and the eighteenth landing after it, named a streamed field of part of its bytes a form the
   printer refuses until a byte case holds it. Read again: no such field reaches a printer. The parser refuses a field
   of part of its bytes in a frame written field by field, whose every field has bytes of its own, so the bits it
   leaves no other field can use (`bit_layout::refusal`, "give it whole bytes, or lay the codec out by byte"); a field
   of part of its bytes is one of a frame laid out by byte, written as a piece of its byte (`Write::Byte`), which the
   printer writes. Decided: it is an invariant of the analysis, as a VLE of eight bits is, and the printer asserts it
   where it writes a streamed fixed field (`STREAMED_WHOLE`). With it, what the Python printer refuses beside the C++
   printer is two named arms of one class, which no variant of the repository has (each arm of every variant imports
   a document of its own, read 2026-11-17), so the census of the codecs the Python backend prints is the C++ census,
   one list of 167 codecs of the resources, and neither refuses a codec of the repository.

   *The nineteenth Python landing, 2026-11-18:* the forms of a list held to its law that the fourteenth landing left
   unlowered beside C++, which the forty-third to forty-fifth C++ landings print, with their resources
   (`codec_repeat_flag_forward_group`, `codec_repeat_flag_aligned_group`, `codec_repeat_flag_aligned_always`), which
   the Python backend had refused by name since they joined the set every backend compiles with flag entries. A
   list a group holds whose law is over the codec's own input says the input by its start, read off the arm the
   value holds (docs/adr/0065 §6 R4b, R8): `input_<name>` reads the group once, as `_g_<group>`, and answers the
   start of the list where the value holds the arm that holds it and `None` otherwise, as the encode binds a member
   off the arm (`python_source::off_arm`, one reading of a member for both). A list held to its law that shares its
   count with another (R5) is the `Lawful` of it in the arm's `Aligned`, whichever of the two it is, as it is alone:
   set as the `Lawful` of the start and the entries the pair is handed, made by default as the `Lawful` of no
   entries, and decoded as the `Lawful` of the start the frame gave, which raises `PresentIfMismatch` for entries
   that break the law before the two are paired. The flag the start says is derived from the list the encode bound
   off the arm, as for a list alone, and the list always there is read off whichever arm the value holds. A pair
   whose lists start with entries made by default is made anew for every value, since such an entry is a dataclass
   a caller can change. With this the census of the codecs the Python backend prints is the C++ census again, one
   list of 170 codecs of the resources.
   *The Kotlin interface, decided 2026-11-16 under the owner's standing rule.* A Kotlin value presents its elements
   as the properties of an interface, `{Codec}Source`, the idiom §2 gives Kotlin. The codec's data class implements
   it with the properties it already had (`override var`), so a read of an element (`frame.msgId`) and a procedure's
   `<assign>` to one stay as they are, as in Python; a consumer's own class joins by implementing the interface. A
   refinement is a value class (`@JvmInline value class`) whose constructor is private and whose one factory, `of`,
   answers `null` outside the bound, as the Rust runtime's `Bounded::new` answers `None`: the factory is the only way
   to make a value, so the check runs once where it is made (Liskov and Zilles, 1974), and a value class is no box at
   run time. A `ByteArray`'s count and a `String` cannot change after they are made, so the value holds its bound
   for as long as it lives. Considered and refused: a class whose `init` block checks and throws, which makes a
   refusal an exception where §2 gives Kotlin `CodecError?`; and a subclass of a runtime type, as Python's is, which
   Kotlin cannot hold a bound in, having no const generics and a final `ByteArray`. The encode over any source is
   `encodeSource(src, w)` of the data class's companion object, beside `decode`, so a parent and a harness reach it
   as `{Codec}.encodeSource` and no property of the value can take its name. It reads each element off the source
   once, into a local, before its first write, as the C++ encode keeps what it reads (`Detail::Kept`): a property
   that computes is read once, and an element a gate decides is narrowed through a stable local, which Kotlin's
   smart cast asks of an interface's property.

   *The first Kotlin landing, 2026-11-16:* the forms of the first Python landing, each printed from its plan
   (`sce-build/src/forge/generator/kotlin_source.rs`, `kotlin_source::unlowered` naming the first form any other
   codec uses, which keeps the template's encode whole, §6): the positional codecs whose elements are integers, flag
   sets, enums and byte strings, each always there, whose every derived element is the length of one byte string,
   held in a field of its own that the length fits, its `sce:length-arith` never taking it below zero, and whose
   encode refuses nothing. A byte string is a value of `{Codec}{Element}Bytes`, the value class of its bound, which
   the data class holds and its decode makes by `of` of the bytes it read; the data class holds no derived length
   (§3), and its `encode` is `encodeSource(this, w)`, which derives each length from the bytes the source presents
   and writes the plan's program by the byte expressions the template's encode wrote. The harness runs each printed
   codec's `cases` without the elements the encode derives, as the Python harness did first, each byte string made
   by the factory of the class the fixture's field names (`refined`, `kotlin_source::refinement_class`, the one
   place the name is made). `a_kotlin_codec_is_printed_from_its_plan_once_it_lowers_every_form` holds a printed
   codec's file and that every codec of the resources is printed exactly where the printer lowers every form of it.
   The broadcast log of records holds a payload past its bound as no value, as its C++ test does, where its Kotlin
   test held the encode's refusal.

   The Kotlin harness then runs each printed codec's `sources` (§5) as the Python harness does. The Kotlin case
   printer (`kotlin_source::source_case`) writes each `source` as the data class made by its constructor, every
   name written by its document's package so the harness imports nothing for a case, a number as a literal of the
   element's type and a byte string by the factory of its value class; it reads the notation where the Rust, C++
   and Python cases read it. The harness encodes the value by the companion's `encodeSource`, compares the bytes,
   decodes them and compares each element, a byte string by its content, as a value class over an array compares
   the array by identity. It states only the shapes the Kotlin printer prints and refuses any other by name.

   *The second Kotlin landing, 2026-11-16:* text of a stated bound, an element one flag of the codec's own decides,
   and the flags the encode derives from it, as the second Python landing. A text is a value of
   `{Codec}{Element}Text`, a value class over a `String` whose `of` counts the text in the bytes UTF-8 encodes it to
   (`com.sce.forge.runtime.Utf8.size`, which the forge runtime gains) and makes no value of a text no UTF-8 holds, a
   lone surrogate, which `encodeToByteArray` would otherwise write as a replacement character; a length of it is
   derived from those bytes. A gated element is `null` where it is absent: the source presents it as the optional
   of its type, the encode reads it into a local, which Kotlin narrows where it tests it, folds the flag that gates
   it into its carrier from whether it is there, worked on as the unsigned type of the carrier's width so a mask of
   its top bit is a literal, in place of the bit the value presents, and writes it only where it is there. A length
   gated beside it is derived as zero where it is absent and written where its gate holds, tested on the carrier as
   the encode writes it. A length the value states where its content is absent (§4.6 R4b), a group, and a gate on
   an input or a chain's entries stay unlowered. An algorithm that iterates a collection of a printed element reads
   the element's byte string or text out of its value class (`entry.pattern.text`), as the C++ algorithm reads it
   through the accessor; the document set threads which elements Kotlin prints as `element_type_kotlin_printed`
   beside the element schemas.

   *The third Kotlin landing, 2026-11-16:* a body of a codec Kotlin prints, handed no flag input, and its encoded
   length, as the third Python landing. Whether an import is printed is read as the import is generated
   (`ImportContext::kotlin_printed`). The source presents the body as the held codec's `{Body}Source`, which the
   wildcard import of its package brings into scope, and the data class holds the held codec's data class, which
   implements it, as an `override var` of that subtype. The encode writes the body by `{Body}.encodeSource`. A
   length of the body's frame is measured by encoding the body once into a buffer (`MutableListSink`), which is then
   what is written where the body is, `null` where a gate leaves the body out, as the Python printer does: the
   Kotlin runtime has no counting sink, and the one encode both counts and writes. The case printer makes a body as
   its codec's data class, written by that codec's own package, and compares it element by element down into the
   codecs it holds, since a data class compares a value class over an array by identity. A body handed flag inputs,
   or whose encode can refuse, stays unlowered, and so does a codec that holds one.

   *The fourth Kotlin landing, 2026-11-16:* a list, and a chain that ends with its scope, of entries of a codec
   Kotlin prints, handed no flag input and held to no law, and the count of its entries, as the fourth Python
   landing. The refinement of R2 is a value class `{Codec}{Element}List` over a `List` of the entries' codec's
   `{Entry}Source`, whose `of` copies the list it is handed, so no list a caller keeps can grow it past its bound
   after (a Kotlin `List` may be a view of a `MutableList`). The encode derives the count from how many entries the
   list holds and walks it, each entry written by `{Entry}.encodeSource`; the decode makes the class of the entries
   it read. A list of a least past zero, and a chain that sets its entries' flag, hands an entry what the one before
   it was, holds a bare entry's place or is held to a law (R6, R7, R8) stay unlowered. The case printer makes a
   list by its class's `of` over `listOf` of its entries and compares it by its size and each entry element by
   element.

   *The fifth Kotlin landing, 2026-11-16:* an encode that refuses, as the Python refusal landing. `encodeSource`
   answers each refusal the plan's totality names as the runtime's `CodecError` object of that name, returned
   before the first byte is written: a gate it cannot derive (`PresentIfMismatch`, tested on the carriers as the
   encode writes them), measures of one number that disagree (`InconsistentSource`, which the Kotlin runtime
   gains), a number past its field (`LengthOutOfRange`), and what a codec it holds answers. A number of several
   measures is the first always there, each other there held to it; where every measure may be absent it is the
   first one there, a local `Int?`. A frame that holds a codec whose encode can refuse is written whole into a
   staged sink first and handed to the caller's sink only once it is, as the Python printer stages it, since the
   Kotlin runtime has no counting sink; a refusal then leaves the caller's sink as it was. A carrier a derived flag
   is folded into is worked on as a `var` of the unsigned type of its width (`_u_<carrier>`), one statement a
   flag, and read back as its own type (`_c_<carrier>`), so a flag the encode folds in under a test, as a length
   held in a flag is, folds in the same way. Considered and refused: a refusal as an exception, which §2 does not
   give Kotlin; and a counting pass before the write, which would encode each held codec twice where it can refuse.
   The Kotlin harness runs the codec's `refused` cases (`EncodeFailure::symbol` names each refusal as
   `com.sce.forge.runtime.CodecError.<Name>`) and holds the sink to empty. An input a held codec says, held to
   what the codec is handed, stays unlowered.

   *The sixth Kotlin landing, 2026-11-16:* a length the value states while the content it measures is absent
   (§4.6 R4b, docs/adr/0058 §1), as the Python landing of it. The source presents it as the optional of its type
   and the data class holds it, `null` by default. The encode derives it from the first content there and reads
   what the value states only where none is, zero where the value states nothing; a value that states it beside
   content that says another number is refused as `InconsistentSource`. The decode makes it `null` where it read
   any of the content, so a decoded value states it exactly where an encode reads it. A group that holds such a
   length beside its content (R4b's group form) stays unlowered.

   *The seventh Kotlin landing, 2026-11-16:* a length held in a flag of a carrier (`sce:length-ref` naming
   `carrier.flag`). The encode folds the number into the carrier's bits as it folds a presence flag, a statement on
   `_u_<carrier>` in place of the bits the value presents, under the test that its content is there where it may
   be absent; a number the flag's bits cannot hold, where no bound holds it within them, is refused as
   `LengthOutOfRange` before the first byte. The value's other flags of that carrier are written as it presents
   them.

   *The eighth Kotlin landing, 2026-11-16:* the flag inputs a codec takes that its value does not say (§1.5, the
   plan's `given`). `encodeSource` takes each after the sink as a `UByte`, as the decode takes it after the
   cursor, and a gate tests it as it tests a flag; the data class's `encode`, whose value says none, hands every
   input on. A body or a list's entry is handed what the plan says it is handed: the bits of a carrier as the
   encode writes it, or an input of the codec's own passed on. The case printer hands a case's inputs to the
   decode and the encode. An input the value says (R4a), and an entry handed what the entry before it was, stay
   unlowered.

   *The ninth Kotlin landing, 2026-11-16:* the inputs a value says (§6 R4a) through the one field gated on the
   input alone, or through a body always there that says it in every value, and what a parent derives from or
   checks against such a body or list entry. `encodeSource` is not handed such an input: it reads it off the value
   where a gate or a check reads it. The companion answers each off any source, `input<Name>(src): UByte`, which
   a printed parent reads to derive the flag it binds to the input (`Measured::Says`) and to hold a body or each
   entry of a list it hands the input from a carrier or an input of its own (`input_checks`, refused as
   `PresentIfMismatch`). The data class's own `encode` keeps every input as a parameter, since a codec on the
   template's encode and the broadcast log hand every input to it, and holds each one the value says to what the
   value says before it writes, refusing a contradiction as `PresentIfMismatch` as the template's encode did; it
   hands the rest on. The decision was between that and dropping the parameter, as the Rust and C++ encodes do,
   which would leave a parent on the template's encode writing its own flag beside a body that says otherwise,
   unchecked; the parameter goes with the template's encode (§6). An input said through a group, a variant, a
   list's law or a body that says it in some values only stays unlowered with those forms.

   *The tenth Kotlin landing, 2026-11-16:* a variant whose arms are codecs Kotlin prints, in each of its shapes: a
   tag of its own, in a field or a flag of a carrier (`Derive::VariantTag`), a peek byte, which the arm's own
   header writes, a tag the caller hands, which the caller writes, and an arm a build feature decides, which every
   Kotlin build keeps as the template does; and a parent whose carrier's bits the arm of an embedded body's
   variant fills (`Derive::ParentTag`). The source presents the arm as `body`, the template's sealed class
   `{Codec}Variant`, each of whose arms holds `{Arm}Source` as a list holds `{Entry}Source`, so a source of the
   arm's codec and not its data class alone is an arm. The catch-all holds its tag in `{Codec}UnknownTag`, the
   refinement of R3 as §2 gives it Kotlin: a value class whose `of` makes a value only of a tag the bits hold and
   no arm names, so the encode refuses nothing of it and the decode, which reaches the catch-all only under such
   a tag, makes it by `of`. The encode reads the arm once, derives the tag as a `ULong` by an exhaustive `when`
   over the sealed class into the tag field or folds it into the carrier, and writes the arm by its codec's
   `encodeSource`; a parent reads each body's arm through the body's source and folds the arm's tag, masked, into
   its carrier. The name of an arm is made in one place (`generator::variant_arm_name`), which the templates and
   the printer read. Considered and refused: arms that hold the arm's data class, which would hold a source the
   encode reads to one class where §2 gives a value any class that presents it, and a catch-all tag of the bare
   number, which the encode would have to refuse where an arm names it, a refusal the analysis does not name. The
   case printer states a variant as the arm's class made of its codec's data class, compares the arm, its body
   element by element and the catch-all's tag, and hands a caller-tag codec's decode the case's tag. An input the
   arms say (R4d, R4e), and a check of an input at an arm, stay unlowered.

   *The eleventh Kotlin landing, 2026-11-16:* the rest of R4a that needs no group, chain or law: an input the arms
   of the variant say, every arm (R4d) or some (R4e), an input a body says in some of its values only, a flag a
   parent derives from what the arm it holds says (`Measured::ArmSays`), and an input check at an arm or of a codec
   that says the input in some values only. A said input is read off the arm by an exhaustive `when` over the
   sealed class, each arm by its codec's `input<Name>`; where the value can say nothing of it the companion's
   function answers `UByte?`, `null` from an arm that does not read it, as the C++ function answers an empty
   `std::optional`, so `null` is the absence §2 gives Kotlin and no sentinel number stands for it. The data class's
   own `encode` and a parent's check hold what is handed to what is said only where something is said, as R4e
   asks; a check at an arm tests the local the arm is read into, which Kotlin narrows to the arm's class. An input
   said through a group or a list's law stays with those forms, and so do a check against the entry before and
   a codec handed what the entry before it was, which only a chain's walk says.

   *The twelfth Kotlin landing, 2026-11-17:* a gate group of flags of the codec's own or of flag inputs (§6 R4b,
   R4c): two elements one flag decides, a length stated on the side without its content (`Absent::Group`), a
   disjunction whose flags each arm sets (`Derive::ArmFlag`), and an input a group decides, which the value says
   (`InputSource::Group`). The value holds the group as one property, named as every printer names it
   (`group_accessor`): a sealed class `{Codec}{Gates}` of a data class for each arm, nested and named after the
   flags the arm sets (`group_arm_name`), as the variant's sealed class nests its arms, or one data class where
   the group has one arm, and `null` where the gates allow no arm. An arm holds each member always there, typed as
   an element alone is, its properties `val`, since an arm is one value of the sum. The encode reads the group once
   and binds each member off the arm the value holds to the local an element is read into, `null` where the arm
   holds none, by a `when` with a case for each arm that holds it (a case of several classes narrows to none of
   them), so every statement that reads a member reads it as it reads a gated element; a member every arm holds,
   of a group the value always holds an arm of, is no `null`, and is written without a test, as kotlinc refuses
   a comparison it calls always true. The decode makes the arm the gates it read select, each member of the local
   it read, asserted present (`!!`) where the decode read it under the gate that selects the arm. The case printer
   states a group as the arm's class made of its members and compares it arm by arm, each member as an element is.
   Considered and refused: arms of `var` properties, which would let a value change one member of an arm in place
   where §2 makes an arm a value; and a flat class of optional members, which is the struct the group replaces.
   Two lists of one count and a group of a chain's memberships stay unlowered. *Amended 2026-11-18:* a value
   made by default holds the arm present while no gate holds, where there is one, else the first, as the Rust,
   C++ and Python values do (`group::declare`); this landing first made it in the first arm, a difference no
   reason held.

   *The thirteenth Kotlin landing, 2026-11-17:* two lists of one count (§6 R5). The Kotlin runtime gains
   `Counted`, what a list class answers its count by, which every printed list class implements, and
   `Aligned<A : Counted, B : Counted>`, a class whose constructor is private and whose `of` makes a value only of
   two lists of one count, so the encode that writes the one count of both refuses nothing, as §2 gives Kotlin a
   refinement. The arm that holds both lists holds them as one member named as every printer names it
   (`aligned_name`); the encode reads each list out of the pair and derives the count from the first, the plan
   measuring the second not at all (`counted_with_another`); the decode pairs the two lists it read. Considered and
   refused: a class generated for each pair, which would spell one check once per codec where the runtime spells
   it once; and a generic `of` handed a counting function, which a list class that answers its own count makes
   needless.

   *The fourteenth Kotlin landing, 2026-11-17:* chains that set their entries' flag and hand an entry what the
   entry before it was (`ListWrite.ends`, `ListWrite.previous`, a `Handed` of no `from`), and an entry held to
   what the entry before it is (`Against::PreviousEntry`). A codec whose value presents one-bit flags of a fixed
   carrier (`EncodePlan::overridable`) declares `{Codec}Overrides`, a class of a `Boolean?` for each, and its
   `encodeSource` takes one last, `null` by default, folding each flag set into the carrier after every flag it
   derives, as the C++ `Overrides` and the Python keywords do: a parameter of default `null` changes no call
   that sets none. The walk of a chain counts the entries left and sets the flag the entries end on through the
   entry codec's class, and keeps, for each input a bind waits on a kind for, whether the entry it wrote last is
   of the kind (`_prev_<input>`), read off the entry by the chain's `entry-id` as a `ULong`; the check of an
   entry that says such an input walks the chain the same way before the first byte. Considered and refused: a
   parameter for each flag, which would grow every codec's signature by its flags; and an entry's own flag the
   chain reads back and checks, which would leave a caller to set where each entry stands.

   *The fifteenth Kotlin landing, 2026-11-17:* a list of a least past zero, which a chain closed by its entries'
   flag is (§6 R7): every such chain holds one entry at least, so the fourteenth landing's chains were refused by
   the form it did not yet print. The class of the list's bound names `LEAST` beside `MOST`, `of` makes no value
   of fewer entries, and `fewest`, what a value made by default holds, holds `LEAST` entries each made by default
   by the entries' codec's data class, as the C++ `Chain` makes its fewest of value-made entries. The bound's
   words, constants, test and fewest are read once (`ListBound`), which a chain an arm of a group holds reads
   too. An `Aligned` made by default pairs the fewest of two lists of one count; only a chain has a least past
   zero, and a chain has no count, so the two fewest are always of one count. Considered and refused: an empty
   list by default with `of` alone checking the least, which would make a value of the class no frame carries.

   *The sixteenth Kotlin landing, 2026-11-17:* a group the gates on a chain's entries decide, and the flags joined
   with them (§6 R6b), and a gate on a chain's kinds no group holds. Each arm holds the chain in a value class of
   the kinds it says the chain holds an entry of and of none (`group::chain_class`, named after those kinds, so
   arms that say the same share one), whose `of` makes a value only of entries within the chain's bound that hold
   each kind it holds and none it lacks, each told apart by the chain's `entry-id` as the walk tells it; every
   such class implements `Counted` and holds `entries`. An arm that says the gated chain holds no kind holds it as
   the optional of its class, as the Rust and C++ arms do, `null` by default. An arm that says the chain holds a
   kind has no chain made by default, so a value made by default holds the arm where no gate holds, whose chain is
   the fewest entries its bound holds, each made by default and held by `of` to no kind it lacks. The encode reads
   the one list every class holds off the arm the value holds, so every statement that reads the chain reads a
   list of entry sources whichever arm it is; a gate on a membership the group holds is what the arm says
   (`group::gate_holds`), and any other a scan of the chain's entries. The decode selects the arm by the kinds the
   chain it read holds, the locals its template declares after the chain for each kind a gate asks of it, and
   makes the arm's class of what it read. The case printer states the chain as its arm's class made of its
   entries. Considered and refused: a class per arm parameterised by its kinds, which Kotlin, having no const
   generics, cannot hold a set of kinds in; and one class of the chain whose kinds the arm's constructor checks,
   which would leave a value of the class that holds kinds the arm denies. A chain that holds a bare entry's
   place (R6a) stays unlowered.

   *The seventeenth Kotlin landing, 2026-11-17:* the one gate on a bare entry (§6 R6a). The arm where it holds
   holds the chain as `{Codec}{Chain}Has{Kind}Placed` (`group::placed_class`), a class of the other entries and
   the named entry's place, as the Rust and C++ runtimes' `Placed` holds them: its constructor is private, and
   `of` makes a value only of others that make, with the named entry, a chain within the bound, of a place at
   most past the last of them, and of others before the place none of the kind, so the named entry is the
   first of its kind. A class and not a value class, as it holds two properties; it compares by both. Its
   `placing` takes the first entry of the kind out of a chain read whole, which is how the decode makes the arm,
   and its `named` makes the entry of the kind: its carrier the identifier, and the arm of its variant the
   identifier's tag selects, whose codec presents nothing, made by default, as the C++ key's `named()` makes it.
   Naming that arm's codec needs its document, so the import's arms whose codec presents nothing carry their
   document's name (`ImportContext::codec_empty_arms`, where they carried their tags), which the roles read as
   before. The encode reads the others off the arm and the place beside them, `null` from the other arm, and
   walks the chain as slots (`ListWrite.slots`): the others in order and the named entry where the value holds
   its place, each but the last handed the flag the chain ends on, the entry before handed on with the named
   entry of its kind. A chain of slots whose entries are held to an input stays refused by name, as the C++
   printer refuses it: a check would see the others alone. The case printer states the arm's chain by
   `placed_parts` as the class made of the others and the place. The same landing binds the walk's
   `ListWrite` in the encode, which the fourteenth landing read and did not bind.

   *The eighteenth Kotlin landing, 2026-11-17:* a list held to the law its entries say their inputs by (§6
   R8), and what the law's start derives. The value holds such a list as `{Codec}{Element}Lawful`
   (`lawful_class`), the Kotlin form of the Rust and C++ runtimes' `Lawful`: the list in the class of its
   bound, and for each input the list hands every entry the start, a `Boolean` property `start{Input}`
   (`start_property`). Its constructor is private and `of` runs the law over the entries: each entry says an
   input the list hands every entry as the start says it, and one a chain hands from the entry before set
   exactly where the entry before is of the kind, the first clear, each read by the entry codec's
   `input{Name}`; an entry that says nothing of an input follows any state (R8b). A value made by default holds
   the fewest entries of the bound from the start the first of them says, as the C++ `Uniform::start_of`
   reads it. The class answers `entries` and `size` as a list's class does, so the walk and a count read it
   unchanged. A flag the parent binds to the input is derived from the start (`Measured::Start`) where the
   list is there and keeps the value's bit where it is not; an input of the codec's own the list binds is said
   by the start (`InputSource::List`). The decode holds what it read to the law from the start the frame gave,
   the bound carrier's bit or the input it is handed. The case printer states the list by `lawful_parts` as
   the class made of the list and each start, and compares the starts beside the entries. Considered and
   refused: a runtime class generic over the law, handed the law as a function, which would spell each law
   as a closure every value carries, where a class of the law spells it once; and the start held in the
   list's own class, which would make every list a class of two properties. A list on one side of a gate
   held to a law, whose start the arm would hold, and a chain of slots held to a law stay refused by name.

   A chain whose `entry-id` names a field of an enum's type is refused where the import is resolved, before
   any language is rendered: SCE_FORGE.md admits as an identifier a multi-bit flag or an integer field of the
   entry codec, and an enum-typed field is neither (`resolve_chain_entry_identifier`). *Amended 2026-11-18:*
   this landing first wrote a Kotlin read of such a field by `toUnderlying()`, which no document reaches; the
   read is gone, and `an_identifier_of_an_enums_type_is_refused_on_every_backend` holds the refusal.

   *The nineteenth Kotlin landing, 2026-11-17:* a CBOR map, printed from its own plan (`kotlin_source::cbor`), as
   the C++ and Python printers print it. Every entry is a property of `{Codec}Source`, nullable for one the map may
   lack: a text or a byte string in the value class of its bound (`bounded_class`, an exact length a least equal to
   its most, so the encode refuses no `CborWrongLength`), a list of records in the value class of its most
   (`bounded_list_of`, the one declaration a list of a codec's entries also takes, its least 0 as Rust's
   `BoundedList` holds it), an unsigned integer as the Kotlin unsigned type of its width. The encode reads each
   entry once and refuses, before the first byte, with `CborOutOfRange`: the refusals the totality names, and a
   record's text past its bound or not UTF-8, which the payload type of a record's Kotlin unit holds in a bare
   `String`, a refusal no Rust or C++ value reaches. It then writes the map head, counting the optional entries
   present, and each entry present in ascending key order (`by_key`), a list of records as an array of one map per
   record, each written by the companion's `encodeRecord{Alias}`, which refuses nothing a sink does not. The
   template's encode of a map and its record writes are gone: every CBOR plan is printed. The value keeps
   `encode` and `encodeToByteArray` as they were, `encodeSource`'s answer. The engine's mesh envelope is such a
   map: the Kotlin router makes each entry by its class's `of`, one past its bound being the `RouterError.Encode`
   the encode answered before, and `stamp` answers no envelope for a vector past its 64 entries. Considered and
   refused: an encode that answers no error where the map refuses nothing, which would give a map two signatures
   by its entries and move every consumer when an entry is added.

   *The twentieth Kotlin landing, 2026-11-17:* a content-line component, printed from its own plan
   (`kotlin_source::content_line`), as the C++ and Python printers print it. Every entry is a property of
   `{Codec}Source`: a property's value and each parameter declared for it, nullable for one that may be absent, a
   `List` for a repeated property and for the values of one line, and a `List` of line records for a repeated
   property that declares a parameter or whose line holds a list of values, each record a source of its own
   `{Codec}{Entry}LineSource`, which the record's data class implements. A line refuses a text by its content as well
   as its size, so no bound refines a text or a list, as in C++ and Python; an integer is the Kotlin type of its
   width. The encode writes each property's lines in declaration order, the judgements of the plan before a line (a
   required list empty, a list past its `sce:max-count`, a parameter given without its property; one every value
   holds is always given), into a staged sink first where a line can refuse the value, so a refusal leaves the
   caller's sink untouched, as a positional encode stages a frame. An optional parameter is read once into a local
   Kotlin narrows. The value's `encode` is `encodeSource`'s answer and keeps its signature; the value and its records
   hold what they held. The template's encode is gone: every content-line plan is printed.

   A bounded collection whose element the Kotlin backend prints reads a text or a byte string `<sce:index-by>`
   field out of the value class of its bound (`element_kotlin_printed` beside `element_cpp_printed` in the
   collection's resolution), as the C++ collection reads the field through its accessor; a byte string is compared
   by its content, printed or not, since a `ByteArray`'s `==` compares identity.

   *The Kotlin printer under the decode of docs/adr/0099, 2026-11-18:* the landings above were written while a
   Kotlin decode answered `null`; they landed after it answers `Decoded`, the value or the kind that refused the
   frame (docs/adr/0099, amendment of 2026-11-16). What the decode makes by a class's `of` is refused, where the
   factory makes no value of what it read, with the kind a guard of the decode refuses that frame with: a byte
   string or a text outside its bound `LengthOutOfRange`, a list past it `TooManyElements`
   (`CodecFailure::over_bound`), a CBOR entry the map lacks `CborRequiredKeyMissing`, and one its class does not
   hold `CborWrongLength` for a byte string of a stated length and `CborOutOfRange` otherwise, as the Rust map is
   refused. A list whose entries break their law from the start the frame gave, and a chain that holds a kind the
   gates of its arm say it lacks or lacks one they say it holds, is `PresentIfMismatch`, a gate given where its
   predicate does not hold, the kind the Rust decode answers for a law broken. Two lists read under one count and
   a catch-all's tag read where no arm names it are no refusal: the decode cannot reach a factory that refuses
   them, and an `error` names the invariant, as the template's unreachable arm does. The content-line encode
   answers its refusal itself, the runtime's `ContentLine.fail` being gone. The Kotlin case printer states the
   `sources` of a CBOR map and of a content-line component (`kotlin_source::cbor::source_case`,
   `kotlin_source::content_line::source_case`), reading the notation where every printer reads it
   (`source_notation`), so every harness of a printed map folds its byte cases.

   *The first Go landing, 2026-11-16:* the positional codecs whose elements are integers, flag sets and enums, each
   always there, written whole, in bytes they share with other fields, or as a VLE of 16, 32 or 64 bits, which derive
   no element and whose encode refuses nothing (`go_source::unlowered` names the first form any other codec uses;
   such a codec keeps the template's encode whole, §6). Each is printed from its plan
   (`sce-build/src/forge/generator/go_source.rs`):
   - the source, an interface `{Codec}Source` with one method for each element the value presents, named as the
     value's field was (`MsgId() uint8`); `*{Codec}` is one, and so is any type with those methods;
   - the value, a struct whose fields are unexported (`go_source::member`, the field's Go name with its first letter
     lowered, which a Go keyword cannot be, since the document refuses one), each presented by its method on a
     pointer receiver, as every other method of a Go codec is. Its flag accessors and setters, its quantity
     accessors and its decode read and write the unexported field (`field.member` in the template);
   - the constructor, `Make{Codec}`, which takes every element in declaration order and returns the value, as the C++
     constructor does. The decision was between this and a keyed `{Codec}Parts` struct: Go has no named arguments,
     and a keyed literal leaves out an element without a word, so a codec that gains an element would leave its
     callers building values without it, where a positional call stops building. A refinement a later landing
     gives an element is a type with an unexported field and its own constructor (§2), so `Make{Codec}` stays a
     constructor that cannot fail;
   - the encode over any source, `EncodeSource(src, w) error`, a function of the codec's package, which is one
     codec's package and so its namespace, as Python's module is: each write of the program, a fixed field's bytes
     in one `WriteBytes` by the byte expressions the template's encode wrote (`encode_whole_field_read_as`,
     `encode_byte_read_by`), a VLE by the runtime's writer. `Encode` is `EncodeSource(s, w)`, and `EncodeToBytes`
     returns the bytes alone, as the plan is total.

   Every reader the generator writes moves with the value (`ImportContext::go_printed`, read as the import is
   generated): a chain reads a printed entry's identifier by its method, as for C++ (`chain_identifier_masks`); a
   procedure that assigns an element makes the value again by `Make{Codec}`, each other element read by its method
   (`rebuilt_element_assign`, the C++ rebuild made one function over the target), and a validator reads an element
   by its method; a bounded collection and an algorithm over one read a printed element's field by its method. The
   set of printed elements is Go's own, beside the C++, Swift and Kotlin ones
   (`ForgeCompileOptions::element_type_go_printed`, `BoundedCollectionResolution::element_go_printed`), and an
   algorithm reads the set of the backend it is generated for. The hand-written Go tests of the queue and the
   broadcast log make their elements by `Make{Codec}` and read them by method. The Go harness makes a printed
   codec's value by `Make{Codec}` from each `cases` row and reads each element back by its method, without the
   elements the encode derives, as the C++ and Python harnesses did before their case printers; the fixture's
   `printed` now says so for Go too. `a_go_codec_is_printed_from_its_plan_once_it_lowers_every_form` holds a printed
   file, the readers above, and that every codec of the resources is printed exactly where the printer lowers every
   form of it. A list is presented, when its landing comes, as §2 says (`iter.Seq`), which raises the Go modules'
   `go` directive from 1.22 to 1.23 in that landing.

   *The second Go landing, 2026-11-16:* byte strings and texts, each always there and written to the end of the
   frame or by a length, and the length that counts one of them, held in a field of its own that it fits, its
   `sce:length-arith` never taking it below zero. The refinement of R1 is the runtime's `codec.Bytes[B]` and
   `codec.Text[B]` (`backends/go/forge-runtime/codec/bounded.go`): a struct with an unexported field, made only by
   `codec.MakeBytes` or `codec.MakeText`, which refuse a length outside the bound (`ErrLengthOutOfRange`) and a text
   that is not well-formed UTF-8 (`ErrInvalidUTF8`). The bound is a type parameter, `B Bound`, whose two methods,
   `Least() int` and `Most() int`, a zero-size type the codec's package names for the element answers
   (`{Codec}{Element}Bound`; `codec.Unbounded`, the largest int, where the protocol states no most, as the C++
   runtime's `kUnbounded` is). The decision was between this and one runtime type holding its bound in fields, which
   a source could then present at any bound, or a generated type per element, which would repeat the check in every
   package: a Go generic carries the bound in the type, as a C++ template parameter does, while the check is written
   once. The zero value is a value of its type, the fewest units the bound admits, zeros or NULs, as the Python
   classes default. A `Bytes` holds the slice it is made of and does not copy it: the bound is on the length, which
   no write to the bytes can change. The value holds no derived length (§3): the struct, `Make{Codec}` and the
   source have no member for it; the encode derives it from the bounded value's `Len()`; the decode reads it to cut
   the bytes and makes the bounded value of what it read before the value's literal, returning the runtime's error,
   which the decode's own check has already refused. Every reader the generator writes reads such an element as the
   slice or the string it holds (`go_source::element_read`): an algorithm's element field, a validator's or a
   procedure's read. The harness makes each bounded value by `codec.MakeBytes` or `codec.MakeText` of the bound the
   codec names and reads it back by `.Bytes()` and `.String()`. A procedure that assigns an element of such a codec
   makes the value again by `Make{Codec}` over every field the codec declares, as the first landing has it; no
   document assigns to a codec with a byte string, a text or a derived length, and the rebuild that would hand a
   bounded value and leave the length out lands with the procedures that need it.

   *The rebuild of a printed Go codec, 2026-11-16:* what a reader in another package knows of a printed codec is
   one value, `go_source::GoPrinted`, which `ImportContext::go_printed` now holds in place of a flag: the elements
   the value presents, in the order `Make{Codec}` takes them, and the lengths the encode derives. A procedure's
   rebuild hands `Make{Codec}` the presented elements alone; an assigned byte string or text is first made into its
   bounded value by `codec.MakeBytes` or `codec.MakeText` (`codec.MakeOptionalBytes` where a gate decides it),
   whose refusal raises `error.execution` as a value past a bytes slot's cap does; an assign to a derived length is
   refused when the procedure is generated (`InvalidLvalue`), since the value has no place for it. A validator or a
   procedure reads a derived length of one byte string always there as the encode derives it, from the bytes it
   counts. The runtime's `codec` package is imported where a file's text names it, read off the text by the render
   door as the `optional` package is (`uses_forge_codec`). A read of a length of gated content, or of several
   measures, outside the codec's package is not printed: no document reads one.

   *The third Go landing, 2026-11-16:* an element a gate decides, on a flag of the codec's own carriers or on a
   flag input the encode is handed, and a length the value states while its content is absent, each presented as
   the runtime's `optional.Of` of what it holds; the flag the presence of one element says, and a length held in a
   flag of a carrier, folded into a local of the carrier in place of the bits the value presents; a length several
   byte strings and texts share, or that may not fit its field; and the refusals these bring (§4): a gate the
   encode cannot derive holds the element it gates (`codec.ErrPresentIfMismatch`), measures that disagree
   (`codec.ErrInconsistentSource`, added to the runtime), and a number past its field or flag
   (`codec.ErrLengthOutOfRange`), each returned before the first byte, the runtime's sentinel unwrapped, so
   `EncodeFailure::symbol` names it for Go. The printer writes exactly the refusals the totality names
   (`Totality::assert_all_written`), as Python's does. The decode reads a gated byte string or text as the
   optional slice or string it did and makes the bounded value by `codec.MakeOptionalBytes` or
   `codec.MakeOptionalText`, so the bound is checked in one function of the runtime. A flag input the value does not
   say is a parameter of `EncodeSource`, named as the decode's. A group, a body, a list, a variant, an input the
   value says and a gate on a chain's entries stay unlowered. The Go harness runs a printed codec's `cases` with
   `EncodeToBytes` answering an error where the codec states `sce:encode-may-fail`; its `sources` and `refused`
   wait for the Go case printer.

   *Amendment of 2026-11-16: a derived number is read by a method of the value.* The rebuild of a printed Go codec
   read a derived length of one byte string always there inline, as the encode derives it. A length of gated content,
   a length several byte strings share, and an encoded length cannot be one Go expression, and spelling each in every
   reader would repeat the encode's derivation in each. The codec's package now gives each number its encode derives
   into a field a method of the value, `Wire{Field}()` (`go_source::wire_method`), which reads the elements the value
   presents as the encode reads them off a source: the first measure always there; where every measure may be
   absent, the first one there, and where none is, what the value states, zero, or, for a length gated beside its
   content, the optional type of the number, absent, as the template's value held it. A value whose measures
   disagree has no frame, and the method answers its first measure. The method is not of the field's own name: a
   length the value states while its content is absent is presented by that name, as what the value states, and
   the source does not present a derived number. `GoPrinted::read` reads every derived number by it, and an
   algorithm over a collection of printed elements reads each field through the element's `GoPrinted`
   (`ForgeCompileOptions::element_type_go_printed`), so a gated byte string or text of an element is read as the
   optional slice or string, as a procedure reads one. A number that counts a body handed a flag input of the
   codec's own has no method, as the value does not hold the input; no document reads one.

   *The fourth Go landing, 2026-11-16:* a body of a codec the Go backend prints, handed each flag input it takes and
   does not say from a carrier or an input of the codec's own, and its encoded length. The decision was how a
   source presents a body: as the body codec's value, or as its source interface. A value of the codec presents
   its elements by methods on a pointer, so a body held as a value is a source of its codec only through its
   address, and a constructor that takes the value cannot be handed what a source's method answers, which the
   rebuild of a procedure hands it. The body is therefore presented, held and made as its codec's source interface
   (`{Body}Source`), as the Rust, C++ and Python printers present any source of the body; the decode holds the
   value it read by its address, and the zero value presents the body's zero value, so the zero value of a parent
   still encodes. A copy of a value shares the body its interface points to, as any Go interface does. The body is
   written by its codec's `EncodeSource`, the carrier a flag of which it is handed read as the encode writes it.
   The runtime gains `codec.CountingSink`, a sink that counts and keeps nothing (ADR 0058 §3): a length that counts
   a body's frame is the count of the body's encode over it, and a body whose encode can refuse is encoded over it
   before the first byte, which returns its refusal while the caller's sink is as it was. A body counted for a
   length is handed the carriers as the source presents them, before the encode folds a flag it derives into them,
   so a body whose length the encode derives and which is handed a flag the encode derives stays unlowered, with a
   body handed an input its parent binds nothing to.

   *The fifth Go landing, 2026-11-16:* a list, and a chain that ends with its scope, of entries of a codec the Go
   backend prints, held to no law and walked with no flag set on or handed to an entry, and the count of its
   entries. The refinement of R2 is the runtime's `codec.List[B, E]` (`backends/go/forge-runtime/codec/list.go`): a
   struct with an unexported field, made only by `codec.MakeList`, which refuses more entries than the bound's most
   (`ErrTooManyElements`) and fewer than its least (`ErrLengthOutOfRange`), its entries read in order through
   `All() iter.Seq[E]`, as §2 gives a Go list. `B` is the bound type the codec names for the element,
   `{Codec}{Element}Bound`, as for a byte string, and `E` the entries' codec's source interface, as a body is
   presented, so a list is made of any sources of its entries. The decode reads the entries as values and makes the
   list of their addresses (`codec.SourcesOf`), refused as the runtime refuses it. The encode derives the count from
   the list's `Len()` and writes each entry the sequence yields by its codec's `EncodeSource`; a list whose entries'
   encode can refuse has each entry encoded over `codec.CountingSink` before the first byte. The zero value of a
   `codec.List` holds no entry, since no entry of an interface type can be made for it; where the bound's least is
   above zero, a value of the list is made by `codec.MakeList`. Range over a function is Go 1.23, so the forge
   runtime's module, the modules the tests build generated Go in, and the Go setup of the forge-conformance and
   mutation workflows move from 1.22 to 1.23. A chain that sets its entries' flag, hands an entry what the one before
   it was, holds a bare entry's place or is held to a law (R6, R7, R8) stays unlowered.

   *Amendment of 2026-11-17: a counted body is handed what the frame carries.* The fourth landing counted a body
   for its length before the encode folded a flag it derives into a carrier, so a body handed such a flag stayed
   unlowered, and a length counting a body handed a flag input of the codec's own had no `Wire{Field}()`. The encode
   now counts every body a length measures once each carrier is folded, as the C++ printer does, so the body is
   handed each flag as the frame carries it; a length of encoded bodies goes into the field `sce:length-from` names,
   so no fold waits on it. The method that answers such a length hands the body the same flags: a carrier with a
   flag the encode derives is answered by `Wire{Carrier}()`, the bits the value presents with each presence flag and
   each number held in a flag folded in as the encode folds them, and an input of the codec's own is a parameter of
   the method (`WireSize(marker byte)`), as it is of the encode, since the value does not hold it. The decision was
   between that parameter and leaving the number without a method: a reader that holds the value and not the input
   cannot know the number, which is a function of the input, and a method over the input states exactly that. Such
   a number is therefore no member a validator, a procedure or an algorithm is offered (`GoPrinted::answers`), so an
   expression that reads it is refused where it is generated; no document reads one.

   *The sixth Go landing, 2026-11-17:* a chain that ends on its entries' flag, and one that hands each entry what
   the entry before it was (R7, `<sce:entry-flag-bind previous-entry-id>`). A codec whose value presents a one-bit
   flag of a fixed carrier declares `{Codec}Overrides`, a struct of an `optional.Of[bool]` for each such flag, and
   `EncodeSourceOverriding(src, w, inputs.., overrides)`, which folds each flag the struct sets into its carrier
   last, so what the parent says is what the wire holds; `EncodeSource` hands it an empty struct. The decision was
   between that struct and an adapter the chain would wrap each entry's source in, presenting the carrier with the
   flag set: the plan names the flags a parent may set as an argument of the encode (`EncodePlan::overridable`), as
   the C++ `Overrides` and the Python keywords spell it, and an adapter would restate in the parent which carrier
   method holds the flag. Go has neither default arguments nor keywords, so the struct is a parameter of a second
   function, and every other caller is unchanged. The walk counts down the entries left from the list's `Len()` and
   sets the flag where any are, and keeps, for each input a bind waits on, whether the entry it wrote last is of the
   kind (`entry-id`, read off the entry by its method, less the flags `entry-id-except` leaves out), clear for the
   first; the walk is one function, so a list whose entries' encode can refuse is counted before the first byte
   handed exactly what it is written with. A chain that holds a bare entry's place (R6b) waits for the groups, and
   a list held to a law (R8) for its refinement.

   *The seventh Go landing, 2026-11-17:* a variant whose arms are codecs the Go backend prints, in each of its
   shapes: a tag of its own in a field or a flag of a carrier (`Derive::VariantTag`), a peek byte, which the arm's
   own header writes, a tag the caller hands, which the caller writes, and an arm a build feature decides, which
   every Go build keeps as the template does; and a parent whose carrier's bits the arm of an embedded body's
   variant fills (`Derive::ParentTag`). The decision was how Go holds a sum. The template's struct of one pointer
   per arm holds no arm or several as readily as one, so it is no refinement; a Go sum is an interface with an
   unexported method that only the package's own types implement, read by a type switch, which is how Go's own
   generated code holds one (the protocol buffers' `oneof`). The value presents the arm as `Body()`, a
   `{Codec}Variant`; each arm is a struct `{Codec}Variant{Arm}` that holds a source of its codec behind `Body()`,
   made by its constructor, as a body is presented as its codec's source (the fourth landing); the catch-all holds
   its tag in `{Codec}UnknownTag`, the refinement of R3 as §2 gives it Go: a struct with unexported fields made by
   a constructor that answers `false` for a tag an arm names or the bits do not hold, whose zero value holds the
   least tag no arm names. The decode, which reaches the catch-all only under such a tag, makes it in the package.
   A value made by default, as the zero value, presents the arm marked `default="true"`, which the plan now says
   (`Arm::by_default`: the marked arm, else the first named, else the catch-all), its codec's value made by its
   constructor where the codec declares one, so a peek-byte arm's header holds its tag. The encode reads the arm
   once, derives the tag by a type switch into the tag field or folds it into its carrier, counts an arm whose
   encode can refuse before the first byte, and writes the arm by its codec's `EncodeSource`, handed what its
   import binds; a parent folds each embedded body's arm's tag, masked, into its carrier, and a parent the template
   still writes switches on a printed body's arm type the same way. A tag the encode derives into a field is
   answered by `Wire{Field}()`, and a variant whose caller hands its tag answers `Tag()` from the arm type. An
   input the arms say (R4d, R4e), and a check of an input at an arm, stay unlowered.

   *The eighth Go landing, 2026-11-17:* a group of fields one flag of the codec's own decides, on one side or on
   both, or a disjunction of such flags decides (docs/adr/0065 §6 R4b, R4c), its members of any kind the printer
   prints, a length stated on the side without its content among them (`Absent::Group`), and two lists of one
   count (R5). A Go value holds the group whole, named as every printer names it (`rust_source::group_type`,
   `group_accessor`, `group_arm_name`): the struct of its one arm, or, where it has several, an interface only
   its arm types satisfy, as the variant is held (the seventh landing); the optional type of either where the
   gates allow no arm. Each arm holds its members in unexported fields behind methods and is made by its
   constructor, so a member is no field of the value's own and no reader names it alone. Two lists of one count
   are one member of the arm that holds both, the runtime's `codec.Aligned[F, S]`
   (`backends/go/forge-runtime/codec/aligned.go`), made only by `codec.MakeAligned`, which answers `false` for
   two lists of different counts, as the Kotlin runtime's factory refuses them and the Rust one answers `None`.
   The encode reads the group once and binds each member to an optional local from the arm it holds, so each
   statement that writes a member is the one that writes an element a gate decides; it derives a one-flag group's
   flag from the presence of its first member (`PresenceFlag::together`), each flag of a disjunction from the
   arm (`Derive::ArmFlag`, the value the gates have while no arm is held), and a length on the side without its
   content from that side's member, zero where the value holds neither. The decode makes the arm the gates it
   read select. `GoPrinted` now holds what `Make{Codec}` takes as parts, an element or a whole (a group, the
   variant), as `cpp_source::CppPrinted` does, and answers no member of a group. A group an input decides, which
   is one of an input the value says, and a group a chain's entries decide (R6) stay unlowered.

   *The Go printer under docs/adr/0099's amendment of 2026-11-16, 2026-11-18:* a printed decode answers the
   kind that refused the frame and never a value: the template's reads answer theirs, and each value the printer
   makes of what they read (a byte string or a text, a list, two lists of one count) is made by the runtime's
   constructor, whose error the decode hands on with no value beside it (`return nil, err`). `codec.MakeAligned`
   answers `codec.ErrInconsistentSource` for two lists of different counts, where it answered `false` and the
   printer named the kind in its place, so every constructor of the runtime names its own refusal as
   `codec.MakeBytes` and `codec.MakeList` do, and as the Python runtime's `Aligned` raises `InconsistentSource`.
   A decode reads both lists by one count and is never refused there.

   *One shape of what a reader knows of a printed codec, 2026-11-18:* `cpp_source::CppPrinted` and
   `go_source::GoPrinted` held the same facts, what the constructor takes in its order, how each element is made
   of what an expression gives, and each number the encode derives with whether a reader holding the value is
   answered it, so they are one `printed_codec::PrintedCodec`, and an import holds it as `ImportContext::printed`
   on a C++ and a Go generation alike. Each printer builds it, spelling its accessors and the function or method
   that answers a number (`cpp_source::printed_codec`, `go_source::printed_codec`); which field a reader may name,
   which assignment is a rebuild and which is refused, and the order of the constructor's arguments are decided
   once (`PrintedCodec::reading`, `PrintedCodec::rebuilt_by`), and each backend spells the read and the call
   (`cpp_read`, `go_read`, `cpp_rebuilt`, `go_rebuilt`). A Go assignment to a body or a list is now refused as the
   C++ one is, since no expression gives one, and an assignment to a member of a group is refused for that reason
   rather than as a derived number. The arm of a variant is named by one function, `variant_arm_name`, which the
   template's `variant_name`, the Rust and Go printers and the Kotlin and Go parents read.
   *The C11 interface, decided 2026-11-16 under the rule of 2026-10-26 (the textbook structure, decided without
   asking).* A C11 value presents its elements to its encode through a table of functions, `{codec}_source_t`, one
   member `T (*element)(const void *ctx)` for each element the value presents, in declaration order, each handed
   the context pointer the table is passed with. This is C's own interface: a table of operations beside the object
   it operates on (Schreiner, *Object-Oriented Programming with ANSI-C*, 1993, ch. 2), the shape of every driver
   table of a C kernel. The encode is a function over any source, `{codec}_encode_source(src, ctx, w)`, which reads
   each element once and derives every other element of the frame from them. The table and the context are passed
   apart, and not as one struct of pointers: a table is `static const`, one for each type, which a build places in
   read-only memory, so presenting a value costs it no RAM (the budgets of docs/adr/0049 and 0057), and any record
   of a consumer's own layout is a source by a table of its own, with no copy into the codec's struct. The codec's
   struct is one source, by the table `{codec}_value_source()` answers, whose context is a `const {codec}_t *`, and
   `{codec}_encode` is the encode over that table; the struct, its members and its decode stay as C holds a value
   with no allocator, and lose only what the encode derives (§3). The encode answers the runtime's status: a total
   codec's only `SCE_FORGE_CODEC_BUFFER_OVERFLOW`, the sink's own failure. A refinement (§2) is a struct made by one
   function that returns a status, and lands with the first form that has one. Considered and refused: one struct
   holding both the context and the functions, which puts a table's pointers in the RAM of every value presented;
   the codec's struct as the interface, which is the builder that stores derived elements and presents no record
   of another layout without a copy; and one reader keyed by an element's index (`get(ctx, id, out)`), which loses
   each element's type to a `void *`.

   *The first C11 landing, 2026-11-16:* the positional codecs whose elements are integers, flag sets and enums,
   each always there, of which the encode derives nothing, that take no flag input and whose encode refuses
   nothing, written by the packed bytes and whole fixed fields of a positional layout and the whole fixed fields
   and variable-length integers of a streamed one (`c11_source::unlowered` names the first form any other codec
   uses; such a codec keeps the template's encode whole, §6). Each is printed from its plan
   (`sce-build/src/forge/generator/c11_source.rs`): the table, the value's readers `{codec}_value_<element>` and
   its table, and the encode over a source, whose writes are the template's byte expressions over the locals each
   element was read into (`encode_whole_field_read_as`, `encode_byte_read_by`, `vle_encode_block`). No consumer's
   code changes, as these codecs derive nothing. The C11 harness runs each printed codec's `sources`
   (`c11_source::source_case`): the case zeroes a static value of the struct and sets each member by a statement,
   since a C value is made by setting its members and a struct that holds a large array is no literal on a stack;
   it encodes the value through `{codec}_value_source()`, compares the bytes, decodes them, and compares each
   member. A refused case is held to the status `EncodeFailure::symbol` names for C11 (the runtime's
   `SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE` and `SCE_FORGE_CODEC_PRESENT_IF_MISMATCH`; it has no status for an
   inconsistent source yet) and to a writer it left at its start. The census
   `a_c11_codec_is_printed_from_its_plan_once_it_lowers_every_form` holds a printed header, that every codec of the
   resources is printed exactly where the printer lowers every form of it, and a consumer's record of its own layout
   encoded through a table of its own to the value's bytes.

   *The second C11 landing, 2026-11-16:* byte strings and texts of a positional layout, always there, and the lengths
   derived from one of them into a field of its own that the length fits, its `sce:length-arith` never taking it
   below zero. The refinement of R1 is the first C11 refinement, and its shape is the C one §2 names: a view of a
   type of its own for each element, `{codec}_{element}_bytes_t` or `{codec}_{element}_text_t`, `{ data, len }`,
   made by one function, `{codec}_{element}_bytes_make(&view, data, len)`, which answers
   `SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE` for a length outside the bound and, for a text, `SCE_FORGE_CODEC_INVALID_UTF8`
   for bytes no UTF-8 holds. The source presents the element as that view, so the encode over a source writes it
   whole and refuses nothing of it, and a consumer views bytes it holds anywhere with no copy. The suffix keeps the
   view's name apart from the struct of another codec named after the element (`codec_zenoh_push` and
   `codec_zenoh_push_body`). A derived length leaves the value (§3): the struct declares no member of it, the
   encode computes it from the view's `len`, less or more its arithmetic, and the decode reads it into a local
   (`_d_<field>`, `c11_source::derived_local`) that it cuts the bytes by. Where the length's field is named as the
   array's own length member (`payload_len` beside `payload`), the array keeps a `size_t` length member of that
   name, as it does beside any other length. The value's struct holds a byte string in an array beside its length,
   as C holds a value with no allocator, and C cannot keep that length within the view's bound by a type, so the
   value's `encode` holds the array to the bound (the element's, or the array's own where a capacity makes it
   smaller), a text to UTF-8, before the first byte, and then encodes over the value's table: a value past them is no
   value of the codec, as docs/adr/0086 §5.4 holds a value a host built in C. A streamed layout's byte strings and the
   lengths it carries, which its decode reads through a statement for each field, wait for their own landing. The C11
   harness sets a byte string's array and length by statements, and its `cases` and the test-vector sidecar
   neither set nor read back an element the encode derives.

   *The third C11 landing, 2026-11-16:* the byte strings and texts of a streamed layout, written whole from the view
   the source presents, and the lengths the frame carries in a fixed field or a variable-length integer of their
   own. A streamed decode reads each field by a statement of its own, which wrote each into the struct and read a
   length off it; it now reads a number the encode derives into a local, `_d_<field>`, declared before the block
   that reads it so that the field it measures reads it there. Which numbers those are is one value the decode's
   helpers are handed, `c11_source::Derived`, read off the plan once: the length a field is cut by
   (`carried_length`) is that local where the value holds no member of it, in the streamed decode as in the bound
   check of the positional one, so no helper asks the plan again. The view's maker asks the runtime's
   `sce_forge_length_within_bound` for its most, as the value's encode does, and compares no bound where it is
   written: a bound of 2^32 - 1 bytes is a comparison a 32-bit `size_t` decides, a warning under `-Wtype-limits` on
   the MCU targets of docs/adr/0057.

   *The fourth C11 landing, 2026-11-16:* elements a gate of a carrier's flags decides, the presence flag that says
   whether one is there, and the length of gated content gated beside it. A source presents a gated number as the
   optional type `sce/forge/optional.h` holds it in, the type the value's struct already holds it in, and a gated
   byte string or text as the optional value of its view, `{codec}_{element}_bytes_optional_t`, which the header
   declares by the runtime's `SCE_OPTIONAL_DECLARE_NAMED` beside the view: the one shape of a C value that may be
   absent, `{ present, value }`, and no pointer that a source could leave dangling. The encode folds each derived
   flag into the carrier the source presents, its bit-range replaced (`_c_<carrier>`, as the C++ printer folds it):
   a presence flag from whether the element it gates is there, and a length a flag holds from the view it measures.
   A length of gated content is zero where the gate leaves the content out (`Absent::Zero`), and is written where
   its gate holds over the carriers the encode writes (`Guard::Gate`); a gated element is written where the source
   holds it (`Guard::Present`). The decode reads a gated length into its local, zero where the gate is clear. The
   value's encode holds a gated array to its bound only where it is present. A flag that several fields gated
   apart say, a length a group or the value states where its content is absent (`Absent::Group`,
   `Presence::Stated`), and a gate of a flag input or of a chain's membership, each an encode that can refuse or
   reads what this printer does not yet hold, stay unlowered and name their form.

   *The fifth C11 landing, 2026-11-16:* bodies of codecs the C11 backend prints (`ImportContext::c11_printed`, read
   as the import generates on C11), handed no flag input, always there or gated, and the encoded length of one. A
   printed header declares `{codec}_source_ref_t`, `{ table, ctx }`, the pair a source is passed as, and its
   optional value; a parent's source presents a body as that pair, and the value's reader points it at the body the
   struct holds, by the body's own value table. The parent's encode writes the body by the body's own
   `{codec}_encode_source` over the pair: the body is read where it is held, with no copy and no RAM but the two
   pointers. The checks a value's encode makes before the first byte, an array within its view's bound and a text
   UTF-8, are a function of their own, `{codec}_check_value`, which a parent's check calls for each body it holds:
   the encode over a source reads a body's arrays through its views, so a body is held to its bounds before its
   parent writes a byte, as the body's own encode holds it. The encoded length of a body is counted by the body's
   encode over a counting sink, `sce_forge_writer_init_counting()` (`sce/forge/codec.h`): a writer with no buffer
   that stores nothing and counts every byte, so the length costs no stack buffer of the body's most, which the MCU
   budgets of docs/adr/0057 could not hold for a body of 64 KiB. The counting sink adds one test of the buffer to the
   runtime's two storing writes and changes nothing a writer over a buffer does. A body whose encode can refuse, a
   length of one in a flag, and a body of a codec the C11 backend does not print stay unlowered and name their form.

   *The sixth C11 landing, 2026-11-16:* the encodes that can refuse a value for what they derive or hold. A number
   of several measures is printed as the Rust encode's `combine` makes it: the first measure always there fixes it,
   and every other one there is held to it; where every measure may be absent it is the first one there, kept in a
   local `_m_<element>` beside the test `_m_<element>_there`. A number the value states while its content is absent
   (`Presence::Stated`, `Absent::Stated`) is presented and held as the optional number `sce/forge/optional.h`
   declares, the type a gated number already has, so no new type is made for it; the encode takes it only where no
   content is there, and holds a stated number beside content to what the content says. The decode reads the number
   into its local, as it reads every derived number, and sets the member once every field is read
   (`C11Source::decoded`), present exactly where none of the content was there. A number past what its field or
   its bits hold, a length its arithmetic takes below zero, two measures that disagree, a stated number beside
   content that says another, and two elements a presence flag gates apart that disagree are each refused by the
   status `EncodeFailure::symbol` names for C11 (`SCE_FORGE_CODEC_INCONSISTENT_SOURCE`, which the C runtime gains,
   and `SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE`), before the first byte: the derivations come before the writes. A
   bound is asked of `sce_forge_length_within_bound`, as the view's maker asks it, never compared where it is
   written. A body whose encode can refuse returns its refusal through the count and the write of it; where the
   value holds one, `{codec}_encode_source` writes the frame over the counting sink first, by `{codec}_write_source`
   (the writes alone), and then over `w`, as the C++ `Wire` counts before it writes, so the writer is left at its
   start by every refusal. Every refusal is written by `Totality::require`, and `render` asserts every one the
   analysis names was written. A number derived into a flag where a gate leaves its content out keeps the bits the
   value presents (`Fold::present`). A group's stated length (`Absent::Group`), gates the encode holds a field to
   rather than derives (`gate_checks`) and inputs a held codec says (`input_checks`) stay unlowered and name their
   form; a gated body of a printed codec is lowered, and the codecs of the resources that hold one wait only on the
   lists they hold.

   *The seventh C11 landing, 2026-11-16:* lists, and chains that end with their scope, of entries of codecs the C11
   backend prints, handed no flag input and held to no law, always there or gated, and the count of one. The C
   spelling of §2's begin and next pair is the runtime's `sce_forge_walk_t` (`sce/forge/codec.h`), a table in
   read-only memory of `begin(ctx)` and `next(ctx, entry)`, each answering an entry as the context the entry codec's
   source table is handed, or `NULL` past the last. The decision was the textbook iterator of C, an opaque position
   its two functions move, over an index into the list (`at(ctx, i)`), which a linked list answers in time linear in
   `i`, and over a callback the list calls for each entry, which turns the encode inside out and cannot stop it at a
   refusal without a status threaded back through the consumer's loop. The entry is its own position, so a walk
   needs no cursor type and an array, a linked list or any record of a consumer's own is walked where it is held.
   The walk is one runtime type and not one per codec, as it names no codec's type. The refinement of R2 and R7 is
   a list of a type of its own for each element, `{codec}_{element}_list_t`, `{ walk, ctx, entry, count }`, made by
   `{codec}_{element}_list_make`, which walks the list once, counts it, and answers
   `SCE_FORGE_CODEC_TOO_MANY_ELEMENTS` as it meets an entry past the most, so a walk that never ends is refused
   where the list has a most, and `SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE` below the fewest (one, for a chain closed by
   an entry flag, R7). The count is derived from the count the maker found (`Measured::Entries`), as a byte
   string's length is from its view's `len`, and the encode writes that many entries, walking from the first, each
   by its codec's `{entry}_encode_source` over the entry's table; a walk is the same each time it is run, which the
   runtime's declaration states. The value's struct keeps its array beside `{list}_len`; its reader presents it by
   a walk of the array, `{codec}_value_{list}_begin` and `_next`, and `{codec}_check_value` holds the length to the
   list's most and each entry to its codec's check. The decode reads a count the encode derives into its local
   (`c11_source::Derived`, now handed to the repeat decodes too). A chain whose walk sets its entries' flag, hands
   an entry what the one before it was, holds a bare entry's place or is held to a law, a list whose entries are
   handed flag inputs, and a list in a group stay unlowered and name their form.

   *The eighth C11 landing, 2026-11-16:* the one-bit flags a parent may set in place of the value's
   (`EncodePlan::overridable`), and the chain that sets the flag its entries end on (`ListWrite::ends`,
   `Derive::ChainContinuation`). C has no default argument, so the textbook spelling of a parameter most callers
   leave unset is a second function: a codec with such flags declares `{codec}_overrides_t`, one
   `sce_optional_bool_t` for each flag, and `{codec}_overrides_none()`, and is encoded by
   `{codec}_encode_source_overriding(src, ctx, w, overrides)`, which folds each flag the overrides hold into its
   carrier after every derived flag, as the C++ printer folds its `Overrides`; `{codec}_encode_source` is that encode
   handed none, so every caller but a chain calls a codec as it calls any other. The overrides are passed by value,
   a few bytes, and not by a pointer a caller could leave null. A chain closed by an entry flag sets it on each entry
   but the last, from the entry's place among the count its list's maker found, and its maker refuses a chain of no
   entry (R7) with `SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE`, which the runtime's declaration now names for a list below
   its fewest. Lowering the overrides also prints the codecs of one-bit flags the value presents whole
   (`codec_flags_basic`), which the fourth landing had left on the template by naming every overridable flag a form.

   *The ninth C11 landing, 2026-11-17:* flag inputs. An input the value does not say (`EncodePlan::given`) is a
   parameter of the encode, `uint8_t <name>` after the writer and before the overrides, named as the decode names it,
   and the value's `encode` and `encode_to_buf` take the same ones (the template's `encode_params_after_first_arg`,
   which on the template's encode is every input). An input the value says (`EncodePlan::said`), off the one field
   gated on it or off a body always there that says it, is read off the source into a local of its name where a gate,
   a check or a held codec reads it, and `{codec}_input_<name>(src, ctx)` answers it for a parent, a
   `sce_optional_uint8_t` where the value says it in some of its values only (§6 R4e). A body and a list's entries are
   handed what their import binds, a carrier's bits as the encode writes them or an input passed on, so a body's
   encoded length is counted once the carriers are folded. A flag a held body says is derived through the body's
   function of it (`Measured::Says`). A gate is tested on the carriers and the inputs alike; the gates the encode
   holds a field to rather than derives (`gate_checks`) and the inputs a held codec says beside what it is handed
   (`input_checks`) are refused with `SCE_FORGE_CODEC_PRESENT_IF_MISMATCH` before the first byte. A parent on the
   template hands a printed codec only the inputs it does not say, and a broadcast log over a printed element reads
   the flags the element says off it, as the C++ log does (`ForgeCompileOptions::element_type_c11_printed`). An input
   an arm, a group or a list's start says, a gate of a chain's membership, and an input a chain's entry says of the
   entry before stay unlowered with the forms that hold them.
5. Swift, when its codec lowers what the plan holds.
   *Swift lowers what the plan holds, 2026-11-16:* its codec writes every codec of the repository Rust writes,
   but the one only an MCU-class backend writes (`a_swift_codec_is_written_wherever_rust_writes_one.rs`): an
   enum field through its carrier, an arm under a build feature by a condition that leaves it out (docs/adr/0072,
   amendment of 2026-11-16), a word in the native byte order, and whole bytes fewer than a field's type in a frame
   read field by field, each of which it had refused by name. The codec matrix answers that Swift ships a codec
   whole (`codegen_matrix::template_ships`), as it answers for Go, Kotlin, Python and C11, which ship on the encode
   of their templates while step 4 reaches them. What is left by name is no codec of the repository's shape: a
   float or a bool field (which the C++ printer refuses too), a signed field narrower than its type, bits that are
   not whole bytes in a frame read field by field, a count or a length held in a signed field, two arms of one
   entry codec, a chain closed by a flag wider than one bit, and a member or local of the generated struct used as
   a field's or an input's name. Swift's decode throws the runtime's typed
   `CodecError`, and its harness holds each reject vector to the case it names (`CodecFailure::symbol`),
   as Rust's and Go's are held.
   *Amendment of 2026-11-19: a signed field narrower than its type is lowered.* The list above no longer holds
   "a signed field narrower than its type": every backend reads such a field as the two's complement of its bits,
   sign-extended into its type, and writes its low bits (docs/adr/0095, amendment of 2026-11-19, "a field narrower
   than its type is extended into it"). Swift sign-extends the word it reads (`Carrier::extended`) and no longer
   refuses the field by name. The rest of the list stands.
   What step 5 still owes is the printer: Swift's encode is its template's, which writes a stored count, length,
   tag and continuation flag from the value rather than deriving each (§3), and its harness runs the catalog's
   `cases` and not its `sources` or `refused`, which a printer's value states (§5). The printer lands as §6 says,
   one form at a time, with a protocol for the interface (the presented elements as read-only properties), an
   abstract type with one failable initializer for each refinement Swift's types cannot carry, a case printer for
   its harness, and the template's encode removed with its last codec.
   *The Swift printer, first landings, 2026-11-16:* `swift_codec::source` prints a codec whose plan holds numbers
   (an integer, a flag set presented whole, an enum through its carrier) laid out by byte or written in turn,
   each always there or decided by a gate; byte strings and texts; a length the encode derives into a field of its
   own or into the bits of a flag; a presence flag; a flag input the value does not say; and a gate the encode
   cannot derive, held as a check before the first byte. Every other form keeps the template's encode whole and
   `source::unlowered` names it (§6). Three decisions are recorded here. First, every Swift decode, printed or
   not, reads each field into a local named by the field and makes the value of the locals once the frame is read:
   a printed value holds no element the encode derives (§3), so the decode cannot build the value field by field,
   and one decode shape for every codec is the textbook one, as the C++ printer's is. Second, the refinement R1 is
   a type of the runtime, `BoundedBytes<B>` and `BoundedText<B>` over a bound type `B: SceByteBound` the codec
   declares for each field (`{Codec}.{Field}Bound`), whose failable initializer is the one door and whose
   `init(decoding:)` throws `lengthOutOfRange`; a text is counted in its UTF-8 bytes. The bound is part of the
   field's type, so a value made for one field is no value of another, as Rust's `Bounded` and C++'s class
   template make it. Third, a printed encode writes every byte into a frame of its own and hands the sink the frame
   in one write, so a refusal anywhere leaves the sink untouched and the encoded length of a held body (the next
   form) is measured by encoding the body into its own bytes once before the length is written, which is the
   counting of ADR 0058 §3 without a second pass. The harness folds `sources` and `refused`
   of every printed codec, the refusal named by the runtime's case (`EncodeFailure::symbol`), and its `cases`
   fragment makes a printed value without a derived element, each byte string through its bound's door.
   *Inputs the value says, 2026-11-16:* a flag input the value says by the one field gated on it alone (docs/adr/0065
   §6 R4a, `InputSource::Gate`) is printed. The encode is not handed it: where a gate reads it, the encode reads it
   off the value into a local of the input's name before any gate, and `{Codec}.input{Name}(_:)` reads it off any
   source, which is what a parent derives a flag it binds to the input from and what a log stores. The decode is
   still handed it. Whether an import is printed is a fact of the import (`ImportContext::swift_printed`, read as
   the codec is generated, its own imports resolved, as `cpp_printed` is), and a template parent hands a printed
   child's encode no input the child says, as every C++ parent does; a Swift broadcast log over a printed element
   reads each such flag off the element (`ForgeCompileOptions::element_type_swift_printed`). An input a held codec
   or a group says, and the checks of an input a held codec says (`input_checks`), enter with the form that holds
   the codec. The case printer hands the decode every input and holds the value to saying each said one through
   its reader. The generic parameter of `encodeSource` is named `Source`, since a flag input is often one capital
   letter and a local of its name would hide a parameter `S`.
   *Embedded bodies, 2026-11-16:* a body, always there or gated, of a codec the Swift printer prints is printed;
   one of a codec it does not print keeps the holder on its template, so a printed codec holds printed bodies alone.
   The protocol presents the body as an associated type bound to a source of the body's codec (`{Field}_Source`,
   whose `_` no codec's type name has, so it hides none), which the generated value binds to the body's struct: a
   host's type is written with no copy, as through Rust's associated source type. The encode writes each body into
   bytes of its own by the body codec's `encodeSource`, before the frame, handed each input it does not say from
   the carrier as the encode writes it (its derived and overridden flags folded in) or from an input of the
   holder's own; the encoded length a field holds is the count of those bytes, so the body is encoded once and
   counted without a second pass (docs/adr/0058 §3), and a refusal of the body is thrown before a byte reaches the
   sink. A flag bound to an input the body says is read off it through the body codec's `input{Name}(_:)`, an input
   the holder hands on to a body that says it in every value (`InputSource::Forward`) is said by the holder, and
   an input the body says that the holder hands it from elsewhere is held to it before the first byte
   (`InputCheck` at an embed). A decode presents each body as its codec's value, and a value of a printed codec is
   `Equatable` down into its bodies, so the case printer makes a body by its codec's initializer and compares it
   whole. An encoded length into the bits of a flag, a body's input bound to nothing, and a body that says an input
   in some of its values only stay refused by name.
   *Lists and chains, 2026-11-16:* a list or a chain, always there or gated, of entries of a codec the Swift printer
   prints is printed. The refinement R2 is the runtime's `BoundedList<Element, Bound: SceCountBound>`, a collection
   of the entries whose failable initializer is the one door and whose `init(decoding:)` throws `tooManyElements`;
   the count bound is a type of its own and no byte bound, so neither stands for the other, and its `least` is one
   for a chain closed by its entries' flag (R7). The protocol presents each entry as an associated type bound to a
   source of the entry codec (`{Field}_Entry`), as a body is, so a host's entries are written with no copy. A count a
   field or a flag holds is derived from the entries (`Measured::Entries`), as a length is from bytes. The walk
   writes the entries in turn into bytes of the list's own by the entry codec's `encodeSource`, appended once every
   entry is written, so a refusal of an entry is thrown before a byte reaches the sink; it hands each entry what its
   import binds and, for an input a chain supplies (`ListWrite::previous`), a local the walk sets from the identifier
   of the entry just written (`EncodePlan::entry_ids`), clear for the first. A chain that ends on its entries' flag
   (`ListWrite::ends`) sets that flag on every entry but the last through the entry codec's `Overrides`, which is
   the one place a flag the entry presents is overridden by its holder, so the derivation `ChainContinuation` has
   no other spelling. An input every entry says that the holder hands them from elsewhere is held to it entry by
   entry before the first byte (`InputCheck` at a list, against the entry before where a chain supplies it). A
   decode makes the list through `init(decoding:)` after its read has refused an entry past the bound as before. A
   list held to its law (R8), a chain of slots or one a group holds (R6), and two lists of one count (R5) wait for
   the forms that hold them.
   *Variants, 2026-11-16:* a variant every arm of which is a codec the Swift printer prints is printed, its tag a
   field, a flag, a peeked byte or the caller's. Two decisions are recorded here. First, the variant is presented as
   a generic enum over the arms (`{Codec}Arms<A0, .., D>`, each parameter bound to a source of its arm's codec),
   which the protocol binds to an associated type for each arm (`{Arm}_Arm`, `Default_Arm`) and the generated value
   to the arm codecs' own values (`typealias {Codec}Variant`), and not as a concrete enum of the arm codecs' values:
   a concrete enum would make a host copy each arm into the generated value, which the presented bodies and entries
   do not ask, and Rust's `{Codec}VariantSource` is generic over its arms for the same reason. A Swift generic
   parameter may go unused, so an arm under a build feature keeps its parameter and its associated type in every
   build and only its case is under the condition (docs/adr/0072), which no Rust-like split per build needs. The
   enum's `tag` is the one table of the arms' tags: the encode derives the tag field or flag from it, a parent
   derives the carrier bits a body's arm fills (`ParentTag`) from the body's, and a case reads a caller's tag from
   it. Second, the catch-all holds its tag in `{Codec}UnknownTag` (§6 R3), for every Swift variant, printed or
   not: a failable initializer that refuses a tag an arm names or one past the tag's bits is the one door, the
   decode, which reaches the catch-all only with an unnamed tag, makes it through a file-private one, and a
   template parent reads the bits it derives off its `value`. An input every arm says (R4d) is read off the arm the
   value holds through the arm codec's reader; one some arms say (R4e) answers `nil` for an arm that does not, and
   is printed only where no gate or held codec of the codec reads it; a flag bound to an input the arm says
   (`ArmSays`) is derived from it, and one the parent hands an arm that says it is held to it before the first
   byte (`InputCheck` at an arm). The case printer states the arm as the arm's value, the catch-all's tag
   through its door, hands a caller's tag to the decode and holds both values to it, and compares the variant
   whole. The hand tests the catalog's cases now state are gone (`CodecVariantUnitTests`' encode of a stored
   tag field); what stays holds what no case can state.
   *Gate groups of numbers, byte strings and texts, 2026-11-17:* a group decided by flags of the codec's own
   carriers or by flag inputs (docs/adr/0065 §6 R4b, R4c) whose members are numbers, byte strings and texts is
   printed. Two decisions are recorded here. First, the group is one property of the value, of a concrete type
   of its own and not one generic over sources: an enum with a case for each arm, each holding the arm's
   members by name, with the arm's place among the arms (`arm`), or a struct of the one arm, optional where the
   gates may name no arm. A member is a number or a bounded byte string or text, which a host makes as cheaply
   as the group, so nothing a host holds is copied that a source would spare; a group that holds a body or a
   list, whose source a host holds, waits for the form that presents it through an associated type. Second, the
   encode reads each member off the arm the value holds into a local (`{member}_member`), which every write, length
   and presence flag reads as it reads an element off the source, so one encode shape serves an element and a
   member; a flag of a disjunction (`ArmFlag`) and an input the group decides (`InputSource::Group`) are what
   the arm's place gives them, and a decode makes the arm its gates name from the locals it read the members
   into. The case printer states a group as the arm `group_choice` reads, and compares it whole. A group of a
   chain's memberships (R6), one whose arm holds two lists of one count (R5), and one that holds a body or a
   list stay refused by name.
   *Gate groups that hold a body or a list, 2026-11-17:* a group whose members include a body or a list of a
   codec the Swift printer prints, or a length or a count the value states on the side where what it measures
   is absent (docs/adr/0065 §6 R4b, `Absent::Group`), is printed. The group's type is now generic over the
   source of each body and list it holds, one parameter a member (`{Field}_Source`, `{Field}_Entry`), named and
   bound as the associated type of the codec's protocol that presents it, which the generated value binds to
   the codec's own value: the protocol requires the group as `{Codec}{Gates}<{Field}_Source, ..>`, and the same
   spelling in the value names its typealiases. This amends the first decision of the entry above for these
   groups alone: a group of numbers, byte strings and texts stays concrete, since a host makes those as cheaply
   as the group, while a body or a list is what a host holds and a source spares it copying, as the variant's
   arms enum is generic for the same reason. One parameter a member, not one an arm, so a member several arms
   hold is one type in all of them, as the Rust group holds it. The type is `Equatable` and `Sendable` where its
   sources are. The encode keeps each member as the source presents it, a local of the source's associated
   type (`Source.{Field}_Source`), and writes a body and walks a list off that local where it writes an
   element's off the source; a stated number is the measure where the arm holds what it measures, else what the
   arm states, zero where it states nothing, as every printer reads it. Considered and refused: a concrete
   group whose arm holds the body codec's own value, which would make a host copy each body into it.
   *Lists held to a law, 2026-11-17:* a list whose entries say the inputs it hands them (docs/adr/0065 §6 R8),
   alone or in the arm of a group, is printed. The Swift runtime gains `Law.swift`: `SceLaw`, an automaton over
   the entries of a list; `SceUniform` and `SceAfterKey`, the two automata, over `SceSaid` (what an entry says of
   an input, `nil` for nothing, which follows any state) and `SceKind` (whether an entry is of a kind);
   `SceLaws` and `SceStarts`, two laws run together, nested for more; and `Lawful<List, Law>`, a list with the
   start the value gives, whose failable initializer is the one door, whose `init(decoding:start:)` throws
   `presentIfMismatch`, and which reads as its list, as the Rust and C++ `Lawful` are. Two decisions are
   recorded here. First, what an entry says and what kind it is are the entry codec's, which the runtime cannot
   name, so the printed codec declares, for each such list, a `SceSaid` for each input through the entry codec's
   `input{Name}(_:)` and a `SceKind` through the chain's identifier of an entry, each generic over the source of
   the entries, and the law as a generic typealias over them (`{Field}Law<Entry>`); the automata stay the
   runtime's, written once. Second, a value that has read no frame holds the fewest entries from the start the
   first of them says (`init(defaultOf:)`), which traps where they break the law: the door stays checked, as no
   unchecked one is public. The encode walks the list as any list, its type having held every entry to the law;
   a flag bound to the input is derived from the list's start (`Measured::Start`), and an input of the codec's
   own that it hands the list is said by the start (`InputSource::List`), `nil` where a gate leaves the list
   out. The decode holds the entries it read to the law from what the frame gave. The fixtures the earlier
   entries took for checks at a list are held to a law (`codec_repeat_flag_carrier`, `codec_chain_prev_carrier`),
   and their tests now hold the law; a list a flag of another declaration derives keeps the check, which a test
   holds on a document of its own. An input a list in a group's arm says stays refused by name.
   *Two lists of one count, 2026-11-17:* a gate group whose arm holds two lists of one count (docs/adr/0065 §6 R5)
   is printed. The Swift runtime gains `Aligned<First: Collection, Second: Collection>`, whose failable
   initializer makes a value only of two lists of one count and whose `init(decoding:_:)` throws
   `inconsistentSource`, as the Rust, C++ and Kotlin `Aligned` are. Kotlin's runtime needed `Counted` for what a
   list answers its count by; in Swift that is `Collection.count`, which `BoundedList` and `Lawful` have, so the
   runtime declares no protocol of its own for it. The arm that holds both holds them as one member named as
   every printer names it (`aligned_name`), the other arm the first list alone; the encode reads each list out of
   the pair into its member's local and derives the count from the first, which every arm holds; the decode
   pairs the two lists it read through the decode's door. A pair one list of which is held to its law stays
   refused by name, as the C++ printer refuses it.
   *Chains of kinds, 2026-11-17:* a group the gates on a chain's entries decide, and the flags joined with them
   (docs/adr/0065 §6 R6b), the one gate on a bare entry (R6a), and a gate on a chain's kinds no group holds are
   printed. The Swift runtime gains `Membership.swift`: `SceKinds`, a set of kinds over the existing `SceKind`,
   nested as the laws are (`SceNoKinds`, `SceKindsOf<First, Rest>`); `SceChain<Element, Bound, Has, Lacks>`,
   a `BoundedList` that holds an entry of each kind of `Has` and of none of `Lacks`, whose failable initializer
   is the one door, whose `init(decoding:)` throws `presentIfMismatch` and which reads as its entries; and
   `ScePlaced<Element, Bound, Kind>`, the other entries and the place of the first entry of the kind, made only
   where the chain they make is within the bound and no entry before the place is of the kind, whose
   `init(decoding:)` takes that entry out of the chain a decode read. Three decisions are recorded here. First,
   the kinds are types and not values: Swift's generics carry a set of kinds as Rust's tuples of keys and C++'s
   template parameters do, so arms that say the same of the chain hold one type and a value of it cannot hold
   a kind its arm denies, which Kotlin, having no such generics, holds in a class per arm. The codec declares
   each kind a gate names (`{Chain}Kind{K}<Entry>`), told apart by the chain's `entry-id` as the walk tells
   it; the sets and the chains stay the runtime's. Second, the encode reads every arm's chain as the one list
   of entry sources each type holds (`entries`, or `rest` and `place`), so the walk, a check and a scan read
   one local whatever the arm; a membership a group's gate is, is what the arm says (`gate_value`), and any
   other a scan of the entries. A chain of a bare entry's place is walked as slots: the others in order and
   the named entry where the value holds its place, made by the entry codec's initializer of its identifier
   and, where it has a variant, the arm the identifier's tag selects, whose codec presents nothing, made of a
   frame of zeros (`.init()`), the flag the chain ends on set on every slot but the last. Third, a decode
   selects the arm by the locals the template binds after the chain for each kind a gate asks of it, and
   makes the arm's chain through the door of its type; where the arm may hold none, it makes it before the
   arm is made, since no throwing closure is mapped over an optional in a decode that throws one error type.
   The template's chain locals read a printed value's bounded list as an array, gated by `?.` and not by
   `?? []`, which no `BoundedList` is. A chain of a bare entry's place whose entries a check holds to an input
   stays refused by name, as the C++ and Kotlin printers refuse it: the check would see the others alone. The
   Zenoh error envelope, which its tests took for a template codec, was printed since lists and chains landed;
   its tests now hold the printed encode.
   *Three shapes left by name, 2026-11-17:* an input a list in the arm of a gate group says, two lists of one
   count one of which is held to its law, and a number the value states where what it measures is absent
   (`Presence::Stated`) are printed. The input is read off the arm the value holds: the encode reads it off
   the member's local the group already binds, and the codec's reader switches over the arms, the start of the
   list where the arm holds it and `nil` where it holds another (docs/adr/0065 §6 R4e, R8). The pair needs no
   new type: `Aligned` is over two collections, and `Lawful` is one that reads as its list, so the arm holds
   `Aligned<Lawful<..>, BoundedList<..>>`, the count derived from the first and a flag bound to its input from
   its start. The C++ printer still refuses this pair by name; Swift's `Aligned` asks of each list only
   `Collection.count`, which `Lawful` answers, so nothing in Swift is left to refuse it for. A stated
   number is an optional element, as every printer presents it: the encode derives it where what it measures
   is there, writes what the value states, zero where it states nothing, where none of it is, and refuses as
   `inconsistentSource` a value that states another number beside what it measures; a decode presents it only
   where none of what it measures was in the frame, as the C++ decode does.

## Consequences

- An encoding rule is written once, in the plan, and reaches every backend by its printer. A parity defect is then a
  printing defect in one backend, and the byte cases of §5 locate it.
- Every backend's total codecs become total in their signatures, which removes a failure path from consumers of C++,
  Go, Kotlin, Python and C11 as each lands.
- The builders of the other backends lose their stored derived fields (§3). This is a consumer-visible change, once
  per backend.
- Until step 4 ends, a backend has two encodes, split by codec and never within one, and its `unlowered` lists what
  is left.
- The oracle grows by the shapes it lacks. That is work owed today: ADR 0058 law 4 is stated for shapes no case
  holds.

## Considered alternatives

- **`rust_source.rs` ported to each backend.** Refused: five more readings of the grammar, each in a language's
  spelling. This is the shape ADR 0058 §2 removed for one backend's three representations.
- **A richer template context, with the decisions in the templates.** Refused: a decision in a template is
  re-made for each language, and a template cannot be tested apart from its spelling.
- **One encoder in C, bound into each language.** Refused: it gives up each backend's native types, so a refinement
  would be checked at the boundary on every call, and the no-alloc tier would carry a foreign-call boundary it does
  not have today.
- **The other backends left as they are, with Rust alone total.** Refused: `sce:encode-may-fail` is a property of the
  document (ADR 0065 §1), and the other backends would contradict it in their signatures for as long as they stay.

## Questions left open

None. Decided 2026-11-03; the first step is 2 above.
