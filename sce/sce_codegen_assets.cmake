# sce_codegen_assets.cmake — SSOT for Codegen-component utility assets
#
# Single source of truth shared by:
#   - CMakeLists.txt install() rules (Codegen install component)
#   - scripts/package_embed.sh (vendor embed payload)
#
# When adding a new CMake utility or template group to the Codegen
# component, edit this file only — both consumers pick it up
# automatically. Hardcoding a file in package_embed.sh without adding
# it here has historically produced embed-payload drift (missing
# SCEClangFormat.cmake / templates/ in the shipped embed tree), so
# keep the SSOT authoritative.

# CMake utility files shipped with the Codegen install component.
# Paths are relative to the SCE source root.
set(SCE_CODEGEN_CMAKE_FILES
    cmake/SCECodegen.cmake
    cmake/SCEFindCodegen.cmake
    cmake/SCEClangFormat.cmake
)

# Jinja2 template tree shipped with the Codegen install component.
# Source-relative path; same layout is preserved inside embed/.
set(SCE_CODEGEN_TEMPLATE_DIR tools/codegen/templates)
set(SCE_CODEGEN_TEMPLATE_PATTERN "*.jinja2")

# clang-format style file referenced by SCEClangFormat.cmake. Source-
# relative; same layout preserved in embed/ and under the Codegen
# install prefix so auto-detection in SCEClangFormat.cmake resolves in
# all three distribution layouts (in-tree, embed, system install).
set(SCE_CODEGEN_STYLE_FILE tools/codegen/default.clang-format)
