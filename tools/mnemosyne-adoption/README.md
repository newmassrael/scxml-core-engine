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
| `scxml_toc_to_manifest.py` | **A1**: vendored spec HTML -> Mnemosyne bulk-section-create manifest + anchor map (h2..h6) |
| `check_spec_drift.py` | **B1**: snapshot integrity (offline) + upstream re-fetch (online) drift check |
| `migrate_citations.py` | **R3**: rewrite prose `W3C SCXML <n>.<m>` citations in source comments to the `§scxml-<id>` form |
| `sce_mesh_md_to_manifest.py` | **C**: SCE_MESH.md markdown headings -> Mnemosyne manifest for the `mesh` design-ledger namespace |
| `sce_wire_rfc_to_manifest.py` | **C**: Wire RFC milestone waves (W0..W5) -> Mnemosyne manifest for the `wire` design-ledger namespace |

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
  shape for `import-sections`. Skeleton only: `normative_excerpt` is projected
  from the EPUB later (see "EPUB-as-content-SSOT" below).
- **anchor map** — `{section_id: {anchor_url, source_revision}}`, preserving the
  spec `#anchor` (lost from the section id) and feeding medium-forge as the
  `id -> anchor` map for EPUB section location + text extraction.

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

## EPUB-as-content-SSOT — normative excerpts (Mnemosyne R401-R407)

The per-section `normative_excerpt.text` is a **derived cache projected from a
committed EPUB**, not hand-authored. The EPUB is the human-readable rendering of
the spec revision and the provenance anchor; `text_sha256` lets the store
re-validate every excerpt offline. SCE owns only the section-id naming (A1); the
generic HTML->EPUB conversion is Mnemosyne's general-purpose `medium-forge`
(spec-agnostic, **not vendored here** — used at author-time from the Mnemosyne
checkout). The old SCE-specific extractor + `set-section-normative-excerpt` apply
driver were retired once `medium-forge --text-scope heading` (R407) generalized
the same heading-delimited extraction.

```bash
# 1. anchor map (id -> anchor_url) from the SCE-specific A1 converter
python3 tools/mnemosyne-adoption/scxml_toc_to_manifest.py \
    --anchor-map out/scxml-anchor-map.json

# 2. EPUB + v2 anchor map (per-section text + text_sha256) from medium-forge.
#    --text-scope heading = each section's direct body up to the next heading
#    (h1..h6), heading excluded — matches the SCE section granularity so sub-div
#    ids (e.g. the Appendix-D h4 helpers, the B.2.8.1 h5) do not collapse.
python3 <mnemosyne>/tools/medium-forge/convert.py \
    --html tools/mnemosyne-adoption/spec-snapshot/scxml-REC-20150901.html \
    --anchor-map out/scxml-anchor-map.json --out out/ \
    --content-xpath "//div[@class='div1']" --text-scope heading \
    --revision REC-scxml-20150901 --source-url https://www.w3.org/TR/scxml/ \
    --title SCXML

# 3. project text + text_sha256 into the store (preserves authored
#    anchor_url + source_revision); then pin the EPUB and gate offline drift
cd docs/spec/scxml
mnemosyne-cli import-epub-excerpts --anchors ../../../out/anchors.json
mnemosyne-cli validate-content-drift          # re-hashes every excerpt + the pinned EPUB
```

The committed EPUB is pinned in `mnemosyne.toml` (`[workspace.spec_source]`
`epub_path` + `epub_sha256`, `[content_drift] severity = "reject"`). For the
vendored snapshot this yields 192 excerpts over 197 sections (5 pure-container
headings carry no direct body). A new section whose id appears in the anchor map
but not yet in the store is created first via `import-sections` (the manifest
entry carries the full `normative_excerpt` so `sha256(text) == text_sha256` is
verified at import).

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
(`§W5` Wire RFC, `§16.7` SCE Mesh). Those are different namespaces. Each
workspace declares `section_namespace = "<ns>"`, so `validate-code-refs` checks
only that workspace's own `§<ns>-...` cites and skips foreign ones by the token
itself (Mnemosyne R376) — which lets a module mixing `§scxml-` and `§mesh-` cites
be gated by both ledgers. The design ledgers below own the `mesh` (SCE_MESH.md)
and `wire` (Wire RFC) namespaces.

The rollout is **one directory at a time**: migrate a directory, add it to
`paths` in `docs/spec/scxml/mnemosyne.toml`'s `[plugins.set_equality_validator]`,
and confirm `validate-code-refs` is green. `severity_missing = "reject"` is the
hallucination gate (strict from the start — the migrator only emits ledger ids);
`severity_binding = "warn"` keeps the Path B Spec↔Code binding axis advisory
until a later round registers `Section.implementations`. The closed-loop test
(skipped without `mnemosyne-cli`) proves migrate -> validate is green and that a
hallucinated `§scxml-9.99` fails the reject gate.

## C — SCE Mesh design-ledger workspace (`mesh` namespace)

