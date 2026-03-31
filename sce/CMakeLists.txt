# SCE Module: sce_core (header-only) + sce_runtime (compiled)
# sce_core: Header-only library for AOT standalone usage (no Logger.cpp dependency)
# sce_runtime: Full compiled library with Interpreter, Parser, JSEngine, Logger

# Build options
option(SCE_ENABLE_HTTP "Enable HTTP/BasicHTTPEventProcessor support (requires cpp-httplib)" ON)

# XML Parser Infrastructure (Unified)
# All platforms: pugixml (lightweight, W3C compliant, header-only)
set(PUGIXML_INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/pugixml/src)
message(STATUS "PUGIXML_INCLUDE_DIR: ${PUGIXML_INCLUDE_DIR}")

# Add QuickJS subdirectory
# WASM: Disable QuickJS stack checking (__wasi__ disables runtime stack checks)
# QuickJS's __builtin_frame_address(0) doesn't work correctly in WASM/Emscripten
if(EMSCRIPTEN)
    add_compile_definitions(__wasi__)
    message(STATUS "WASM build: QuickJS stack checking disabled (__wasi__)")
endif()
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/quickjs ${CMAKE_CURRENT_BINARY_DIR}/quickjs)

# Add Lua 5.4 subdirectory (for LuaEngine support)
option(SCE_ENABLE_LUA "Enable Lua scripting engine support" ON)
if(SCE_ENABLE_LUA)
    add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/lua ${CMAKE_CURRENT_BINARY_DIR}/lua)
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

target_compile_features(sce_core INTERFACE cxx_std_20)

# ============================================================================
# sce_runtime: Full compiled library with Interpreter, Parser, JSEngine, Logger
# ============================================================================
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

    # Common utilities
    src/common/EventDataHelper.cpp
    src/common/GuardUtils.cpp
    src/common/Logger.cpp
    
    # Logger backends (conditional based on SCE_USE_SPDLOG)
    src/backends/DefaultBackend.cpp  # Always compiled
    $<$<BOOL:${SCE_USE_SPDLOG}>:src/backends/SpdlogBackend.cpp>
    src/common/NodeFactory.cpp
    src/common/UniqueIdGenerator.cpp
    src/common/DataModelInitHelper.cpp

    # Moved to their natural subsystem directories
    src/scripting/PlatformExecutionHelper.cpp
    src/runtime/TypeRegistry.cpp
    src/runtime/JsonUtils.cpp
    src/parsing/XmlSerializationHelper.cpp

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

    # Scripting implementation
    src/scripting/JSEngine.cpp
    src/scripting/JSEngineImpl.cpp
    src/scripting/JSExecutionEngineImpl.cpp
    src/scripting/ScriptResultUtils.cpp
    src/scripting/ScriptEngineProvider.cpp
    src/scripting/EcmaScriptToLuaTransformer.cpp
    src/scripting/SessionRegistry.cpp
    src/scripting/XMLDOMWrapper.cpp  # W3C SCXML B.2: XML DOM (pugixml for all platforms)
    src/scripting/DOMBinding.cpp      # QuickJS bindings for XML DOM API

    # Lua scripting engine (conditional)
    $<$<BOOL:${SCE_ENABLE_LUA}>:src/scripting/LuaEngine.cpp>
    $<$<BOOL:${SCE_ENABLE_LUA}>:src/scripting/LuaDOMBinding.cpp>

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

    # XML Parser abstraction (Platform-specific implementation)
    src/parsing/XMLParserFactory.cpp
    
    # High-level API
    src/ReadySCXMLEngineImpl.cpp
)

# XInclude processing (platform-independent with different implementations)
# XIncludeProcessor stub delegates to PugiXMLDocument::processXInclude()  
# WASM: XIncludeProcessor stub delegates to IXMLDocument::processXInclude()
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
# All platforms: PugiXMLParser (pugixml wrapper)
target_sources(sce_runtime PRIVATE
    src/parsing/PugiXMLParser.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/pugixml/src/pugixml.cpp
)

