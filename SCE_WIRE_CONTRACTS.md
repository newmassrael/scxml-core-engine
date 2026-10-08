# SCE Wire Contracts — Stability & Deprecation Policy

This document is the single registry for SCE's **surfaces**: the
checked-in schemas, stores and emitted artifacts that something outside
the code writing them reads as authority. Two kinds share the one table
below, and the `Format owner` column is what separates them.

**Wire surfaces** are produced by SCE, which owns their schema. An
external consumer builds tooling against them — a code generator that
reads SCE's output, a validator that lints Extended SCXML before
codegen, a traceability tool that maps generated symbols back to source
— so for these the registry answers one question without the consumer
reading the engine source: **"How stable is this surface, and how will
I learn when it changes?"**

**Spec-bearing surfaces** are checked-in machine-readable artifacts
carrying the specification model itself: which sections exist, which
code and which tests are bound to them, which fixtures the conformance
suites run, and what the vendored upstream said. Several are owned by
mnemosyne, an external tool, so registering one records **that it
exists and what it holds** — it does not move the format into SCE.
They are here because a surface nobody can find gets rebuilt: on
2026-09-13 this repository spent a day designing a requirement-trace
table that `docs/spec/scxml/.atomic/` had been running for months,
because the search vocabulary did not overlap and no entry document
named the stores.

This registry therefore covers the *stability contract* for wire
surfaces and *existence and content* for spec-bearing ones. The *shape*
of each surface is defined by its own schema file and per-surface
governance doc (linked below).

## Surfaces

