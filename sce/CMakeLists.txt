# SCE Module: 4-Tier Library Architecture
# sce_core:      Header-only library for AOT standalone usage (templates, concepts, interfaces)
# sce_base:      Compiled base utilities for AOT (Logger, UniqueIdGenerator, etc. — no script engines)
# sce_scripting: Script engines (QuickJS/Lua) and engine-dependent helpers (optional)
# sce_runtime:   Full interpreter with Parser, StateMachine, Actions, Events

# Build options
option(SCE_ENABLE_HTTP "Enable HTTP/BasicHTTPEventProcessor support (requires cpp-httplib)" ON)
option(SCE_ENABLE_MESH "Enable SCE Mesh distributed transport layer (EventQueueBridge, SchedulerConcepts)" OFF)

# XML Parser Infrastructure (Unified)
# All platforms: pugixml (lightweight, W3C compliant, header-only)
set(PUGIXML_INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/pugixml/src)
message(STATUS "PUGIXML_INCLUDE_DIR: ${PUGIXML_INCLUDE_DIR}")

# Script engine options (both optional, at least one required for interpreter/hybrid)
option(SCE_ENABLE_QUICKJS "Enable QuickJS scripting engine support" ON)
option(SCE_ENABLE_LUA "Enable Lua scripting engine support" ON)

# Add QuickJS subdirectory (conditional)
if(SCE_ENABLE_QUICKJS)
    # WASM: Disable QuickJS stack checking (__wasi__ disables runtime stack checks)
    # QuickJS's __builtin_frame_address(0) doesn't work correctly in WASM/Emscripten
    if(EMSCRIPTEN)
        add_compile_definitions(__wasi__)
        message(STATUS "WASM build: QuickJS stack checking disabled (__wasi__)")
    endif()
    add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/quickjs ${CMAKE_CURRENT_BINARY_DIR}/quickjs)
    # qjs is a PUBLIC link dep of sce_scripting, so it belongs in the same
    # tier-scoped export set. quickjs already has its own install(TARGETS qjs
    # EXPORT qjsConfig); CMake permits a target to belong to multiple export
    # sets, and the duplicate ARCHIVE/LIBRARY destinations are idempotent.
    include(GNUInstallDirs)
    install(TARGETS qjs EXPORT SCE_ScriptingTargets
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Scripting
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Scripting
    )
endif()

# Add Lua 5.4 subdirectory (conditional)
if(SCE_ENABLE_LUA)
    add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/lua ${CMAKE_CURRENT_BINARY_DIR}/lua)
    # lua54's own CMakeLists.txt registers it into SCE_ScriptingTargets.
endif()

# Determine if any script engine is available (propagate to parent scope)
if(SCE_ENABLE_QUICKJS OR SCE_ENABLE_LUA)
    set(SCE_HAS_SCRIPTING TRUE)
    set(SCE_HAS_SCRIPTING TRUE PARENT_SCOPE)
else()
    set(SCE_HAS_SCRIPTING FALSE)
    set(SCE_HAS_SCRIPTING FALSE PARENT_SCOPE)
    message(STATUS "SCE: No script engine enabled — only pure static AOT available")
endif()

# ============================================================================
# spdlog Logger Configuration (Optional Dependency)
# ============================================================================
option(SCE_USE_SPDLOG "Use spdlog for logging (default: ON)" ON)

if(SCE_USE_SPDLOG)
    message(STATUS "SCE: Building with spdlog support (header-only mode)")

    # spdlog: header-only mode (no compilation, include-only)
    add_library(spdlog_header_only INTERFACE)
    target_include_directories(spdlog_header_only INTERFACE
        ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/spdlog/include
    )
    target_compile_definitions(spdlog_header_only INTERFACE
        SPDLOG_HEADER_ONLY=1
        SPDLOG_FMT_EXTERNAL=0  # Use bundled fmt (header-only)
    )

    # Create alias for compatibility
    add_library(spdlog::spdlog ALIAS spdlog_header_only)
