# SCE Integration Fixture Layout (Non-W3C-IRP)

This document records the per-backend layout for hand-curated integration
fixtures — SCXML programs that exercise semantics not covered by the W3C
IRP suite. The W3C IRP suite (under `resources/<N>/test<N>.txml`, where
each fixture is regenerated to `test<N>.scxml` and consumed by the
per-backend W3C harnesses) remains the spec-conformance baseline; the
integration layer described here is strictly additive.

Only one such fixture exists today: `donedata_local_invoke`.

The full uniformity roadmap (per-backend layout migration, AOT/Interpreter
two-channel parity, SSoT canonical fixture path) lives in
`claudedocs/rfc-donedata-5-backend-layout.md`. This document records the
**current state** at HEAD; rows update as each phase of that RFC lands.

## Why this layer exists

W3C SCXML §5.5 (`<donedata>`) and §6.3.1 (`done.invoke.<id>` event
emission) interact when an inline child machine reaches a top-level
`<final>` carrying `<donedata>` and the parent reads `_event.data` on
the `done.invoke.<id>` event. No public W3C IRP fixture exercises this
combination directly — the IRP `<donedata>` tests cover machine-level
done emission, not the child-invoke-to-parent round trip. A repository
grep `for f in resources/*/test*.txml; do donedata && invoke; done`
confirms zero W3C IRP fixtures combine both.

The mesh suite covers the same contract for the AOT wire-18 path via
`test_mesh_session_f_donedata`. The integration layer documented here
covers the parallel *local-invoke* path.

## Two architectural axes

The 6 backends differ along two orthogonal axes that the previous
revision of this document conflated under a single "Interpreter
first-class vs AOT-only" framing.

### Axis 1 — Committed generated tree vs build-time generation

| Backend | Committed generated tree? | Source |
|---|---|---|
| Rust | yes (W3C `src/generated/` + integration `src/integration/`) | hand-committed; regen via `sce-codegen generate-w3c -l rust` + per-fixture scripts |
| Kotlin | yes (W3C `…/com/sce/generated/` + integration intermixed under same) | hand-committed; regen via `sce-codegen generate-w3c -l kotlin` + per-fixture scripts |
| Go | hybrid — W3C `backends/go/tests/generated/` is `.gitignore`d (CI regen), donedata `backends/go/tests/donedata_local_invoke/` is committed | mixed |
| C++ | no — `${CMAKE_CURRENT_BINARY_DIR}/w3c_static_generated/` | CMake build-time |
| Python | no — `backends/python/tests/generated/` is `.gitignore`d | CI regen via `sce-codegen generate-w3c -l python` |
| C11 | no — `${CMAKE_CURRENT_BINARY_DIR}/backends/c/tests/generated/` | CMake build-time |

Committed-tree backends are §6.2.6 drift-gated (per-context source-hash
+ template-hash invariant via `b9_drift_detection::verify_passes_on_real_committed_*`);
build-time backends rely on CMake/CI to regenerate fresh trees on every
build, so the build process itself is the §6.2.6 freshness invariant.

### Axis 2 — Engine path (Interpreter vs AOT)

| Backend | Interpreter channel | AOT channel |
|---|---|---|
| Rust | n/a (AOT-only backend) | committed integration tree |
| Kotlin | n/a (AOT-only backend) | committed integration tree |
| Go | n/a (AOT-only backend) | committed integration tree |
| C++ | `tests/integration/DonedataLocalInvokeTest.cpp` (gtest against `runtime/StateMachine.h`) | `tests/integration/DonedataLocalInvokeAotTest.cpp` against build-time `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/donedata_local_invoke_sm.{h,inl}` (CMake `sce_generate_static_integration_test`) |
| Python | `backends/python/bindings/tests/test_donedata_local_invoke.py` (pybind11 wrapping `ReadySCXMLEngine` over C++ Interpreter, commit `0589bb35`) | `backends/python/tests/integration/donedata_local_invoke/test_donedata_local_invoke_aot.py` against gitignored `*_sm.py` (regen via `scripts/regen_donedata_local_invoke_python.sh` / `sce-codegen generate-integration -l python`) |
| C11 | n/a (AOT-only backend) | `backends/c/tests/integration/test_donedata_local_invoke.c` against build-time `${CMAKE_CURRENT_BINARY_DIR}/backends/c/tests/integration_generated/donedata_local_invoke_sm.{h,c}` (CMake `sce_generate_static_integration_c_test`) |

C++ and Python are the only backends with both engine paths in
production: Interpreter (embedded usage — consumer loads SCXML at
runtime) and AOT (codegen-compiled consumers). The layout RFC adds the
AOT channel without removing the Interpreter channel — both are
production code paths whose execution traces differ, so both are
verified independently.

