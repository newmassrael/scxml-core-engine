# SCE Module: 4-Tier Library Architecture
# sce_core:      Header-only library for AOT standalone usage (templates, concepts, interfaces)
# sce_base:      Compiled base utilities for AOT (Logger, UniqueIdGenerator, etc. — no script engines)
# sce_scripting: Script engines (QuickJS/Lua) and engine-dependent helpers (optional)
# sce_runtime:   Full interpreter with Parser, StateMachine, Actions, Events

include(${CMAKE_CURRENT_SOURCE_DIR}/../cmake/SCEVendoredSubdirectory.cmake)
include(${CMAKE_CURRENT_SOURCE_DIR}/../cmake/SCEForceInclude.cmake)

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
    # Built with QuickJS's own warning flags, not this repository's -Werror
    # (cmake/SCEVendoredSubdirectory.cmake says why).
    sce_add_vendored_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/quickjs ${CMAKE_CURRENT_BINARY_DIR}/quickjs)
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
    sce_add_vendored_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/cpp-httplib ${CMAKE_CURRENT_BINARY_DIR}/cpp-httplib)
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

# The Forge C++ runtime (backends/cpp/forge-runtime) is part of what the
# generated C++ compiles against: a `datamodel="sce-static"` machine includes
# <sce/forge/algorithm.h> and <sce/forge/optional.h>, and a list or byte
# parameter is an SCE::Forge::Span (docs/adr/0059, section 4). Linked here,
# every tier carries its include directory, in the build tree and installed,
# so a consumer compiles generated code by linking the tier it already links.
target_link_libraries(sce_core INTERFACE sce_forge_runtime)

# ============================================================================
# sce_mesh_common: Mesh transport abstractions + wire envelope (optional)
# ============================================================================
# Headers: SchedulerConcepts, EventQueueBridge, ShmChannel, PatternKind,
#          PayloadCodec, RpcStatus, MeshEnvelope, MeshEnvelopeCodec,
#          CausalEntry, CausalRules, CausalLayer, CausalRoute.
# Compiled: MeshEnvelopeCodec.cpp — the envelope's CBOR through the codec
# generated from sce:std/mesh/envelope.scxml (src/mesh/generated/, committed
# so a C++ consumer needs no Rust toolchain), whose CBOR items are the C++
# forge runtime's header-only sce/forge/cbor.h; CausalRules.cpp — the causal
# route's rule through the algorithms generated from sce:std/mesh beside it.
# Both are a private include, since no public header names a generated type.
# STATIC (not INTERFACE) because both are compiled units; consumers
# (generated transport routing, mesh integration tests) link the static
# target.
if(SCE_ENABLE_MESH)
    add_library(sce_mesh_common STATIC
        src/mesh/MeshEnvelopeCodec.cpp
        src/mesh/CausalRules.cpp
        # The client of the `https` binding (docs/adr/0107, D2), over any
        # adapter of the port mesh/transports/HttpsTransport.h. It links no
        # network library: the adapters are targets of their own
        # (cmake/SCEMeshHttps.cmake).
        src/mesh/HttpsClient.cpp
        # Its message signatures (docs/adr/0109, D9). It links no
        # cryptographic library: the primitives are the host's
        # (host/CoseCrypto.h), handed to the client at construction.
        src/mesh/HttpsSignature.cpp
    )

    target_include_directories(sce_mesh_common
        PUBLIC
            $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    )

    # The forge runtime is PUBLIC and named here although `sce_core` carries it
    # too, because two of this target's own public headers name it:
    # EventQueueBridge.h is the forge runtime's SCQ queue, and ShmChannel.h
    # places it in shared memory.
    target_link_libraries(sce_mesh_common
        PUBLIC
            sce_core
            sce_forge_runtime
    )

    # POSIX shared memory requires -lrt on Linux (shm_open, shm_unlink).
    # Windows has neither librt nor POSIX shared memory.
    if(UNIX AND NOT APPLE)
        target_link_libraries(sce_mesh_common PUBLIC rt)
    endif()
    # The https client draws an installation secret with BCryptGenRandom.
    if(WIN32)
        target_link_libraries(sce_mesh_common PRIVATE bcrypt)
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
    # XMLDOMWrapper/SessionRegistry/ScriptResultUtils whose headers use
    # unqualified includes). It is re-exposed PUBLIC on
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

# Force include DisableStdOut.h for sce_base source files only.
# ExternTemplates.h suppresses implicit std::basic_string<char>
# instantiation so consumers don't drag sce_base archive members via
# std::string weak-symbol resolution — see header for details. Spelled for the
# compiler that builds the target (cmake/SCEForceInclude.cmake).
sce_force_include(sce_base PRIVATE common/ExternTemplates.h common/DisableStdOut.h)