else()
    message(STATUS "SCE: Building without spdlog (using DefaultBackend)")
endif()

# Add cpp-httplib subdirectory (native builds only, not WASM, HTTP enabled)
if(NOT EMSCRIPTEN AND SCE_ENABLE_HTTP)
    add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/cpp-httplib ${CMAKE_CURRENT_BINARY_DIR}/cpp-httplib)
endif()

# Add nlohmann/json (header-only library)
# Modern C++ JSON library with full C++20 support
set(NLOHMANN_JSON_INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/nlohmann_json/include)

# ============================================================================
# sce_core: Header-only core library for AOT standalone usage
# ============================================================================
# Contains: StaticExecutionEngine, Helper classes, core algorithms, model interfaces
# No .cpp files, no Logger.cpp dependency
# All SCE_LOG_* macros are no-ops when using sce_core standalone
#
# Include scope: Only the root include/ directory is exposed.
# All sce_core headers use qualified includes (e.g., "common/Foo.h", "core/Bar.h")
# which resolve via this single root path. Runtime-only subdirectory flat paths
# (model/, runtime/, scripting/, etc.) are provided by sce_runtime for its consumers.
add_library(sce_core INTERFACE)

target_include_directories(sce_core
    INTERFACE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<BUILD_INTERFACE:${NLOHMANN_JSON_INCLUDE_DIR}>
        $<BUILD_INTERFACE:${PUGIXML_INCLUDE_DIR}>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>
)

target_compile_features(sce_core INTERFACE cxx_std_17)

# ============================================================================
# sce_mesh_common: Mesh transport abstractions + wire envelope (optional)
# ============================================================================
# Headers: SchedulerConcepts, EventQueueBridge, ShmChannel, PatternKind,
#          PayloadCodec, RpcStatus, MeshEnvelope, MeshEnvelopeCodec.
# Compiled: MeshEnvelopeCodec.cpp (CBOR encode/decode using vendored tinycbor).
# STATIC (not INTERFACE) because MeshEnvelopeCodec.cpp is a compiled
# unit; consumers (generated transport routing, mesh integration tests) link
# the static target.
if(SCE_ENABLE_MESH)
    add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/tinycbor
                     ${CMAKE_CURRENT_BINARY_DIR}/tinycbor)

    add_library(sce_mesh_common STATIC
        src/mesh/MeshEnvelopeCodec.cpp
    )

    target_include_directories(sce_mesh_common
        PUBLIC
            $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    )

    target_link_libraries(sce_mesh_common
        PUBLIC
            sce_core
        PRIVATE
            tinycbor
    )

    # POSIX shared memory requires -lrt on Linux (shm_open, shm_unlink).
    if(NOT APPLE)
        target_link_libraries(sce_mesh_common PUBLIC rt)
    endif()

    target_compile_features(sce_mesh_common PUBLIC cxx_std_17)

    message(STATUS "SCE: Mesh transport layer enabled (sce_mesh_common)")
endif()

# ============================================================================
# sce_base: Compiled base utilities for AOT (no script engine dependency)
# ============================================================================
# Contains: Logger, UniqueIdGenerator, ScriptResultUtils, EventDataHelper, etc.
# These are engine-agnostic compiled utilities needed by AOT generated code.
# Pure static AOT consumers link only sce_base (no QuickJS/Lua).
#
# Source list: sce_base_sources.cmake (SSOT — shared with embed package)
include(${CMAKE_CURRENT_SOURCE_DIR}/sce_base_sources.cmake)

