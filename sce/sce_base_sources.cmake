# sce_base_sources.cmake — Single Source of Truth for sce_base file list
#
# Included by both sce/CMakeLists.txt (in-tree) and embed CMakeLists.txt (vendored).
# Also read by scripts/package_embed.sh to know which files to copy.
#
# To add a new file: edit ONLY this file. All consumers pick it up automatically.

set(SCE_BASE_SOURCES
    # Common utilities (engine-agnostic)
    src/common/EventDataHelper.cpp
    src/common/GuardUtils.cpp
    src/common/Logger.cpp
    src/common/UniqueIdGenerator.cpp
    src/common/Uuid.cpp

    # Logger backends
    src/backends/DefaultBackend.cpp

    # Engine-agnostic scripting utilities (work with ScriptResult value types only)
    src/scripting/ScriptResultUtils.cpp
    src/scripting/XMLDOMWrapper.cpp
    src/scripting/SessionRegistry.cpp
    src/scripting/EcmaScriptToLuaTransformer.cpp

    # Runtime utilities (engine-agnostic)
    src/runtime/TypeRegistry.cpp
    src/runtime/JsonUtils.cpp
    # W3C SCXML B.2 text helpers (normalizeWhitespace, isXMLContent).
    # Pure string manipulation with no engine/runtime dependency;
    # referenced from sce_scripting's LuaEngine.cpp and
    # DataModelInitializer, so it lives in sce_base to keep the
    # dependency direction sce_scripting → sce_base unidirectional.
    src/runtime/DataContentHelpers.cpp
)

# SpdlogBackend is conditional — not in the base list
set(SCE_BASE_SPDLOG_SOURCES
    src/backends/SpdlogBackend.cpp
)

# Header directories required for sce_core + sce_base consumers
# Used by package_embed.sh to know which include/ subdirectories to copy.
set(SCE_BASE_INCLUDE_DIRS
    common
    core
    static
    events
    scripting
    runtime
    backends
    model
    actions
    states
)