target_compile_features(sce_base PUBLIC cxx_std_17)

# ============================================================================
# sce_scripting: Script engines and engine-dependent helpers (optional)
# ============================================================================
# Contains: JSEngine, LuaEngine, ScriptEngineProvider, DataModelInitHelper,
#           EventRaiserService / EventRaiserRegistry
# Requires at least one script engine (QuickJS or Lua).
# Static hybrid AOT and Interpreter consumers link sce_scripting.
#
# ⚠ Depends on sce_base and never on sce_runtime: sce_runtime links this
# tier, so a symbol defined there and called from here is a cycle between
# two static archives. lld resolves archive members in any order and hid
# it; GNU ld reads them in link-line order and refused a consumer that
# pulled no runtime member first (mesh_synth_invoke_override_e2e_test,
# 2026-09-24).
if(SCE_HAS_SCRIPTING)
    add_library(sce_scripting
        # Script engine provider (compile-time engine selection)
        src/scripting/ScriptEngineProvider.cpp

        # Engine-dependent helpers
        src/common/DataModelInitHelper.cpp

        # The session -> EventRaiser registry both engines route a script's
        # raise through. It belongs to this tier: the service validates
        # sessions through ScriptEngineProvider, and the engines call it —
        # defined in sce_runtime, it was a back-edge that only an
        # order-insensitive linker (lld) resolved.
        src/events/EventRaiserRegistry.cpp
        src/events/EventRaiserService.cpp

        # QuickJS engine (conditional)
        $<$<BOOL:${SCE_ENABLE_QUICKJS}>:src/scripting/JSEngine.cpp>
        $<$<BOOL:${SCE_ENABLE_QUICKJS}>:src/scripting/JSEngineImpl.cpp>
        $<$<BOOL:${SCE_ENABLE_QUICKJS}>:src/scripting/PlatformExecutionHelper.cpp>
        $<$<BOOL:${SCE_ENABLE_QUICKJS}>:src/scripting/DOMBinding.cpp>

        # Lua engine (conditional)
        $<$<BOOL:${SCE_ENABLE_LUA}>:src/scripting/LuaEngine.cpp>
        $<$<BOOL:${SCE_ENABLE_LUA}>:src/scripting/LuaDOMBinding.cpp>

        # The one translation unit that calls sce-build's C surface. Gated on
        # the same option as the link below, because the link is its reason to
        # exist; without it the class compiles to a scope that refuses every
        # expression, which is what a wasm build already gets.
        $<$<BOOL:${SCE_ENABLE_LUA}>:src/scripting/LoweringScope.cpp>
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

    # W3C SCXML B.2: the ECMAScript operators Lua does not share, from the
    # same shared source every other backend loads. The generated code calls
    # these by name (`_scxml_add`, `_scxml_eq`, the bitwise family), so an
    # engine that does not load this file runs generated Lua that references
    # undefined globals.
    set(_ECMA_LUA_SRC "${CMAKE_CURRENT_SOURCE_DIR}/include/scripting/ecma_semantics.lua")
    set(_ECMA_LUA_HDR "${CMAKE_CURRENT_BINARY_DIR}/generated/ecma_semantics_lua.h")
    add_custom_command(
        OUTPUT "${_ECMA_LUA_HDR}"
        COMMAND ${CMAKE_COMMAND}
            -D "LUA_SRC=${_ECMA_LUA_SRC}"
            -D "HDR_OUT=${_ECMA_LUA_HDR}"
            -P "${CMAKE_CURRENT_SOURCE_DIR}/cmake/EmbedLuaScript.cmake"
        DEPENDS "${_ECMA_LUA_SRC}"
        COMMENT "Embedding ecma_semantics.lua as C++ string literal"
    )
    add_custom_target(ecma_semantics_header DEPENDS "${_ECMA_LUA_HDR}")
    add_dependencies(sce_scripting ecma_semantics_header)

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
    sce_force_include(sce_scripting PRIVATE common/DisableStdOut.h)

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

        # sce-build's ECMAScript frontend, called from LuaEngine's seam.
        #
        # The owner's decision, 2026-08-29: link it and retire
        # `EcmaScriptToLuaTransformer`. Both halves are done — the
        # rewriter is deleted, and what this frontend refuses is refused
        # rather than handed to a second translator.
        #
        # Every tree takes it, the emscripten trees included: the module
        # builds the staticlib for the tree's own target
        # (wasm32-unknown-emscripten, with atomics when the tree has
        # threads), so there is no tree that evaluates Lua alone and
        # refuses every ECMAScript document by design.
        #
        # PUBLIC rather than PRIVATE: LuaEngine.cpp includes
        # `scripting/SceLowering.h`, which SCE::Lowering carries as an
        # interface include directory, and a consumer building against an
        # installed sce_scripting resolves the same symbols. The function
        # sets the definition and the link together, in one place, so
        # LuaEngine.cpp cannot call a symbol that was not linked.
        include(${CMAKE_CURRENT_SOURCE_DIR}/../cmake/SCEBuildLowering.cmake)
        sce_link_ecmascript_frontend(sce_scripting)
    endif()

    # Compile-time script engine selection for ScriptEngineProvider
    #
    # QuickJS, because the C++ backend does not translate expressions. A
    # generated state machine carries the author's ECMAScript verbatim and so
    # does the Interpreter, so the selected engine IS the semantics of
    # `datamodel="ecmascript"` — which W3C SCXML B.2 requires to be ECMAScript.
    #
    # This defaulted to `lua`, which reaches ECMAScript through a
    # string-rewriting transformer. Measured against the shared ECMA-262 table
    # (`tests/ecmascript/ecma262_semantics.json`), the quickjs selection
    # answers every case and the lua selection does not: `0 && x` is true,
    # `-7 % 3` is 2 and `5 ^ 3` is 125 there, silently, with the whole W3C
    # suite green because no fixture in the corpus asks.
    #
    # How far the lua selection is from the language is NOT stated here. It
    # was, as a hand-counted "26 of 58", and the table then grew to 98 cases
    # while the sentence stayed — the real number that day was 44. It now
    # lives in `tests/ecmascript/lua_engine_divergences.json`, case by case
    # with the clause each one cites, and `ecmascript_semantics_test` fails if
    # the engine diverges anywhere that file does not say it does, or agrees
    # anywhere it says it will not.
    set(SCE_SCRIPT_ENGINE "quickjs" CACHE STRING "Script engine for ScriptEngineProvider")
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

    # ADR 0059: a consumer compiles the interface at C++17; these sources are C++20.
    target_compile_features(sce_scripting PUBLIC cxx_std_17 PRIVATE cxx_std_20)
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
    src/runtime/InterpreterDocument.cpp
    src/runtime/StateMachineContext.cpp
    src/runtime/StateMachineBuilder.cpp
    src/runtime/StateMachineFactory.cpp
    src/runtime/ActionExecutorImpl.cpp
    src/runtime/ExecutionContextImpl.cpp
    src/runtime/StateHierarchyManager.cpp
    src/runtime/EventRaiserImpl.cpp
    src/runtime/InvokeExecutor.cpp
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
    src/actions/NativeAction.cpp
    src/actions/NativeActionContract.cpp

    # Events implementation
    src/events/EventDescriptor.cpp
    src/events/InternalEventTarget.cpp
    src/events/ParentEventTarget.cpp
    src/events/InvokeEventTarget.cpp
    src/events/EventSchedulerImpl.cpp
    src/events/EventDispatcherImpl.cpp
    src/events/EventTargetFactoryImpl.cpp
    src/events/PlatformEventRaiserHelper.cpp

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
        # The fetch client calls emscripten_fetch, which links only under
        # -sFETCH, and waits on a Node request through EM_ASYNC_JS, which
        # needs -sASYNCIFY. They are what this library's own code asks of the
        # link, so the library states them for every program that links it:
        # a program that left -sFETCH out failed to link with emscripten_fetch
        # undefined as soon as it reached the client through the archive.
        target_link_options(sce_runtime INTERFACE -sFETCH -sASYNCIFY)
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
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/dispatchers>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/model>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/runtime>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/factory>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/parsing>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/actions>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/events>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}/dispatchers>
    PRIVATE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/backends>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/src>
)

