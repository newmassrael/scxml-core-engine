# The transitions of a machine: the export and the generated table

A machine's transitions are one fact with two surfaces, both rendered from one list
(`sce-build/src/transition_records.rs`, `transition_records`):

| Surface | What it is | When to use it |
|---|---|---|
| `sce-codegen transition-table` | A design-time export: NDJSON, one row per node of the design that can carry a requirement. Registered wire surface, `schemas/sce-transition-table.v1.schema.json` (`SCE_WIRE_CONTRACTS.md`). | Drawing or reviewing a design; anything that reads the SCXML and not a running machine. |
| `sce-codegen generate --emit-transition-table` | A constant table in the generated source of every backend, with the `source_hash` of the run. | A host that holds the compiled machine and wants to check or draw it without reading the SCXML. |

Nothing in a machine reads either one. The generated table is constant data that no
dispatch touches, and without the flag every generated file is byte-identical to what it was
before the option existed.

## What an entry holds

| Field | Meaning |
|---|---|
| `from` | The owning state. |
| `to` | The first target, when it names one of the machine's states. |
| `event` | The event, when the transition names exactly one literal event the machine holds. |
| `index` | The transition's position within its state, in document order: the `N` of `states.<id>.transitions[N]`. |
| `annotation` | The `sce:req` tokens as written, space-separated. Empty when there are none. |

`to` and `event` are absent where a transition has no single id in the name tables:

- no target (a targetless transition), or a first target that is a `<history>`;
- no event (an eventless transition), a wildcard `*`, a prefix such as `error.*`, or several descriptors.

For those, and for the second and later targets, the export is the complete answer: its
`transition` rows carry `targets` (the whole target set) and `event` (every descriptor as
written), under the same `index`.

The annotation is an opaque token. The generator never reads its shape, so a step number
`12` arrives as the text `12`, and the integer is the consumer's own parse of it.

## Accessors, by backend

| Backend | Table | Hash | Entry |
|---|---|---|---|
| C++ | `Policy::transitions` (`std::array`), `Policy::transitionCount()` | `Policy::sourceHash()` | `Policy::TransitionRecord{from, to, event, index, annotation}`; `to` and `event` are `std::optional`. |
| C11 | `<prefix><name>_transitions[]`, `<PREFIX><NAME>_TRANSITION_COUNT` | `<PREFIX><NAME>_SOURCE_HASH` | `<prefix><name>_transition_t`; `to` is `<PREFIX><NAME>_STATE_COUNT` and `event` is `<PREFIX><NAME>_EVENT_NONE` when absent. No array is declared for a machine with no transition. |
| Rust | `<Name>Transition::ALL` | `<Name>Transition::SOURCE_HASH` | `<Name>Transition`; `to` and `event` are `Option`. |
| Go | `<Name>Transitions` | `<Name>SourceHash` | `<Name>Transition`; `HasTo` and `HasEvent` say whether `To` and `Event` hold a value. |
| Kotlin | `<Name>Transitions.all` | `<Name>Transitions.SOURCE_HASH` | `<Name>Transition`; `to` and `event` are nullable. |
| Python | `TRANSITIONS` | `SOURCE_HASH` | `<Name>Transition` (frozen dataclass); `from_state`, and `to` and `event` are `None` when absent. |

The states and events in an entry are that backend's own enumerators, so a consumer compares
them with the machine it compiled and not with a string.

## Checking a machine against its design

`source_hash` is the §synth-6.2.6 digest of the source set the machine was generated from:
the `source-hash` of the file's own header, held as a value. The export states the same value
when asked:

```
sce-codegen generate -l cpp -o out --emit-transition-table machine.scxml
sce-codegen transition-table --header machine.scxml > machine.transitions.ndjson
```

The first line of the second is `{"record":"header","v":1,"generator":...,"machine":...,"source_hash":...}`.
Compare its `source_hash` with the table's, then walk the table beside the `kind: "transition"`
rows in order: the same count, and for each pair the same `from`, `index` and `annotation`.

Two flags matter when the document is not generated from its own directory. `transition-table`
takes `--input-root` and `--deploy` and they mean what they mean to `generate`: pass the values
`generate` was given, or the two hashes name different source sets.

## What holds this together

`sce-build/tests/the_emitted_transition_table_equals_the_export.rs` reads each backend's table back
out of its generated source and compares it with the export of the same document, for all six
backends, including the identifiers, the hash, and an annotation carrying characters a string
literal has to escape. Where a toolchain is present the output is also compiled (C++ with a static
assertion for every field of every entry, C), imported (Python) or parsed (Rust, Go).

`a_computed_event_name_is_matched_like_any_other` is opted in (`EMIT_TRANSITION_TABLE` in its CMake
registration, `--emit-transition-table` in its regeneration scripts), so each channel's own build
compiles the table in a real machine; its Rust, Go and Kotlin trees are committed with it.

`sce-build/tests/transition_table_closure.rs` holds the export to the two witnesses `generate`
wrote for the same document: the `source_hash` of the sourcemap, and its `transition` symbols.
