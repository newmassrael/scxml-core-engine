# Mnemosyne adoption tooling

Glue for adopting [Mnemosyne](https://github.com/newmassrael/mnemosyne) as the
ledger for SCE's W3C SCXML citations (the `§scxml-...` anchors used across the
codebase and comments). This directory is **adoption tooling, not part of the
SCE engine**, and is deliberately kept out of `src/` and the engine crates.

## Why this lives here (and not in an engine crate)

SCE's engine is deterministic and offline: `cargo build` / `ctest` must produce
the same result without touching the network. The tools here either (a) parse a
**vendored** spec snapshot (fully deterministic), or (b) run only inside a
**separate CI drift-check job** that must never gate the normal build/test.

The placement rule we follow: the `deterministic-only` boundary is about the
*engine/build/test reproducibility* and about keeping LLM/AI code out of tree —
not about forbidding a non-AI, self-contained spec-fetch utility. So these live
in `tools/mnemosyne-adoption/`, physically separated from the engine, with the
network-touching step isolated to a dedicated CI job.

## Contents

| Path | Role |
|------|------|
| `spec-snapshot/scxml-REC-20150901.html` | Vendored W3C SCXML Recommendation snapshot (offline input) |
| `spec-snapshot/PROVENANCE.json` | URL + revision + `fetched_sha256` + date for the snapshot |
| `scxml_toc_to_manifest.py` | **A1**: vendored spec HTML -> Mnemosyne bulk-section-create manifest + anchor map |
| `check_spec_drift.py` | **B1**: snapshot integrity (offline) + upstream re-fetch (online) drift check |
| `scxml_extract_excerpts.py` | **R2**: vendored spec HTML -> per-section normative excerpt JSON |
| `apply_excerpts.py` | **R2** driver: feed excerpts JSON into a workspace via `set-section-normative-excerpt` |
| `migrate_citations.py` | **R3**: rewrite prose `W3C SCXML <n>.<m>` citations in source comments to the `§scxml-<id>` form |

## A1 — TOC to manifest

```bash
python3 tools/mnemosyne-adoption/scxml_toc_to_manifest.py \
    --manifest out/scxml-manifest.json \
    --anchor-map out/scxml-anchor-map.json
```

Standard library only (`html.parser.HTMLParser` for parsing); deterministic
(same vendored HTML in -> byte-identical JSON out). Emits two artifacts (both
regenerable, neither committed):

- **manifest** — `[{section_id, parent_doc, title, parent_section?}]`, the input
  shape for Mnemosyne's future bulk-section-create primitive (A2). Skeleton
  only: `normative_excerpt` is added per section later, using the anchor map.
- **anchor map** — `{section_id: {anchor_url, source_revision}}`, preserving the
  spec `#anchor` (lost from the section id) for the excerpt-assembly step.

### Section-id naming policy

This is SCE's own policy (its SSOT is the converter module). Ids in the manifest
are **bare** (no § sigil) — `import-sections` stores `section_id` literally, so a
§ would double on render (`§§scxml-1`). The § is the *citation* form used in code
and in the rendered heading.

| Spec heading | stored id (manifest) | cited as | Rule |
|--------------|----------------------|----------|------|
| `5.10 System Variables` | `scxml-5.10` | `§scxml-5.10` | numeric: keep dots |
| `6.2.6 Message Content` | `scxml-6.2.6` | `§scxml-6.2.6` | numeric |
| `A.1 Conforming Documents` | `scxml-A-1` | `§scxml-A-1` | lettered: dots -> hyphens |
| `B.2.11 <foreach>` | `scxml-B-2-11` | `§scxml-B-2-11` | lettered |
| `D Algorithm ...` | `scxml-D` | `§scxml-D` | appendix root |
| `procedure interpret(...)` (`#interpret`) | `scxml-D-interpret` | `§scxml-D-interpret` | unnumbered appendix-D helper -> letter + spec anchor |

The policy is designed so every id is citation-safe under Mnemosyne's code
citation extractor (which keeps a `.` in a citation token only when flanked by
ASCII digits, so `§39.implementations` splits as section `39` + prose suffix).
That grammar is **owned by Mnemosyne, not mirrored here**. Rather than re-encode
the rule (a second source of truth that could drift), the policy's compatibility
is proven by the closed-loop integration test, which feeds representative ids
through the real `mnemosyne-cli validate-code-refs`.

### Tests

```bash
python3 -m unittest discover -s tools/mnemosyne-adoption/tests
```

