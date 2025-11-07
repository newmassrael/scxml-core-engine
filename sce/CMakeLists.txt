# SCE Unified Module
# Complete SCE implementation with domain models, runtime engine, and scripting

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

# Add cpp-httplib subdirectory (native builds only, not WASM)
if(NOT EMSCRIPTEN)
    add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/cpp-httplib ${CMAKE_CURRENT_BINARY_DIR}/cpp-httplib)
endif()

# Add nlohmann/json (header-only library)
# Modern C++ JSON library with full C++20 support
set(NLOHMANN_JSON_INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/nlohmann_json/include)

add_library(sce_unified
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
    src/common/PlatformExecutionHelper.cpp
    src/common/TypeRegistry.cpp
    src/common/UniqueIdGenerator.cpp
    src/common/JsonUtils.cpp
    src/common/DataModelInitHelper.cpp
    src/common/XmlSerializationHelper.cpp

    # Runtime implementation
    src/runtime/SCXMLEngineImpl.cpp
    src/runtime/StateMachine.cpp
    src/runtime/StateMachineContext.cpp
    src/runtime/StateMachineFactory.cpp
    src/runtime/ActionExecutorImpl.cpp
    src/runtime/StateMachineEventRaiser.cpp
    src/runtime/ExecutionContextImpl.cpp
    src/runtime/StateHierarchyManager.cpp
    src/runtime/EventRaiserImpl.cpp
    src/runtime/InvokeExecutor.cpp
    src/runtime/SessionManagerImpl.cpp
    src/runtime/DataContentHelpers.cpp
    
    # History state management (W3C SCXML 3.11)
    src/runtime/HistoryManager.cpp
    src/runtime/HistoryStateAutoRegistrar.cpp
    src/runtime/HistoryValidator.cpp

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
    src/scripting/XMLDOMWrapper.cpp  # W3C SCXML B.2: XML DOM (pugixml for all platforms)
    src/scripting/DOMBinding.cpp      # QuickJS bindings for XML DOM API

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
target_sources(sce_unified PRIVATE
    src/parsing/XIncludeProcessor.cpp
)

# HTTP infrastructure (native builds only, not WASM)
# W3C SCXML C.2: BasicHTTP Event I/O Processor requires native sockets
if(NOT EMSCRIPTEN)
    target_sources(sce_unified PRIVATE
        src/common/HttpResponseUtils.cpp
        src/common/UrlEncodingHelper.cpp
        src/events/HttpEventTarget.cpp
        src/events/HttpEventReceiver.cpp
        src/events/HttpEventBridge.cpp
        src/events/HttpEventCoordinator.cpp
        src/events/CppHttplibClient.cpp
    )
else()
    target_sources(sce_unified PRIVATE
        src/common/UrlEncodingHelper.cpp
        src/events/HttpEventTarget.cpp
        src/events/HttpClientFactory.cpp
        src/events/EmscriptenFetchClient.cpp
)
endif()

# XML Parser implementation (Unified)
# All platforms: PugiXMLParser (pugixml wrapper)
target_sources(sce_unified PRIVATE
    src/parsing/PugiXMLParser.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/pugixml/src/pugixml.cpp
)

target_include_directories(sce_unified
    PUBLIC
        include
        include/model
        include/runtime
        include/scripting
        include/factory
        include/common
        include/backends
        include/parsing
        include/actions
        include/events
        include/states
        ${NLOHMANN_JSON_INCLUDE_DIR}
)

# Unified XML parser includes (pugixml)
target_include_directories(sce_unified PUBLIC
    ${PUGIXML_INCLUDE_DIR}
)

# Optional spdlog includes (header-only)
if(SCE_USE_SPDLOG)
    target_include_directories(sce_unified PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/../third_party/spdlog/include
    )
endif()


# SCE codebase: Disable std::cout usage
# printf family disabled due to system header conflicts

# Conditional spdlog support (compile definition)
if(SCE_USE_SPDLOG)
    target_compile_definitions(sce_unified PUBLIC SCE_USE_SPDLOG)
endif()

# Force include DisableStdOut.h globally
target_compile_options(sce_unified
    PRIVATE
        -include common/DisableStdOut.h
)

target_link_libraries(sce_unified
    PUBLIC
        qjs  # QuickJS library
        # nlohmann/json is header-only, no linking required
        # spdlog is header-only, no linking required
    PRIVATE
        pthread
)

# Note: pugixml is header-only, no linking required

# WASM pthread configuration for EventDispatcherImpl and JSEngine
# W3C SCXML: EventDispatcherImpl uses std::thread for invoke operations
# ARCHITECTURE.md: JSEngine uses SynchronousExecutionHelper, but EventDispatcherImpl needs pthread
# Note: PROXY_TO_PTHREAD removed to reduce overhead while keeping pthread support
if(EMSCRIPTEN)
    target_compile_options(sce_unified PRIVATE
        -pthread
        -sUSE_PTHREADS=1
    )
    target_link_options(sce_unified PRIVATE
        -pthread
        -sUSE_PTHREADS=1
        -sPTHREAD_POOL_SIZE=4          # Worker thread pool for EventDispatcherImpl
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

# HTTP library (native builds only, not WASM)
if(NOT EMSCRIPTEN)
    target_link_libraries(sce_unified PUBLIC httplib::httplib)
endif()

# Set C++20 standard
target_compile_features(sce_unified
    PUBLIC cxx_std_20
)