| Surface | Format owner | Schema / artifact | Status source (producer ↔ schema header) | Current status | Shape governance |
|---|---|---|---|---|---|
| Diagnostics (`--error-format=json`) | SCE | `schemas/sce-diagnostic.v1.schema.json` | `SCHEMA_STATUS` (`sce-build/src/forge/diagnostic.rs`) ↔ `x-sce-schema-status` | `pre-release` | `SCE_ERROR_CONTRACT.md` §8 |
| Forge AST export (`--emit-ast`) | SCE | `apis/forge-ast.v1.schema.json` | `FORGE_AST_SCHEMA_STATUS` (`sce-build/src/forge/ast_export.rs`) ↔ `x-sce-schema-status` | `pre-release` | `docs/SCE_FORGE_AST.md` §3 |
| Sourcemap sidecar (`out/{lang}/sce_sourcemap.json`) | SCE | `schemas/sce-sourcemap.v1.schema.json` | `SOURCEMAP_SCHEMA_STATUS` (`sce-build/src/forge/sourcemap.rs`) ↔ `x-sce-schema-status` | `pre-release` | This doc + `sce-build/src/forge/sourcemap.rs` (producer); reverse-lookup via `sce-codegen addr2sce` |
| Authoring grammar (Extended SCXML) | SCE | `schemas/sce-forge.xsd`, `schemas/sce-forge-ext.xsd` | `<xs:documentation>x-sce-schema-status: …</xs:documentation>` (first child of `<xs:schema>`) | `pre-release` | `docs/SCE_ACCEPTED_SUBSET.md` |
| Stdout manifest (`generate`, `check`, `orchestrate`) | SCE | `schemas/sce-manifest.v1.schema.json` | `MANIFEST_SCHEMA_STATUS` (`sce-build/src/manifest.rs`) ↔ `x-sce-schema-status` | `pre-release` | `SCE_ERROR_CONTRACT.md` §10 |
| Symbol lookup (`addr2sce`, `sce2sym`) | SCE | `schemas/sce-symbol-lookup.v1.schema.json` | `SYMBOL_LOOKUP_SCHEMA_STATUS` (`sce-build/src/forge/sourcemap.rs`) ↔ `x-sce-schema-status` | `pre-release` | This doc + `sce-build/src/forge/sourcemap.rs` (producer) |
| Kind catalog (`sce-codegen kinds`) — what each `sce:kind` is for, the evidence a specification offers for it, what separates it from its neighbours, and an embedded example `check` accepts, for an author choosing a kind outside this tree | SCE | `schemas/sce-kind-catalog.v1.schema.json` | `KIND_CATALOG_SCHEMA_STATUS` (`sce-build/src/forge/kind_catalog.rs`) ↔ `x-sce-schema-status` | `pre-release` | This doc + `sce-build/src/forge/kind_catalog.rs` (producer, one exhaustive table over `ForgeKind`); read by the authoring MCP's `scxml_kinds` |
| Authoring profile (`--profile` on `check`, `generate`, `orchestrate`, `accept`, `acceptance-check`) — what a specification's owner configures about how a design is authored for them: a file beside the specification, written by the owner, never emitted by SCE. Every setting belongs to one class the schema fixes (enforced, reported, guidance); the enforced ones are the interface, the spelling of the names a document defines and the anchoring of its evidence, the one guidance setting is a list of instructions nothing checks, and no setting is reported yet | SCE | `schemas/sce-authoring-profile.v1.schema.json` | `PROFILE_SCHEMA_STATUS` (`sce-build/src/authoring_profile.rs`) ↔ `x-sce-schema-status` | `pre-release` | `docs/SCE_ACCEPTED_SUBSET.md` §2.17 + `sce-build/src/authoring_profile.rs` (producer of nothing and the only reader: a setting this build does not know refuses the whole profile as `cli/profile-unusable`, so a profile written for a newer tool is never half applied); a setting is added without a `v` bump. Its sha256 is pinned by the acceptance record and published on the manifest's `profile` |
| Scenario set (`sce-codegen scenarios`) — examples of what a specification says a machine does, each anchored to a sentence of it word for word: events sent in, virtual time passing, and what must be observed (events sent outward, whether the machine finished, a named condition, a named data item), written against an interface the owner accepts with them: a file beside the specification, written by the owner or proposed by an AI client for the owner to confirm, never emitted by SCE. It is the input of the evidence column `needs-scenario` waits for; this surface runs nothing and says nothing about any design | SCE | `schemas/sce-scenario-set.v1.schema.json` | `SCENARIO_SET_SCHEMA_STATUS` (`sce-build/src/scenario_set.rs`) ↔ `x-sce-schema-status` | `pre-release` | `docs/SCE_ACCEPTED_SUBSET.md` §2.18 + `sce-build/src/scenario_set.rs` (producer of nothing and the only reader: a file it does not know is refused as `cli/closure-input-unusable`, so a set written for a newer tool is never half read, while a set that is one and has problems is answered with every problem at once, `PROBLEM_CODES`); a field is added without a `v` bump |
| Observation trace (`sce-codegen judge-scenarios`) — what an engine driver observed running a scenario set against one design: per step of every scenario, the events sent outward, whether the machine had finished, the active states and the data it could read, with a declaration of which of those the driver can see at all. Written by a driver, a program separate from SCE, never emitted by SCE; read by the judge, which compares it with what the scenarios expect and answers `pass`, `fail`, `not-judged` (a check the driver could not see: a gap, not a failure), `blocked` or `awaiting-decision`. It records a run and says nothing about whether a design is right | SCE | `schemas/sce-observation-trace.v1.schema.json` | `TRACE_SCHEMA_STATUS` (`sce-build/src/scenario_judge.rs`) ↔ `x-sce-schema-status` | `pre-release` | `docs/SCE_ACCEPTED_SUBSET.md` §2.19 + `sce-build/src/scenario_judge.rs` (producer of nothing and the only reader: a file it does not know is refused as `cli/closure-input-unusable`; a trace that is one and has problems is answered with them and the scenarios it cannot vouch for are `not-judged`, `TRACE_PROBLEM_CODES`); a field is added without a `v` bump |
| Saved state (a `sce-static` machine's `save()` / `restore()`) | SCE | `schemas/sce-saved-state.v1.schema.json` | `SCHEMA_STATUS` (`backends/rust/runtime/src/saved_state.rs`) and `SavedState.SCHEMA_STATUS` (`backends/kotlin/runtime/.../SavedState.kt`) ↔ `x-sce-schema-status`; each producer's own test reads the header | `pre-release` | `docs/SCE_ACCEPTED_SUBSET.md` §2.15 "Saving and restoring"; both backends are held to the shared instances in `sce-build/tests/fixtures/static_datamodel/saved/`, which `sce-build/tests/saved_state_schema.rs` validates against the schema |
| Provenance roster (`sce-codegen provenance-roster`) — per `DiagnosticCode`, whether a diagnostic of that code carries a spec anchor, and the reason when it does not | SCE | none — a TSV stream, `<code>\t<verdict>\t<reason>`, with no checked-in schema file | `PROVENANCE_ROSTER_STATUS` (`sce-build/src/forge/diagnostic.rs`) ↔ this row, guarded by `the_registry_declares_the_rosters_status`; ⚠ it has no schema file, so the reverse walk over `schemas/` + `apis/` cannot reach it and the anchor is per-surface | `pre-release` | `SCE_ERROR_CONTRACT.md` §2.1.2, which directs a consumer to this command as the lookup that settles what an absent `spec_provenance` means; wire held to the producer by `sce-build/tests/cli_provenance_roster_wire.rs` (spawns the binary) |
| Acceptance record (`sce-codegen accept`) — what a person accepted: the document, every file its parse read, the requirement manifest, the variant, and the specification and decision record it was authored from and the authoring profile and scenario set it was held to, each by sha256, and the text of each house rule of the profile the design applied (`applied_rules`, with the places that cite it), and which surface stated the acceptance (`channel`: `direct` for the surface the owner acts on, `relayed` for a client's report; the caller's statement, recorded and never verified), and what each requirement rested on (`evidence`: per requirement id the sorted sha256 digests of the report rows of its dependency closure; `unclaimed`: the digests of the rows that claim nothing; stated, never enforced, and it cannot lapse), and the record it replaced (`succeeds`: path and sha256); read back by `acceptance-check`, by `acceptance-impact` (many records at once, each lapse as data beside its sentence: its report lines carry the status of this row) and the authoring MCP's `scxml_acceptance_check` / `scxml_accepted_for` / `scxml_acceptance_impact` | SCE | none — a bare record naming itself (`record: "sce-acceptance-record"`, `v: 1`), with no checked-in schema file | `ACCEPTANCE_RECORD_STATUS` (`sce-build/src/acceptance_record.rs`) ↔ this row, guarded by `the_registry_declares_the_records_status` | `pre-release` | `sce-build/src/acceptance_record.rs` (producer and the only reader: `from_json` refuses what `take` would not write); an optional field is added without a `v` bump, and `authored_from`, `applied_rules`, `evidence` and `unclaimed` are omitted when empty, and `channel` and `succeeds` when not stated, so a record taken without them keeps its bytes (the `examples` role of `authored_from`, `applied_rules`, `channel`, `evidence`, `unclaimed` and `succeeds` were added this way; `docs/adr/0007-an-acceptance-keeps-what-each-requirement-rests-on.md`). Wire held to the library by `sce-build/tests/cli_acceptance_record_wire.rs` (spawns the binary) |
| W3C SCXML section mirror — the section set SCE's `§scxml-<id>` citations are checked against, each section carrying its normative excerpt, `coverage_expectation`, `decision_status` and its `implements` / `verifies` bindings | mnemosyne | `docs/spec/scxml/.atomic/workspace.atomic.json` | none — the store carries mnemosyne's own `schema_version`; declared to SCE by `[atomic].sidecar_path` in `docs/spec/scxml/mnemosyne.toml` | `registered` | mnemosyne (upstream tool); regenerated from the vendored snapshot by `tools/mnemosyne-adoption/scxml_toc_to_manifest.py`, read by `mnemosyne-cli` through `docs/spec/scxml/mnemosyne.toml` |
| W3C SCXML verify bindings — which test file and symbol verifies which section | mnemosyne | `docs/spec/scxml/.atomic/verifies-catalog.json` | none — the file declares `format: verifies-catalog/v1`; pinned by `[verifies_catalog].sha256` in `docs/spec/scxml/mnemosyne.toml` | `registered` | mnemosyne takes the neutral `verifies-catalog/v1` contract only; the records are generated from each test's W3C `metadata.txt specnum` by `tools/mnemosyne-adoption/gen_verifies_catalog.py` |
| W3C SCXML Recommendation, human-readable rendering — the content SSOT every stored `normative_excerpt` is hashed against | W3C | `docs/spec/scxml/.atomic/epub/scxml-REC-20150901.epub` | none — pinned by `[workspace.spec_source].epub_sha256` in `docs/spec/scxml/mnemosyne.toml`, re-hashed on every load | `registered` | W3C (`REC-scxml-20150901`); vendored, never edited here |
| Protocol-synthesis RFC section mirror — the `§synth-<id>` section set | mnemosyne | `docs/spec/synth/.atomic/workspace.atomic.json` | none — mnemosyne `schema_version`; declared by `[atomic].sidecar_path` in `docs/spec/synth/mnemosyne.toml` | `registered` | mnemosyne (upstream tool); regenerated from `docs/spec/synth/rfc-sce-protocol-synthesis.md` by `tools/mnemosyne-adoption/synth_rfc_to_manifest.py` |
| Mesh design-ledger section mirror — the `§mesh-<id>` section set | mnemosyne | `docs/sce-ledger/mesh/.atomic/workspace.atomic.json` | none — mnemosyne `schema_version`; declared by `[atomic].sidecar_path` in `docs/sce-ledger/mesh/mnemosyne.toml` | `registered` | mnemosyne (upstream tool); regenerated from `SCE_MESH.md` by `tools/mnemosyne-adoption/sce_mesh_md_to_manifest.py` |
| Mesh verify bindings — which test verifies which `§mesh-<id>` section | mnemosyne | `docs/sce-ledger/mesh/.atomic/verifies-catalog.json` | none — `format: verifies-catalog/v1`; declared by `[verifies_catalog].path` in `docs/sce-ledger/mesh/mnemosyne.toml` | `registered` | mnemosyne takes the `verifies-catalog/v1` contract only; records generated by `tools/mnemosyne-adoption/gen_mesh_verifies_catalog.py` |
| Wire RFC section mirror — the `§wire-W<id>` section set | mnemosyne | `docs/sce-ledger/wire/.atomic/workspace.atomic.json` | none — mnemosyne `schema_version`; declared by `[atomic].sidecar_path` in `docs/sce-ledger/wire/mnemosyne.toml` | `registered` | mnemosyne (upstream tool); regenerated from `docs/sce-ledger/wire/rfc-sce-diagnostic-wire-unification.md` by `tools/mnemosyne-adoption/sce_wire_rfc_to_manifest.py` |
| Bytes-guard RFC section mirror — the `§bytesguard-<id>` section set | mnemosyne | `docs/sce-ledger/bytesguard/.atomic/workspace.atomic.json` | none — mnemosyne `schema_version`; declared by `[atomic].sidecar_path` in `docs/sce-ledger/bytesguard/mnemosyne.toml` | `registered` | mnemosyne (upstream tool); regenerated from `docs/sce-ledger/bytesguard/rfc-eventschema-bytes-guard.md` by `tools/mnemosyne-adoption/bytesguard_rfc_to_manifest.py` |
| W3C statechart conformance registry — which upstream tests this repository runs and which harness each needs | SCE | `tests/w3c/conformance/fixtures.json` | none — carries `version`; `$schema` points at the sibling below | `registered` | `tests/w3c/conformance/fixtures.schema.json`; read by `sce-codegen list-fixtures` / `generate-w3c`, by `tests/CMakeLists.txt`, and by the visualizer's test list. Registration procedure: `CLAUDE.md`, "Adding W3C Tests". ⚠ It names no spec section — which section a fixture verifies is answered only by `docs/spec/scxml/.atomic/verifies-catalog.json`, derived from `resources/<id>/metadata.txt specnum`, and `W3cRegistry::load` refuses a `summary` that states one |
| Shape of the W3C conformance registry | SCE | `tests/w3c/conformance/fixtures.schema.json` | none — a draft-07 schema with no `x-sce-schema-status`; it describes an in-tree registry, not a wire payload | `registered` | itself; the registry it describes is the row above |
| Forge cross-language conformance registry — the fixture catalog every language arm runs | SCE | `tests/forge/conformance/fixtures.json` | none — carries `version`; ⚠ its `$schema` is editor-only, runtime validation is hand-rolled in `sce-build/src/conformance.rs` | `registered` | `tests/forge/conformance/fixtures.schema.json` (advisory) + `sce-build/src/conformance.rs` (enforcing) |
| Shape of the forge conformance registry | SCE | `tests/forge/conformance/fixtures.schema.json` | none — draft-07, advisory only per the row above | `registered` | itself; ⚠ not the enforcing authority — `sce-build/src/conformance.rs` is |
| Forge numerical reference — the values all five language arms must agree on | SCE | `tests/forge/conformance/numerical_reference.json` | none — carries `version` and `float_tolerance` | `registered` | itself (`description` / `expected_shape` fields); consumed by the per-language conformance arms |
| Queue contract scenarios — the single-threaded operation sequences, and the result each must give, that every language arm of `sce:kind="queue"` runs (SCE Protocol-Synthesis RFC §synth-5-P, verification layer 1) | SCE | `tests/forge/conformance/queue_contract.json` | none — carries `version`; its step grammar is stated in its own `description` | `registered` | itself (`description`); read by `backends/rust/forge-conformance/tests/forge_queue.rs`, which refuses by name a storage row it has no runtime for |
| Embedded-header symbol manifest — the C++ symbols the vendored embed surface exposes | SCE | `embed/MANIFEST.json` | none — declares `schema: sce-embed-manifest.v1` and stamps `embed_version` + `clang_version` | `registered` | `scripts/emit_embed_manifest.sh` (producer); the `embed-manifest-failfast` gate holds its fail-fast case |
| Workbench command replies — every reply, and every kind of refusal, of the command layer the workbench's screen, its browser shell and the authoring MCP's `sce-work` all reach the works folder through, produced by running the commands against a real folder with a fixed clock | SCE | `app-core/contract/replies.json` | none — carries `command_set_version`, which `COMMAND_SET_VERSION` (`app-core/src/commands.rs`) moves when a command's arguments or answer change in a way a caller written against the last one would misread | `registered` | `app-core/tests/contract.rs` (producer: fails when a reply differs from the file, and when a command has no reply in it; regenerated with `SCE_UPDATE_CONTRACT=1`) and `app/ui/test/contract.test.ts` (reader: holds the screen's guards to the file). The works folder those commands write is described in `app/README.md` |
| Vendored W3C SCXML Recommendation snapshot — the offline input the section mirror is built from | W3C | `tools/mnemosyne-adoption/spec-snapshot/scxml-REC-20150901.html` | none — hashed by the `fetched_sha256` in the provenance row below | `registered` | W3C (`REC-scxml-20150901`); vendored so the manifest build is offline and deterministic, never edited here |
| Provenance of that snapshot — url, revision, hash and fetch date | SCE | `tools/mnemosyne-adoption/spec-snapshot/PROVENANCE.json` | none — a bare record; mirrored into `[workspace.spec_source]` in `docs/spec/scxml/mnemosyne.toml` | `registered` | `tools/mnemosyne-adoption/check_spec_drift.py`, which re-fetches the URL and compares; `scripts/gates/spec-snapshot.sh` is the lane |

