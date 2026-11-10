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
5. Swift, when its codec lowers what the plan holds.

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
