# SCE Wire Contracts — Stability & Deprecation Policy

This document is the single registry for SCE's **agent-facing wire
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

All four surfaces are currently **`pre-release`**. SCE has not yet made
a stability promise on any of them.

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
2. **Additive growth is compatible.** Adding a new optional field is
   compatible within the current version and does NOT bump it.
   Consumers MUST ignore unknown fields.
3. **Each status claim is machine-checked.** A drift guard test pins
   the producer-side constant to the schema-file header for every
   surface, so the table above cannot silently go stale:
   - `diagnostic.rs::tests::schema_file_declares_status`
   - `ast_export` / `forge_ast_export` schema-header test
   - `sourcemap.rs::tests::schema_file_declares_status`
   - `sce-build/tests/wire_surface_stability.rs` (cross-surface: every
     surface declares a valid status, and this registry lists every
     surface)

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

- Where a surface carries a producer-version stamp (e.g. forge-AST's
  optional `sce_producer_version`), a consumer reporting an issue can
  pin the exact release that produced a payload.
- Surface changes are tracked in git history against the schema file
  and this registry. There is no separate changelog feed while
  `pre-release`; the commit-pin discipline in policy item 1 is the
  contract.
