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
| C++ | `<stem>_transition_function.h` | `SCE::Generated::<name>::transition_function`, nested in the machine's namespace | `enum class State`, `enum class Event` | `ALL_STATES`, `ALL_EVENTS` (`std::array`) | `INITIAL` | `SOURCE_HASH` (`std::string_view`) | `constexpr std::optional<State> transition(State, Event)` | `constexpr bool isFinal(State)` | `constexpr Configuration configuration(State)`, a range of `std::string_view` | `getStateName(State)`, `getStateFromName(std::string_view) -> std::optional<State>`, `getEventName`, `getEventFromName` |
| C11 | `<stem>_transition_function.h` | every symbol prefixed `<name>_transition_function_` (`P_` below), `P_` upper-cased for constants | `P_state_t` (`P_STATE_<ID>`), `P_event_t` (`P_EVENT_<NAME>`) | `P_all_states()`, `P_all_events()`, `P_NUM_STATES` and `P_NUM_EVENTS` of them | `P_INITIAL` | `P_SOURCE_HASH` | `bool P_transition(P_state_t, P_event_t, P_state_t *next)` | `bool P_is_final(P_state_t)` | `P_configuration_t P_configuration(P_state_t)` (`ids`, `count`) | `P_state_name(P_state_t)`, `bool P_state_from_name(const char *, P_state_t *)`, `P_event_name`, `P_event_from_name` |
| Go | `<package>/<stem>_transition_function.go` | package `<package>` = `<name>_transition_function`, a directory inside the machine's | `type State int` (`State<Id>`), `type Event int` (`Event<Name>`) | `AllStates`, `AllEvents` | `Initial` | `SourceHash` | `func Transition(State, Event) (State, bool)` | `func IsFinal(State) bool` | `func Configuration(State) []string` | `State.String()`, `ParseState(string) (State, bool)`, `Event.String()`, `ParseEvent(string) (Event, bool)` |
| Kotlin | `<stem>TransitionFunction.kt` | package `<prefix>.<name>_transition_function`, `<prefix>` the machine's (`com.sce.generated` by default) | `enum class State`, `enum class Event` | `ALL_STATES`, `ALL_EVENTS` (`List`) | `INITIAL` | `SOURCE_HASH` (`const val`) | `fun transition(State, Event): State?` | `fun isFinal(State): Boolean` | `fun configuration(State): List<String>` | `State.scxmlName`, `stateFromName(String): State?`, `Event.scxmlName`, `eventFromName(String): Event?` |
| Python | `<stem>_transition_function.py` | the module `<stem>_transition_function`, beside the machine's | `class State(Enum)`, `class Event(Enum)`, each member's value its id | `ALL_STATES`, `ALL_EVENTS` (tuples) | `INITIAL` | `SOURCE_HASH` | `transition(State, Event) -> Optional[State]` | `is_final(State) -> bool` | `configuration(State) -> Tuple[str, ...]` | `.value` (and `str()`), `State.from_name(str) -> Optional[State]`, the same on `Event` |

`<stem>` is the document's file stem and `<name>` the machine's name, which is the same word. A run
that moves the machine's names moves the function's with them: `--cpp-namespace-prefix p` makes the
C++ namespace `SCE::Generated::p::<name>::transition_function`, `--kotlin-package-prefix q` the
Kotlin package `q.<name>_transition_function`, and `--c-symbol-prefix r` every C symbol
`r_<name>_transition_function_…`. The Go function imports nothing, so `--go-module-prefix` does not
reach it.

Each spelling of a state and an event is the filter the machine's own enumeration uses in that
backend, so a state is the same identifier in the function and in the machine: `capitalize` and
`to_cpp_event_variant` in C++, `to_c11_upper_ident` in C, `to_state_variant` and `to_event_variant`
in Go, `to_state_class_name` for a Kotlin state, `to_python_const` in Python. The names
`scxml/generated-name-collision` and `validation/reserved-code-identifier` refuse are refused for
every backend before any file is written. Every function's names live where the machine's cannot
reach them, so the two files of one machine compile in one build unit: a C++ namespace of their
own; a C prefix that no symbol of the machine begins with, since the machine's are `<name>_`
followed by something else (`<name>_state_t`, `<name>_init`, `<NAME>_STATE_…`); a Go package, a
Kotlin package and a Python module of their own. The one machine symbol a document spells is a
published variable's C reader, `<name>_<variable>`, so a variable whose reader would begin
`transition_function_` is refused in every document, as one beginning `_sce_synth_invoke__` is.
The other way round, a function's names are no members of the machine's type: a C++ machine
publishes a variable `size` beside a function whose `Configuration` has a `size()`.

### What each language forces