The `Current status` column carries one of two vocabularies, and the
`Format owner` column says which applies:

- **`pre-release`** / **`stable`** — a wire stability promise, pinned by
  a machine check to both the producer-side constant and the schema-file
  header (policy item 4 below). Every wire surface is currently
  `pre-release`; SCE has not yet made a stability promise on any of them.
- **`registered`** — existence and content are on record here, and a gate
  holds this table to the tree. SCE makes **no** cross-version stability
  promise and there is no SCE status constant, because SCE either does
  not own the format or does not publish the artifact to an external
  consumer. `registered` is not a weaker `pre-release`; it is a different
  claim, and a surface does not graduate from one to the other.

### What is a row, and what is not

A row is a **checked-in artifact** that something outside the code
writing it reads as authority. Each kind has its own derivation from the
tree, so the table cannot fall behind what is on disk:

- Wire surfaces — `sce-build/tests/wire_surface_stability.rs` keys the
  registry by schema filename and walks `schemas/`/`apis/` in reverse, so
  a schema cannot land without a row. That is also why something with no
  schema file cannot acquire one.
- Spec-bearing surfaces — `sce-build/tests/spec_surface_registration.rs`
  derives the population from the tree by shape: a `.atomic/` store
  directory, a `conformance/` registry, a `spec-snapshot/` vendored
  upstream, or a JSON whose name puts it in the catalog / reference /
  manifest / provenance / verification family. Any derived path this
  table does not name fails the gate. The population is **derived rather
  than listed** because a hand-written list is how the defect this
  section exists for is produced — measured the same day in a sibling
  gate, a hand-listed set covered 6 of 16 trees and stayed green while
  misclassifying 2000+ files.

