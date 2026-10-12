# sce-python-runtime

Pure-Python runtime for AOT-generated SCXML state machines. Mirrors the role
of `backends/go/runtime`, `backends/rust/runtime`, and the Kotlin runtime: `sce-codegen
--language python` emits a `*_sm.py` module that depends on this package.

## Atomic α scope

This is the foundation release. Supported today:

- Atomic states (no compound, no parallel, no history)
- Basic transitions (event + target + guard cond)
- `<onentry>` / `<onexit>` script execution
- Transition `<script>` action execution
- External event injection (`engine.send_event`)
- Eventless transition drain

Deferred to Atomic β / γ:

- Compound states, `<initial>` transitions
- `<parallel>` regions and the active-states set
- `<history>` states (shallow + deep)
- `<invoke>` (SCXML child sessions, HTTP, mesh-rpc)
- `<data>` / `<datamodel>` / `<assign>` (Python AOT datamodel)
- `<send>` / `<cancel>` and the delayed event scheduler
- `<raise>` internal events from action payloads
- Microstep / macrostep refinement (currently single-source dispatch)

`reject_python_unsupported_features` in `sce-build/src/generator.rs` fails
codegen loudly on any of the deferred surface — Atomic α never silently
degrades, it stops.

## Channel separation

There are two Python integrations for SCE:

| Package | Mode | Mechanism |
|---|---|---|
| `sce` (under `backends/python/bindings/`) | **Interpreter** | pybind11 → C++ Interpreter parses SCXML at runtime |
| `sce_runtime` (this package) | **AOT** | Generated `*_sm.py` is the state machine; this runtime is a generic driver |

Both pass W3C SCXML 1.0 conformance in their respective modes — the
interpreter channel today (202/202), the AOT channel as Atomic β / γ land.

## Observing a run

`engine.set_observer(observer)` attaches a read-only tap; `set_observer(None)`
detaches it. `observer(kind, fields)` is called, in the order things happen, with:

| `kind` | when | `fields` |
|---|---|---|
| `event` | an event is taken for processing | `name` |
| `discarded` | no transition of any active state matched it | `name` |
| `transition` | one was selected | `source`, `event` (the descriptor as the document wrote it; `""` eventless; `None` when the generated code predates the field), `targets`, `internal` |
| `exit` / `enter` | a state leaves / joins the configuration | `state` |

A diagnostic and nothing more: it is told names and plain values, never asked
anything, and may not call back into the engine, so a machine with an observer
reaches the same configurations and sends as one without. An exception it raises
is not swallowed. This is how `sce_author verify --explain` says WHY a statechart
wrote a position when it did — including the `error.execution` the platform
raises, which no value of the document shows.
