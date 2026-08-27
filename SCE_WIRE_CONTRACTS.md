# SCE Wire Contracts — Stability & Deprecation Policy

This document is the single registry for SCE's **wire
surfaces**: the checked-in schemas and emitted artifacts that an
external consumer builds tooling against (a code generator that reads
SCE's output, a validator that lints Extended SCXML before codegen, a
traceability tool that maps generated symbols back to source).

It exists so a downstream consumer can answer one question without
reading the engine source: **"How stable is this surface, and how will
I learn when it changes?"**

This registry covers only the *stability contract*. The *shape* of each
surface is defined by its own schema file and per-surface governance
doc (linked below).

## Surfaces

| Surface | Schema / artifact | Status source (producer ↔ schema header) | Current status | Shape governance |
|---|---|---|---|---|
| Diagnostics (`--error-format=json`) | `schemas/sce-diagnostic.v1.schema.json` | `SCHEMA_STATUS` (`sce-build/src/forge/diagnostic.rs`) ↔ `x-sce-schema-status` | `pre-release` | `SCE_ERROR_CONTRACT.md` §8 |
| Forge AST export (`--emit-ast`) | `apis/forge-ast.v1.schema.json` | `FORGE_AST_SCHEMA_STATUS` (`sce-build/src/forge/ast_export.rs`) ↔ `x-sce-schema-status` | `pre-release` | `docs/SCE_FORGE_AST.md` §3 |
| Sourcemap sidecar (`out/{lang}/sce_sourcemap.json`) | `schemas/sce-sourcemap.v1.schema.json` | `SOURCEMAP_SCHEMA_STATUS` (`sce-build/src/forge/sourcemap.rs`) ↔ `x-sce-schema-status` | `pre-release` | This doc + `sce-build/src/forge/sourcemap.rs` (producer); reverse-lookup via `sce-codegen addr2sce` |
| Authoring grammar (Extended SCXML) | `schemas/sce-forge.xsd`, `schemas/sce-forge-ext.xsd` | `<xs:documentation>x-sce-schema-status: …</xs:documentation>` (first child of `<xs:schema>`) | `pre-release` | `docs/SCE_ACCEPTED_SUBSET.md` |
| Stdout manifest (`generate`, `check`, `orchestrate`) | `schemas/sce-manifest.v1.schema.json` | `MANIFEST_SCHEMA_STATUS` (`sce-build/src/manifest.rs`) ↔ `x-sce-schema-status` | `pre-release` | `SCE_ERROR_CONTRACT.md` §10 |
| Symbol lookup (`addr2sce`, `sce2sym`) | `schemas/sce-symbol-lookup.v1.schema.json` | `SYMBOL_LOOKUP_SCHEMA_STATUS` (`sce-build/src/forge/sourcemap.rs`) ↔ `x-sce-schema-status` | `pre-release` | This doc + `sce-build/src/forge/sourcemap.rs` (producer) |

All six surfaces are currently **`pre-release`**. SCE has not yet made
a stability promise on any of them.

### What is not a row here

A row in this table is a **serialized artifact**: a schema file checked into
`schemas/` or `apis/`, carrying an `x-sce-schema-status` header, matched by a
producer-side status constant, and validated against real instances.
`sce-build/tests/wire_surface_stability.rs` keys the registry by that
filename and walks `schemas/`/`apis/` in reverse so a schema cannot land
without a row — which is also why something with no schema file cannot
acquire one.

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
   would have to regenerate all of them; its `source_hash` /
   `template_hash` identify the inputs instead, per `build.rs` on why a
   stamp that goes stale is worse than no stamp. A consumer needing the
   emitting commit for a sidecar reads it from the manifest of the run
   that produced it, or from a lookup record naming that sidecar. That
   exemption is registered in the same test (`ATTRIBUTION_EXEMPT`, with
   its reason) and checked both ways: the sidecar must not carry the
   field, and this registry must state why.
3. **Additive growth is compatible.** Adding a new optional field is
   compatible within the current version and does NOT bump it.
   Consumers MUST ignore unknown fields.
4. **Each status claim is machine-checked.** A drift guard test pins
   the producer-side constant to the schema-file header for every
   surface, so the table above cannot silently go stale:
   - `diagnostic.rs::tests::schema_file_declares_status`
   - `ast_export` / `forge_ast_export` schema-header test
   - `sourcemap.rs::tests::schema_file_declares_status`
   - `manifest.rs::tests::schema_file_declares_status`
   - `sourcemap.rs::tests::symbol_lookup_schema_file_declares_status`
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

## Flipping a surface to `stable`

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