# Force include DisableStdOut.h for sce_runtime source files only
sce_force_include(sce_runtime PRIVATE common/DisableStdOut.h)

# Identity of the C++ diagnostic producer: the digest of the sources this
# library is built from, which `src/parsing/Diagnostic.cpp` stamps as
# `generator` on every record it emits.
#
# `schemas/sce-diagnostic.v1.schema.json` requires that field, and
# `include/parsing/Diagnostic.h` declares this side conforms to that schema
# field-for-field. The value is a SHA-256 of every file under this directory
# (`sce/`), keyed by its path relative to it, so it is a function of the
# sources alone: two builds of the same sources stamp the same value whatever
# commit, checkout or machine they were made at, and an edit to any of them
# moves it. It was the commit until 2026-11, and that made every commit a new
# `Diagnostic.cpp` object, a new library and a relink of every program linking
# it, with no source changed; the git refs it watched also re-ran this
# configure on every commit. The third-party trees this library compiles are
# pinned by the superproject's submodule records, which the digest does not
# read: reading them needs git and the commit, the dependency this replaced.
#
# Taken at build time, not at configure time: a configure-time digest would
# have to re-run the configure on every edit of any source. The listing is
# written at configure time (the glob re-checks itself on every build, so an
# added or removed file reconfigures), and the custom command hashes the files
# it names whenever one of them changes. `SourceDigest.cmake` rewrites the
# header only when the digest moves, so an edit that leaves it where it was
# recompiles nothing (Ninja re-reads the header's time after the command).
#
# The header goes to a directory only `Diagnostic.cpp` is given, so no other
# translation unit of the target depends on it. Compiled without it (outside
# this project) the file answers `unavailable`, which the schema allows; the
# test `DiagnosticWire.GeneratorStampIsTheSourceDigestOfThisBuild` is what
# tells that apart from a build here that lost the header.
set(_SCE_DIGEST_DIR "${CMAKE_CURRENT_BINARY_DIR}/source_digest")
set(_SCE_DIGEST_HEADER "${_SCE_DIGEST_DIR}/SceRuntimeSourceDigest.h")
set(_SCE_DIGEST_LISTING "${_SCE_DIGEST_DIR}/inputs.txt")
file(GLOB_RECURSE _sce_digest_inputs CONFIGURE_DEPENDS LIST_DIRECTORIES false
    RELATIVE "${CMAKE_CURRENT_SOURCE_DIR}" "${CMAKE_CURRENT_SOURCE_DIR}/*")
