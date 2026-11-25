# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `ledger-citations`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["spec-citations.yml"],
    "runner_workflow": True,
    # Where the time goes, re-measured 2026-08-11 evening after the
    # mnemosyne pin moved to c9b276bf. `validate-code-refs` used to be
    # 108.9s of the 110s this gate took alone, because the symbol resolver
    # re-read, re-parsed and re-compiled its tree-sitter query once per
    # CITATION. Upstream now resolves once per FILE, and this tree is where
    # that ratio is visible: 4487 citations over 290 files across the five
    # workspaces, so the call count fell 15.5x and the wall clock fell
    # further still (synth 82.1 -> 0.90, scxml 11.5 -> 0.54, mesh 12.5 ->
    # 0.29, wire 0.40, bytesguard 0.45 — 2.6s in total). The whole-tree
    # existence sweep, 4.4s over 6357 files, is now the larger half.
    # `cost_s` stays at the `--measure` figure the runner compares against.
    "ci_only": "121s across five workspaces. The registry's own header "
               "already told this story once — the hook's prose called it "
               "sub-second per workspace while a measurement put the whole "
               "gate at 157s — and the 15s it was declared at was the same "
               "mistake in the other direction, read off a warmed full "
               "sweep. spec-citations.yml runs it.",
    "cost_s": 121,
    "summary": "spec-citation ledgers, 5 workspaces",
    # The validator is the revision a tracked workflow pins. Its git reads
    # (`git diff --cached`, `checkout-index`) are the commit hook's
    # `--staged` mode; the full gate a unit runs reads the tracked tree.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
