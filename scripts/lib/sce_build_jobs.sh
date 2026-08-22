# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# How many parallel jobs a build run by this repository's own tooling may ask
# this machine for — the ONE place that decides it.
#
# NOT `$(nproc)`, which means "every core, whatever else is on the box". None
# of this tooling runs on a dedicated runner: this workstation carries sixteen
# concurrent sessions and six repositories at once, and a push here runs
# twenty-seven gates back to back.
#
# MEASURED 2026-08-19. One push gate reached `cargo test --workspace` while the
# machine was otherwise idle and produced eight concurrent rustc processes plus
# a linker at 335% CPU, driving 600MB/s of reads and 400MB/s of writes through
# the build cache; forty processes sat blocked on disk and CPU idle fell to
# 43%. Earlier the same day an uncapped build in a sibling repository pushed
# 13GB into swap and took the box to load 29 while 82% of its CPU sat idle —
# every bit of that load was processes waiting on the paging disk. Asking for
# all thirty-two cores does not make a build finish sooner once the disk is the
# thing that is short.
#
# ⚠ Linux load counts uninterruptible sleep, so a box thrashing its disk reads
# as busier than its idle CPU suggests. That error is in the safe direction
# here: a machine already waiting on a disk is precisely the one that should
# not be handed thirty-two more compile jobs.
#
# ⚠ WHY THIS FILE EXISTS RATHER THAN THE FUNCTION LIVING IN `gates/lib.sh`.
# The rule started there and the gates were its only readers. They are not any
# more: `smoke_embed_consumer.sh` and `check_clang_format.sh` are reached by a
# gate and a hook but are standalone scripts, and they cannot source
# `gates/lib.sh` — it `cd`s to the repository root and derives a gate slug from
# its caller, so sourcing it moves a script that is not a gate and makes it
# report as one. A rule with readers outside the gate library needs a home
# outside the gate library, which is the same conclusion
# `scripts/lib/sce_http_endpoint.sh` reached about the fixture endpoint.
#
# `$SCE_BUILD_JOBS` overrides, for a runner that really does own its cores.
#
# Usage:
#   source "<repo>/scripts/lib/sce_build_jobs.sh"
#   cmake --build "$dir" --parallel "$(sce_build_jobs_value)"

sce_build_jobs_value() {
    if [ -n "${SCE_BUILD_JOBS:-}" ]; then
        printf '%s' "$SCE_BUILD_JOBS"
        return 0
    fi
    local cores queued jobs
    cores="$(nproc)"
    queued="$(awk '{printf "%d", ($1 == int($1)) ? $1 : int($1) + 1}' /proc/loadavg)"
    jobs=$(( cores - queued ))
    if [ "$jobs" -lt 1 ]; then jobs=1; fi
    printf '%s' "$jobs"
}