So a **build-time refusal is not a surface here**, however contractual it is.
`<invoke type="sce:mesh-rpc">` on a backend with no mesh arm is the worked
example: it is a promise about what SCE will not silently accept, it has no
serialization, no status constant and no instances to validate, and a row for
it would be unkeyable by the very test that guards this table. Its contract
lives where the refusal is derived from — `SCE_MESH.md` §9.5's
backend-coverage table, derived from `tools/codegen/templates/mesh/<dir>/` and
held to the template tree and the CLI by
`sce-build/tests/mesh_rpc_backend_contract.rs`.

The general rule: if a claim has no artifact on the wire, its home is the
document that owns the behaviour, with a test binding it to the tree. This
note exists because the question was asked and answered once already, and the
answer was reached by reading `wire_surface_stability.rs` rather than by
anything written down.

## Where to read the status

Wire surfaces only; a spec-bearing surface has no SCE status header, and
`Spec-bearing surfaces` below says what stands in its place.

The status signal lives in the **schema file header**, never in the
emitted payload:

- JSON schemas (`diagnostic`, `forge-ast`, `sourcemap`): the top-level
  `x-sce-schema-status` field.
- XSD schemas (`sce-forge`, `sce-forge-ext`): the `<xs:documentation>`
  line `x-sce-schema-status: <status>` as the first annotation under
  `<xs:schema>`.