add_library(sce_base
    ${SCE_BASE_SOURCES}
    $<$<BOOL:${SCE_USE_SPDLOG}>:${SCE_BASE_SPDLOG_SOURCES}>
    # XMLDOMWrapper.cpp is in ${SCE_BASE_SOURCES} and pulls pugixml symbols
    # directly (`pugi::xml_document`, `pugi::xml_node`). Compiling pugixml
    # into sce_base here makes the link self-contained: any consumer
    # linking sce_base gets XMLDOMWrapper + pugixml resolved transitively,
    # without needing to link sce_runtime just for the parser's
    # `PugiXMLParser.cpp` (which also uses pugixml). One compile unit
    # for pugixml.cpp keeps ODR happy — sce_runtime's target_sources
    # below no longer lists it.
    ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/pugixml/src/pugixml.cpp
)

target_include_directories(sce_base
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/common>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/common>
    # include/scripting is PRIVATE here (sce_base's own .cpp files compile
    # XMLDOMWrapper/SessionRegistry/ScriptResultUtils/EcmaScriptToLuaTransformer
    # whose headers use unqualified includes). It is re-exposed PUBLIC on
    # sce_scripting below so tier-respecting consumers pick it up only when
    # they actually link the scripting tier.
    PRIVATE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/scripting>
)

target_link_libraries(sce_base
    PUBLIC
        sce_core
)

# spdlog includes (optional)
if(SCE_USE_SPDLOG)
    target_include_directories(sce_base PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../third_party/spdlog/include>
    )
    target_compile_definitions(sce_base PUBLIC SCE_USE_SPDLOG)
endif()

# Runtime logging: PRIVATE so the #ifdef SCE_ENABLE_RUNTIME_LOGGING branch in
# include/core/LogMacros.h is only taken when compiling SCE's own libraries.
# External consumers (AOT-generated or embedding code) fall through to the
# no-op branch, avoiding spdlog include requirements at install-tree compile
# time — the spdlog headers are exposed only via BUILD_INTERFACE and are not
# part of the installed tree.
target_compile_definitions(sce_base PRIVATE SCE_ENABLE_RUNTIME_LOGGING)

# Force include DisableStdOut.h for sce_base source files only.
# ExternTemplates.h suppresses implicit std::basic_string<char>
# instantiation so consumers don't drag sce_base archive members via
# std::string weak-symbol resolution — see header for details.
# SHELL: wrapper keeps -include + its argument as two separate tokens
# (target_compile_options otherwise deduplicates the repeated -include).
target_compile_options(sce_base
    PRIVATE
        "SHELL:-include common/ExternTemplates.h"
        "SHELL:-include common/DisableStdOut.h"
)

target_compile_features(sce_base PUBLIC cxx_std_17)