# A build tree placed inside this directory holds outputs, the header among
# them, and a digest of its own output would never settle.
file(RELATIVE_PATH _sce_digest_build_tree "${CMAKE_CURRENT_SOURCE_DIR}" "${CMAKE_BINARY_DIR}")
if(_sce_digest_build_tree AND NOT _sce_digest_build_tree MATCHES "^\\.\\./"
   AND NOT IS_ABSOLUTE "${_sce_digest_build_tree}")
    set(_sce_digest_sources "")
    foreach(_sce_digest_input IN LISTS _sce_digest_inputs)
        string(FIND "${_sce_digest_input}" "${_sce_digest_build_tree}/" _sce_digest_at)
        if(NOT _sce_digest_at EQUAL 0)
            list(APPEND _sce_digest_sources "${_sce_digest_input}")
        endif()
    endforeach()
    set(_sce_digest_inputs "${_sce_digest_sources}")
endif()
list(SORT _sce_digest_inputs)
list(JOIN _sce_digest_inputs "\n" _sce_digest_listing)
string(APPEND _sce_digest_listing "\n")
# Written only when it changes, so a reconfigure that found the same files
# does not re-run the digest.
set(_sce_digest_previous "")
if(EXISTS "${_SCE_DIGEST_LISTING}")
    file(READ "${_SCE_DIGEST_LISTING}" _sce_digest_previous)
endif()
if(NOT _sce_digest_previous STREQUAL _sce_digest_listing)
    file(WRITE "${_SCE_DIGEST_LISTING}" "${_sce_digest_listing}")
endif()
list(TRANSFORM _sce_digest_inputs PREPEND "${CMAKE_CURRENT_SOURCE_DIR}/" OUTPUT_VARIABLE _sce_digest_paths)
add_custom_command(
    OUTPUT "${_SCE_DIGEST_HEADER}"
    COMMAND "${CMAKE_COMMAND}"
        -D "SCE_DIGEST_ROOT=${CMAKE_CURRENT_SOURCE_DIR}"
        -D "SCE_DIGEST_LISTING=${_SCE_DIGEST_LISTING}"
        -D "SCE_DIGEST_HEADER=${_SCE_DIGEST_HEADER}"
        -P "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SourceDigest.cmake"
    DEPENDS ${_sce_digest_paths} "${_SCE_DIGEST_LISTING}"
    COMMENT "Taking the digest of the sources of the C++ diagnostic producer"
    VERBATIM
)
target_sources(sce_runtime PRIVATE "${_SCE_DIGEST_HEADER}")
set_source_files_properties(src/parsing/Diagnostic.cpp PROPERTIES
    INCLUDE_DIRECTORIES "${_SCE_DIGEST_DIR}")

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

# WASM: whether the libraries are compiled with pthreads is the tree's one
# setting, SCE_WASM_THREADS in the root list, because it is part of every
# object's ABI. The thread pool, heap, stack, environment and exit behaviour
# are each program's to choose at its own link; a static library is never
# linked, so it states only what its code requires of a link (the fetch
# client's options, above) and nothing of a program's shape.

# HTTP support (optional feature)
if(SCE_ENABLE_HTTP)
    target_compile_definitions(sce_runtime PUBLIC SCE_ENABLE_HTTP)
    if(NOT EMSCRIPTEN)
        target_link_libraries(sce_runtime PUBLIC httplib::httplib)
    endif()
endif()

# ADR 0059: a consumer compiles the interface at C++17; these sources are C++20.
target_compile_features(sce_runtime PUBLIC cxx_std_17 PRIVATE cxx_std_20)
