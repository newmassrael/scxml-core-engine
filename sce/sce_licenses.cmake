# sce_licenses.cmake — SSOT for license-text distribution
#
# Single source of truth shared by:
#   - CMakeLists.txt install() rules (per-tier components)
#   - scripts/package_embed.sh (vendor embed payload)
#
# Both LGPL-2.1 §1 and MIT (the licence used by every vendored
# third_party dep with a standalone LICENSE file) require the licence
# text to ride along with the distributed binary or source. Hardcoding
# the file list in either consumer drifts the moment a new third_party
# dep lands — mirror the established sce_base_sources.cmake /
# sce_codegen_assets.cmake pattern instead, so a single edit here is
# replayed by the find_package install rules and the embed packager.
#
# Path rule: every entry is relative to the SCE source root and is
# preserved verbatim under the install / embed root, so consumers
# inspecting either delivery see the same paths they'd find in the
# upstream repo (LICENSE, third_party/<dep>/LICENSE*).
#
# Embedded-notice deps (no separate file in upstream tarball):
#   - nlohmann/json — MIT notice rides in the SPDX header of
#     include/nlohmann/json.hpp (always shipped with sce_base headers).
#   - lua          — MIT notice rides in third_party/lua/src/lua.h's
#     copyright comment block (always shipped with the lua54 sources).
# Both satisfy MIT §1 by virtue of the source/header itself being
# distributed; do not fabricate LICENSE files for them.

# SCE itself — the dual LGPL-2.1+exception / Commercial covenant. All
# six are always shipped: the linking-exception text and the commercial
# branch text are *both* load-bearing because the choice of branch
# belongs to the consumer, not the upstream.
set(SCE_LICENSE_FILES_SCE
    LICENSE
    LICENSE-LGPL-2.1.md
    LICENSE-EXCEPTION.md
    LICENSE-COMMERCIAL.md
    LICENSE-GENERATED.md
    LICENSE-THIRD-PARTY.md
)

# Third-party deps that link unconditionally into sce_base (Core tier).
# pugixml is statically linked into sce_base via XMLDOMWrapper.
set(SCE_LICENSE_FILES_THIRD_PARTY_CORE
    third_party/pugixml/LICENSE.md
)

# Gated by SCE_ENABLE_MESH (still Core tier — sce_mesh_common ships
# under Core when the option is on; tinycbor is PRIVATE-linked into it).
set(SCE_LICENSE_FILES_THIRD_PARTY_MESH
    third_party/tinycbor/LICENSE
)

# Gated by SCE_HAS_SCRIPTING (Scripting tier). quickjs is the only
# scripting dep with a standalone LICENSE; lua54's MIT notice rides
# embedded in lua.h (see file header).
set(SCE_LICENSE_FILES_THIRD_PARTY_SCRIPTING
    third_party/quickjs/LICENSE
)

# Gated by SCE_ENABLE_HTTP (Runtime tier). cpp-httplib is PUBLIC-linked
# into sce_runtime when HTTP is on.
set(SCE_LICENSE_FILES_THIRD_PARTY_RUNTIME
    third_party/cpp-httplib/LICENSE
)

# Gated by SCE_USE_SPDLOG (Core tier — SpdlogBackend lives in sce_base).
# Mirrors the embed packager's --with-spdlog flag.
set(SCE_LICENSE_FILES_THIRD_PARTY_SPDLOG
    third_party/spdlog/LICENSE
)