# ============================================================================
# sce_scripting: Script engines and engine-dependent helpers (optional)
# ============================================================================
# Contains: JSEngine, LuaEngine, ScriptEngineProvider, DataModelInitHelper
# Requires at least one script engine (QuickJS or Lua).
# Static hybrid AOT and Interpreter consumers link sce_scripting.
if(SCE_HAS_SCRIPTING)
    add_library(sce_scripting
        # Script engine provider (compile-time engine selection)
        src/scripting/ScriptEngineProvider.cpp

        # Engine-dependent helpers
        src/common/DataModelInitHelper.cpp

        # QuickJS engine (conditional)
        $<$<BOOL:${SCE_ENABLE_QUICKJS}>:src/scripting/JSEngine.cpp>
        $<$<BOOL:${SCE_ENABLE_QUICKJS}>:src/scripting/JSEngineImpl.cpp>
        $<$<BOOL:${SCE_ENABLE_QUICKJS}>:src/scripting/JSExecutionEngineImpl.cpp>
        $<$<BOOL:${SCE_ENABLE_QUICKJS}>:src/scripting/PlatformExecutionHelper.cpp>
        $<$<BOOL:${SCE_ENABLE_QUICKJS}>:src/scripting/DOMBinding.cpp>

        # Lua engine (conditional)
        $<$<BOOL:${SCE_ENABLE_LUA}>:src/scripting/LuaEngine.cpp>
        $<$<BOOL:${SCE_ENABLE_LUA}>:src/scripting/LuaDOMBinding.cpp>
    )

    # W3C SCXML B.2: Generate json_builtins_lua.h from shared json_builtins.lua
    # Single Source of Truth: shared with Rust sce-rust-lua (via include_str!)
    set(_JSON_LUA_SRC "${CMAKE_CURRENT_SOURCE_DIR}/include/scripting/json_builtins.lua")
    set(_JSON_LUA_HDR "${CMAKE_CURRENT_BINARY_DIR}/generated/json_builtins_lua.h")
    add_custom_command(
        OUTPUT "${_JSON_LUA_HDR}"
        COMMAND ${CMAKE_COMMAND}
            -D "LUA_SRC=${_JSON_LUA_SRC}"
            -D "HDR_OUT=${_JSON_LUA_HDR}"
            -P "${CMAKE_CURRENT_SOURCE_DIR}/cmake/EmbedLuaScript.cmake"
        DEPENDS "${_JSON_LUA_SRC}"
        COMMENT "Embedding json_builtins.lua as C++ string literal"
    )
    add_custom_target(json_builtins_header DEPENDS "${_JSON_LUA_HDR}")
    add_dependencies(sce_scripting json_builtins_header)

    target_include_directories(sce_scripting
        PUBLIC
            # Scripting flat path — moved here from sce_base so only the
            # scripting tier exposes scripting/* short-name headers
            # (unqualified #include "IScriptEngine.h" etc.).
            $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/scripting>
            $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/scripting>
        PRIVATE
            # Implementation headers (Impl classes in src/)
            $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/src>
            # Generated headers (json_builtins_lua.h)
            $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/generated>
    )

    # Force include DisableStdOut.h for sce_scripting source files only
    target_compile_options(sce_scripting
        PRIVATE
            -include common/DisableStdOut.h
    )

    # Plain target references (no generator-expression wrapper): CMake's
    # install(EXPORT) only rewrites bare targets to their namespaced form
    # (qjs → SCE::qjs) when they appear unwrapped in INTERFACE_LINK_LIBRARIES.
    # A gen-expr such as `$<$<BOOL:ON>:qjs>` is treated as an opaque string
    # and leaks to consumers as `-lqjs`, which their linker then fails to
    # resolve. Splitting into conditional target_link_libraries() calls
    # preserves the same semantics while keeping the export rewrite intact.
    target_link_libraries(sce_scripting PUBLIC sce_base)
    if(SCE_ENABLE_QUICKJS)
        target_link_libraries(sce_scripting PUBLIC qjs)
    endif()
    if(SCE_ENABLE_LUA)
        target_link_libraries(sce_scripting PUBLIC lua54)
    endif()

    # Compile-time script engine selection for ScriptEngineProvider
    set(SCE_SCRIPT_ENGINE "lua" CACHE STRING "Script engine for ScriptEngineProvider")
    set_property(CACHE SCE_SCRIPT_ENGINE PROPERTY STRINGS "quickjs" "lua")

    # Normalize and validate engine id
    string(TOLOWER "${SCE_SCRIPT_ENGINE}" _SCE_ENGINE_LOWER)
    set(_SCE_VALID_ENGINES "quickjs" "lua")
    list(FIND _SCE_VALID_ENGINES "${_SCE_ENGINE_LOWER}" _SCE_ENGINE_INDEX)
    if(_SCE_ENGINE_INDEX EQUAL -1)
        message(FATAL_ERROR "Unknown SCE_SCRIPT_ENGINE: '${SCE_SCRIPT_ENGINE}'. Valid engines: ${_SCE_VALID_ENGINES}")
    endif()

    # Per-engine prerequisite validation
    if(_SCE_ENGINE_LOWER STREQUAL "lua" AND NOT SCE_ENABLE_LUA)
        message(FATAL_ERROR "SCE_SCRIPT_ENGINE=lua requires SCE_ENABLE_LUA=ON")
    endif()
    if(_SCE_ENGINE_LOWER STREQUAL "quickjs" AND NOT SCE_ENABLE_QUICKJS)
        message(FATAL_ERROR "SCE_SCRIPT_ENGINE=quickjs requires SCE_ENABLE_QUICKJS=ON")
    endif()

    string(TOUPPER "${_SCE_ENGINE_LOWER}" _SCE_ENGINE_UPPER)
    target_compile_definitions(sce_scripting PUBLIC "SCE_SCRIPT_ENGINE_${_SCE_ENGINE_UPPER}")
    message(STATUS "SCE: Script engine provider default: ${SCE_SCRIPT_ENGINE}")

    # Lua/QuickJS compile definitions
    if(SCE_ENABLE_LUA)
        target_compile_definitions(sce_scripting PUBLIC SCE_ENABLE_LUA)
    endif()
    if(SCE_ENABLE_QUICKJS)
        target_compile_definitions(sce_scripting PUBLIC SCE_ENABLE_QUICKJS)
    endif()

    target_compile_features(sce_scripting PUBLIC cxx_std_20)

    # sce_scripting .cpp files call SCE_LOG_* — add PRIVATE so the logging
    # branch compiles here but does not propagate to external consumers.
    target_compile_definitions(sce_scripting PRIVATE SCE_ENABLE_RUNTIME_LOGGING)
