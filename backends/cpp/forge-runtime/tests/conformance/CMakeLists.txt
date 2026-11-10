# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Cross-language numerical conformance harness (C++ half).
#
# Standalone CMake project. Configure and build with:
#
#   cargo build --bin sce-codegen --features cli -p sce-build
#   cmake -S backends/cpp/forge-runtime/tests/conformance -B build_conformance_cpp
#   cmake --build build_conformance_cpp
#   ctest --test-dir build_conformance_cpp --output-on-failure
#
# Asks sce-codegen at configure time for the plan of each generation
# (cmake/SCECodegenPlan.cmake), and generates each fixture's C++ header at
# build time into ${CMAKE_CURRENT_BINARY_DIR}/generated/, which is added to
# the test target's include path. The test binary then links against
# sce_forge_runtime (header-only, INTERFACE library) and nlohmann::json
# (header-only, vendored under third_party) to compare generated output
# against tests/forge/conformance/numerical_reference.json.

cmake_minimum_required(VERSION 3.20)
project(sce_forge_numerical_conformance CXX)

# RFC §synth-7 A6: the §synth-5-A algorithm kind joins this harness beside the
# baseline kinds. C++17 is the standard a consumer compiles the generated C++ at
# (docs/adr/0059, section 1), so the harness compiles every fixture there. Per
# RFC §synth-5-J-5 a `bytes` or list parameter is the runtime's
# `SCE::Forge::Span` (sce/forge/span.h, the same type at C++17 and C++20), an
# aggregate is initialized by position and a record compares member by member,
# so no fixture needs C++20. A test below whose own source is C++20 says so on
# its target.
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(REFERENCE_JSON "${REPO_ROOT}/tests/forge/conformance/numerical_reference.json")
set(FIXTURE_MANIFEST "${REPO_ROOT}/tests/forge/conformance/fixtures.json")
set(CONFORMANCE_TEMPLATE_DIR "${REPO_ROOT}/tools/codegen/templates/forge/cpp/conformance")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/backends/cpp/forge-runtime/include")
set(NLOHMANN_JSON_INCLUDE_DIR "${REPO_ROOT}/third_party/nlohmann_json/include")

# Rebuild sce-codegen from the current sce-build sources when the Rust
# toolchain is available (local dev). Cargo's incremental build makes this
# a near-instant no-op when nothing has changed; the value is eliminating
# the stale-binary foot-gun where a schema change in conformance.rs would
# otherwise be silently ignored by a C++ test invoking the old binary.
#
# The `file(GLOB_RECURSE ... CONFIGURE_DEPENDS)` call re-runs this glob at
# build time and re-runs configure — and thus the cargo build below — when
# its RESULT changes: a .rs file added or removed. Editing one does not
# reach it. CMake's own generated check is a list comparison
# (`if(NOT "${NEW_GLOB}" STREQUAL "${OLD_GLOB}")`, reporting "files were
# added" / "files were removed"), so content is never consulted.
#
# This comment claimed the opposite until 2026-08-13 — "whenever any .rs
# file changes" — which is the interesting half: the stale-binary foot-gun
# it names was therefore NOT closed here for the ordinary case of editing
# the generator. What closes it is the content witness in
# `cmake/SCEFindCodegen.cmake`, which compares a digest of the generator's
# sources against the one the binary was stamped with, at configure and
# again at build time. The glob's remaining value is narrow and real: a new
# .rs file re-runs the cargo build without a manual reconfigure.
#
# In CI the conformance-cpp job downloads a pre-built artifact from the
# build-codegen job and has no Rust toolchain; `find_program` returns
# NOTFOUND and the configure-time rebuild is skipped, falling through to
# the existence check.
find_program(CARGO_EXECUTABLE cargo)
if(CARGO_EXECUTABLE)
    file(GLOB_RECURSE _sce_build_sources CONFIGURE_DEPENDS
        "${REPO_ROOT}/sce-build/src/*.rs"
        "${REPO_ROOT}/sce-build/Cargo.toml"
    )
    execute_process(
        COMMAND "${CARGO_EXECUTABLE}" build
                --bin sce-codegen
                --features cli
                -p sce-build
        WORKING_DIRECTORY "${REPO_ROOT}"
        RESULT_VARIABLE _cargo_rc
    )
    if(NOT _cargo_rc EQUAL 0)
        message(FATAL_ERROR
            "cargo build of sce-codegen failed (rc=${_cargo_rc})"
        )
    endif()
endif()

# Every generation this project builds from is declared from the
# generator's plan of it (`sce_codegen_generation`, cmake/SCECodegenPlan.cmake),
# as the tree declares its own: a fixture's document, a document set compiled
# together (`orchestrate`) and the harness (`generate-conformance`) alike. The
# files each writes, its depfile and its one command are the plan's answer,
# and no file name is written here. The module resolves the generator through
# cmake/SCEFindCodegen.cmake, which every harness finds it by and which aborts
# with the build command when no profile holds a binary.
include("${REPO_ROOT}/cmake/SCECodegenPlan.cmake")

# Every program this project builds is a test program, and takes the warning
# classes every test program turns off from the one definition the C++ test tree
# reads too (cmake/SCETestWarnings.cmake, which says why each is off). Set for
# the directory, before the first target, so no target below can miss it; each
# target's blanket -Werror stays as it sets it.
include("${REPO_ROOT}/cmake/SCETestWarnings.cmake")
add_compile_options(${SCE_TEST_WARNING_OPTIONS})

if(NOT EXISTS "${REFERENCE_JSON}")
    message(FATAL_ERROR "reference JSON not found at ${REFERENCE_JSON}")
endif()

if(NOT EXISTS "${NLOHMANN_JSON_INCLUDE_DIR}/nlohmann/json.hpp")
    message(FATAL_ERROR
        "nlohmann/json not found at ${NLOHMANN_JSON_INCLUDE_DIR}"
    )
endif()

