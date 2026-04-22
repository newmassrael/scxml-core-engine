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
              └── write embed/MANIFEST.json          (sorted, deterministic)
```

The manifest lives inside the otherwise-gitignored `embed/` tree and is
un-ignored via a `!embed/MANIFEST.json` entry in `.gitignore`, so that
consumers receive it with the vendored copy and so CI can drift-guard it
against the checked-in version.

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

`scripts/verify_embed_manifest.sh` re-emits the manifest to a temp file
and byte-compares against `embed/MANIFEST.json`. Any mismatch exits
non-zero with the diff. Intended for CI on changes touching
`sce/include/` (the canonical source) or any file that feeds into
`scripts/package_embed.sh`.

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
