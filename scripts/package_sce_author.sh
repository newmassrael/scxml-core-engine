#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Build the authoring MCP server as a bundle that runs where this tree is not.
#
#   scripts/package_sce_author.sh OUT_DIR [--codegen PATH] [--work PATH]
#
# Writes OUT_DIR/sce-author/ and OUT_DIR/sce-author-<commit>-<os>-<arch>.tar.gz:
#
#   bin/sce-author-mcp      the launcher an AI client registers (stdio), or
#                           runs with --http HOST:PORT for a remote client
#   bin/sce-codegen         the code generator every tool answers from
#   bin/sce-work            with --work: the workbench application's own command
#                           layer, which the works tools read and save a work
#                           through (SCE_WORK); without it those tools say so
#   share/sce/templates/    the templates it renders; an installed binary
#                           has no source tree to find them in
#   python/sce_author/      the MCP server
#   python/schema/          the schemas it reads a pack and a decision record
#                           against, found beside the package (`SCHEMA_DIR`)
#   LICENSE*                the terms the bundle is distributed under
#
# WHY A BUNDLE. `scripts/sce_author_mcp.sh` runs the server out of a
# checkout, which asks a specification owner for a Rust toolchain and a
# build before the first question. The generator reads its templates from
# the tree it was compiled in (`find_template_base`), and refuses to run
# where that tree is absent unless SCE_TEMPLATE_DIR names them — so the
# templates travel with the binary and the launcher names them, together
# with the generator itself (SCE_CODEGEN).
#
# Without --codegen the generator is found, or built, by
# scripts/lib/sce_codegen.sh; --codegen names one explicitly.

set -euo pipefail

usage() {
    printf 'usage: %s OUT_DIR [--codegen PATH] [--work PATH]\n' "$0" >&2
    exit 2
}

[[ $# -ge 1 ]] || usage
out="$1"
shift
codegen=""
work=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --codegen) [[ $# -ge 2 ]] || usage; codegen="$2"; shift 2 ;;
        --work) [[ $# -ge 2 ]] || usage; work="$2"; shift 2 ;;
        *) usage ;;
    esac
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -z "$codegen" ]]; then
    # The one locator every shell consumer resolves the generator through:
    # it builds when no profile holds a generator of these sources.
    # shellcheck source=lib/sce_codegen.sh
    source "$repo_root/scripts/lib/sce_codegen.sh"
    codegen="$(sce_codegen_require "$repo_root")"
fi
[[ -x "$codegen" ]] || { printf 'package: %s is not an executable generator\n' "$codegen" >&2; exit 1; }
# The works command is named, never built here: it belongs to the application's
# workspace (`cargo build -p sce-app-core --features cli --bin sce-work`), and a
# bundle that quietly carried a stale one would be the second writer of the folder.
if [[ -n "$work" && ! -x "$work" ]]; then
    printf 'package: %s is not an executable sce-work\n' "$work" >&2
    exit 1
fi

mkdir -p "$out"
out="$(cd "$out" && pwd)"
bundle="$out/sce-author"
rm -rf "$bundle"
mkdir -p "$bundle/bin" "$bundle/share/sce" "$bundle/python"

cp "$codegen" "$bundle/bin/sce-codegen"
if [[ -n "$work" ]]; then
    cp "$work" "$bundle/bin/sce-work"
fi
cp -R "$repo_root/tools/codegen/templates" "$bundle/share/sce/templates"
cp -R "$repo_root/tools/authoring/sce_author" "$bundle/python/sce_author"
cp -R "$repo_root/tools/authoring/schema" "$bundle/python/schema"
find "$bundle/python" -name '__pycache__' -type d -prune -exec rm -rf {} +
cp "$repo_root"/LICENSE "$repo_root"/LICENSE-*.md "$bundle/"

cat > "$bundle/bin/sce-author-mcp" <<'LAUNCHER'
#!/usr/bin/env bash
# The SCE authoring MCP server, from this bundle. stdio by default;
#   sce-author-mcp --http 127.0.0.1:8765 [--token-file FILE]
# serves a client on another machine (a token is required off loopback).
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export SCE_CODEGEN="$here/bin/sce-codegen"
if [[ -x "$here/bin/sce-work" ]]; then
    export SCE_WORK="$here/bin/sce-work"
fi
export SCE_TEMPLATE_DIR="$here/share/sce/templates"
export PYTHONPATH="$here/python${PYTHONPATH:+:$PYTHONPATH}"
if ! python3 -c 'import yaml' 2>/dev/null; then
    printf 'sce-author-mcp: needs Python 3 with PyYAML (pip install pyyaml)\n' >&2
    exit 1
fi
exec python3 -m sce_author.mcp "$@"
LAUNCHER
chmod +x "$bundle/bin/sce-author-mcp"

commit="$(git -C "$repo_root" rev-parse --short=12 HEAD 2>/dev/null || echo unknown)"
cat > "$bundle/README.txt" <<README
SCE authoring MCP server -- bundle of commit $commit.

Register bin/sce-author-mcp as a local stdio MCP server in the AI client
(use its absolute path). For a client on another machine, run

    bin/sce-author-mcp --http 0.0.0.0:8765 --token-file TOKEN_FILE

and point the client at http://HOST:8765/mcp with the header
"Authorization: Bearer <the token>". A remote client hands every document
over as text (document_text); a path is refused.

works_list, works_read, works_save_model and works_save_requirements read the
specification the owner keeps in the workbench application and save the model
and the requirement list written for it. They run
bin/sce-work when this bundle carries one, on the works folder the application
opens (SCE_WORKS_DIR, else the per-user data directory), and are never offered
to a remote client.

Needs Python 3.10 or later with PyYAML, and jsonschema to read a decision
record or a pack (without it, those tools refuse and say why). Nothing
else is installed or read outside this directory. The AI client may send
the specification, the
drafts and the tool results to its model service: check that against the
agreement the specification is under before using restricted material.
README

os_name="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
archive="$out/sce-author-$commit-$os_name-$arch.tar.gz"
tar -czf "$archive" -C "$out" sce-author
printf '%s\n%s\n' "$bundle" "$archive"
