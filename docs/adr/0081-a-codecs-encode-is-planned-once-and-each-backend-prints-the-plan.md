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
3. The oracle's byte cases for every shape (§5), held first by Rust.
4. C++, then Go, Kotlin, Python and C11, each by §6, with its runtime's counting sink, refinements and walk. C++ comes
   first because it is the engine's own language and its type system carries a refinement as a type, as Rust's
   does. That tests the plan against a second interface idiom where the refinements stay static. C11 comes last
   because its interface, a table of functions over a context pointer, is the furthest from the others, and it
   then meets a plan four printers have already read.
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
