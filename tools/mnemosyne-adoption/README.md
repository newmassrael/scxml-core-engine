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