# Runtime-specific include directories for short (unqualified) includes
# sce_core only exposes the root include/ for qualified paths ("common/Foo.h").
# sce_runtime adds subdirectory flat paths so that runtime headers can use
# cross-directory short includes (e.g., #include "IEventTarget.h" in events/).
target_include_directories(sce_runtime
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/model>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/runtime>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/scripting>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/factory>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/common>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/parsing>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/actions>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/events>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/states>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/dispatchers>
    PRIVATE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/backends>
        # Implementation headers (Impl classes) — private to sce_runtime, not installed
        # Only the root src/ is exposed, so Impl headers require qualified includes
        # (e.g., "runtime/EventRaiserImpl.h") preventing accidental unqualified usage
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/src>
)

# spdlog includes (runtime-only dependency)
if(SCE_USE_SPDLOG)
    target_include_directories(sce_runtime PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../third_party/spdlog/include>
    )
endif()


# SCE codebase: Disable std::cout usage
# printf family disabled due to system header conflicts

# Conditional spdlog support (compile definition)
if(SCE_USE_SPDLOG)
    target_compile_definitions(sce_runtime PUBLIC SCE_USE_SPDLOG)
endif()

# Enable runtime logging for sce_runtime and all consumers
# Header-only code uses SCE_LOG_* macros from LogMacros.h which resolve to
# real Logger calls when this is defined, or no-ops when using sce_core standalone
target_compile_definitions(sce_runtime PUBLIC SCE_ENABLE_RUNTIME_LOGGING)

# Force include DisableStdOut.h for sce_runtime source files only
target_compile_options(sce_runtime
    PRIVATE
        -include common/DisableStdOut.h
)

target_link_libraries(sce_runtime
    PUBLIC
        sce_core  # Header-only core (include directories propagated)
        qjs       # QuickJS library
        $<$<BOOL:${SCE_ENABLE_LUA}>:lua54>  # Lua 5.4 library (conditional)
        # nlohmann/json is header-only, no linking required
        # spdlog is header-only, no linking required
    PRIVATE
        pthread
)

# Lua compile definition for conditional compilation
if(SCE_ENABLE_LUA)
    target_compile_definitions(sce_runtime PUBLIC SCE_ENABLE_LUA)
endif()

# Compile-time script engine selection for ScriptEngineProvider
# Controls which engine ScriptEngineProvider::getScriptEngine() returns.
# Both engines may still be compiled (SCE_ENABLE_LUA) for direct injection use cases.
#
# To add a new engine:
#   1. Add engine id to _SCE_VALID_ENGINES and STRINGS list below
#   2. Add prerequisite validation if needed (e.g., SCE_ENABLE_XXX check)
#   3. Add engine singleton + metadata in ScriptEngineProvider.cpp
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

string(TOUPPER "${_SCE_ENGINE_LOWER}" _SCE_ENGINE_UPPER)
target_compile_definitions(sce_runtime PUBLIC "SCE_SCRIPT_ENGINE_${_SCE_ENGINE_UPPER}")
message(STATUS "SCE: Script engine provider default: ${SCE_SCRIPT_ENGINE}")

# Note: pugixml is header-only, no linking required

# WASM pthread configuration for EventDispatcherImpl and JSEngine
# W3C SCXML: EventDispatcherImpl uses std::thread for invoke operations
# ARCHITECTURE.md: JSEngine uses SynchronousExecutionHelper, but EventDispatcherImpl needs pthread
# Note: PROXY_TO_PTHREAD removed to reduce overhead while keeping pthread support
if(EMSCRIPTEN)
    target_compile_options(sce_runtime PRIVATE
        -pthread
        -sUSE_PTHREADS=1
    )
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

# C++20 standard inherited from sce_core INTERFACE, but set explicitly for sce_runtime too
target_compile_features(sce_runtime PUBLIC cxx_std_20)


