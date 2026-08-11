# SCE Integration Fixture Layout (Non-W3C-IRP)

This document records the per-backend layout for hand-curated integration
fixtures — SCXML programs that exercise semantics not covered by the W3C
IRP suite. The W3C IRP suite (under `resources/<N>/test<N>.txml`, where
each fixture is regenerated to `test<N>.scxml` and consumed by the
per-backend W3C harnesses) remains the spec-conformance baseline; the
integration layer described here is strictly additive.

The stems are the directories under `integration_resources/`; this document
does not restate them. An earlier revision did, and the list was wrong within
one round of a stem being added — `event_origin_is_a_location` landed and the
sentence still named eleven.

A fixture placed there is a **seven-channel commitment** — C++ Interpreter,
C++ AOT, Rust, Go, Kotlin, Python, C11 — and the commitment splits in two.

The sites that GENERATE each channel are loud when they are missing:
`scripts/regen_<stem>{,_go,_kotlin,_python}.sh` (without them
`regen_all_committed_trees.sh` exits non-zero), the `pub mod <stem>;` in the
Rust integration tree (`rust-modrs-drift` blocks the push), and the two CMake
registrations (the build fails).

The sites that ASSERT are silent. Generated code that nobody runs still
compiles, so a stem can land with a machine in all seven channels and a test
driver in two while every gate stays green. That is not hypothetical:
measured 2026-08-11, `event_origin_is_a_location` had exactly that shape, and
when the five silent channels were finally asked, all five were violating the
clause the fixture exists to prove. `sce-build/tests/integration_stem_registration.rs`
now requires both halves — including this document's own entry, because a
fixture with no recorded axis is an axis nobody chose.

A fixture whose semantics only some backends implement does not belong here —
its committed trees would advertise coverage that does not exist. Close the
parity gap first, then add the fixture.

`autoforward_event_fields` covers W3C §6.4's exact-copy requirement for
`<invoke autoforward="true">`: the forwarded event must reach the child with
its `_event.data`, `_event.origin` and `_event.invokeid` intact. Every channel
asserts it — C++ Interpreter + AOT, Rust, Go, Kotlin, Python, C11 — because
each backend forwarded only the event name (Python: name + payload) until the
carrier landed.

**Fixtures stay on one axis.** `autoforward_event_fields` returns the child's
verdict as its own event rather than as `<donedata>`, so a regression in the
donedata lift cannot surface as an autoforward failure; `donedata_local_invoke`
owns that axis. An earlier revision coupled the two and a C11 donedata gap
masqueraded as an autoforward bug on that backend alone.

The donedata axis is split the same way. `donedata_local_invoke` pins the
payload *shapes* — a `<param>` table and a `<content>` scalar — on a child
whose initial configuration is already its top-level `<final>`, so the lift
and the `done.invoke.<id>` raise sit in the same call. `donedata_late_completion`
pins the *timing*: the child answers an event first and reaches `<final>` two
macrosteps in, which is a different completion-detection site in every AOT
backend. It reuses the sibling's `<param name="result" expr="42"/>` payload
verbatim so a shape the sibling already proves green cannot be what fails
there. Deleting the late-completion lift from
`tools/codegen/templates/c/invoke_methods.jinja2` reds
`c11_integration_donedata_late_completion` while
`c11_integration_donedata_local_invoke` stays green — the sibling is
structurally blind to that site.

`send_param_payload` covers W3C §6.2's `<param>` payload contract on the two
send paths that had no runtime witness — one `<send target="#_parent">` from a
`datamodel="null"` child, which needs no script engine, and one
`<send target="#_internal">` whose params must arrive as `_event.data`. Both
were fixed at the template layer while no committed fixture had a machine of
either shape, so every suite could show was that nothing regressed. The two
land in distinct final states (`failChildPayload` / `failInternalPayload`) so a
failure names the path rather than reporting "payload lost". Adding it closed
two C11 parity gaps that the missing fixture had hidden: a literal param was
formatted through the runtime Lua formatter, which does not compile in a
machine with no `lua_State`; and `<send target="#_internal">` with `<param>`
children fell through to the unenumerated-shape fallback and raised
`error.execution`.

The autoforward family is three fixtures on three questions, and each was
built blind to the other two. `autoforward_done_invoke` pins *which* events
are forwarded: Appendix D's `mainEventLoop` forwards whatever comes off the
external queue without consulting its name, so `done.invoke.<id>` is inside
the set and the only exclusion — the cancel event — is expressed as control
flow rather than as a name test. `autoforward_internal_queue` pins the
negative half, which the same loop expresses purely by position: the internal
drain has no forwarding step at all, so an `error.execution` raised there must
never reach a child, and must be excluded by *where it was raised* rather than
by a filter that recognised its name. `autoforward_dequeue_point` pins *when*:
the forward sits one statement after the dequeue and before transition
selection, which neither sibling can see, because both were deliberately built
to be blind to it.

