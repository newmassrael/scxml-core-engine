# tinycbor 0.6.0 - Built from source (vendored, like Lua/QuickJS)
# Upstream: https://github.com/intel/tinycbor (MIT License, Intel)
#
# Trimmed source set: encoder + parser + error strings only. Float/JSON/pretty
# helpers omitted — MeshEnvelope wire format does not use floats and never
# converts to JSON via tinycbor (we use nlohmann_json for that).

set(TINYCBOR_SOURCES
    src/cborencoder.c
    src/cborencoder_close_container_checked.c
    src/cborparser.c
    src/cborparser_dup_string.c
    src/cborerrorstrings.c
)

add_library(tinycbor STATIC ${TINYCBOR_SOURCES})

target_include_directories(tinycbor
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

# tinycbor 0.6.0 omits the float/half-float APIs when this is set, matching
# our trimmed source list. Removes dangling extern declarations in cbor.h.
target_compile_definitions(tinycbor
    PUBLIC
        WITHOUT_FLOATING_POINT
        WITHOUT_OPEN_MEMSTREAM
)

# Suppress warnings in third-party code (matches lua54/qjs pattern).
if(CMAKE_C_COMPILER_ID MATCHES "Clang|GNU")
    target_compile_options(tinycbor PRIVATE -w)
elseif(MSVC)
    target_compile_options(tinycbor PRIVATE /w)
endif()

# Position-independent (matches project-wide CMAKE_POSITION_INDEPENDENT_CODE).
set_target_properties(tinycbor PROPERTIES POSITION_INDEPENDENT_CODE ON)

message(STATUS "tinycbor 0.6.0: Building from source (static library)")
