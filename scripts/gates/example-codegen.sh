#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: example-codegen.yml
#
# Example SCXML codegen smoke + authored-document lint.
#
# The emscripten-backed doom_wasm / visualizer builds are too heavy for a
# push-time gate, but codegen failures (the kind that broke the
# `urn:sce:extensions` -> `http://sce.dev/ext` namespace migration) surface
# well before link time. Run sce-codegen over each example SCXML in a
# scratch output dir and fail if any produce no artifacts.
#
# The second sweep runs `check --lint` over every document this repository
# AUTHORS. The statechart lints are off by default because the W3C corpus
# declares unreachable states on purpose (a fixture proving `initial` is
# respected has states no event can enter), so a lint that rejected them
# would be wrong about conformance fixtures. Documents we write ourselves
# carry no such excuse, and until this sweep existed nothing in the repo
# turned `--lint` on at all — the flag shipped and no caller used it.
#
# Measured when it was added: three of the doom examples had a compound
# state whose children disagreed about an event, each an intentional gap now
# declared on the child that has it with `sce:unhandled` and the reason
# beside it. Two of the three prose comments were wrong about which child
# the gap belonged to; declaring it per child is what surfaced that.
#
# `--lint` also carries the ECMAScript acceptance verdict. A refused
# expression is generated rather than refused — §scxml-5.9.1 obliges the
# machine to raise `error.execution` instead — so it is reported on every
# run and fatal where the author has no conformance excuse for writing
# one. Both sweeps that judge an authored document now live in Rust and
# derive their population: `sce-build/tests/cli_expression_refusal.rs`
# for the refusal half and `sce-build/tests/cli_lint_sweep.rs` for the
# lints. This gate keeps the codegen smoke, which is a different claim —
# that the examples still GENERATE.
#
# ⚠ Measured 2026-09-22, which is what made the move worth doing: the
# hand-written pair of directories was 56 of the 560 tracked statecharts,
# and widening to the derived population turned up three repairs — an
# authored document calling six functions it never declared, a mesh
# fixture whose unreachable state worked around the generator emitting no
# transport for a srcexpr-only document, and a probe whose deliberate
# event-handling gaps were undeclared. None of them was an exemption.
#
# This gate is `ci_only`: the registry gives it no push-time trigger,
# because generating from every authored document is work proportional to
# the corpus (99s measured against a declared 4s — the widest gap in the
# table). `example-codegen.yml` is what runs it, and it calls
# `scripts/gate example-codegen` rather than restating these commands, so
# the check has one spelling for both callers.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CODEGEN="$(sce_gate_codegen)"

GEN_OUT="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$GEN_OUT'"

failures=0
for scxml in examples/doom_wasm/scxml/*.scxml examples/smart_light/smart_light.scxml; do
    [[ -f "$scxml" ]] || continue
    if ! "$CODEGEN" generate "$scxml" \
            --language cpp \
            --output-dir "$GEN_OUT/" >/dev/null 2>&1; then
        printf '  FAIL: %s\n' "$scxml" >&2
        failures=$((failures + 1))
    fi
done
(( failures == 0 )) || sce_gate_fail "$failures example(s) failed codegen"

# The lint sweep that used to live here is now
# `sce-build/tests/cli_lint_sweep.rs`, and this gate keeps the codegen
# smoke above.
#
# ⚠ WHAT MOVED, AND WHY IT IS NOT A LOSS. The loop here named two
# directories — `examples/*.scxml` and `integration_resources/*/*.scxml`,
# 56 of the 560 tracked statecharts — and this gate is `ci_only`, so the
# claim "every document this repository authors is lint-clean" was made
# over a tenth of them, in one workflow. The Rust sweep derives its
# population instead (tracked statecharts, less the W3C corpus, less the
# documents another document `<xi:include>`s) and runs with the workspace
# tests, so it is judged on every push as well as in CI. Both halves of
# the claim moved with it, including "some backend accepts this
# document", which the manifest answers and `--lint`'s exit does not.
