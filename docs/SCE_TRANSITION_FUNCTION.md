# A statechart as a pure transition function

A statechart root may declare `<scxml sce:transition-function="emit">` (SCE Accepted Subset
§2.21). `sce-codegen generate` then writes, beside the machine it always writes, a second file:
the document as a pure function of (current state, event). The file depends on no SCE runtime and
no SCE library. It compiles alone with the language's standard library, and in the same build unit
as the machine, because its names live in a namespace of their own.

It answers one question, the one a caller that holds a state and an event asks: is the event taken,
and what is the state afterwards. Nothing is executed and nothing is scheduled; there is no engine
to start. A program that needs the machine itself uses the machine.

## The boundary

The declaration is accepted only for a document in which every macrostep is exactly one microstep
and one leaf state fixes the configuration. For such a document the next configuration depends on
the current leaf and the event alone, and the generator computes it at build time with the W3C
selection rule: the leaf's transitions, then each enclosing state's, outward, each state's in
document order, the first whose descriptors match the event. No engine is simulated, because none is
needed. §2.21 lists what is admitted and what is refused, each refusal with its code; a document
that declares nothing is judged by none of it and generates files byte-identical to before.

## What the function holds

| Item | Meaning |
|---|---|
| states | Every atomic state and every `<final>` child of `<scxml>`, in document order. A compound state is never the current state, so it is not a value of the state type. |
| events | Every event the transitions name literally, in the order they first name them (states in document order, each state's transitions by position). A descriptor such as `error.*` or `*` names no event; it matches the ones listed. |
| initial | The default entry of the document: the state `<scxml initial>` names, entered by its `initial` attribute or `<initial>` element or first child, down to a leaf. |
| transition(state, event) | The state the event leaves the machine in: the state itself when the transition taken has no target, the default entry of the target otherwise, for an internal and an external transition alike. Absent when no transition takes the event, which the machine then discards. A final state takes no event. |
| is_final(state) | Whether the state is a `<final>` child of `<scxml>`, where the machine ends. |
| configuration(state) | The ids of every state active while this one is the current state, outermost first and the state last. For a flat machine it is the state alone. |
| names | Each state and event as the document writes its id, and back. |
| source hash | The §synth-6.2.6 digest of the source set the file was generated from: the `source-hash` of its own header, as a value, as the transition table carries it (`docs/SCE_TRANSITION_TABLE.md`). |

The whole table, every state against every event, is computed once, by
`TransitionFunction::of` (`sce-build/src/transition_function.rs`), and every backend renders that
value and nothing else. The generated code is a total match over it, so it cannot disagree with the
table, and two backends cannot disagree with each other.

## Accessors, by backend

| Backend | File | Namespace | State and event | Every state, every event | Initial | Hash | Transition | Final | Configuration | Names |
|---|---|---|---|---|---|---|---|---|---|---|
| Rust | `<stem>_transition_function.rs` | the module the file is declared as (`mod <stem>_transition_function;`) | `enum State`, `enum Event` | `ALL_STATES`, `ALL_EVENTS` | `INITIAL` (also `State::default()`) | `SOURCE_HASH` | `const fn transition(State, Event) -> Option<State>` | `const fn is_final(State) -> bool` | `const fn configuration(State) -> &'static [&'static str]` | `State::as_str`, `State::from_name(&str) -> Option<State>`, the same on `Event` |
| C++ | generated next | a nested namespace | | | | | | | | |
| C11 | generated next | a symbol prefix distinct from the machine's | | | | | | | | |
| Go | generated next | a package directory of its own | | | | | | | | |
| Kotlin | generated next | a package of its own | | | | | | | | |
| Python | generated next | a module of its own | | | | | | | | |

The five backends without a file yet refuse a document that declares the function with
`generate/unsupported-feature`, naming the language, rather than generating the machine without the
file the document asked for. `check` reports that refusal per backend, beside Rust's `ok`.

### Rust

The module uses `core` alone and names no crate: it compiles in a `#![no_std]` crate with no
allocator, and its functions are `const fn` where the language allows it. `State` and `Event` carry
the derives the machine's own `<Machine>State` and `<Machine>Event` carry (`Debug`, `Clone`, `Copy`,
`PartialEq`, `Eq`, `Hash`, and `Default` on `State`, marked on the initial state), but none of the
extra derives a caller may add to the machine's enums: an extra derive names a crate, and this file
names none. Variants are spelled by the filters the machine uses (`to_state_variant`,
`to_event_variant`), so a state is the same identifier in both, and the names the machine refuses
(`scxml/generated-name-collision`, `validation/reserved-code-identifier`) are refused before either
file is written.

`from_name` matches a name whole. The events of the function are the names its document writes; a
longer name a descriptor would match on a running machine is not one of them.

The function is generated by `sce-codegen generate`, where the digest of the source set is known.
`check` judges the declaration and writes nothing; the library entry points that compute no digest
refuse what the declaration does not admit and write the machine alone.

## What holds this together

`sce-build/src/transition_function.rs` tests the producer on every selection rule the declaration
relies on; `sce-build/src/scxml_transition_function.rs` tests one refusal each and all of them in one
run.

`sce-build/tests/the_emitted_transition_function_equals_the_producer.rs` generates two fixtures
(`sce-build/tests/fixtures/transition_function/`, a flat cycle and a nested lifecycle that reaches
every selection rule), and:

- reads the Rust arms back from the source and holds them to the producer's table, on any host;
- compiles the Rust file alone, in a `#![no_std]` library under `-D warnings`, which is what proves it
  names no SCE crate and needs no allocator, then compiles a program around it that prints every
  answer, and compares the print with the producer's table;
- builds the machine generated beside it against the real Rust runtime, drives it to every state of
  the function by the shortest event path the function gives, sends it every event there, and holds
  the configuration the engine reaches, and whether it took the event or discarded it, to the
  function;
- holds the declaration's one value, the forge root that does not take it, the refusal of the other
  five backends, and `check` and `generate` reporting every refusal of an impure document at once.

`sce-build/tests/mutations/the_emitted_transition_function_equals_the_producer.cases` makes the
producer and the Rust template wrong once each in a mutation round, so the tests above are known to
be able to fail.
