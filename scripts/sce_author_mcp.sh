#!/usr/bin/env bash
# Launch the pack-free SCE authoring MCP server from any working directory.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
codegen="$repo_root/target/debug/sce-codegen"

if [[ ! -x "$codegen" ]]; then
  printf 'sce-author MCP: build the code generator first: cargo build -p sce-build --features cli --bin sce-codegen\n' >&2
  exit 1
fi

export PYTHONPATH="$repo_root/tools/authoring${PYTHONPATH:+:$PYTHONPATH}"
exec python3 -m sce_author.mcp "$@"