The surface is the same everywhere, and it differs only where a language cannot say what another
says. These are those places, and there are no others:

- **An absent next state.** Rust, C++, Kotlin and Python return their optional (`None`,
  `std::nullopt`, `null`, `None`). Go has none, and answers in its own idiom: `Transition` returns the
  zero `State` and `false`. C has none either: `P_transition` returns `false` and leaves `*next` as it
  was; it writes `*next` only when it returns `true`, and `next` must point to a state.
- **A namespace.** C has none, so the function's names are prefixed instead of nested; Go ties a
  package to a directory, so the function's package is a directory inside the machine's output
  rather than a file beside it.
- **Constant lists.** C cannot return an array, and has no array of length zero: `P_all_states()` and
  `P_all_events()` return a pointer to constant storage, `P_NUM_STATES` and `P_NUM_EVENTS` long, and
  `P_all_events()` is `NULL` for a document with no event. The two counts are the last enumerators of
  their types, so they cannot be spelled by a name of the document, every other enumerator being
  `P_STATE_<id>` or `P_EVENT_<name>`. C++17 has no `std::span`, so `configuration` returns
  `Configuration`, a pointer and a count with `begin`, `end`, `size` and `[]`, over a constant array.
- **Names on an enumeration.** A C++ `enum class` holds no function and a Kotlin enum already has a
  `name` (the entry's own identifier), so the C++ names are free functions spelled as the machine's
  policy spells its own (`getStateName`, `getStateFromName`), and the Kotlin one is the property
  `scxmlName`. Kotlin's functions are top-level rather than in a `companion object`: an entry is
  PascalCase, and `Companion` is a name an entry can have.
- **Events in Kotlin.** The machine's Kotlin events are a tree of nested types, one per token of a
  name; an enum cannot nest, so the function's `Event` entries are the flat spelling the machine uses
  for an event's payload type (`to_event_variant`: `data.ready` is `DataReady`).
- **Reading a name in Go.** The read half is `ParseState` and `ParseEvent`, not `StateFromName`: every
  other name of the package beginning with `State` or `Event` is a constant spelled from the document,
  and a state `from_name` would spell `StateFromName`.

### C++

A header that includes `<array>`, `<cstddef>`, `<optional>` and `<string_view>` and nothing else,
written in C++17 — the rest of the C++ output's floor — though the repository builds it at C++20 with
the machine. Every function is `constexpr` and `noexcept`, and `[[nodiscard]]`, so a pair is a
constant expression: `static_assert(tf::transition(tf::State::Idle, tf::Event::Start) ==
tf::State::Running);` compiles.

### C11

One header of `static inline` functions over constant storage, including `<stdbool.h>`,
`<stddef.h>` and `<string.h>` and nothing of the C runtime. Nothing allocates. It compiles alone
under `-std=c11 -Wall -Wextra -Wpedantic -Werror`, and in one translation unit with the machine's
header. It carries the same `extern "C"` guard the machine's header does.

### Go

A package that imports nothing. `AllStates` and `AllEvents` are package variables, as the machine's
`<Machine>AllStates` is; `Configuration` returns a new slice on every call, so a caller that changes
one changes nothing else.

### Kotlin

A file of its own package, which needs nothing beyond the Kotlin standard library. Its functions are
top-level, so a caller imports them by name (`import <package>.transition`) or with the package.

### Python

A module that imports `enum` and `typing` and nothing else. Each enumeration's member values are
the ids and names as the document writes them, so `State("idle")` reads an id back as `from_name`
does (raising where `from_name` answers `None`), and `str(member)` is the id, as it is for the
machine's own enumerations.

### Rust

The module uses `core` alone and names no crate: it compiles in a `#![no_std]` crate with no
allocator, and its functions are `const fn` where the language allows it. Every function only
returns its answer, so each carries `#[must_use]`: a call whose answer is dropped is named by the
compiler. `transition` keeps one match arm per pair, the table's own form, and allows
`clippy::match_same_arms` for that reason. With that the file is clean under `clippy::pedantic` in
a crate that denies it, which a test holds by running clippy over the file alone. `State` and `Event` carry
the derives the machine's own `<Machine>State` and `<Machine>Event` carry (`Debug`, `Clone`, `Copy`,
`PartialEq`, `Eq`, `Hash`, and `Default` on `State`, marked on the initial state), but none of the
extra derives a caller may add to the machine's enums: an extra derive names a crate, and this file
names none. Variants are spelled by the filters the machine uses (`to_state_variant`,
`to_event_variant`), so a state is the same identifier in both, and the names the machine refuses
(`scxml/generated-name-collision`, `validation/reserved-code-identifier`) are refused before either
file is written.

