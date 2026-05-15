# sce-python-runtime

Pure-Python runtime for AOT-generated SCXML state machines. Mirrors the role
of `sce-go-runtime`, `sce-rust-runtime`, and the Kotlin runtime: `sce-codegen
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
| `sce` (under `sce-python/`) | **Interpreter** | pybind11 → C++ Interpreter parses SCXML at runtime |
| `sce_runtime` (this package) | **AOT** | Generated `*_sm.py` is the state machine; this runtime is a generic driver |

Both pass W3C SCXML 1.0 conformance in their respective modes — the
interpreter channel today (202/202), the AOT channel as Atomic β / γ land.