The other 4 backends (Rust / Kotlin / Go / C11) are AOT-only — they
have no Interpreter and the AOT channel is the canonical contract test.

## Per-backend coverage at HEAD

| Backend | Coverage form | Location | Drift-verify CI gate |
|---|---|---|---|
| Rust | AOT-generated tree | `backends/rust/tests/src/integration/donedata_local_invoke/` | yes |
| Kotlin | AOT-generated tree | `backends/kotlin/tests/src/main/kotlin/com/sce/generated/donedata_local_invoke/` | yes |
| Go | AOT-generated tree | `backends/go/tests/donedata_local_invoke/` | yes |
| C++ | Interpreter gtest + AOT build-time | Interpreter: `tests/integration/DonedataLocalInvokeTest.cpp`; AOT: `tests/integration/DonedataLocalInvokeAotTest.cpp` against `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/` | n/a (build-time generation, freshness invariant = CMake build) |
| Python | Interpreter via pybind11 + Python AOT | pybind11: `backends/python/bindings/tests/test_donedata_local_invoke.py`; AOT: `backends/python/tests/integration/donedata_local_invoke/test_donedata_local_invoke_aot.py` against gitignored `*_sm.py` | n/a (`*_sm.py` gitignored, regenerated by CI before pytest; mirrors W3C IRP Python pattern) |
| C11 | codegen donedata literal-shape + AOT integration fixture | Literal-shape: `tools/codegen/templates/c/state_machine.c.jinja2` (`6eec3a95`), verified by W3C IRP donedata tests 294/527/528/529/176/179/186/578/298. Cross-SM `done.invoke.<id>._event.data` lift: `tools/codegen/templates/c/invoke_methods.jinja2` (Phase E) + `tools/codegen/templates/c/scriptengine.jinja2` (`_sce_donedata_to_lua_literal`). Integration fixture: `backends/c/tests/integration/test_donedata_local_invoke.c` against build-time `${CMAKE_CURRENT_BINARY_DIR}/backends/c/tests/integration_generated/` | n/a (build-time generation, freshness invariant = CMake build) |

The 3 AOT-generated trees are regenerated by per-backend scripts
(`scripts/regen_donedata_local_invoke{,_kotlin,_go}.sh`) and guarded by
`.github/workflows/drift-verify.yml` plus the
`scripts/hooks/pre-commit` drift-verify trigger.

### C11 coverage detail

The c11 codegen template lifts `<donedata>` literal shape via lua stash
+ JSON-quoted `_event.data` carry (SSoT mirror of cpp
`DoneDataHelper::emitContentLiteral`, commit `6eec3a95` 2026-04-29).
W3C IRP donedata tests _294/527/528/529/176/179/186/578/298_ are
generated under the c11 backend at CMake build time and verify the
literal-shape contract end-to-end (`backends/c/tests/CMakeLists.txt`).

