#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# What a run-time lowering caller would owe the SCOPE, counted.
#
# The second of the four questions docs/SCE_LUA_TRANSLATION_SEAM.md has to
# answer before a C-callable lowering surface is a decision rather than a
# preference. Unlike the first (per-call cost), this one is NOT a stopwatch:
# the doubt is whether a run-time caller must maintain a document scope at
# all, and that is a correctness count over the corpus.
#
# WHAT IT MEASURES. Every expression in every tracked .scxml is lowered once
# per scope stage and compared against the stage a build-time caller always
# has (`Everything`, the whole document read before anything is lowered).
# The stages nest, so each column is attributable to exactly one kind of
# declaration:
#
#   installed      nothing of the document read -> what an FFI with no scope
#                  handle would be wrong about
#   datamodel      plus every <data id>, which early binding (W3C SCXML 5.3)
#                  puts in the datamodel before the first macrostep, so a
#                  run-time caller can reach this stage from the model alone
#   write_targets  plus names an <assign location>/<send idlocation>/<foreach>
#                  brings into existence by writing to it
#
# The residue printed after the census is what remains at `write_targets` —
# the sites reachable only by `declare_chunk`, which is the population that
# decides whether the C surface needs that entry point at all.
#
# NOT A GATE, and the reason is not flakiness this time. The counts are
# deterministic; what makes them unsuitable as a bound is that the corpus
# grows, so a hard-coded total would turn every new document into a failure.
# The invariants that CAN rot are asserted, in the test itself:
# `every_stage_boundary_is_observable` refuses a blind instrument (a staging
# argument that is ignored reports zero divergence everywhere, and zero is
# the reading that would retire the scope handle), and `the_stages_nest`
# refuses a staging whose columns cannot be attributed.
#
# Usage:  scripts/measure-scope-obligation.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> host $(hostname), $(nproc) core(s)"
echo "==> corpus: $(git ls-files '*.scxml' | wc -l) tracked .scxml file(s)"
echo

LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT

# `--nocapture` is what makes the census reach stdout: it is printed rather
# than asserted, for the reason in the header.
cargo test --quiet -p sce-build --test scope_obligation -- --nocapture > "$LOG" 2>&1 || {
    echo "measure-scope-obligation: the probe failed — the census below is not an answer." >&2
    cat "$LOG" >&2
    exit 1
}

sed -n '/ScopeObligation census/,$p' "$LOG" | sed '/^test result/,$d'

python3 - "$LOG" <<'PY'
import re, sys

text = open(sys.argv[1]).read()
m = re.search(r"ScopeObligation census: (.*)", text)
if not m:
    raise SystemExit("the probe printed no census line")
f = {k: int(v) for k, v in (kv.split("=", 1) for kv in m.group(1).split() if "=" in kv)}

sites = f["sites"]
installed = f["installed_diverging"]
datamodel = f["datamodel_diverging"]
load_time = f["load_time_diverging"]
writes = f["write_targets_diverging"]

print()
print(f"  Of {sites} expression site(s) in {f['documents']} document(s):")
print(f"    {installed:5d} ({100*installed/sites:.1f}%) lower differently with NO document read")
print(f"    {datamodel:5d} ({100*datamodel/sites:.1f}%) still do once every <data id> is declared")
print(f"    {load_time:5d} ({100*load_time/sites:.1f}%) still do once every document-level <script> is too")
print(f"    {writes:5d} ({100*writes/sites:.1f}%) still do once every write target is declared as well")
print()
print(f"  `declare` over <data id> discharges {installed - datamodel} of the {installed} site(s) an FFI")
print(f"  with no scope handle would be wrong about; `declare_chunk` over the")
print(f"  document-level <script>s discharges the remaining {datamodel - load_time}.")
print()
if load_time == 0:
    print("  ANSWER: both sources are readable from the model BEFORE the first")
    print("  macrostep (W3C SCXML 5.3 early binding, 5.8 load-time scripts), so a")
    print("  run-time caller needs `declare` + `declare_chunk` and NO scope that")
    print(f"  tracks execution — write targets add {load_time - writes} beyond that.")
else:
    print(f"  ⚠ {load_time} site(s) remain after both pre-run sources. A run-time")
    print("  caller would need scope maintained THROUGH execution, which is a")
    print("  different C surface from the one the ledger prices.")
PY