Unit tests cover the naming policy, parenting, title extraction, and a
self-consistency invariant over the full snapshot, with no Mnemosyne dependency.
The closed-loop test (skipped when `mnemosyne-cli` is not on PATH) delegates
citation-safety to Mnemosyne's extractor.

## B1 — snapshot drift (`.github/workflows/spec-snapshot-drift.yml`)

`check_spec_drift.py --mode integrity` (offline) checks the snapshot's sha256
against `PROVENANCE.fetched_sha256` and runs as a push/PR gate. `--mode upstream`
(online) re-fetches the URL and compares; it runs only on schedule / manual
dispatch, so an upstream change or network failure never breaks the engine
build. The `fetched_sha256` format (`^[0-9a-f]{64}$`) is the value Mnemosyne's
`B2` rev-diff scan consumes.

## R2 — normative excerpts

```bash
# extract one excerpt per section (vendored HTML -> JSON), then apply to a workspace
python3 tools/mnemosyne-adoption/scxml_extract_excerpts.py --out out/scxml-excerpts.json
cd docs/spec/scxml
python3 ../../../tools/mnemosyne-adoption/apply_excerpts.py --excerpts out/scxml-excerpts.json
```

A section's excerpt is its **direct body text** (between its heading and the
next heading of any level); container sections with no direct prose are omitted.
The extractor imports A1 so the section-id mapping stays single-source. The apply
driver calls `set-section-normative-excerpt` per section (`--no-regenerate`, one
final render); `normative_excerpt` is frozen after first set, so it runs once on
the skeleton. For the vendored snapshot this yields 191 excerpts over the 196
sections. The result is the vendored quote Mnemosyne renders (B3 read-path) and
anchors for drift (B2).

## R3 — migrate code citations to `§scxml-<id>`

Once the ledger exists (A1 skeleton + R2 excerpts), SCE's code still cites the
spec in prose form (`// W3C SCXML 6.2: ...`). R3 migrates that prose to the
`§scxml-<id>` form so Mnemosyne's `set_equality_validator` (validate-code-refs)
can check every citation against the ledger and reject hallucinated ones.

```bash
# dry-run: print the plan (default), then apply
python3 tools/mnemosyne-adoption/migrate_citations.py sce/include/events sce/src/events
python3 tools/mnemosyne-adoption/migrate_citations.py sce/include/events sce/src/events --apply
```

This is **not** a `sed`. Three rules keep it safe:

- **Only real section labels migrate.** Numeric (`6.2`, `5.10.1`) and lettered
  appendix (`C.2`, `B.2`) labels are rewritten; word-led mentions
  (`W3C SCXML BasicHTTPEventProcessor`, `... specification`) are prose and left
  alone. Normalization reuses A1's `label_to_leaf` (the SSOT), so numeric keeps
  dots and lettered turns dots into hyphens.
- **Only ledger-present ids migrate.** A label whose normalized id is not a
  section — the spec *version* `1.0`, or a typo'd/hallucinated number — is left
  as prose and **reported**. That report is the human-review surface for the
  cutover: versions stay prose, genuine citation errors get fixed at the source.
- **Only comments are edited.** A per-file comment scanner (`//`, `/* */`, and
  Rust nested block comments) means string- and char-literal text is never
  touched, so no runtime string can change.

A slash chain (`W3C SCXML 3.8/3.9`, meaning sections 3.8 and 3.9) becomes
`§scxml-3.8 / §scxml-3.9` — each member rewritten and rejoined with `" / "` so
the extractor sees two distinct ids. The whole chain stays prose if any member
is absent from the ledger. `W3C SCXML I/O` is never matched (`I` is not a
section label).

Note on namespaces: SCE code also cites its own design specs
(`RFC §W5`, `SCE_MESH.md §9.6`). Those are a different namespace and are **not**
migrated by this tool — `validate-code-refs` checks every `§id` against the one
scxml ledger, so a module mixing W3C and SCE-internal cites cannot be gated by
this workspace. `paths` lists only citation-clean modules; mixed modules wait
for a dedicated SCE design-ledger workspace.

The rollout is **one directory at a time**: migrate a directory, add it to
`paths` in `docs/spec/scxml/mnemosyne.toml`'s `[plugins.set_equality_validator]`,
and confirm `validate-code-refs` is green. `severity_missing = "reject"` is the
hallucination gate (strict from the start — the migrator only emits ledger ids);
`severity_binding = "warn"` keeps the Path B Spec↔Code binding axis advisory
until a later round registers `Section.implementations`. The closed-loop test
(skipped without `mnemosyne-cli`) proves migrate -> validate is green and that a
hallucinated `§scxml-9.99` fails the reject gate.