Keeping the status out of the payload means the emitted artifacts
(including the 404 committed `sce_sourcemap.json` sidecars) stay
byte-stable, and a consumer reads the stability signal from the
checked-in schema without linking the `sce-build` crate.

## Policy while `pre-release`

This section governs **wire** surfaces — the rows whose `Format owner`
is SCE and whose status is `pre-release` or `stable`. A spec-bearing
surface carries `registered` and is governed by `Spec-bearing surfaces`
below instead; reading these five items onto one would claim SCE
promises it does not make and enforcement that does not exist.

1. **No cross-version stability guarantee.** While a surface is
   `pre-release`, a non-additive shape change MAY land without a
   version bump. **Consumers MUST pin to a specific SCE commit** rather
   than rely on a version number alone. (This is why, for example, a
   forge-AST consumer pins the exact commit it deserializes against.)
2. **A payload names the commit that produced it.** A MUST the consumer
   cannot check is a convention, so every surface a consumer reads
   without a second invocation carries the generator commit in-band
   under one name — `generator`, matching
   `^([0-9a-f]{7,40}|unknown)$` — on the stdout manifest, on every
   diagnostic record, on every symbol-lookup record, and on every
   forge-AST export. One name across surfaces means a consumer reading
   two of them from the same run does not need a per-surface key to ask
   the same question. The diagnostic case is the one that has to be
   per-record — a rejected run writes **no** manifest at all (stdout is
   empty, the exit code carries the failure), so on the path a repair
   loop iterates on, the diagnostic is the only record the consumer
   receives.

   `wire_surface_stability.rs::every_json_surface_names_the_commit_that_produced_it`
   enforces this per surface, including the pattern: a required stamp
   whose shape is unconstrained is discharged by a value that names
   nothing. The forge-AST export is why the check exists — it reached
   `pre-release` carrying an optional `sce_producer_version` holding
   the `sce-build` crate version, which is frozen pre-1.0 and so
   identified nothing, while the status, instance and negative checks
   all stayed green because none of them looks at attribution.

   The sourcemap sidecar deliberately does **not** carry it. The sidecar
   is a committed artifact, so a commit stamp would be invalidated by
   the very commit that wrote it, and every commit touching any tree
   would have to regenerate all of them; its `source_hash` identifies the
   inputs instead, per `build.rs` on why a stamp that goes stale is worse
   than no stamp. A consumer needing the
   emitting commit for a sidecar reads it from the manifest of the run
   that produced it, or from a lookup record naming that sidecar. That
   exemption is registered in the same test (`ATTRIBUTION_EXEMPT`, with
   its reason) and checked both ways: the sidecar must not carry the
   field, and this registry must state why.

   The saved state deliberately does **not** carry it either. It is
   written by generated code, which would have to embed the commit, so
   every committed machine and every shared saved-state fixture would
   change on every commit. What a saved state must identify is what it
   restores into, and its `shape` does that — a digest of the states and
   typed variables it names, which a machine checks before it reads
   anything else; the commit that built the machine is the build's to
   record (the `generate` manifest names it). Registered the same way.

   The authoring profile deliberately does **not** carry it. It is written
   by the specification's owner and never emitted by SCE, so there is no
   run to stamp, and a commit the owner never chose would sit in a file
   whose bytes are what an acceptance record pins — every regeneration
   would change the digest that names the profile. What identifies a
   profile is its own sha256, which the manifest's `profile` and the
   acceptance record both carry. Registered the same way.

   The scenario set deliberately does **not** carry it. It is written by
   the specification's owner, or proposed by an AI client for the owner to
   confirm, and never emitted by SCE, so there is no run to stamp, and a
   commit nobody chose would sit in a file whose bytes an acceptance can
   pin. What identifies a scenario set is its own sha256, and the
   specification it quotes is named by `specification.doc_id` and `rev`.
   Registered the same way.

   The observation trace deliberately does **not** carry it either. It is
   written by an engine driver, a program separate from SCE, and never
   emitted by SCE itself, so there is no SCE run to stamp; the producer a
   consumer needs to know is the driver, which the trace names in
   `engine`. What ties a trace to what it was taken against is the digest
   of the scenario set in `scenario_set.sha256`. Registered the same way.
