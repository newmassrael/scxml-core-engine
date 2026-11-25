# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `windows-msvc`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The C++ client parts built with MSVC on Windows (docs/adr/0107, D11): the
# reference host, its COSE operations on vcpkg's libsodium and the generated
# machines through CMake, and the Bazel consumers through both channels.
# No other gate builds on Windows, and no push host is one.
#
# ⚠ `cost_s` is an ESTIMATE, absent from COST_MEASURED: the lane has no run.
# Its parts are a cached generator build, a cold configure and a build of
# sce_base and one test, two vcpkg ports and a cold Bazel through two
# channels. Time the gate on a Windows host and date it here.
{
    "workflows": ["windows-msvc.yml"],
    # Not delegated: the workflow runs the gate's script itself, not through
    # `scripts/gate`. The runner opens with this file's self-test and
    # `scripts/lib/require_submodules.sh`, which read the tree through
    # Python on POSIX paths and are held on the Linux lanes; on the Windows
    # runner they would be a second, unrelated verdict ahead of the one this
    # lane exists for. The commands live once, in the script.
    "ci_only": "a Windows host with Visual Studio 2022 and vcpkg, which no "
               "push host is, and minutes long, estimated rather than "
               "measured. windows-msvc.yml runs it on a hosted Windows "
               "runner.",
    "cost_s": 1500,
    "summary": "C++ host, COSE and generated machines with MSVC; Bazel channels",
    # Off the fleet, so even a full fleet run (`--all --landing`) leaves it
    # to the hosted lane (see "Where a gate can run"). The host is the
    # platform: every build machine is the Linux bx.toml declares, and the
    # verdict is MSVC's on Windows, so a unit of it could never be placed
    # and a full run that kept it would come back unlanded. When bx.toml
    # declares a Windows platform this attribute names it, and the gate
    # joins a full fleet run.
    "platform": "github-hosted-windows-2022",
    "platform_reason": "the verdict is MSVC's, with Visual Studio 2022 and "
                       "vcpkg on the hosted windows-2022 image; every build "
                       "machine is the Linux platform bx.toml declares",
    "cache": False,
    "no_cache_reason": "a host compiler and ports no declaration pins: the "
                       "verdict is MSVC's as the Windows image carries it, and "
                       "vcpkg's libxml2 and libsodium at the versions its "
                       "checkout holds",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