`invoke_precedes_external_dequeue` and `invoke_precedes_dequeue_midrun` split
Appendix D's invoke-before-dequeue ordering the same way. The first pins the
start-up case: the external queue is named exactly once in `mainEventLoop` and
it is after `invoke(inv)`, so an engine that folds the external drain into the
macrostep loop consumes what `<onentry>` queued while the children do not yet
exist — a lost event, not a reordered one. The second pins that the ordering
is not a property of start-up: `statesToInvoke` is filled by `enterStates`, so
a state entered by an *external* event's transition arms an invoke that must
start before the next event comes off the queue. An engine that drains to
exhaustion inside one step passes the first and fails the second.

`nested_final_not_terminal` covers W3C SCXML 3.7: only a `<final>` whose parent
is the `<scxml>` element ends the session. Appendix D's `enterStates` splits
the two cases in one branch, so `isFinalState(s)` is a structural question and
not by itself the completion criterion. An engine that answers "has this
session ended?" with the bare structural predicate reports completion the
moment a compound state finishes, while the machine is still live. The trap is
the naming: Appendix D's separate `isInFinalState(s)` is a third thing again,
asking whether a compound or parallel state has completed for the done.state
computation.

`event_origin_is_a_location` covers W3C SCXML Appendix C.1: the origin of a
delivered event is the `location` the sending session published for the SCXML
Event I/O Processor, and that location is a usable `<send>` target. The public
IRP suite cannot separate the two halves — test336 and test350 both check
`_event.origin` by sending to it with the sender and the receiver being the
same session, so any value at all round-trips. This fixture puts a peer session
on the other end, which is the only arrangement where a bare session id and a
published location differ. A mismatch lands in `fail`; a routing violation
leaves the parent parked in `await_reply` and the harness times out, which is a
weaker signal on purpose, because a target that resolves nowhere produces no
event to transition on.

The fixture is single-axis to the point of comparing the two strings for
equality rather than testing a prefix: the guard is evaluated by whichever
engine the backend embeds, and a failed evaluation raises `error.execution` and
reads as a false condition — so a probe written with a method one engine lacks
reports a violation that is not there.

Every channel asserts it, and the reason is the history: the C++ pair landed
first and its own comment claimed the other five answered the same question.
They had no driver at all. All five were violating the clause, in five
different ways — two names for one child session (Rust, Go), no conversion at
the `_event` boundary (all five), a `#_<invokeid>` origin that is §6.4's
addressing form rather than C.1's published location (C11), and two buffers
sized for an id rather than an address (C11), where a truncated address is
indistinguishable from a spec violation.

`invoke_unsupported_type` covers W3C §6.4.1: an `<invoke>` whose `type` names
no processor the platform implements is valid SCXML that must raise
`error.execution`, not a document to reject. Both engines were silent here in
different ways — the Interpreter substituted its SCXML handler for the unknown
type and started a child session the author never asked for, while AOT dropped
the `<invoke>` from the model outright and produced no observable at all. The
fixture is single-axis to the point of carrying no `src` and no `<content>`:
§6.4.1 classifies on `type` alone, before any child document would be
resolved, so a fixture that supplied one would let a child-materialization
regression masquerade as an unsupported-type regression. It is also why this
stem is the only one whose CMake registration passes no
`SYNTH_INVOKE_CHILDREN` — there is no child to synthesize.

Every channel asserts it, because wiring one backend does not close the
contract for the rest: the `Invoke::Unsupported` model variant is skipped by
each template's `scxml`-family filter until that backend is wired explicitly,
which moves the silent drop from the parser into the templates rather than
removing it. Measured directly during the work that added this fixture —
after the C++ lowering landed, five backends still emitted zero raise sites
while reporting successful generation. C11 needed three separate gates opened
(the entry-action switch, the `scxml_family` include guards, and an
`execute_pending_invokes` arm past the `| scxml` filter's index space), and
Rust passed a generated-source assertion while still resting in `probe` at
runtime, so the emit-site check and the runtime channel are not
substitutes for one another.

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
build-time backends rely on CMake to regenerate on every build, so the
build process itself is the §6.2.6 freshness invariant.

That invariant holds only while every codegen step declares the
templates as an input. CMake learns the SCXML dependency from `DEPENDS`
and the ~120 template dependencies only from a `DEPFILE` written by
`sce-codegen --write-deps`. Steps missing it were measured to reuse
stale artefacts after a template edit — 0 of 21 C++ integration outputs
regenerated, 74 of 270 C11 — while the build reported success. All ten
steps now carry it, and `sce-build/tests/codegen_depfile_coverage.rs`
holds them there; each site was individually mutated to confirm the gate
catches its removal.

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