Phase E (LANDED) closed the `<donedata> + <invoke> +
done.invoke.<id>._event.data` *combination* contract that the W3C IRP
itself does not test (Phase 0 grep confirmed zero W3C IRP fixtures
combine all three). The fix added cross-SM payload carry at every
`done.invoke` raise site in `tools/codegen/templates/c/invoke_methods.jinja2`
(execute_pending_invokes scxml/hybrid + drive_active_children
scxml/hybrid, gated on `invoke_info.child_needs_script_engine`) plus a
generic Lua-source serializer (`_sce_donedata_to_lua_literal` in
`scriptengine.jinja2`'s `lua_init_engine`) that converts the child's
`_pending_donedata` lua global into a Lua-source expression. The
parent's existing `process_event_queues` external dequeue path
(`state_machine.c.jinja2:3493-3498`) rebinds `_event.data = (<literal>)`
on its own `sm->L` so the parent's `done.invoke.<id>` transition cond
(`_event.data.result === 42`, `_event.data === 'hello_content'`)
evaluates against the typed donedata value. Mirrors cpp's
`donedataAtFinal()` carried through `EventMetadataHelper::createDoneInvokeEvent`
— cpp ships JSON because `EventWithMetadata` is engine-agnostic; C11
ships Lua source per its P1 lock-in (Lua 5.4 only) which is the
round-trip-free equivalent producing a typed value rather than forcing
a re-parse.

The C11 integration fixture
(`backends/c/tests/integration/test_donedata_local_invoke.c`) drives both the
`<param name="result" expr="42"/>` (table-shaped donedata) and
`<content expr="'hello_content'"/>` (scalar-shaped donedata) cond
branches through the parent's `done.invoke.{inv_param,inv_content}`
transitions, asserting the run reaches the `pass` final state — a
regression on either the lift macro or the parent's external-dequeue
override trips the test immediately.

### Python coverage detail

The Python channel is `pybind11 → ReadySCXMLEngine → C++ Interpreter`.
Commit `0589bb35` 2026-04-24 added
`backends/python/bindings/tests/test_donedata_local_invoke.py` (109 LOC) and the
fixture (85 LOC) without any template or runtime change — the pybind11
binding wraps the C++ Interpreter's `pendingDonedataAtFinal_` +
`SCXMLInvokeHandler` completion path, so the donedata stash/lift
contract is inherited automatically. The script verifies both `<param>`
(=== 42) and `<content>` (=== `'hello_content'`) branches and was
authored with load-bearing bites (param 42→99 reaches fail; content
`'hello_content'`→`'goodbye_content'` reaches fail; both restored reach
pass), so a future regression in either the C++ Interpreter or the
pybind11 wrapper trips this script.

The Python AOT channel (Phase D of the layout RFC) is separate from
this pybind11 path and verifies the codegen-emitted Python state-table
code independently.

## Adding a new custom integration fixture

When a future SCXML contract requires this layer:

1. Author the source `.scxml` at the canonical fixture root:
   `integration_resources/<stem>/<stem>.scxml` (per-fixture dir,
   mirroring the W3C IRP `resources/<N>/test<N>.txml` convention).
   The top-level `integration_resources/` dir sits outside
   `resources/` because `compute_source_hash` recurses through the
   input root — nesting integration under `resources/` would fold
   the integration fixture into the W3C source-hash domain.
2. Author per-backend regen scripts following the
   `scripts/regen_donedata_local_invoke{,_kotlin,_go}.sh` pattern.
   These are now thin wrappers around the canonical CLI surface
   (see step 3); each script encodes the per-language TMP staging,
   `--input-root` override, and post-processing (Rust `mod.rs`
   synthesis, Kotlin `// Source:` rewrite, Kotlin
   `--kotlin-package-prefix com.sce.integration`).
3. Bulk regenerate via the uniform CLI:
   `sce-codegen generate-integration -l <rust|kotlin|go> --stem <stem>`
   (single fixture) or omit `--stem` to walk every
   `integration_resources/<stem>/` dir. The
   `scripts/regen_all_committed_trees.sh` master script bundles W3C
   + integration + forge round-trip so a template touch lands one
   coherent commit across every drift context.
4. Register the new sub-module in each backend's integration entry
   point: Rust `backends/rust/tests/src/integration/mod.rs`, Kotlin
   `backends/kotlin/tests/src/main/kotlin/com/sce/integration/package-info.kt`,
   Go `backends/go/tests/integration/doc.go`.
5. Wire the §6.2.6 drift-verify CI gate to each backend's new
   generated directory in `.github/workflows/drift-verify.yml` and
   the `scripts/hooks/pre-commit` drift-verify trigger.
6. For cpp / C11 / Python pybind11 channels (no committed tree),
   wire the fixture into the per-backend build/CI entry point
   (`tests/CMakeLists.txt` for cpp; Python CI workflow;
   `backends/c/tests/CMakeLists.txt` for C11).

## RFC reference

The full long-term-correct end state is defined in
`claudedocs/rfc-donedata-5-backend-layout.md` (locked 2026-05-22, 9
Q-locks decided). Key end-state guarantees once all phases land:

- Single canonical fixture source `integration_resources/<stem>/<stem>.scxml`
  for all 6 backends (Q-8 + Q-8a per-fixture dir, separate top-level
  from W3C `resources/` to keep drift contexts disjoint).
- Committed-tree backends (Rust / Kotlin / Go) share canonical
  `integration/` layout sibling to W3C `generated/` (Q-1).
- Per-language anchor file convention: Rust `mod.rs` /
  Kotlin `package-info.kt` / Go `doc.go` (Q-1a).
- Build-time backends (C++ / C11) share canonical
  `sce_generate_static_integration_test` CMake function (Q-2).
- C++ / Python retain both Interpreter and AOT channels (Q-3, Q-4).
- `sce-codegen generate-integration -l <lang> [--stem <stem>]`
  subcommand parallel to `generate-w3c` (Q-6, LANDED).
- `scripts/regen_all_committed_trees.sh` master regen script
  bundling W3C + integration + forge round-trip (Q-7, LANDED).
- Every backend has ≥1 channel for the `donedata_local_invoke`
  contract — "uncovered" eliminated.
