#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: deploy-visualizer.yml
#
# The diagram's geometry, judged without a browser.
#
# ## Why there is anything to judge
#
# A rendered diagram is not bytes a test can diff, and that has been taken to
# mean the visualizer cannot be checked at all. The LAYOUT behind it is data:
# node boxes, bend points, label rectangles. Every defect this gate exists for
# was found in those numbers and none of them had a test:
#
#   - ELK's routes were deleted 23 lines after being stored, so the branch
#     consuming them was unreachable — and had rotted into `M NaN NaN`
#   - a container ELK sized was shrunk afterwards, stranding two edges 20px
#     outside it
#   - a cross-hierarchy route arrived in an ancestor's coordinate frame and
#     drew 753px from the states it joins
#   - collapsing a compound left eight edges touching nothing, because the
#     edges redirected to the collapsed ancestor were never given to ELK
#
# ## What it asserts, and what it only reports
#
# ⭐ INVARIANTS fail the gate. They are not matters of degree: an edge that
# does not reach its states, a label with no coordinate, two unrelated states
# sharing area, a node the layout never placed. Each is a drawing that is
# wrong rather than crowded.
#
# The crowding numbers are REPORTED, not asserted. Their thresholds
# ("within 24px over a 25px run") were chosen because a label is about that
# tall, and nobody has checked that a diagram scoring better on them reads
# better to a person. A gate that failed on them would be enforcing a number
# whose meaning is unestablished — see web/visualizer/LAYOUT_MEASUREMENT.md,
# which also records the axis this measurement structurally cannot see.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

command -v node >/dev/null 2>&1 \
    || sce_gate_cannot_run "the layout measurement runs the visualizer under node, and node was not found"

[[ -f web/visualizer/vendor/elkjs/elk.bundled.js ]] \
    || sce_gate_cannot_run "the vendored layout engine is missing; nothing can be laid out"

[[ -f web/visualizer/visualizer.js ]] \
    || sce_gate_cannot_run "the engine WASM glue is missing, so no document can be turned into a structure"

# The fixtures, and the reason each one is here, live beside the measurement
# in a file of their own.
#
# ⚠ An earlier version DERIVED them — every tracked `.scxml`, largest first.
# That reads like this repository's own discipline and was wrong for a reason
# the registry's self-test stated better than this comment could: it made the
# gate's inputs the whole repository while the workflow running it watches
# only `web/visualizer/**`, so a commit touching a document elsewhere would be
# told this lane judges it and the lane would never run. The list is pinned,
# and `fixtures.txt` carries what that costs.
FIXTURE_LIST="web/visualizer/measure/fixtures.txt"
[[ -f "$FIXTURE_LIST" ]] || sce_gate_fail "$FIXTURE_LIST is missing; there is nothing to draw"

mapfile -t FIXTURES < <(grep -vE '^\s*(#|$)' "$FIXTURE_LIST")

if [[ ${#FIXTURES[@]} -eq 0 ]]; then
    sce_gate_fail \
        "$FIXTURE_LIST names no document.
  An empty fixture list is not a clean result: it is a gate that measures nothing
  and reports success, which is the failure this whole measurement exists to catch."
fi

# Every one must exist. A renamed fixture would otherwise shrink the
# population silently, one document at a time.
missing=0
for f in "${FIXTURES[@]}"; do
    [[ -f "$f" ]] || { printf 'ERROR: fixture not in the tree: %s\n' "$f" >&2; missing=$((missing + 1)); }
done
[[ $missing -eq 0 ]] || sce_gate_fail \
    "$missing fixture(s) named in $FIXTURE_LIST are not in the tree; the list owes an update"

printf 'visualizer-layout: %d fixture(s)\n' "${#FIXTURES[@]}"

node web/visualizer/measure/census.js "${FIXTURES[@]}" \
    || sce_gate_fail "the layout census reported a broken drawing; its output above names which"

# The interaction half. Everything above is one frame — the layout as first
# computed — and a gesture is where geometry changes, which is where every
# defect in the header came from. The first fixture is the nested `<parallel>`
# one; a flat document exercises neither the collapse redirect nor container
# sizing, so it is taken from the list rather than chosen here.
node web/visualizer/measure/interaction.js "${FIXTURES[0]}" \
    || sce_gate_fail "the drawing did not survive a drag or a collapse; its output above names which"

# The stress half: the same gestures, but MANY of them, in a seeded random
# order, over every fixture.
#
# ⚠ It exists because the probe above performs one drag and one collapse in a
# fixed order, and a reader does not. Its first run found ELK routes ending 16
# to 18px short of a state on every seed it tried — always just after a
# collapse, which is a sequence no fixed-order probe reaches.
#
# ⚠⚠ The operation count is deliberately modest here. Sixty operations over
# ten documents takes ~33s against this gate's ~6s, and a gate nobody wants to
# wait for is one that gets skipped. `SCE_STRESS_OPS` raises it for a hunt,
# and `SCE_STRESS_SEED` replays exactly what failed — per document, so
# narrowing a run to one fixture reproduces the same sequence.
# ⚠⚠⚠ The SEED IS PINNED here, and that is the important line.
#
# Left to itself the probe seeds from the clock, which explores more — and
# makes a red non-reproducible. A gate that fails on a sequence the next run
# does not generate is flaky in the way this repository has been bitten by
# before: the second run is green, the red is filed as noise, and the defect
# stays. Pinned, this lane is a regression fence: the same twenty-five
# gestures per document, every run, and a red is always replayable.
#
# ⭐ Exploring is still worth doing — it is how the 16-to-18px routes were
# found — but deliberately, not as a lottery inside a gate:
#     for s in $(seq 1 20); do SCE_STRESS_SEED=$s SCE_STRESS_OPS=60 \
#         node web/visualizer/measure/stress.js || break; done
# A seed that finds something belongs here, as another pinned run.
SCE_STRESS_SEED="${SCE_STRESS_SEED:-20260918}" \
SCE_STRESS_OPS="${SCE_STRESS_OPS:-25}" \
    node web/visualizer/measure/stress.js \
    || sce_gate_fail "the drawing did not survive a run of random gestures; its output above names the seed to replay"

printf 'visualizer-layout: OK\n'