`from_name` matches a name whole. The events of the function are the names its document writes; a
longer name a descriptor would match on a running machine is not one of them.

Every way of generating the document produces the same machine, and the function with it. It is
generated by each route that writes drift-headed files, `sce-codegen generate`, `orchestrate` and
`generate-w3c`, bound to the digest those files' headers carry. An in-line child
(`<invoke><content><scxml sce:transition-function="emit">`) means what a root means: it is judged
with the document that holds it, its refusals placed on the rows its author wrote there, and
`generate` and `generate-w3c` write its function beside its machine, named after the child's stem
(`<parent>__sce_synth_invoke__<invoke id>` where the table above writes `<stem>`) and bound to the
hash its machine carries. A route names the function under the roots it gives the machine: `generate`
passes it its `--cpp-namespace-prefix`, `--kotlin-package-prefix` and `--c-symbol-prefix`, and
`orchestrate` and `generate-w3c`, which give the machine none, give the function none; a renamed W3C
Kotlin suite renames the function's package root as it renames the machine's. `check` judges the declaration and writes nothing, on one document and on a set alike.

The library entry points compute no source hash, so they cannot generate the function. A document
that declares it is refused there with `generate/source-hash-unavailable`, naming the declaration
and the entry and saying to generate it with `sce-codegen generate`, rather than handed back as a
machine without it (a declared transition table is refused by the same check); the
in-memory entry the WASM build calls is among them. `compile_scxml_with_imports` generates the
function when the caller gives it `ForgeCompileOptions::source_hash`, and refuses it, naming that
option, when the caller does not. A document the declaration does not admit is refused by them on
the first of the records `generate` reports, their result carrying one error.

## What holds this together

`sce-build/src/transition_function.rs` tests the producer on every selection rule the declaration
relies on; `sce-build/src/scxml_transition_function.rs` tests one refusal each and all of them in one
run.

`generator::transition_function_files` is tested in `sce-build/src/generator.rs` on each backend's
file name, namespace and naming prefixes, and on the lines that carry the table.

`sce-build/tests/the_emitted_transition_function_equals_the_producer.rs` generates two documents
(`integration_resources/flat_lifecycle/` and `integration_resources/nested_lifecycle/`, a flat cycle
and a nested lifecycle that reaches every selection rule) in all six backends, and:

- reads every backend's table back from its source and holds it to the producer's, on any host;
- compiles each backend's file alone, with its own toolchain and nothing else — Rust in a
  `#![no_std]` library under `-D warnings`, C++ and C under `-Wall -Wextra -Wpedantic -Werror` with
  no include path, Go in a module that requires nothing, Kotlin with no classpath, Python imported in
  an isolated interpreter from its own directory — then runs a program around it that prints every
  answer, and compares the print with the producer's table; for C++ the compiler also checks every
  pair as a constant expression, and the header compiles in one unit with the machine's;
- builds the machine generated beside it against the real runtime of Rust, Go, C, Kotlin and Python,
  drives it to every state of the function by the shortest event path the function gives, sends it
  every event there, and holds the configuration the engine reaches, and whether it took the event
  or discarded it, to the function;
- holds the declaration's one value, the forge root that does not take it, every backend accepting
  it, and every route reporting the refusals of an impure document: `check` on one document and on
  a set, `generate`, `orchestrate` and `generate-w3c` every record at once and the same records, the
  library's file and in-memory entry points the first of them;
- holds every route that writes the machine to the function beside it, in Rust: `orchestrate` and
  `generate-w3c` for a document, `generate` and `generate-w3c` for an in-line child, each file
  carrying the hash its machine carries, and the W3C Rust module index naming each function module;
  an in-line child's refusal on the row its author wrote; and the library entry points refusing, by
  name, the function or the table they have no hash for.

`sce-build/tests/no_conformance_document_declares_the_transition_function.rs` holds the CMake route
of the conformance suite to the documents it can reach: none declares the function, so the outputs
`cmake/SCEStaticW3CTest.cmake` lists by hand are every file a C++ or C11 fixture's generation writes.
It is a target of its own so that it runs on every push: the document that would break the route is
one an author adds under `resources/`, which the suite's own lane does not filter on.

The two documents are integration stems, so every channel holds its engine to its function the same
way, in its own suite: the Rust, Go, Kotlin, Python and C11 drivers, and the C++ AOT engine and the
Interpreter, which this file cannot build (`docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md`). The
Interpreter generates nothing, and is held to the function the build computed for the same
document.

`sce-build/tests/mutations/the_emitted_transition_function_equals_the_producer.cases` makes the
producer and each backend's template wrong once in a mutation round, so the tests above are known
to be able to fail.