3. **Additive growth is compatible.** Adding a new optional field is
   compatible within the current version and does NOT bump it.
   Consumers MUST ignore unknown fields.
4. **Each status claim is machine-checked.** A drift guard test pins
   the producer-side constant to the schema-file header for every
   surface, so the table above cannot silently go stale:
   - `schema_file_declares_status` in `diagnostic.rs`
   - `ast_export` / `forge_ast_export` schema-header test
   - `sourcemap.rs::tests::schema_file_declares_status`
   - `manifest.rs::tests::schema_file_declares_status`
   - `sourcemap.rs::tests::symbol_lookup_schema_file_declares_status`
   - `authoring_profile.rs::tests::schema_file_declares_status`
   - `scenario_set.rs::tests::schema_file_declares_status`
   - `scenario_judge.rs::tests::schema_file_declares_status`
   - `saved_state.rs::tests::schema_file_declares_status` (Rust runtime)
     and `StaticDatamodelTest.theSavedStateSchemaFileDeclaresTheStatusThisRuntimeDoes`
     (Kotlin) — one per producer
   - `sce-build/tests/wire_surface_stability.rs` (cross-surface: every
     surface declares a valid status, this registry lists every
     surface, and — walking the other way — every schema checked into
     `schemas/` or `apis/` is a declared surface, so a schema cannot
     land on disk and stay unregistered)