endif()

# ============================================================================
# sce_runtime: Full interpreter with Parser, StateMachine, Actions, Events
# ============================================================================
# Links sce_base + sce_scripting. Provides the complete SCXML interpreter.
# Test executables and interpreter consumers link sce_runtime.
add_library(sce_runtime
    # Model implementation
    src/model/SCXMLModel.cpp
    src/model/StateNode.cpp
    src/model/TransitionNode.cpp
    src/model/GuardNode.cpp
    src/model/InvokeNode.cpp
    src/model/StateHierarchy.cpp
    src/model/DataModelItem.cpp
    src/model/SCXMLContext.cpp

    # Common utilities (interpreter-specific)
    src/common/NodeFactory.cpp

    # Runtime implementation
    src/runtime/SCXMLEngineImpl.cpp
    src/runtime/StateMachine.cpp
    src/runtime/StateMachineContext.cpp
    src/runtime/StateMachineBuilder.cpp
    src/runtime/StateMachineFactory.cpp
    src/runtime/ActionExecutorImpl.cpp
    src/runtime/StateMachineEventRaiser.cpp
    src/runtime/ExecutionContextImpl.cpp
    src/runtime/StateHierarchyManager.cpp
    src/runtime/EventRaiserImpl.cpp
    src/runtime/InvokeExecutor.cpp
    src/runtime/SessionManagerImpl.cpp
    src/runtime/TransitionDomainCalculator.cpp
    src/runtime/DataModelInitializer.cpp

    # History state management (W3C SCXML 3.11)
    src/runtime/HistoryManager.cpp
    src/runtime/HistoryStateAutoRegistrar.cpp
    src/runtime/HistoryValidator.cpp
    src/runtime/StateSnapshot.cpp

    # Actions implementation
    src/actions/BaseAction.cpp
    src/actions/ScriptAction.cpp
    src/actions/AssignAction.cpp
    src/actions/LogAction.cpp
    src/actions/RaiseAction.cpp
    src/actions/IfAction.cpp
    src/actions/SendAction.cpp
    src/actions/CancelAction.cpp
    src/actions/ForeachAction.cpp

    # Events implementation
    src/events/InternalEventTarget.cpp
    src/events/ParentEventTarget.cpp
    src/events/InvokeEventTarget.cpp
    src/events/EventSchedulerImpl.cpp
    src/events/EventDispatcherImpl.cpp
    src/events/EventTargetFactoryImpl.cpp
    src/events/EventRaiserRegistry.cpp
    src/events/EventRaiserService.cpp
    src/events/PlatformEventRaiserHelper.cpp

    # Concurrent states implementation
    src/states/ConcurrentStateNode.cpp
    src/states/ConcurrentRegion.cpp
    src/states/ParallelRegionOrchestrator.cpp
    src/states/ConcurrentEventTypes.cpp
    src/states/ConcurrentEventBroadcaster.cpp
    src/states/ConcurrentCompletionMonitor.cpp
    src/states/ExternalTransitionHandler.cpp
    src/states/StateExitExecutor.cpp

    # Parsing implementation
    src/parsing/StateNodeParser.cpp
    src/parsing/TransitionParser.cpp
    src/parsing/SCXMLParser.cpp
    src/parsing/ActionParser.cpp
    src/parsing/GuardParser.cpp
    src/parsing/InvokeParser.cpp
    src/parsing/DataModelParser.cpp
    src/parsing/DoneDataParser.cpp
    src/parsing/ParsingCommon.cpp
    src/parsing/Diagnostic.cpp
    src/parsing/DiagnosticBatchFormatter.cpp
    src/parsing/PositionMap.cpp
    src/parsing/ParseError.cpp
    src/parsing/SemanticError.cpp
    src/parsing/TemplateError.cpp
    src/parsing/TemplateExpander.cpp
    src/parsing/XIncludeError.cpp
    src/parsing/XIncludeExpander.cpp
    src/parsing/XmlSerializationHelper.cpp

    # XML Parser abstraction (Platform-specific implementation)
    src/parsing/XMLParserFactory.cpp

    # High-level API
    src/ReadySCXMLEngineImpl.cpp
)