# Pull the fixture catalog from sce-codegen itself in the cmake-list format
# (single semicolon-separated literal). The Rust binary already owns the
# manifest schema, so cmake never has to touch JSON or shell out to python.
#
# RFC §synth-5-B B2-test-vector: passing `--language cpp` activates the
# content-aware filter (`lang_supports_fixture`) so fixtures whose
# SCXML carries `<sce:test-vector>` are dropped on backends that have
# not yet shipped the sidecar emitter. Each closure widens the
# supported set as it lands; until then the cpp harness skips the
# gated fixture instead of surfacing a hard codegen error.
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
            --language cpp
            --resource-dir "${RESOURCE_DIR}"
    OUTPUT_VARIABLE FIXTURES
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _manifest_rc
)
if(NOT _manifest_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures --language cpp failed for ${FIXTURE_MANIFEST}")
endif()

# Each fixture's document, in the same order: a path under RESOURCE_DIR, or
# the `sce:std/...` name of a standard document, which `generate` reads from
# the library it embeds. Asked rather than spelled here, so this build does
# not know where a fixture's document lives.
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
            --language cpp
            --resource-dir "${RESOURCE_DIR}"
            --documents
    OUTPUT_VARIABLE FIXTURE_DOCUMENTS
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _documents_rc
)
if(NOT _documents_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures --documents --language cpp failed for ${FIXTURE_MANIFEST}")
endif()

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

# Step 1: per-fixture product code generation. What a generation writes is
# its plan's answer: a fixture whose document carries `<sce:test-vector>`
# rows (RFC §synth-5-B B2-test-vector) writes a `<fixture>_test.h` sidecar
# beside its header, which the harness includes and whose failure count it
# folds into `g_failures`, and its plan names it. Every fixture is generated
# into one directory, so they claim in one list: a file two generations would
# write is the first's.
set(GENERATED_HEADERS "")
list(LENGTH FIXTURES _fixture_count)
foreach(_fixture_index RANGE ${_fixture_count})
    if(_fixture_index EQUAL _fixture_count)
        break()
    endif()
    list(GET FIXTURES ${_fixture_index} fixture)
    list(GET FIXTURE_DOCUMENTS ${_fixture_index} scxml)
    sce_codegen_generation(_FIXTURE
        STEM "${fixture}"
        DOCUMENT "${scxml}"
        OUTPUT_DIR "${GENERATED_DIR}"
        LANGUAGE cpp
        CLAIM SCE_FORGE_CONFORMANCE_CPP_DECLARED_OUTPUTS
        COMMENT "Generating C++ fixture ${fixture}")
    list(APPEND GENERATED_HEADERS ${_FIXTURE_HEADERS})
endforeach()

# Step 2: render the test harness itself from the shared template, a
# generation of the fixture catalog (`generate-conformance`). The scaffold
# pulls in per-kind fragments through a dynamic Jinja2 include and folds every
# fixture document under tests/forge/resources into the harness
# `source-hash`: all of it is what the generator's depfile and its plan name.
sce_codegen_generation(_HARNESS
    MANIFEST "${FIXTURE_MANIFEST}"
    OUTPUT_DIR "${GENERATED_DIR}"
    LANGUAGE cpp
    CLAIM SCE_FORGE_CONFORMANCE_CPP_DECLARED_OUTPUTS
    COMMENT "Generating C++ conformance harness from template")

# Every fixture's header and the harness, for the programs that include
# them to wait for.
add_custom_target(generate_forge_fixtures_cpp)
target_sources(generate_forge_fixtures_cpp PRIVATE ${GENERATED_HEADERS} ${_HARNESS_FILES})

# The harness is the one file its generation writes, a C++ translation unit.
add_executable(numerical_conformance_test ${_HARNESS_FILES})
add_dependencies(numerical_conformance_test generate_forge_fixtures_cpp)

target_include_directories(numerical_conformance_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${NLOHMANN_JSON_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)

target_compile_definitions(numerical_conformance_test PRIVATE
    "REFERENCE_JSON_PATH=\"${REFERENCE_JSON}\""
)

target_compile_options(numerical_conformance_test PRIVATE
    # ⚠ BLANKET -Werror, not a hand-listed set of classes.
    #
    # This used to promote exactly three classes — narrowing,
    # unused-parameter, type-limits — chosen because each had caught a real
    # codegen defect. The reasoning was sound and the shape was not: a
    # hand-enumerated list silently admits every class nobody has been bitten
    # by yet.
    #
    # `-Wparentheses` was one of those. The C++ emitter left `&&` bare inside
    # `||` (precedence-correct, so `child_needs_parens` saw no need), this
    # target warned and passed, and the header only failed when a real
    # component was generated and fed to a downstream product build that does
    # use `-Wall -Werror`:
    #
    #     error: suggest parentheses around '&&' within '||'
    #            [-Werror=parentheses]
    #
    # The C backend has always used blanket `-Wall -Wextra -Werror -Wpedantic`
    # (backends/c/tests/CMakeLists.txt); C++ was the asymmetric one.
    #
    # ⚠⚠ The flip was MEASURED, not assumed: all 162 generated fixture headers
    # were compiled standalone under `-std=gnu++20 -Wall -Wextra -Wpedantic
    # -Werror`, and ZERO tripped a warning class. The five that failed did so
    # on include context this standalone harness does not provide (cross-file
    # imports, the interpreter's core/ headers), not on diagnostics. The flip
    # therefore costs nothing and closes the class rather than one member of
    # it.
    -Wall -Wextra -Wpedantic -Werror
)

enable_testing()
add_test(NAME numerical_conformance COMMAND numerical_conformance_test)

# The CBOR maps whose encode refuses a value no type of theirs holds back
# (docs/adr/0081 step 4): a negative value of an enum whose carrier is signed, a
# list of fewer records than its lower count, and a record whose byte string is
# past its bound. The documents are sce-build's CBOR fixtures, with the enum and
# the event schemas they import, and are generated into a directory of their
# own, as the caller-tag variants are.
set(CBOR_REFUSING_FIXTURE_DIR "${REPO_ROOT}/sce-build/tests/fixtures/cbor_codec")
set(CBOR_REFUSING_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/cbor_refusing_headers")
file(MAKE_DIRECTORY "${CBOR_REFUSING_GENERATED_DIR}")
set(CBOR_REFUSING_DOCUMENTS
    "${CBOR_REFUSING_FIXTURE_DIR}/cbor_causal_entry.scxml"
    "${CBOR_REFUSING_FIXTURE_DIR}/cbor_digest_entry.scxml"
    "${CBOR_REFUSING_FIXTURE_DIR}/cbor_signed_status.scxml"
    "${CBOR_REFUSING_FIXTURE_DIR}/codec_cbor_signed_enum.scxml"
    "${CBOR_REFUSING_FIXTURE_DIR}/codec_cbor_record_list_least.scxml"
    "${CBOR_REFUSING_FIXTURE_DIR}/codec_cbor_record_list_bytes.scxml"
)
set(CBOR_REFUSING_GENERATED_HEADERS "")
foreach(_cbor_refusing_document IN LISTS CBOR_REFUSING_DOCUMENTS)
    get_filename_component(_cbor_refusing_fixture "${_cbor_refusing_document}" NAME_WE)
    sce_codegen_generation(_CBOR_REFUSING
        DOCUMENT "${_cbor_refusing_document}"
        OUTPUT_DIR "${CBOR_REFUSING_GENERATED_DIR}"
        LANGUAGE cpp
        CLAIM SCE_CBOR_REFUSING_CPP_DECLARED_OUTPUTS
        COMMENT "Generating C++ CBOR fixture ${_cbor_refusing_fixture}")
    list(APPEND CBOR_REFUSING_GENERATED_HEADERS ${_CBOR_REFUSING_HEADERS})
endforeach()
add_custom_target(generate_cbor_refusing_fixtures_cpp)
target_sources(generate_cbor_refusing_fixtures_cpp PRIVATE ${CBOR_REFUSING_GENERATED_HEADERS})

# A printed codec's encode over a consumer's own types (docs/adr/0081, step 4),
# at C++17, the standard the interface is for (docs/adr/0059 section 3), and
# under the same blanket -Werror: which types are sources is asserted at compile
# time, and that each writes the generated value's bytes at run time. It reads
# headers the harness's generation writes, so each codec it uses is a fixture,
# and the CBOR maps whose encode refuses, from the directory of their own above.
add_executable(codec_source_generated_test "${CMAKE_CURRENT_SOURCE_DIR}/codec_source_generated_test.cpp")
add_dependencies(codec_source_generated_test generate_forge_fixtures_cpp generate_cbor_refusing_fixtures_cpp)
set_target_properties(codec_source_generated_test PROPERTIES CXX_STANDARD 17)
target_include_directories(codec_source_generated_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
    "${CBOR_REFUSING_GENERATED_DIR}"
)
target_compile_options(codec_source_generated_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME codec_source_generated COMMAND codec_source_generated_test)

# The runtime's CBOR items on their own (sce/forge/cbor.h), under the same
# blanket -Werror; what a generated CBOR codec does with them is judged by
# the harness above (codec_cbor_map).
add_executable(cbor_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/cbor_runtime_test.cpp")
target_include_directories(cbor_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(cbor_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME cbor_runtime COMMAND cbor_runtime_test)

# The runtime's content lines on their own (sce/forge/content_line.h), under the
# same blanket -Werror; what a generated content-line codec does with them is
# judged by the harness above (codec_content_line_event).
add_executable(content_line_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/content_line_runtime_test.cpp")
target_include_directories(content_line_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(content_line_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME content_line_runtime COMMAND content_line_runtime_test)

# The removal of a text template's fence markers on its own
# (sce/forge/text_template.h, docs/adr/0087 section 3.1), under the same
# blanket -Werror; what a generated template unit does with it is judged by the
# rendering goldens of sce-build and the lane's scenario.
add_executable(text_template_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/text_template_runtime_test.cpp")
target_include_directories(text_template_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(text_template_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME text_template_runtime COMMAND text_template_runtime_test)

# The runtime's span on its own (sce/forge/span.h, docs/adr/0059 section 4),
# at C++17, the standard it is for, and under the same blanket -Werror: what
# converts to a view is asserted at compile time, what a view reads at run
# time. What a generated function does with one is judged by the harness above.
add_executable(span_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/span_runtime_test.cpp")
set_target_properties(span_runtime_test PROPERTIES CXX_STANDARD 17)
target_include_directories(span_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(span_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME span_runtime COMMAND span_runtime_test)

# The runtime's bounded list on its own (sce/forge/bounded.h, docs/adr/0077
# section 3, docs/adr/0081 step 4), at C++17 and under the same blanket -Werror:
# what converts to what is asserted at compile time, what `make` admits and what
# a held list reads at run time. What a generated value presents through it is
# judged by codec_source_generated_test and the harness above.
add_executable(bounded_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/bounded_runtime_test.cpp")
set_target_properties(bounded_runtime_test PROPERTIES CXX_STANDARD 17)
target_include_directories(bounded_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(bounded_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME bounded_runtime COMMAND bounded_runtime_test)

# The runtime's lawful list on its own (sce/forge/law.h, docs/adr/0065 section 6
# R8, docs/adr/0081 step 4), at C++17 and under the same blanket -Werror: what
# converts to what is asserted at compile time, what `make` admits, the start a
# list made by default follows and what a held list reads at run time. What a
# generated value presents through it is judged by codec_source_generated_test
# and the harness above.
add_executable(law_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/law_runtime_test.cpp")
set_target_properties(law_runtime_test PROPERTIES CXX_STANDARD 17)
target_include_directories(law_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(law_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME law_runtime COMMAND law_runtime_test)

# The runtime's chains refined by the kinds of their entries on their own
# (sce/forge/membership.h, docs/adr/0065 section 6 R6, docs/adr/0081 step 4), at
# C++17 and under the same blanket -Werror: what converts to what is asserted at
# compile time, what `make` admits and what a held chain reads at run time. What
# a generated value presents through them is judged by
# codec_source_generated_test and the harness above.
add_executable(membership_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/membership_runtime_test.cpp")
set_target_properties(membership_runtime_test PROPERTIES CXX_STANDARD 17)
target_include_directories(membership_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(membership_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME membership_runtime COMMAND membership_runtime_test)

# The runtime's wrapping and saturating integer helpers on their own
# (sce/forge/algorithm.h, docs/adr/0063) against their definition, at C++17,
# the standard the interface is for, and under the same blanket -Werror with the
# conversion warnings a bare-metal consumer builds with: the helpers convert only
# where they say so. The call shape the generator writes is evaluated at compile
# time there too. What a generated algorithm does with them is the conformance
# fixtures' (docs/adr/0063, section 9).
add_executable(total_arith_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/total_arith_runtime_test.cpp")
set_target_properties(total_arith_runtime_test PROPERTIES CXX_STANDARD 17)
target_include_directories(total_arith_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(total_arith_runtime_test PRIVATE
    -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion)
add_test(NAME total_arith_runtime COMMAND total_arith_runtime_test)

# The runtime's queues on their own (sce/forge/queue.h, RFC §synth-5-P), under
# the same blanket -Werror: the contract scenarios the Rust arm runs
# (tests/forge/conformance/queue_contract.json), the properties those do not
# reach, and real threads. The library itself starts none; the test does, so it
# links the platform's thread library.
find_package(Threads REQUIRED)
set(QUEUE_CONTRACT_JSON "${REPO_ROOT}/tests/forge/conformance/queue_contract.json")
if(NOT EXISTS "${QUEUE_CONTRACT_JSON}")
    message(FATAL_ERROR "queue contract not found at ${QUEUE_CONTRACT_JSON}")
endif()
add_executable(queue_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/queue_runtime_test.cpp")
# The test itself is C++20 (`constinit` is how it proves a queue is constant-
# initialised); it is SCE's own source, which docs/adr/0059 section 2 lets use
# C++20, while the runtime headers it includes are held to C++17 by the gate.
target_compile_features(queue_runtime_test PRIVATE cxx_std_20)
target_include_directories(queue_runtime_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${NLOHMANN_JSON_INCLUDE_DIR}"
)
target_compile_definitions(queue_runtime_test PRIVATE
    "QUEUE_CONTRACT_JSON_PATH=\"${QUEUE_CONTRACT_JSON}\""
)
target_compile_options(queue_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(queue_runtime_test PRIVATE Threads::Threads)
add_test(NAME queue_runtime COMMAND queue_runtime_test)

# What the generator writes for a queue, compiled against that runtime. The
# element codec and one queue per bounded algorithm row come from fixtures the
# numerical catalog does not list (they assert no numerical oracle value), and
# are generated into the same directory the catalog's headers are, so the
# queues' `#include "queue_conformance_event.h"` resolves; they claim in the
# catalog's list. Each is declared from its plan, as every other generation
# is.
set(QUEUE_GENERATED_FIXTURES
    queue_conformance_event
    queue_conformance_spsc
    queue_conformance_scq
    queue_conformance_intrusive
    queue_conformance_intrusive_many
    queue_conformance_segmented
    queue_conformance_segmented_many
)
set(QUEUE_GENERATED_HEADERS "")
foreach(_queue_fixture IN LISTS QUEUE_GENERATED_FIXTURES)
    sce_codegen_generation(_QUEUE_FIXTURE
        DOCUMENT "${RESOURCE_DIR}/${_queue_fixture}.scxml"
        OUTPUT_DIR "${GENERATED_DIR}"
        LANGUAGE cpp
        CLAIM SCE_FORGE_CONFORMANCE_CPP_DECLARED_OUTPUTS
        COMMENT "Generating C++ queue fixture ${_queue_fixture}")
    list(APPEND QUEUE_GENERATED_HEADERS ${_QUEUE_FIXTURE_HEADERS})
endforeach()
add_custom_target(generate_queue_fixtures_cpp)
target_sources(generate_queue_fixtures_cpp PRIVATE ${QUEUE_GENERATED_HEADERS})

add_executable(queue_generated_test "${CMAKE_CURRENT_SOURCE_DIR}/queue_generated_test.cpp")
add_dependencies(queue_generated_test generate_queue_fixtures_cpp)
# The test itself is C++20 (`constinit`, and designated initializers for its
# own elements), as `queue_runtime_test` is above; the generated queue headers
# it includes are held to C++17 by the gate (docs/adr/0059, section 7).
target_compile_features(queue_generated_test PRIVATE cxx_std_20)
target_include_directories(queue_generated_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)
target_compile_options(queue_generated_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(queue_generated_test PRIVATE Threads::Threads)
add_test(NAME queue_generated COMMAND queue_generated_test)

# A variant arm compiled under a build feature (docs/adr/0072), compiled and run
# in both builds. The two variants are the documents the Rust arm runs
# (backends/rust/forge-conformance/fixtures), and their arm bodies come from
# tests/forge/resources. They are generated into a directory of their own, so
# the arm bodies the numerical catalog also lists (codec_peek_arm_a/b) are not
# two rules' output in `GENERATED_DIR`. One source is built twice: as it is,
# where the headers define SCE_FEATURE_GATED_ARM to 1, and with the macro
# defined to 0, where the gated arms are not alternatives and their tags are
# refused.
set(GATED_ARM_FIXTURE_DIR "${REPO_ROOT}/backends/rust/forge-conformance/fixtures")
set(GATED_ARM_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/feature_gated_arm_headers")
file(MAKE_DIRECTORY "${GATED_ARM_GENERATED_DIR}")
set(GATED_ARM_DOCUMENTS
    "${RESOURCE_DIR}/codec_default_marker_arm_a.scxml"
    "${RESOURCE_DIR}/codec_default_marker_arm_b.scxml"
    "${RESOURCE_DIR}/codec_peek_arm_a.scxml"
    "${RESOURCE_DIR}/codec_peek_arm_b.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_feature_gated_arm.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_feature_gated_arm_no_catch_all.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_gated_held_a.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_gated_held_b.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_feature_gated_arm_held.scxml"
)
set(GATED_ARM_GENERATED_HEADERS "")
foreach(_gated_document IN LISTS GATED_ARM_DOCUMENTS)
    get_filename_component(_gated_fixture "${_gated_document}" NAME_WE)
    sce_codegen_generation(_GATED_ARM
        DOCUMENT "${_gated_document}"
        OUTPUT_DIR "${GATED_ARM_GENERATED_DIR}"
        LANGUAGE cpp
        CLAIM SCE_GATED_ARM_CPP_DECLARED_OUTPUTS
        COMMENT "Generating C++ feature-gated arm fixture ${_gated_fixture}")
    list(APPEND GATED_ARM_GENERATED_HEADERS ${_GATED_ARM_HEADERS})
endforeach()
add_custom_target(generate_feature_gated_arm_fixtures_cpp)
target_sources(generate_feature_gated_arm_fixtures_cpp PRIVATE ${GATED_ARM_GENERATED_HEADERS})

foreach(_gated_build IN ITEMS on off)
    set(_gated_target "feature_gated_arm_${_gated_build}_test")
    add_executable(${_gated_target} "${CMAKE_CURRENT_SOURCE_DIR}/feature_gated_arm_generated_test.cpp")
    add_dependencies(${_gated_target} generate_feature_gated_arm_fixtures_cpp)
    target_include_directories(${_gated_target} PRIVATE
        "${RUNTIME_INCLUDE_DIR}"
        "${GATED_ARM_GENERATED_DIR}"
    )
    target_compile_options(${_gated_target} PRIVATE -Wall -Wextra -Wpedantic -Werror)
    if(_gated_build STREQUAL "off")
        target_compile_definitions(${_gated_target} PRIVATE SCE_FEATURE_GATED_ARM=0)
    endif()
    add_test(NAME feature_gated_arm_${_gated_build} COMMAND ${_gated_target})
endforeach()

# A variant whose caller hands its tag, printed from its plan (docs/adr/0081
# step 4), and the codecs that hold one, compiled and run against the frames
# the Rust arm holds them to. The documents are the Rust arm's
# (backends/rust/forge-conformance/fixtures) with the codecs they import, which
# are generated into a directory of their own as the feature-gated arms are.
set(CALLER_TAG_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/caller_tag_variant_headers")
file(MAKE_DIRECTORY "${CALLER_TAG_GENERATED_DIR}")
set(CALLER_TAG_DOCUMENTS
    "${RESOURCE_DIR}/codec_variant_session_open.scxml"
    "${RESOURCE_DIR}/codec_variant_session_close.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_source_leaf.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_source_parent_tag.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_every_arm_says_near.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_every_arm_says_far.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_every_arm_says.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_every_arm_says_parent.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_some_arms_say_plain.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_some_arms_say.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_some_arms_say_parent.scxml"
)
set(CALLER_TAG_GENERATED_HEADERS "")
foreach(_caller_tag_document IN LISTS CALLER_TAG_DOCUMENTS)
    get_filename_component(_caller_tag_fixture "${_caller_tag_document}" NAME_WE)
    sce_codegen_generation(_CALLER_TAG
        DOCUMENT "${_caller_tag_document}"
        OUTPUT_DIR "${CALLER_TAG_GENERATED_DIR}"
        LANGUAGE cpp
        CLAIM SCE_CALLER_TAG_CPP_DECLARED_OUTPUTS
        COMMENT "Generating C++ caller-tag variant fixture ${_caller_tag_fixture}")
    list(APPEND CALLER_TAG_GENERATED_HEADERS ${_CALLER_TAG_HEADERS})
endforeach()
add_custom_target(generate_caller_tag_variant_fixtures_cpp)
target_sources(generate_caller_tag_variant_fixtures_cpp PRIVATE ${CALLER_TAG_GENERATED_HEADERS})
add_executable(caller_tag_variant_generated_test "${CMAKE_CURRENT_SOURCE_DIR}/caller_tag_variant_generated_test.cpp")
add_dependencies(caller_tag_variant_generated_test generate_caller_tag_variant_fixtures_cpp)
set_target_properties(caller_tag_variant_generated_test PROPERTIES CXX_STANDARD 17)
target_include_directories(caller_tag_variant_generated_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${CALLER_TAG_GENERATED_DIR}"
)
target_compile_options(caller_tag_variant_generated_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME caller_tag_variant_generated COMMAND caller_tag_variant_generated_test)

# Derived numbers whose encode can refuse, printed from their plan
# (docs/adr/0081 step 4): a length two byte strings share, one the value states
# where its gated content is absent, and the encoded length of a body that can
# refuse, in a field that may not hold it; an enum a group holds, with the enum
# it imports; and a gate on a chain's membership no group holds, with the
# chain's entry codec; and an input a body says in some of its values only,
# handed on from the codec that holds it, with the codecs of the body's arms;
# and a chain that hands each entry an input from the entry before.
# The test holds them to the frames and refusals the Rust
# encode gives the same values.
set(POSITIONAL_FORMS_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/positional_forms_headers")
file(MAKE_DIRECTORY "${POSITIONAL_FORMS_GENERATED_DIR}")
set(POSITIONAL_FORMS_DOCUMENTS
    "${RESOURCE_DIR}/codec_length_shared.scxml"
    "${RESOURCE_DIR}/codec_length_stated.scxml"
    "${RESOURCE_DIR}/codec_encoded_length_checked.scxml"
    "${RESOURCE_DIR}/enum_session_state.scxml"
    "${RESOURCE_DIR}/codec_group_enum.scxml"
    "${RESOURCE_DIR}/codec_chain_has_tagged_entry.scxml"
    "${RESOURCE_DIR}/codec_chain_has_input.scxml"
    "${RESOURCE_DIR}/codec_chain_handed_entry.scxml"
    "${RESOURCE_DIR}/codec_chain_handed.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_every_arm_says_near.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_some_arms_say_plain.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_some_arms_say.scxml"
    "${GATED_ARM_FIXTURE_DIR}/codec_some_arms_say_forward.scxml"
)
set(POSITIONAL_FORMS_GENERATED_HEADERS "")
foreach(_positional_document IN LISTS POSITIONAL_FORMS_DOCUMENTS)
    get_filename_component(_positional_fixture "${_positional_document}" NAME_WE)
    sce_codegen_generation(_POSITIONAL_FORMS
        DOCUMENT "${_positional_document}"
        OUTPUT_DIR "${POSITIONAL_FORMS_GENERATED_DIR}"
        LANGUAGE cpp
        CLAIM SCE_POSITIONAL_FORMS_CPP_DECLARED_OUTPUTS
        COMMENT "Generating C++ positional-form fixture ${_positional_fixture}")
    list(APPEND POSITIONAL_FORMS_GENERATED_HEADERS ${_POSITIONAL_FORMS_HEADERS})
endforeach()
add_custom_target(generate_positional_forms_fixtures_cpp)
target_sources(generate_positional_forms_fixtures_cpp PRIVATE ${POSITIONAL_FORMS_GENERATED_HEADERS})
add_executable(positional_forms_generated_test "${CMAKE_CURRENT_SOURCE_DIR}/positional_forms_generated_test.cpp")
add_dependencies(positional_forms_generated_test generate_positional_forms_fixtures_cpp)
set_target_properties(positional_forms_generated_test PROPERTIES CXX_STANDARD 17)
target_include_directories(positional_forms_generated_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${POSITIONAL_FORMS_GENERATED_DIR}"
)
target_compile_options(positional_forms_generated_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME positional_forms_generated COMMAND positional_forms_generated_test)

# The queues on real threads, written as histories and judged by the command
# every backend's histories are judged by (verification layer 2). The recorder
# runs the runtime's queues and writes one history per run; the second test
# hands the files to `sce-codegen check-queue-history`, so this arm carries no
# linearizability checker of its own. 4 Lamport capacities recorded 3 times,
# 6 SCQ shapes, 5 shapes of the intrusive list and 5 of the segmented queues
# recorded 25 times: 412 histories.
add_executable(queue_history_recorder "${CMAKE_CURRENT_SOURCE_DIR}/queue_history_recorder.cpp")
target_include_directories(queue_history_recorder PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(queue_history_recorder PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(queue_history_recorder PRIVATE Threads::Threads)
set(QUEUE_HISTORIES_DIR "${CMAKE_CURRENT_BINARY_DIR}/queue_histories")
add_test(NAME queue_history_record COMMAND queue_history_recorder "${QUEUE_HISTORIES_DIR}")
set_tests_properties(queue_history_record PROPERTIES FIXTURES_SETUP queue_histories)
add_test(NAME queue_history_check
    COMMAND "${CMAKE_COMMAND}"
            "-DCODEGEN=${SCE_CODEGEN}"
            "-DDIR=${QUEUE_HISTORIES_DIR}"
            -DMINIMUM=412
            -P "${REPO_ROOT}/tests/forge/conformance/check_queue_histories.cmake")
set_tests_properties(queue_history_check PROPERTIES FIXTURES_REQUIRED queue_histories)

# The same script over the corpus of histories sce-build keeps, which holds
# histories that are not linearizable: it must refuse them, saying so. A check
# that could only pass would prove nothing about the histories the arm wrote.
add_test(NAME queue_history_check_refuses
    COMMAND "${CMAKE_COMMAND}"
            "-DCODEGEN=${SCE_CODEGEN}"
            "-DDIR=${REPO_ROOT}/sce-build/tests/fixtures/queue_histories"
            -DMINIMUM=1
            -P "${REPO_ROOT}/tests/forge/conformance/check_queue_histories.cmake")
set_tests_properties(queue_history_check_refuses PROPERTIES
    PASS_REGULAR_EXPRESSION "queue histories refused")

# The broadcast-log kind (ADR 0062). The runtime's log on its own
# (sce/forge/broadcast_log.h), under the same blanket -Werror: the unit tests of
# the Rust row, the properties the C++ row adds, and real threads. The test is
# C++20 for `constinit`, which is how it proves a log is constant-initialised; the
# header it includes is held to C++17 by the gate.
add_executable(broadcast_log_runtime_test "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_runtime_test.cpp")
target_compile_features(broadcast_log_runtime_test PRIVATE cxx_std_20)
target_include_directories(broadcast_log_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(broadcast_log_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_runtime_test PRIVATE Threads::Threads)
add_test(NAME broadcast_log_runtime COMMAND broadcast_log_runtime_test)

# What the generator writes for a broadcast log, compiled against that runtime. A
# log is generated from its document SET and not document by document: an entry
# is sized from the element document, which only the set compile reads, and the
# second log's retention is a deploy key, resolved against the one machine of the
# deploy written below (`event_retention: 5`). The two element codecs and the two
# logs come from fixtures the numerical catalog does not list, and are generated
# into the same directory the catalog's headers are, so the logs'
# `#include "broadcast_log_conformance_event.h"` resolves; it claims in the
# catalog's list. One generation writes all four, so the outputs are one rule's,
# and every document it reads is a dependency of it, by its plan and its
# depfile.
set(BROADCAST_LOG_GENERATED_FIXTURES
    broadcast_log_conformance_event
    broadcast_log_conformance_record
    broadcast_log_conformance_literal
    broadcast_log_conformance_deployed
)
set(BROADCAST_LOG_DEPLOY "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_deploy.yaml")
sce_codegen_write_input("${BROADCAST_LOG_DEPLOY}"
"version: \"1.0\"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        limits:
          event_retention: 5
")
set(BROADCAST_LOG_DOCUMENTS "")
foreach(_log_fixture IN LISTS BROADCAST_LOG_GENERATED_FIXTURES)
    list(APPEND BROADCAST_LOG_DOCUMENTS "${RESOURCE_DIR}/${_log_fixture}.scxml")
endforeach()
sce_codegen_generation(_BROADCAST_LOG
    DOCUMENTS ${BROADCAST_LOG_DOCUMENTS}
    OUTPUT_DIR "${GENERATED_DIR}"
    LANGUAGE cpp
    DEPLOY "${BROADCAST_LOG_DEPLOY}"
    CLAIM SCE_FORGE_CONFORMANCE_CPP_DECLARED_OUTPUTS
    ARGS --target-machine mcu_node
    COMMENT "Generating C++ broadcast-log fixtures (one document set)")
add_custom_target(generate_broadcast_log_fixtures_cpp)
target_sources(generate_broadcast_log_fixtures_cpp PRIVATE ${_BROADCAST_LOG_HEADERS})

add_executable(broadcast_log_generated_test "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_generated_test.cpp")
add_dependencies(broadcast_log_generated_test generate_broadcast_log_fixtures_cpp)
# The test itself is C++20 (designated initializers for its own elements), as
# `queue_generated_test` is above; the generated headers it includes are held to
# C++17 by the gate (docs/adr/0059, section 7).
target_compile_features(broadcast_log_generated_test PRIVATE cxx_std_20)
target_include_directories(broadcast_log_generated_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)
target_compile_options(broadcast_log_generated_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_generated_test PRIVATE Threads::Threads)
add_test(NAME broadcast_log_generated COMMAND broadcast_log_generated_test)

# The logs over an element whose codec asks its caller for what its wire form does
# not carry (ADR 0062 section 4, "Caller inputs are part of the element"): one over a
# codec with a flag input, one over a variant that has no tag of its own. The six
# documents are the ones every backend's conformance lane reads, in one set, in the
# order that puts what a document imports before it; `orchestrate` reads a set from
# the one directory they are in. The headers go to a directory of their own, so the
# names of these six do not meet the other logs' in `GENERATED_DIR`.
set(BLOG_CONTEXT_DIR "${REPO_ROOT}/tests/forge/conformance/broadcast_log_context")
set(BLOG_CONTEXT_CPP_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_context_headers")
file(MAKE_DIRECTORY "${BLOG_CONTEXT_CPP_GENERATED_DIR}")
set(BLOG_CONTEXT_CPP_SET
    broadcast_log_conformance_flagged_event
    broadcast_log_conformance_flagged
    broadcast_log_conformance_choice_small
    broadcast_log_conformance_choice_large
    broadcast_log_conformance_choice
    broadcast_log_conformance_chosen)
set(BLOG_CONTEXT_CPP_DOCUMENTS "")
foreach(_document IN LISTS BLOG_CONTEXT_CPP_SET)
    list(APPEND BLOG_CONTEXT_CPP_DOCUMENTS "${BLOG_CONTEXT_DIR}/${_document}.scxml")
endforeach()
sce_codegen_generation(_BLOG_CONTEXT_CPP
    DOCUMENTS ${BLOG_CONTEXT_CPP_DOCUMENTS}
    OUTPUT_DIR "${BLOG_CONTEXT_CPP_GENERATED_DIR}"
    LANGUAGE cpp
    COMMENT "Generating C++ broadcast logs over elements that ask their caller for inputs")
add_custom_target(generate_broadcast_log_context_headers_cpp)
target_sources(generate_broadcast_log_context_headers_cpp PRIVATE ${_BLOG_CONTEXT_CPP_HEADERS})

add_executable(broadcast_log_context_generated_test
    "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_context_generated_test.cpp")
add_dependencies(broadcast_log_context_generated_test generate_broadcast_log_context_headers_cpp)
# SCE's own source, as `broadcast_log_generated_test` is; the generated headers it
# includes are held to C++17 by the gate (docs/adr/0059, section 7).
target_compile_features(broadcast_log_context_generated_test PRIVATE cxx_std_20)
target_include_directories(broadcast_log_context_generated_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${BLOG_CONTEXT_CPP_GENERATED_DIR}"
)
target_compile_options(broadcast_log_context_generated_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_context_generated_test PRIVATE Threads::Threads)
add_test(NAME broadcast_log_context_generated COMMAND broadcast_log_context_generated_test)

# The narrow row of the broadcast log (ADR 0062, "Stamp width"), for a target whose atomics are
# 32 bits wide or only loads and stores. The runtime's row on its own
# (sce/forge/broadcast_log_narrow.h) in two programs, under the same blanket -Werror and at
# C++17, the standard the header is held to: a stamp of 3 bits, so that the head's wrap is a few
# publishes away (the unit tests of the Rust lane `Narrow<3>`), and the stamp a row keeps, 32
# bits, with real threads. The two widths are lanes of one class template, so they could be one
# program; they are two to keep the wrap cases apart from the thread stress, as the C11 arm does.
add_executable(broadcast_log_narrow_runtime_test
    "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_narrow_runtime_test.cpp")
target_include_directories(broadcast_log_narrow_runtime_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(broadcast_log_narrow_runtime_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME broadcast_log_narrow_runtime COMMAND broadcast_log_narrow_runtime_test)

add_executable(broadcast_log_narrow_runtime_32_test
    "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_narrow_runtime_32_test.cpp")
target_include_directories(broadcast_log_narrow_runtime_32_test PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(broadcast_log_narrow_runtime_32_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_narrow_runtime_32_test PRIVATE Threads::Threads)
add_test(NAME broadcast_log_narrow_runtime_32 COMMAND broadcast_log_narrow_runtime_32_test)

# What the generator writes for a log whose deploy names a target without 64-bit atomics. The
# generator chooses the row from the target of the machine's device, and judges the log against
# the stamp the row keeps, so the deploy also states the longest pause in publishes the design
# relies on. Two sets, one for each way the row hands out its one publisher: a target with a
# 32-bit compare-and-swap (thumbv7em), and a target with loads and stores only (thumbv6m), whose
# header says to take the publisher by ownership. Each log is derived from the literal document,
# as the C11 arm's is, and generated beside a copy of its element document, because
# `orchestrate` reads a set from one directory.
set(BLOGN_CPP_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_narrow_headers")
file(MAKE_DIRECTORY "${BLOGN_CPP_GENERATED_DIR}")
file(READ "${RESOURCE_DIR}/broadcast_log_conformance_literal.scxml" _blogn_literal)

set(BLOGN_CPP_NARROW_DOCS_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_narrow_documents")
file(MAKE_DIRECTORY "${BLOGN_CPP_NARROW_DOCS_DIR}")
configure_file("${RESOURCE_DIR}/broadcast_log_conformance_record.scxml"
               "${BLOGN_CPP_NARROW_DOCS_DIR}/broadcast_log_conformance_record.scxml" COPYONLY)
string(REPLACE "name=\"broadcast_log_conformance_literal\"" "name=\"broadcast_log_cpp_narrow\"" _blogn_text "${_blogn_literal}")
string(REPLACE "<sce:element-type>broadcast_log_conformance_event</sce:element-type>"
       "<sce:element-type>broadcast_log_conformance_record</sce:element-type>" _blogn_text "${_blogn_text}")
sce_codegen_write_input("${BLOGN_CPP_NARROW_DOCS_DIR}/broadcast_log_cpp_narrow.scxml" "${_blogn_text}")
sce_codegen_write_input("${BLOGN_CPP_NARROW_DOCS_DIR}/deploy.yaml" [=[
version: "1.0"
topology:
  ecu1:
    target: thumbv7em-none-eabihf
    machines:
      mcu_node:
        source: mcu_node.scxml
        queues:
          broadcast_log_cpp_narrow:
            min_wrap_ops: 1000000
]=])
# The same deploy without the pause: the target says the stamp wraps, and the generator
# refuses the log rather than guess a bound.
sce_codegen_write_input("${BLOGN_CPP_NARROW_DOCS_DIR}/deploy_unstated.yaml" [=[
version: "1.0"
topology:
  ecu1:
    target: thumbv7em-none-eabihf
    machines:
      mcu_node:
        source: mcu_node.scxml
]=])

set(BLOGN_CPP_OWNERSHIP_DOCS_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_ownership_documents")
file(MAKE_DIRECTORY "${BLOGN_CPP_OWNERSHIP_DOCS_DIR}")
configure_file("${RESOURCE_DIR}/broadcast_log_conformance_event.scxml"
               "${BLOGN_CPP_OWNERSHIP_DOCS_DIR}/broadcast_log_conformance_event.scxml" COPYONLY)
string(REPLACE "name=\"broadcast_log_conformance_literal\"" "name=\"broadcast_log_cpp_ownership\"" _blogn_text "${_blogn_literal}")
sce_codegen_write_input("${BLOGN_CPP_OWNERSHIP_DOCS_DIR}/broadcast_log_cpp_ownership.scxml" "${_blogn_text}")
sce_codegen_write_input("${BLOGN_CPP_OWNERSHIP_DOCS_DIR}/deploy.yaml" [=[
version: "1.0"
topology:
  ecu1:
    target: thumbv6m-none-eabi
    machines:
      mcu_node:
        source: mcu_node.scxml
        queues:
          broadcast_log_cpp_ownership:
            min_wrap_ops: 1000000
]=])

# Both sets are generated into one directory, and claim in one list.
sce_codegen_generation(_BLOGN_CPP_NARROW
    DOCUMENTS "${BLOGN_CPP_NARROW_DOCS_DIR}/broadcast_log_conformance_record.scxml"
              "${BLOGN_CPP_NARROW_DOCS_DIR}/broadcast_log_cpp_narrow.scxml"
    OUTPUT_DIR "${BLOGN_CPP_GENERATED_DIR}"
    LANGUAGE cpp
    DEPLOY "${BLOGN_CPP_NARROW_DOCS_DIR}/deploy.yaml"
    CLAIM SCE_BLOGN_CPP_DECLARED_OUTPUTS
    ARGS --target-machine mcu_node
    COMMENT "Generating the C++ broadcast log of a target with a 32-bit compare-and-swap")
sce_codegen_generation(_BLOGN_CPP_OWNERSHIP
    DOCUMENTS "${BLOGN_CPP_OWNERSHIP_DOCS_DIR}/broadcast_log_conformance_event.scxml"
              "${BLOGN_CPP_OWNERSHIP_DOCS_DIR}/broadcast_log_cpp_ownership.scxml"
    OUTPUT_DIR "${BLOGN_CPP_GENERATED_DIR}"
    LANGUAGE cpp
    DEPLOY "${BLOGN_CPP_OWNERSHIP_DOCS_DIR}/deploy.yaml"
    CLAIM SCE_BLOGN_CPP_DECLARED_OUTPUTS
    ARGS --target-machine mcu_node
    COMMENT "Generating the C++ broadcast log of a target with loads and stores only")
add_custom_target(generate_broadcast_log_narrow_headers_cpp)
target_sources(generate_broadcast_log_narrow_headers_cpp PRIVATE
    ${_BLOGN_CPP_NARROW_HEADERS} ${_BLOGN_CPP_OWNERSHIP_HEADERS})

add_executable(broadcast_log_narrow_generated_test
    "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_narrow_generated_test.cpp")
add_dependencies(broadcast_log_narrow_generated_test generate_broadcast_log_narrow_headers_cpp)
# SCE's own source (designated initializers for its elements), as `broadcast_log_generated_test`
# is; the generated headers it includes are held to C++17 by the gate (docs/adr/0059, section 7).
target_compile_features(broadcast_log_narrow_generated_test PRIVATE cxx_std_20)
target_include_directories(broadcast_log_narrow_generated_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${BLOGN_CPP_GENERATED_DIR}"
)
target_compile_options(broadcast_log_narrow_generated_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME broadcast_log_narrow_generated COMMAND broadcast_log_narrow_generated_test)

# What the generator refuses of a narrow log is as much its contract as what it writes: the same
# set with no pause stated is refused, naming the key the deploy would state it under.
add_test(NAME broadcast_log_narrow_unstated_pause
    COMMAND "${SCE_CODEGEN}" orchestrate
            --forge "${BLOGN_CPP_NARROW_DOCS_DIR}/broadcast_log_conformance_record.scxml"
            --forge "${BLOGN_CPP_NARROW_DOCS_DIR}/broadcast_log_cpp_narrow.scxml"
            --language cpp --output-dir "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_narrow_refused"
            --deploy "${BLOGN_CPP_NARROW_DOCS_DIR}/deploy_unstated.yaml" --target-machine mcu_node)
set_tests_properties(broadcast_log_narrow_unstated_pause PROPERTIES
    PASS_REGULAR_EXPRESSION "queues.broadcast_log_cpp_narrow.min_wrap_ops")

# The narrow row over an element whose codec asks its caller for what its wire form does not
# carry: the six documents of the context set again, generated for a machine whose device is
# built for a target with 32-bit atomics, with the pause stated for each of the two logs. The
# headers go to a directory of their own, so the names of these six do not meet the other
# narrow logs' above, and the 64-bit set's of the same names.
set(BLOGN_CONTEXT_CPP_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_narrow_context_headers")
file(MAKE_DIRECTORY "${BLOGN_CONTEXT_CPP_GENERATED_DIR}")
sce_codegen_write_input("${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_narrow_context_deploy.yaml" [=[
version: "1.0"
topology:
  ecu1:
    target: thumbv7em-none-eabihf
    machines:
      mcu_node:
        source: mcu_node.scxml
        queues:
          broadcast_log_conformance_flagged:
            min_wrap_ops: 1000000
          broadcast_log_conformance_chosen:
            min_wrap_ops: 1000000
]=])
sce_codegen_generation(_BLOGN_CONTEXT_CPP
    DOCUMENTS ${BLOG_CONTEXT_CPP_DOCUMENTS}
    OUTPUT_DIR "${BLOGN_CONTEXT_CPP_GENERATED_DIR}"
    LANGUAGE cpp
    DEPLOY "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_narrow_context_deploy.yaml"
    ARGS --target-machine mcu_node
    COMMENT "Generating C++ narrow broadcast logs over elements that ask their caller for inputs")
add_custom_target(generate_broadcast_log_narrow_context_headers_cpp)
target_sources(generate_broadcast_log_narrow_context_headers_cpp PRIVATE ${_BLOGN_CONTEXT_CPP_HEADERS})

add_executable(broadcast_log_narrow_context_generated_test
    "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_narrow_context_generated_test.cpp")
add_dependencies(broadcast_log_narrow_context_generated_test generate_broadcast_log_narrow_context_headers_cpp)
target_include_directories(broadcast_log_narrow_context_generated_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${BLOGN_CONTEXT_CPP_GENERATED_DIR}"
)
target_compile_options(broadcast_log_narrow_context_generated_test PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME broadcast_log_narrow_context_generated COMMAND broadcast_log_narrow_context_generated_test)

# The log on real threads, written as histories and judged by the command every
# backend's broadcast-log histories are judged by (verification layer 3). The
# recorder writes one history per run; the second test hands the files to
# `sce-codegen check-broadcast-history`, so this arm carries no linearizability
# checker of its own. 5 shapes recorded 20 times: 100 histories.
add_executable(broadcast_log_history_recorder "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_history_recorder.cpp")
target_include_directories(broadcast_log_history_recorder PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(broadcast_log_history_recorder PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_history_recorder PRIVATE Threads::Threads)
set(BROADCAST_HISTORIES_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_histories")
add_test(NAME broadcast_history_record COMMAND broadcast_log_history_recorder "${BROADCAST_HISTORIES_DIR}")
set_tests_properties(broadcast_history_record PROPERTIES FIXTURES_SETUP broadcast_histories)
add_test(NAME broadcast_history_check
    COMMAND "${CMAKE_COMMAND}"
            "-DCODEGEN=${SCE_CODEGEN}"
            "-DDIR=${BROADCAST_HISTORIES_DIR}"
            -DMINIMUM=100
            -P "${REPO_ROOT}/tests/forge/conformance/check_broadcast_histories.cmake")
set_tests_properties(broadcast_history_check PROPERTIES FIXTURES_REQUIRED broadcast_histories)

# The same script over the corpus of histories sce-build keeps, which holds
# histories that are not linearizable: it must refuse them, saying so.
add_test(NAME broadcast_history_check_refuses
    COMMAND "${CMAKE_COMMAND}"
            "-DCODEGEN=${SCE_CODEGEN}"
            "-DDIR=${REPO_ROOT}/sce-build/tests/fixtures/broadcast_histories"
            -DMINIMUM=1
            -P "${REPO_ROOT}/tests/forge/conformance/check_broadcast_histories.cmake")
set_tests_properties(broadcast_history_check_refuses PROPERTIES
    PASS_REGULAR_EXPRESSION "broadcast-log histories refused")
