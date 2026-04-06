# SCE Module: 4-Tier Library Architecture
# sce_core:      Header-only library for AOT standalone usage (templates, concepts, interfaces)
# sce_base:      Compiled base utilities for AOT (Logger, UniqueIdGenerator, etc. — no script engines)
# sce_scripting: Script engines (QuickJS/Lua) and engine-dependent helpers (optional)
# sce_runtime:   Full interpreter with Parser, StateMachine, Actions, Events

# Build options
option(SCE_ENABLE_HTTP "Enable HTTP/BasicHTTPEventProcessor support (requires cpp-httplib)" ON)

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
endif()

# Add Lua 5.4 subdirectory (conditional)
if(SCE_ENABLE_LUA)
    add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/lua ${CMAKE_CURRENT_BINARY_DIR}/lua)
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
# sce_base: Compiled base utilities for AOT (no script engine dependency)
# ============================================================================
# Contains: Logger, UniqueIdGenerator, ScriptResultUtils, EventDataHelper, etc.
# These are engine-agnostic compiled utilities needed by AOT generated code.
# Pure static AOT consumers link only sce_base (no QuickJS/Lua).
add_library(sce_base
    # Common utilities (engine-agnostic)
    src/common/EventDataHelper.cpp
    src/common/GuardUtils.cpp
    src/common/Logger.cpp
    src/common/UniqueIdGenerator.cpp

    # Logger backends (conditional based on SCE_USE_SPDLOG)
    src/backends/DefaultBackend.cpp
    $<$<BOOL:${SCE_USE_SPDLOG}>:src/backends/SpdlogBackend.cpp>

    # Engine-agnostic scripting utilities (work with ScriptResult value types only)
    src/scripting/ScriptResultUtils.cpp
    src/scripting/XMLDOMWrapper.cpp
    src/scripting/SessionRegistry.cpp
    src/scripting/EcmaScriptToLuaTransformer.cpp

    # Runtime utilities (engine-agnostic)
    src/runtime/TypeRegistry.cpp
    src/runtime/JsonUtils.cpp
)

target_include_directories(sce_base
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/common>
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

# Enable runtime logging for sce_base and all consumers
target_compile_definitions(sce_base PUBLIC SCE_ENABLE_RUNTIME_LOGGING)

# Force include DisableStdOut.h for sce_base source files only
target_compile_options(sce_base
    PRIVATE
        -include common/DisableStdOut.h
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

    target_link_libraries(sce_scripting
        PUBLIC
            sce_base
            $<$<BOOL:${SCE_ENABLE_QUICKJS}>:qjs>
            $<$<BOOL:${SCE_ENABLE_LUA}>:lua54>
    )

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
    src/runtime/DataContentHelpers.cpp
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
            src/common/UrlEncodingHelper.cpp
            src/events/HttpEventTarget.cpp
            src/events/HttpEventReceiver.cpp
            src/events/HttpEventBridge.cpp
            src/events/HttpEventCoordinator.cpp
            src/events/CppHttplibClient.cpp
        )
    else()
        target_sources(sce_runtime PRIVATE
            src/common/UrlEncodingHelper.cpp
            src/events/HttpEventTarget.cpp
            src/events/HttpClientFactory.cpp
            src/events/EmscriptenFetchClient.cpp
        )
    endif()
endif()

# XML Parser implementation (Unified)
target_sources(sce_runtime PRIVATE
    src/parsing/PugiXMLParser.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/pugixml/src/pugixml.cpp
)

# Runtime-specific include directories for short (unqualified) includes
# sce_base already provides common/ and scripting/ flat paths.
# sce_runtime adds remaining subdirectory flat paths for cross-directory short includes.
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