# XInclude processing
target_sources(sce_runtime PRIVATE
    src/parsing/XIncludeProcessor.cpp
)

# RFC 3986 percent-encoding. Built unconditionally: the BasicHTTP form encoder
# is one caller, but _ioprocessors location URIs are assembled on every session
# whether or not HTTP support is compiled in, and the encoder itself depends on
# nothing but <string>.
target_sources(sce_runtime PRIVATE
    src/common/UrlEncodingHelper.cpp
)

# HTTP infrastructure (native builds only, not WASM, HTTP enabled)
# W3C SCXML C.2: BasicHTTP Event I/O Processor requires native sockets
if(SCE_ENABLE_HTTP)
    if(NOT EMSCRIPTEN)
        target_sources(sce_runtime PRIVATE
            src/events/HttpResponseUtils.cpp
            src/events/HttpEventTarget.cpp
            src/events/HttpEventReceiver.cpp
            src/events/HttpEventBridge.cpp
            src/events/HttpEventCoordinator.cpp
            src/events/CppHttplibClient.cpp
        )
    else()
        target_sources(sce_runtime PRIVATE
            src/events/HttpEventTarget.cpp
            src/events/HttpClientFactory.cpp
            src/events/EmscriptenFetchClient.cpp
        )
    endif()
endif()

# XML Parser implementation (Unified).
# pugixml.cpp is compiled into sce_base now — see the sce_base
# definition above — so sce_runtime only adds the SCE-side parser
# that wraps pugixml through the IXMLElement interface. The previous
# arrangement (pugixml.cpp here) forced consumers whose only pugixml
# dependency was XMLDOMWrapper.cpp (already in sce_base) to link the
# full sce_runtime umbrella just to resolve pugi:: symbols.
target_sources(sce_runtime PRIVATE
    src/parsing/PugiXMLParser.cpp
)

