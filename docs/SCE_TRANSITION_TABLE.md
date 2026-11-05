# The transitions of a machine: the export and the generated table

A machine's transitions are one fact with two surfaces, both rendered from one list
(`sce-build/src/transition_records.rs`, `transition_records`):

| Surface | What it is | When to use it |
|---|---|---|
| `sce-codegen transition-table` | A design-time export: NDJSON, one row per node of the design that can carry a requirement. Registered wire surface, `schemas/sce-transition-table.v1.schema.json` (`SCE_WIRE_CONTRACTS.md`). | Drawing or reviewing a design; anything that reads the SCXML and not a running machine. |
| `<scxml sce:transition-table="emit">` | A constant table in the generated source of every backend, with the `source_hash` of the run. | A host that holds the compiled machine and wants to check or draw it without reading the SCXML. |

The table is requested by the design and not by the command line (SCE Accepted Subset §2.20).
The declaration is inside the source set the machine's `source-hash` covers, so every way of
generating the document produces the same machine, and no flag has to be remembered at a
build that someone else runs.

Nothing in a machine reads either one. The generated table is constant data that no
dispatch touches, and a document that does not declare it generates files byte-identical to
what they were before the attribute existed. A C++, C or Rust program that never names the
table carries none of it, under the conditions [What it costs a program](#what-it-costs-a-program)
states and measures.

## Every route that writes the machine binds the table

The table carries the digest of the run that wrote it, so it is written by each route that writes
drift-headed files, bound to the digest those files' headers carry (`sce_build::bind_run_digest`):
`sce-codegen generate`, `orchestrate` for each document of the set, and `generate-w3c` for each
fixture. An in-line child (`<invoke><content><scxml sce:transition-table="emit">`) declares it as a
root does: its machine is a file of the run beside its parent's, at any depth, on all three routes,
and its table is in its machine with the hash its machine's header carries. The three prepare the
child through one library operation (`sce_build::carried_children`), so a child's table is the same
whichever route wrote it. The C++ conformance suite takes a fixture's in-line children from the
fixture's own `generate`. `check` renders the table where `generate` and `orchestrate` would, a
child's included, and writes nothing.

The library entry points compute no digest, so they cannot write the table: a document that
declares it, or whose in-line child does, is refused there with `generate/source-hash-unavailable`
rather than handed back as a machine without it, and `compile_scxml_with_imports` writes it when the
caller gives it `ForgeCompileOptions::source_hash` (SCE Accepted Subset §2.20).

## What an entry holds

| Field | Meaning |
|---|---|
| `from` | The owning state. |
| `to` | The first target, when it names one of the machine's states. |
| `event` | The event, when the transition names exactly one literal event the machine holds. |
| `targets` | The whole target set as written, space-separated, a `<history>` id included. Empty for a targetless transition. |
| `events` | Every descriptor as written, space-separated: `*`, a prefix such as `error.*` and a literal name alike, none resolved. Empty for an eventless transition. |
| `index` | The transition's position within its state, in document order: the `N` of `states.<id>.transitions[N]`. |
| `annotation` | The `sce:req` tokens as written, space-separated. Empty when there are none. |

`to` and `event` are the typed ids a consumer compares with the machine it compiled, and they are
absent where a transition has no single id in the name tables:

- no target (a targetless transition), or a first target that is a `<history>`;
- no event (an eventless transition), a wildcard `*`, a prefix such as `error.*`, or several descriptors.

For those, and for the second and later targets, `targets` and `events` are the complete answer,
and they are the same fact the export states: its `transition` rows carry `targets` (the target
set) and `event` (every descriptor as written, or `(eventless)`), under the same `index`. A
consumer that splits `targets` and `events` on spaces has what the export has, without reading
the SCXML.

The annotation is an opaque token. The generator never reads its shape, so a step number
`12` arrives as the text `12`, and the integer is the consumer's own parse of it.

## Accessors, by backend

| Backend | Table | Hash | Entry |
|---|---|---|---|
| C++ | `Policy::transitions` (`std::array`; `transitions.size()` is the count) | `Policy::sourceHash()` | `Policy::TransitionRecord{from, to, event, targets, events, index, annotation}`; `to` and `event` are `std::optional`. |
| C11 | `<prefix><name>_transitions()`, a pointer to `<PREFIX><NAME>_TRANSITION_COUNT` entries, NULL for a machine with no transition | `<PREFIX><NAME>_SOURCE_HASH` | `<prefix><name>_transition_t{from, to, has_to, event, has_event, targets, events, index, annotation}`; `has_to` and `has_event` say whether `to` and `event` hold a value. |
| Rust | `<Name>Transition::ALL` | `<Name>Transition::SOURCE_HASH` | `<Name>Transition`; `to` and `event` are `Option`. |
| Go | `<Name>Transitions` | `<Name>SourceHash` | `<Name>Transition`; `HasTo` and `HasEvent` say whether `To` and `Event` hold a value; `Targets` and `Events` are strings. |
| Kotlin | `<Name>Transitions.all` | `<Name>Transitions.SOURCE_HASH` | `<Name>Transition`; `to` and `event` are nullable. |
| Python | `TRANSITIONS` | `SOURCE_HASH` | `<Name>Transition` (frozen dataclass); `from_state`, and `to` and `event` are `None` when absent. |
| Swift | `<Name>Transition.all` | `<Name>Transition.sourceHash` | `<Name>Transition` (a `Hashable`, `Sendable` struct); `to` and `event` are optional, `nil` when absent. Each id is written with its type, so a state named `none` is never read as `nil`. |

`targets` and `events` are a string in every backend (`const char*` in C and C++), so the table is
the same shape in seven languages and needs no array of arrays.

The states and events in an entry are that backend's own enumerators, so a consumer compares
them with the machine it compiled and not with a string.

### What each language forces

The table is the same everywhere: one entry type with the seven fields above, one constant
sequence of entries in the order the export lists them, the hash as a constant beside it, and
the states and events as the machine's own enumerators. Every name is spelled the way its
backend spells the rest of its machine, as the table above shows. Beyond that the surface
differs only where a language cannot say what another says. These are those places, and there
are no others:

- **An absent id.** C++, Rust, Kotlin, Python and Swift have an optional, and `to` and `event`
  are one: `std::nullopt`, `None`, `null`, `None` and `nil` where the transition has no single
  id. Go has none, and says it with a flag beside each: `HasTo` and `HasEvent`, with `To` and
  `Event` left at the zero value of their type when the flag is false. C has none either, and
  says it the same way: `has_to` and `has_event`, with `to` and `event` the zero of their
  enumeration when the flag is false. In both, a reader reads the flag first; the value beside
  a false flag means nothing, and no value of an enumeration (a count, the event a scheduler
  polls with) stands for absence.
- **A field named `from`.** `from` is a keyword in Python, so its field is `from_state`.
- **A constant sequence.** C cannot return an array and has no array of length zero, so an
  array declared for every machine cannot be written, and one declared only for some would make
  a consumer test the count before naming it. `<prefix><name>_transitions()` is declared for
  every machine that declares the table, as the C transition function's `P_all_states()` and
  `P_all_events()` are (`docs/SCE_TRANSITION_FUNCTION.md`), and returns a pointer to constant storage the machine's `.c` defines once,
  `<PREFIX><NAME>_TRANSITION_COUNT` entries long, or NULL for a machine with no transition.
  Go has no constant of a composite type, so `<Name>Transitions` is a package variable, an
  array whose length is a constant.
- **Strings.** `targets`, `events` and `annotation` are `const char *` in C, the one string C
  has, and `const char*` in C++, the type the machine's policy hands out every other name of
  the document in (`getStateName`), so an entry compares with the machine's own names without a
  conversion. Each is a constant of static storage, terminated by NUL.
- **The type of `index`.** C, C++, Rust and Go hold it as an unsigned 32-bit integer. Kotlin
  and Swift hold it as `Int`, the type their collections count and index by, which none of their
  unsigned types converts to without a call; Python has one integer type.
- **Holding the table at compile time.** In C++ and Rust the table is a constant expression,
  so a program can hold it to its design before it runs: `static_assert` over `transitions` in
  C++, a `const` item over `ALL` in Rust. Go, Kotlin, Python, C and Swift have no compile-time
  evaluation of a table of records: Go's and Kotlin's constants are scalars and strings, a C
  `const` object is not a constant expression, and Python and Swift build the table when the
  program runs. There, comparing every field when the program runs is the check.

## Checking a machine against its design

`source_hash` is the §synth-6.2.6 digest of the source set the machine was generated from:
the `source-hash` of the file's own header, held as a value. The export states the same value
when asked:

```
sce-codegen generate -l cpp -o out machine.scxml
sce-codegen transition-table --header machine.scxml > machine.transitions.ndjson
```

(`machine.scxml` declares `sce:transition-table="emit"`.) The first line of the second is
`{"record":"header","v":1,"generator":...,"machine":...,"source_hash":...}`. Compare its
`source_hash` with the table's, then walk the table beside the `kind: "transition"` rows in order:
the same count, and for each pair the same `from`, `index`, `annotation`, `targets` (the row's
`targets` joined by spaces) and `events` (the row's `event`, empty where it is `(eventless)`).

Two flags matter when the document is not generated from its own directory. `transition-table`
takes `--input-root` and `--deploy` and they mean what they mean to `generate`: pass the values
`generate` was given, or the two hashes name different source sets.

## Reading a run against the table

Every runtime has a transition observer (docs/adr/0089): a read-only callback told each
transition the engine takes, after the selection and before the microstep's first exit, each
transition of the optimal enabled set once and in selection order. It is told the pair a row of
this table is named by, `from` and `index`, in the types this table holds them in, so the set of
pairs a run is told is a set of rows, and its share of the table is the run's transition
coverage, measured on the engine that ran.

| Backend | Setter | Told |
|---|---|---|
| C++ | `setTransitionObserver(std::function<void(const TransitionTaken &)>)` on the engine | `TransitionTaken{source, index}` |
| C11 | the host defines `<prefix><name>_observe_transition(const <prefix><name>_t *sm, <prefix><name>_state_t source, uint32_t index)`, called when the build sets `<PREFIX><NAME>_TRANSITION_OBSERVER` or `SCE_TRANSITION_OBSERVER` to 1 | the call's arguments |
| Rust | `Engine::set_transition_observer`, with the runtime's `transition-observer` feature | `TransitionTaken { source, index }` |
| Go | `(*Engine).SetTransitionObserver` | `TransitionTaken[S]{Source, Index}` |
| Kotlin | `StateMachineEngine.transitionObserver` | `TransitionTaken(source, index)` |
| Python | `Engine.set_observer`, kind `"transition"` | `fields["source"]` (the state's name, `str` of the row's `from_state`) and `fields["index"]` |
| Swift | `Engine.setTransitionObserver(_:)` | `TransitionTaken(source:index:)` |

Set it before the run starts to be told the transitions the first macrostep takes. On C11 a
build that does not set the switch carries nothing of it, and on Rust a build without the feature
carries nothing of it; on the other backends an observer that is not set costs one check per
microstep.

## A document written against a feature model

A document that gates its elements with `sce:requires` (ADR 0057, SCE Accepted Subset §2.22)
denotes one machine for each configuration of its feature model. Its export is the featured
transition table of ADR 0057 §7: every node of every variant, in document order, and each row of
a node that is not present in every variant carries `presence`, the node's effective condition
(its own and those of the elements around it) in canonical form, the value the forge AST and the
sourcemap carry under the same name for the same node:

```
{"source":"(none)","from":"idle","event":"secure","guard":"-","after":"-","to":"open","action":"-","node_path":"states.idle.transitions[2]","kind":"transition","index":2,"targets":["open"],"presence":[["session-tls"],["session-unicast-accept"]]}
```

`presence` is any of its terms, each all of its features: the row is present in a configuration
where every feature of one term is on. A term is a least configuration of the condition, written
by its features that no other feature of it requires (`session-unicast-accept` stands for itself
and the `session-unicast` it requires); its features are in byte order, the terms in the order of
their features, and no term contains another, so two rows carry the same `presence` exactly when
their nodes are present in the same variants, whatever the author wrote. A row with no `presence`
is in every variant, which is every row of a document of no feature model.

**The rows of a variant** are the rows with no `presence` and those whose `presence` holds in its
configuration, in the order the export lists them. Each keeps what its node states of itself:
`source`, `from`, `event`, `guard`, `after`, `kind` and `targets`, and `to` and `action` except
on the two kinds below. What a row counts or summarises from other nodes is the variant's own:

| Column | In a variant |
|---|---|
| `index` of a transition | its position among the kept transitions of its state, which a consumer counts |
| `node_path` | names the node by the variant's positions; the export's names it in the document, as the requirements report and the manifest classification of the same document do |
| `to` and `action` of a state | name its initial child, which for a state that writes none is its first child in the variant |
| `action` of a transition | lists the transition's actions that the variant keeps |

`sce-codegen transition-table --features a,b` writes the table of the variant those features
denote, as `generate --features` generates it, and those columns with it; its rows carry no
`presence`. Its header names the same `source_hash`: the hash covers the source set, not the
selection, so every build of the document carries it.

**The table a machine emits** (`sce:transition-table="emit"`) is always one variant's. The Rust,
C11 and C++ backends generate a gated document as one output for every build (ADR 0057 §10), and
each build's table is its own variant's, selected as the rest of the output is: one `ALL` under
each `#[cfg]` in Rust, one `transitions` under each `#if` in C++, and in C one
`<PREFIX><NAME>_TRANSITION_COUNT` and the entries it counts under each `#if`. Every other backend generates one variant with `--features`, and its table is
that variant's. Either way a build is checked against its design as above, beside the rows of its
own variant.

`sce-build/tests/the_featured_transition_table_derives_each_variant.rs` derives every
configuration's rows from the featured table of a document with a node of each shape a condition
reaches and holds them to `--features`, row for row; it also finds each column of the table above
computed differently by some variant, so none of them could have been carried.

## What it costs a program

A C++, C or Rust program that drives the machine and never names the table carries none of it:
not its entries, not its strings, not the hash. What makes that so differs by language, and so
does the condition it holds under:

| Backend | Why the table is left out | Condition | Measured on |
|---|---|---|---|
| C++ | `transitions` is an inline variable and `sourceHash()` an inline function, emitted by a unit that uses them and by no other. | No flag: the language emits an inline variable only where a unit uses it. | The object of a unit compiled with `-std=c++20 -Os -ffunction-sections -fdata-sections -c`. The C++ machine is linked against `sce_base`, which the test does not build, so the object is what is measured; no other part of a program includes the generated header, so an image is free of the table when every object of it is. |
| C | The storage is defined inside `<prefix><name>_transitions()`, each string a named object of its own, and the hash is a macro. A link that drops what no one references drops the function, then the table, then its strings. | The link drops unreferenced sections: `-ffunction-sections -fdata-sections` and `-Wl,--gc-sections` (`-Wl,-dead_strip` on macOS), and the deployment names no section for the machine's functions (`platform.c11_section_attribute`), which would place the function with every other one of the machine. Without them the table is in the image, as every other function of the `.c` that is never called is. | The image of the machine's `.c` and a program, linked with `-std=c11 -Os -ffunction-sections -fdata-sections -Wl,--gc-sections`. |
| Rust | `ALL` and `SOURCE_HASH` are constants, evaluated where a program names them and stored nowhere else. | None: no LTO or link flag is needed. | The image of `cargo build --release` with the default release profile (no LTO). |

The strings of the C table are named objects rather than literals because a compiler places the
literals a file's data points to in one shared, mergeable section, and a linker that keeps or
drops a section whole keeps that one for any other literal in it: a table of literal pointers
would leave its strings in the image of a program that never names it.

`sce-build/tests/a_program_that_never_names_the_transition_table_carries_none_of_it.rs` measures
each row on a document whose one transition carries a distinctive annotation: neither the
annotation nor the source hash is among the bytes of what the build makes. Beside each it builds,
the same way, a program that prints the annotation and the hash from the table, and finds both,
so each measurement is one that can fail. Nothing is claimed for Go, Kotlin, Python and Swift.
Kotlin and Python load what they run rather than link an image of it; Go and Swift link one, and
whether their linkers leave out a package variable or a static that nothing names is not
measured here.

## What holds this together

`sce-build/tests/the_emitted_transition_table_equals_the_export.rs` reads each backend's table back
out of its generated source and compares it with the export of the same document, for all seven
backends, including the identifiers, the whole target set and every descriptor, the hash, and an
annotation carrying characters a string literal has to escape. Where a toolchain is present each backend's table is also held to the export
by the language itself: C++ by a static assertion for every field of every entry, Rust by a
constant item that asserts every field of every entry, Python by importing the module, and Rust,
Go, Kotlin, C and Swift by building a program against the real runtime, running it, and comparing
every field of every entry. A C machine with no transition is built too, and its accessor answers
NULL and its count zero. Those programs skip on a host without the toolchain; the Swift one is also
run by the Swift lane (`scripts/gate swift-backend`), on the release `bx.toml` pins and with a skip
refused. The same file asks each source for the annotation literal
its language needs, which a read-back cannot, since it unescapes what the template wrote (a bare `$`
in Kotlin reads back as a `$` and is read by the compiler as an interpolation). The same file holds
every route to the table, in every backend it writes: `orchestrate` for a document of a set,
`generate-w3c` for a fixture (C11 is refused by the route), an in-line child's table under
`generate` and `orchestrate`, and under `generate-w3c` on every backend whose suite takes the child
from it (all but C++), each read back and held to the export with the hash of its own file's header;
and `check` accepting a child's declaration on a document and on a set.

`sce-build/tests/mutations/the_emitted_transition_table_equals_the_export.cases` makes each backend's
table wrong once in a mutation round, and C's presence flag once more, so the tests above are known
to be able to fail.

`sce-build/tests/a_backend_holds_what_the_other_backends_hold.rs` reads the table as a capability of
every backend: one whose own templates hold no `emitted-transition-table` block lacks it, and a
backend that lacks it is a gap that test names, the same way it names a missing transition function.

`a_computed_event_name_is_matched_like_any_other` declares its table (`sce:transition-table="emit"` on
its root), so each channel's own build compiles the table in a real machine; its Rust, Go, Kotlin and
Swift trees are committed with it.

`sce-build/tests/transition_table_closure.rs` holds the export to the two witnesses `generate`
wrote for the same document: the `source_hash` of the sourcemap, and its `transition` symbols.
