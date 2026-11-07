# embed/ Public-Header Manifest

`embed/MANIFEST.json` records the public C++ API surface of the SCE
embed package — classes, struct layouts, free functions, enum members,
and typedefs declared inside `embed/include/**/*.h`. It exists so
downstream projects that vendor the embed package can diff the old
manifest against a fresh one at sync time and see exactly which public
symbols changed.

## Pipeline

```
sce/include/**/*.h                       (canonical source)
       │
       ▼
scripts/package_embed.sh                 (regenerates embed/)
       │
       ├── copy headers + sources
       └── run scripts/emit_embed_manifest.sh
              │
              ├── clang++-19 -Xclang -ast-dump=json … per header
              ├── scripts/_embed_manifest_filter.py   (JSON → per-symbol lines)
              ├── write embed/MANIFEST.json          (sorted, deterministic)
              └── write embed/MANIFEST.inputs        (what it was emitted from)
```

`scripts/embed_manifest.py` is the one place that says what the manifest is
made of: the headers `package_embed.sh` copies (`sce/include/*.h`, each
directory `SCE_BASE_INCLUDE_DIRS` names in `sce/sce_base_sources.cmake`, and
the Forge C++ runtime's headers), the third-party include roots the emitter
adds, and the tools. `package_embed.sh` reads `SCE_BASE_INCLUDE_DIRS` through
it and the emitter its include roots, so the files copied, the files parsed
and the files the stamp records are one list.

The manifest and its stamp live inside the otherwise-gitignored `embed/` tree
and are un-ignored by `.gitignore`, so that consumers receive the manifest with
the vendored copy and so CI can drift-guard both against the checked-in
versions.

## Regenerating

Both files are committed generated output, regenerated with every other
committed tree (`bx --actions`, every step of
`scripts/regen_all_committed_trees.sh` on the build machines) or by their step
alone:

```bash
bx --action regen-embed-manifest   # the embed-manifest step, on a build machine
git add embed/MANIFEST.json embed/MANIFEST.inputs
```

The stage packages into a scratch directory and emits there. The label fields
(`embed_version`, `clang_version`) change with every emit, so where the rest of
the manifest is unchanged the committed bytes are kept; the stamp holds no
label and is taken as emitted. The emitter needs `clang++-19`, which every
build machine has.

## Schema

```jsonc
{
  "schema": "sce-embed-manifest.v1",
  "embed_version": "<git describe>",
  "clang_version": "<clang --version first line>",
  "symbol_count": 189,
  "non_self_contained_headers": [
    "core/EventQueueManager.h"       // produced diagnostics during scan
  ],
  "symbols": [
    {
      "kind": "function",
      "name": "SCE::GuardUtils::isConditionExpression",
      "header": "GuardUtils.h",
      "signature": "bool SCE::GuardUtils::isConditionExpression(const std::string &)"
    },
    {
      "kind": "class",
      "name": "SCE::SCXMLEngine",
      "header": "SCXMLEngine.h",
      "members": [
        "method void SCE::SCXMLEngine::start()",
        "method void SCE::SCXMLEngine::stop()"
      ]
    },
    {
      "kind": "enum",
      "name": "SCE::EventPriority",
      "header": "SCXMLTypes.h",
      "values": ["INTERNAL", "EXTERNAL"]
    },
    {
      "kind": "typedef",
      "name": "SCE::EventId",
      "header": "SCXMLTypes.h",
      "type": "std::string"
    }
  ]
}
```

### Field semantics

- `symbols`: sorted by the compact JSON representation of each entry, so
  two manifests produced from the same input compare byte-identical.
- `non_self_contained_headers`: headers whose standalone parse produced
  clang diagnostics (typically missing transitive includes). Symbols are
  still emitted best-effort from the partial AST; class `members` for
  these entries may be incomplete. Fix the underlying include so the
  header drops off this list.
- `clang_version`: included so a manifest diff that only reflects a
  toolchain bump is distinguishable from a real API change.

## Consumer workflow — diff on vendor sync

```bash
# Keep the previous manifest alongside the refresh
cp third_party/sce/embed/MANIFEST.json /tmp/sce-manifest-before.json

# Re-vendor (or re-run scripts/package_embed.sh against the SCE source tree)
./vendor-sce.sh

# See what changed
diff -u /tmp/sce-manifest-before.json third_party/sce/embed/MANIFEST.json
```

For structural queries without `jq`, Python works:

```bash
python3 -c '
import json, sys
a = json.load(open("/tmp/sce-manifest-before.json"))
b = json.load(open("third_party/sce/embed/MANIFEST.json"))
names = lambda m: {s["name"] for s in m["symbols"]}
print("removed:", sorted(names(a) - names(b)))
print("added:",   sorted(names(b) - names(a)))
'
```

This is intentionally a textual diff tool, not an ABI-analysis tool.
The manifest does **not** classify changes as breaking / non-breaking,
does **not** enforce semver, and does **not** version-negotiate with
the consumer. Humans read the diff and decide.

## SCE-side drift guard

Two questions, asked at different costs:

- **Was the manifest emitted from the inputs it is committed with?**
  `embed/MANIFEST.inputs` lists the git object id and path of each input, a
  submodule as the commit it pins. `scripts/embed_manifest.py check` compares
  a snapshot's inputs with the stamp in it, with no clang and in well under a
  second, and names each input that differs. The pre-commit hook asks it of
  the index (`check --staged`), so the commit that edits a public header
  carries the regenerated manifest or is refused; the gate
  `embed-manifest-stamp` asks it of HEAD at the push and in CI, the backstop
  for a commit made with the hook skipped. Until this check the drift was
  found after the push, and five of eight manifest commits in October and
  November 2026 were repairs made after the fact.
- **Is the manifest's content right?** `scripts/verify_embed_manifest.sh`
  packages, re-emits and compares against `embed/MANIFEST.json`, label fields
  aside; any mismatch exits non-zero with the diff. It runs in the CI lane
  `embed-vendor`, and `regen-reproduces` asks the same of the regeneration.

The stamp answers the first from git alone, so a header edit, even to a
comment, asks for a regeneration; a regeneration that changes nothing but the
stamp keeps the manifest's bytes.

## Limitations

- Only what clang sees is captured; macro-defined APIs (e.g. symbols
  declared inside `#if BUILD_FOO` blocks) are scoped to the build
  configuration used at emit time (`-std=c++17`, no `-D` flags).
- Optional-dependency headers are excluded from the scan:
  `SpdlogBackend.h` (needs `spdlog/`) and `EmscriptenFetchClient.h`
  (Emscripten-only). Their API drift must be reviewed manually.
- Template members report as `template ClassTemplateDecl <name>` rather
  than per-instantiation; this is by design so unused specialisations do
  not inflate the manifest.