5. **Each shape claim is checked against real instances.** A schema
   nothing is validated against is a document, not a contract: the
   drift guards in item 4 compare a constant to a header and never put
   a produced artifact through the schema. For every JSON surface a
   test runs emitted artifacts through a draft-07 validator, and a
   negative case pins that the validator rejects — a positive sweep
   alone proves only that everything is accepted. The table lives in
   `wire_surface_stability.rs::INSTANCE_VALIDATION`, which fails if a
   surface has no row, if a named test no longer exists, or if this
   list stops naming it:
   - Diagnostics — `every_golden_record_validates_against_the_wire_schema`
     (the golden table, which `every_code_has_a_golden` proves reaches
     every `DiagnosticCode`) and
     `every_cli_diagnostic_in_the_fixture_corpus_validates_against_the_schema`
     (the CLI's own stderr, over the fixture corpus in every backend).
     The two record sets are disjoint: goldens are hand-authored
     instances, the corpus produces different ones.
   - Forge AST — `round_trip_every_kind`
   - Sourcemap sidecar — `committed_sourcemaps_validate_against_the_wire_schema`
     (every committed sidecar; paired with the regeneration gate in the
     same file, this also covers what the generator emits today)
   - Stdout manifest — `generate_manifest_instance_validates_against_schema`,
     `check_manifest_validates_against_the_wire_schema` and
     `orchestrate_manifest_names_exactly_the_files_it_wrote` (which also
     pins the record against a walk of the directory it describes)
   - Symbol lookup — `both_lookup_directions_validate_against_the_wire_schema`
   - Saved state — `every_shared_saved_state_fixture_is_a_saved_state`
     (the shared fixtures, which the Rust and Kotlin static suites each
     save byte for byte and restore from)
   - Kind catalog — `the_catalog_validates_against_the_wire_schema`
     (the whole catalog as the producer builds it; every embedded
     example is separately put through `check --lint` and `pseudo` by
     `sce-build/tests/every_kind_example_is_accepted.rs`)
   - Authoring profile — `every_profile_the_product_reads_validates_against_the_wire_schema`
     (every profile shape the reader takes; it is written by an owner, so
     there is no producer to sweep, and the reader is what is held to the
     schema)
   - Scenario set — `every_set_the_product_reads_validates_against_the_wire_schema`
     (the four sets shipped as fixtures, each also read, quoted word for
     word against its specification and counted; written by an owner or
     an AI client, so the reader is what is held to the schema)
   - Observation trace — `every_trace_the_product_reads_validates_against_the_wire_schema`
     (the trace of a correct retry machine, written by hand before any
     driver existed, judged `pass` on all six scenarios of its set; a driver
     is a separate program, so the reader is what is held to the schema)

   The negative half is enforced separately, by
   `wire_surface_stability.rs::NEGATIVE_VALIDATION` and
   `every_json_surface_has_a_negative_validation_test`. Listing only the
   positives left the requirement unenforced, and the symbol-lookup
   surface reached `pre-release` with no negative case at all while its
   row above looked complete. Each of these starts from a record it
   first asserts is valid and changes exactly one thing, so the refusal
   is pinned to that change:
   - Diagnostics — `diagnostic_schema_rejects_a_missing_required_field`
   - Forge AST — `ast_schema_rejects_an_envelope_missing_a_required_field`
     and `ast_schema_rejects_a_generator_that_names_no_commit` (which
     pins both ways the stamp can fail: absent, and present-but-naming-
     nothing — the crate version this surface used to emit)
   - Sourcemap sidecar — `sourcemap_schema_rejects_a_missing_required_field`
   - Stdout manifest — `schema_rejects_a_missing_required_field`
   - Symbol lookup — `lookup_schema_rejects_a_record_without_the_generator_stamp`
   - Saved state — `the_schema_refuses_what_no_backend_writes`
   - Kind catalog — `the_catalog_schema_rejects_a_kind_the_product_does_not_have`
   - Authoring profile — `the_profile_schema_rejects_a_setting_the_product_does_not_know`
   - Scenario set — `the_wire_schema_rejects_what_the_reader_does_not_accept`
     (twenty changes to a set it first shows valid, each of which the
     reader must also refuse or find a problem with)
   - Observation trace — the same name in `scenario_judge.rs`
     (fifteen changes to a trace it first shows valid; the reader refuses
     the file, or the judge reports a problem with it)

   The authoring grammar is in neither table because it is not validated
   by a test: `forge::xsd_validator` validates input documents against
   `sce-forge.xsd` on the production path, before codegen, which is the
   stronger property where it holds.

   Stated as the producer states it, because this paragraph used to
   claim **every input document** and that is a promise the code does
   not make. `validate_or_skip` spells the guarantee as *"if a schema is
   available, validation runs"*, not *"every invocation validates"*, and
   it has two non-validating outcomes it returns rather than swallows:
   `NotValidated(SchemaNotFound)` when `schemas/` is absent — a
   downstream crate vendoring `sce-build` without it must still build —
   and `NotValidated(FeatureDisabled)` on a build without the `xsd`
   feature, which is how a `wasm32` target avoids reporting the same
   success as a validated one. `warn_if_not_validated` is what makes
   either case audible.

   The distinction is not pedantry here: it is the same axis the C++
   Interpreter sits on. That engine has no XSD stage at all, so an
   unconditional reading of this paragraph would suggest the two engines
   accept the same documents when they do not.

## Spec-bearing surfaces

### What registration means

Registering one of these says two things and no more: **it exists**, and
**this is what it holds**. It does not move the format into SCE, does
not make SCE its producer, and does not add a stability promise. The
distinction is load-bearing for the mnemosyne rows: that format belongs
to an external tool, SCE has no code dependency on it, and each store is
reached only through the `mnemosyne.toml` that declares it. Editing a
store's shape from this side would be SCE appropriating a format it does
not own — the `Format owner` column exists so a reader cannot make that
mistake from the table alone.

For the same reason a spec-bearing row's `Status source` cell reads
`none`. There is no producer-side constant to pin a header to, so what
stands in for it is the declaration path: which config file names this
artifact, and which hash pins it where one does. Those are what the
second derivation below walks.

### How the population is derived

`sce-build/tests/spec_surface_registration.rs` answers *is there a
surface in the tree this table does not name* with two independent
derivations that must agree:

1. **Shape walk** over `git ls-files` — the tree's own answer, not a
   list in the test. Four rules select candidates, and each is asserted
   non-vacuous, so a rule that stops matching is a failure rather than a
   quiet narrowing.
2. **Config walk** over every tracked `mnemosyne.toml` — each artifact
   path a workspace declares (`[atomic].sidecar_path`,
   `[verifies_catalog].path`, `[workspace.spec_source].epub_path`) must
   be both derived by the shape walk and named here. This is what keeps
   rule 1 from certifying itself: if a workspace ever moves its store out
   of a `.atomic/` directory, the shape rule silently stops seeing it and
   this walk fails.

The gate prints how many surfaces it checked and asserts a floor, so a
walk that finds nothing fails instead of passing vacuously. It also
refuses an exclusion that swallows more than it leaves, and carries a
case that **fabricates** an unregistered surface and asserts the refusal
fires — a guard whose failure path is never exercised is a guard nobody
has seen work.

### The one exclusion, and what it costs

A candidate under a `fixtures/` **directory** is a test input, not a
surface: it is one member of a corpus some test owns, where a surface
names a population. The discriminator is the directory segment, not the
filename — `tests/w3c/conformance/fixtures.json` is a registry and is
registered above, while
`sce-build/tests/fixtures/requirement_closure/*.manifest.json` is an
input to one test and is not.

What that costs, stated rather than hidden: a genuine surface parked
under a `fixtures/` directory would be excluded and nothing would notice.
The mitigation is the ratio assertion above rather than a promise, and
the right repair if it ever happens is to move the surface, not to widen
the rule.

### Adding one

Add the row to the table above — do **not** start a second table or a
second document. The registry says of itself that it is the single one,
and a parallel register is precisely the defect the gate exists to
prevent. If the artifact's shape is not one the four rules select, add
the rule as well, so the next artifact of that shape is caught rather
than remembered.

## Flipping a surface to `stable`

Wire surfaces only — `registered` is not a rung on this ladder, and a
spec-bearing surface reaches `pre-release` only by SCE taking ownership
of its format and its producer, which is a different decision from
deciding a shape has settled.

The flip is a deliberate editorial act, not an automated threshold. A
maintainer decides a surface has settled — e.g. an external consumer has
committed to the format, or it has been churn-free long enough — and
lands a **single commit** that updates all three of:

1. the producer-side status constant,
2. the schema-file `x-sce-schema-status` header, and
3. this registry's status column for that surface.

Once `stable`, the surface's own evolution rules apply strictly: a
non-additive change requires a new versioned schema (`v2`) coexisting
with the prior version for at least one minor-release cycle. See each
surface's shape-governance doc for the exact rule.

## Announcing changes

- Every surface a consumer reads without a second invocation stamps the
  producing commit in-band (policy item 2), so an issue report can name
  the exact generator that produced the payload it quotes — including
  the failure path, where there is no manifest to correlate against.
- Surface changes are tracked in git history against the schema file
  and this registry. There is no separate changelog feed while
  `pre-release`; the commit-pin discipline in policy items 1-2 is the
  contract.
- A spec-bearing surface announces nothing to an external consumer,
  because it has none: it changes when the document or the test tree it
  mirrors changes, and its regeneration tool and its gate — both named
  in its `Shape governance` cell — are what hold it to that source. The
  announcement a reader of this registry needs is the row itself.
