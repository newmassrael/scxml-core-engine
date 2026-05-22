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
| Go | hybrid — W3C `sce-go-tests/generated/` is `.gitignore`d (CI regen), donedata `sce-go-tests/donedata_local_invoke/` is committed | mixed |
| C++ | no — `${CMAKE_CURRENT_BINARY_DIR}/w3c_static_generated/` | CMake build-time |
| Python | no — `sce-python-tests/generated/` is `.gitignore`d | CI regen via `sce-codegen generate-w3c -l python` |
| C11 | no — `${CMAKE_CURRENT_BINARY_DIR}/sce-c-tests/generated/` | CMake build-time |

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
| C++ | `tests/integration/DonedataLocalInvokeTest.cpp` (gtest against `runtime/StateMachine.h`) | pending (Phase C of layout RFC) |
| Python | `sce-python/tests/test_donedata_local_invoke.py` (pybind11 wrapping `ReadySCXMLEngine` over C++ Interpreter, commit `0589bb35`) | pending (Phase D of layout RFC) |
| C11 | n/a (AOT-only backend) | pending (Phase E of layout RFC) |

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
| Rust | AOT-generated tree | `sce-rust-tests/src/integration/donedata_local_invoke/` | yes |
| Kotlin | AOT-generated tree | `sce-kotlin-tests/src/main/kotlin/com/sce/generated/donedata_local_invoke/` | yes |
| Go | AOT-generated tree | `sce-go-tests/donedata_local_invoke/` | yes |
| C++ | Interpreter gtest | `tests/integration/DonedataLocalInvokeTest.cpp` | n/a (no committed generated tree) |
| Python | Interpreter via pybind11 | `sce-python/tests/test_donedata_local_invoke.py` + `sce-python/tests/fixtures/donedata_local_invoke.scxml` | n/a (pybind11 channel inherits contract from C++ Interpreter) |
| C11 | codegen donedata literal-shape support only | `tools/codegen/templates/c/state_machine.c.jinja2` (`6eec3a95`); W3C IRP donedata tests 294/527/528/529/176/179/186/578/298 verify literal contract at build time | n/a (build-time generation; W3C IRP only — local-invoke contract not yet covered) |

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
literal-shape contract end-to-end (`sce-c-tests/CMakeLists.txt`).
What c11 does **not** yet cover is the `<donedata> + <invoke> +
done.invoke.<id>._event.data` *combination* contract that the W3C
IRP itself does not test (Phase 0 grep confirmed zero W3C IRP fixtures
combine all three). Phase E of the layout RFC closes this gap with a
single integration fixture mirroring the other backends.

### Python coverage detail

The Python channel is `pybind11 → ReadySCXMLEngine → C++ Interpreter`.
Commit `0589bb35` 2026-04-24 added
`sce-python/tests/test_donedata_local_invoke.py` (109 LOC) and the
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

When a future SCXML contract requires this layer, follow the
per-backend convention as it exists at HEAD; the layout RFC will revise
this procedure to a 6-backend uniform workflow once its phases land.
Current procedure:

1. Author the source `.scxml` under the canonical fixture root that
   the layout RFC's Q-8 establishes (target:
   `resources/integration/<stem>.scxml`). Until Phase B lands,
   per-backend `fixtures/` directories hold copies.
2. Author per-backend regen scripts following the
   `scripts/regen_donedata_local_invoke{,_kotlin,_go}.sh` pattern.
3. Register the new sub-module in each backend's integration entry
   point (Rust `sce-rust-tests/src/integration/mod.rs`, Kotlin via
   `generated/` for now, Go top-level for now — the layout RFC
   uniformises these).
4. Wire the §6.2.6 drift-verify CI gate to each backend's new
   generated directory in `.github/workflows/drift-verify.yml` and the
   `scripts/hooks/pre-commit` drift-verify trigger.
5. For cpp/Python/C11 (no committed tree), wire the fixture into the
   per-backend build/CI entry point (`tests/CMakeLists.txt` for cpp;
   Python CI workflow; `sce-c-tests/CMakeLists.txt` for C11).

## RFC reference

The full long-term-correct end state is defined in
`claudedocs/rfc-donedata-5-backend-layout.md` (locked 2026-05-22, 9
Q-locks decided). Key end-state guarantees once all phases land:

- Single canonical fixture source `resources/integration/<stem>.scxml`
  for all 6 backends (Q-8).
- Committed-tree backends (Rust / Kotlin / Go) share canonical
  `integration/` layout sibling to W3C `generated/` (Q-1).
- Per-language anchor file convention: Rust `mod.rs` /
  Kotlin `package-info.kt` / Go `doc.go` (Q-1a).
- Build-time backends (C++ / C11) share canonical
  `sce_generate_static_integration_test` CMake function (Q-2).
- C++ / Python retain both Interpreter and AOT channels (Q-3, Q-4).
- `sce-codegen generate-integration -l <lang>` subcommand parallel to
  `generate-w3c` (Q-6).
- `scripts/regen_all_committed_trees.sh` master regen script (Q-7).
- Every backend has ≥1 channel for the `donedata_local_invoke`
  contract — "uncovered" eliminated.
