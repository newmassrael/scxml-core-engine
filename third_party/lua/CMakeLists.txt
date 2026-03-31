# Lua 5.4.6 - Built from source (like QuickJS)
# Provides static library for all platforms including WASM/Emscripten

# Lua core library sources (excludes lua.c and luac.c which are standalone executables)
set(LUA_SOURCES
    src/lapi.c
    src/lauxlib.c
    src/lbaselib.c
    src/lcode.c
    src/lcorolib.c
    src/lctype.c
    src/ldblib.c
    src/ldebug.c
    src/ldo.c
    src/ldump.c
    src/lfunc.c
    src/lgc.c
    src/linit.c
    src/liolib.c
    src/llex.c
    src/lmathlib.c
    src/lmem.c
    src/loadlib.c
    src/lobject.c
    src/lopcodes.c
    src/loslib.c
    src/lparser.c
    src/lstate.c
    src/lstring.c
    src/lstrlib.c
    src/ltable.c
    src/ltablib.c
    src/ltm.c
    src/lundump.c
    src/lutf8lib.c
    src/lvm.c
    src/lzio.c
)

add_library(lua54 STATIC ${LUA_SOURCES})

target_include_directories(lua54
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

# Platform-specific definitions
if(EMSCRIPTEN)
    # WASM: POSIX subset available via Emscripten, no dlopen
    target_compile_definitions(lua54 PRIVATE LUA_USE_POSIX)
    target_link_libraries(lua54 PRIVATE m)
    # WASM pthread: Required for --shared-memory linking compatibility
    target_compile_options(lua54 PRIVATE -pthread -sUSE_PTHREADS=1)
elseif(WIN32)
    # Windows: No extra definitions needed (Lua defaults to Windows APIs)
elseif(UNIX)
    target_compile_definitions(lua54 PRIVATE LUA_USE_POSIX LUA_USE_DLOPEN)
    target_link_libraries(lua54 PRIVATE m dl)
endif()

# Suppress warnings in third-party code (Lua uses its own coding style)
if(CMAKE_C_COMPILER_ID MATCHES "Clang|GNU")
    target_compile_options(lua54 PRIVATE
        -w
    )
endif()

message(STATUS "Lua 5.4.6: Building from source (static library)")