`docs/spec/scxml` mirrors an **external** standard (the W3C SCXML Recommendation).
SCE code also cites SCE's **own** design document, `SCE_MESH.md` (`§16.7`,
`§9.6.2`), which the scxml ledger cannot resolve. The `mesh` namespace gives
those cites their own ledger under `docs/sce-ledger/mesh`.

```bash
python3 tools/mnemosyne-adoption/sce_mesh_md_to_manifest.py --manifest out/mesh-manifest.json
cd docs/sce-ledger/mesh && mnemosyne-cli import-sections --manifest out/mesh-manifest.json
```

The converter is the markdown sibling of A1. Differences that follow from the
source being an internal markdown doc rather than a vendored HTML standard:

- **Markdown ATX headings**, not HTML. `## N.` / `### N.M` / `#### N.M.K` map to
  `mesh-<n>`; the numeric labels keep their dots, matching A1's numeric policy
  (`§mesh-16.7`). Un-numbered headings (`### Problem`, `### Rationale`) carry no
  number, so no `§mesh-<n>` cite can target them — they are skipped.
- **Fenced code blocks are skipped**, so a `#### 99.`-shaped line inside a
  ```` ``` ```` block is never mistaken for a heading.
- **No external upstream**: `SCE_MESH.md` is tracked in this repo, so there is no
  vendored snapshot, no `[workspace.spec_source]`, and no drift CI. The manifest
  is skeleton only (no `normative_excerpt`): the ledger exists to make `§mesh-<n>`
  cites resolve for the gate, not to render a vendored quote.

`docs/sce-ledger/mesh/mnemosyne.toml` declares `section_namespace = "mesh"`. The
mesh citations are migrated from bare `§<n>` to `§mesh-<n>` with
`migrate_citations.py --namespace mesh` (the bare-sigil path, guarded by ledger
membership, a cross-namespace ambiguity check against the scxml ledger, and a
foreign-standard marker — see "R3" above). The mesh/common/static modules, which
mix W3C and SCE-internal cites, are then gated here for their `§mesh-` cites
while their `§scxml-` cites stay gated by the scxml workspace.

## C — Wire RFC design-ledger workspace (`wire` namespace)

The Wire RFC (`claudedocs/rfc-sce-diagnostic-wire-unification.md`) defines a
commit-series of milestone waves W0..W5 (plus the half-wave W4.5). SCE code in
`parsing/` and `runtime/` cites them as `§W<n>` ("RFC §W4", "§W5 D5"); the `wire`
namespace gives those cites their own ledger under `docs/sce-ledger/wire`.

```bash
python3 tools/mnemosyne-adoption/sce_wire_rfc_to_manifest.py --manifest out/wire-manifest.json
cd docs/sce-ledger/wire && mnemosyne-cli import-sections --manifest out/wire-manifest.json
```

`sce_wire_rfc_to_manifest.py` extracts the `### W<n>` wave headings (skipping the
retained "RFC (legacy section header ...)" duplicate of each), keeping the wave
label verbatim (`W4.5 -> wire-W4.5`; the dot is digit-flanked so the extractor
reads it whole). Migration uses `migrate_citations.py --namespace wire`: wire
labels (`W<n>`) are unique, so no cross-namespace guard is needed, and a hyphen
range like `§W3-5` (waves W3 through W5) is refused by the glued-suffix guard
rather than corrupted into one id. The parsing/runtime modules are then gated
here for their `§wire-` cites alongside their `§scxml-` cites in the scxml gate.

### A note on `[commit_ledger]`

All three SCE workspaces set `[commit_ledger] severity = "warn"`. Mnemosyne's
commit↔ledger drift gate (R293/R301) scans recent commit subjects for `(R<n>)`
round labels and expects a matching `Round <n>` changelog entry. SCE's `(R<n>)`
are **adoption-round counters**, not Mnemosyne changelog rounds, so the gate is
advisory here (the drift line still prints; it just does not gate the exit code).
Mnemosyne R377 also path-scopes the scan to each workspace's subtree so a sibling
workspace's labels no longer bleed in.

## CI enforcement (`.github/workflows/spec-citations.yml`)

The citation gate runs in CI on every change to `sce/**` or a workspace, across
all three namespaces. Because `mnemosyne-cli` is an external Rust binary (a
separate repo, intentionally not vendored), the job installs it at a pinned
Mnemosyne revision via `cargo install --git ... --rev <sha> --locked` and caches
it by that rev — the consumer CI pattern from Mnemosyne's SCHEMA_GUIDE. It then
runs `validate-workspace` + `validate-code-refs` in each workspace (so a
hallucinated §<ns>-<id> citation fails the build), plus `validate-content-drift`
on the scxml workspace (so a tampered `normative_excerpt` or a swapped EPUB fails
offline against the pinned `epub_sha256`). The store is the sole SSOT
(Mnemosyne R400 retired the GENERATED.md render path, so there is no round-trip
view to drift). Bump `MNEMOSYNE_REV` deliberately, re-validating the three
workspaces locally against the new rev first; the closed-loop tooling tests
(which self-skip without the CLI) cover the migrators' grammar contract.