# Runtime-specific include directories for short (unqualified) includes
# sce_base already provides common/ and scripting/ flat paths.
# sce_runtime adds remaining subdirectory flat paths for cross-directory short includes.
# Flat paths stay available post-install so a find_package(SCE) consumer
# hits the same unqualified-include surface as an in-tree build (e.g.
# factory/NodeFactory.h does `#include "DataModelItem.h"`). Without the
# INSTALL_INTERFACE entries the consumer only sees ${prefix}/include and
# the short-name resolution breaks.
target_include_directories(sce_runtime
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/model>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/runtime>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/factory>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/parsing>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/actions>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/events>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/states>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/dispatchers>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/model>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/runtime>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/factory>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/parsing>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/actions>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/events>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/states>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/dispatchers>
    PRIVATE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/backends>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/src>
)

# Force include DisableStdOut.h for sce_runtime source files only
target_compile_options(sce_runtime
    PRIVATE
        -include common/DisableStdOut.h
)

# sce_runtime links sce_scripting (which transitively provides sce_base → sce_core)
# This makes sce_runtime an umbrella target: linking sce_runtime gives you everything.
if(SCE_HAS_SCRIPTING)
    target_link_libraries(sce_runtime
        PUBLIC
            sce_scripting
        PRIVATE
            pthread
    )
else()
    # No script engine: sce_runtime links sce_base directly (interpreter won't work)
    target_link_libraries(sce_runtime
        PUBLIC
            sce_base
        PRIVATE
            pthread
    )
    message(WARNING "SCE: No script engine enabled. Interpreter and static hybrid will not work.")
endif()

# WASM pthread configuration
# All SCE library tiers must be compiled with pthread/atomics for --shared-memory linking
if(EMSCRIPTEN)
    set(_SCE_PTHREAD_TARGETS sce_base sce_runtime)
    if(SCE_HAS_SCRIPTING)
        list(APPEND _SCE_PTHREAD_TARGETS sce_scripting)
    endif()
    foreach(_target IN LISTS _SCE_PTHREAD_TARGETS)
        target_compile_options(${_target} PRIVATE -pthread -sUSE_PTHREADS=1)
    endforeach()

    target_link_options(sce_runtime PRIVATE
        -pthread
        -sUSE_PTHREADS=1
        -sPTHREAD_POOL_SIZE=15         # Worker thread pool (for ConcurrentEventBroadcaster + EventDispatcherImpl)
        -sPTHREAD_POOL_SIZE_STRICT=0   # Allow on-demand thread creation (Emscripten Issue #21959)
        -sFETCH                         # Enable Fetch API for HTTP requests (EmscriptenFetchClient)
        -sEXIT_RUNTIME=0                # Allow async operations after main() returns
        -sENVIRONMENT=node,web          # Target Node.js and Web environments for Fetch API
        -sASYNCIFY                       # Enable async/await support
        -sASSERTIONS=2                   # Runtime validation for development
        -sINITIAL_MEMORY=${WASM_INITIAL_MEMORY}  # Centrally managed in root CMakeLists.txt
        -sALLOW_MEMORY_GROWTH=0          # Disable runtime heap expansion (use fixed INITIAL_MEMORY)
        -sSTACK_SIZE=2097152             # 2MB stack (reduced to fit within 384MB heap)
        -sDISABLE_EXCEPTION_CATCHING=0   # Enable C++ exception catching (required for tests)
    )
    message(STATUS "WASM build: pthread enabled for EventDispatcherImpl (no PROXY_TO_PTHREAD)")
endif()

# HTTP support (optional feature)
if(SCE_ENABLE_HTTP)
    target_compile_definitions(sce_runtime PUBLIC SCE_ENABLE_HTTP)
    if(NOT EMSCRIPTEN)
        target_link_libraries(sce_runtime PUBLIC httplib::httplib)
    endif()
endif()

target_compile_features(sce_runtime PUBLIC cxx_std_20)

# sce_runtime .cpp files call SCE_LOG_* — add PRIVATE so the logging branch
# compiles here but does not propagate to external consumers.
target_compile_definitions(sce_runtime PRIVATE SCE_ENABLE_RUNTIME_LOGGING)
