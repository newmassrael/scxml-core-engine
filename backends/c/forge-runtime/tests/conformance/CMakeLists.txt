# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Cross-language numerical conformance harness (C11 half).
#
# Standalone CMake project. Configure and build with:
#
#   cargo build --bin sce-codegen --features cli -p sce-build
#   cmake -S backends/c/forge-runtime/tests/conformance -B build_conformance_c
#   cmake --build build_conformance_c
#   ctest --test-dir build_conformance_c --output-on-failure
#
# Asks sce-codegen at configure time for the plan of each generation
# (cmake/SCECodegenPlan.cmake), and generates each fixture's C header at
# build time into ${CMAKE_CURRENT_BINARY_DIR}/generated/, which is added
# to the test target's include path. The
# test binary then links against sce_forge_runtime_c (header-only
# INTERFACE library) and runs against pre-baked
# oracle arrays emitted directly by the harness template (C has no
# zero-deps JSON parser within the backend's zero-dependency rule, so
# the oracle is materialised at codegen time, not parsed at runtime).

cmake_minimum_required(VERSION 3.20)
project(sce_forge_numerical_conformance_c C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(REFERENCE_JSON "${REPO_ROOT}/tests/forge/conformance/numerical_reference.json")
set(FIXTURE_MANIFEST "${REPO_ROOT}/tests/forge/conformance/fixtures.json")
set(CONFORMANCE_TEMPLATE_DIR "${REPO_ROOT}/tools/codegen/templates/forge/c/conformance")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/backends/c/forge-runtime/include")
# Tier 1 header tree. Generated forge C headers reach into it for
# `<sce/sample.h>` (ownership contract) and `<sce/portability.h>`
# (`SCE_STATIC_ASSERT`); see backends/c/forge-runtime/CMakeLists.txt.
set(CORE_RUNTIME_INCLUDE_DIR "${REPO_ROOT}/backends/c/runtime/include")

# Rebuild sce-codegen from current sce-build sources when the Rust
# toolchain is available (local dev). Mirrors the cpp version's
# stale-binary safeguard exactly. CI without a Rust toolchain falls
# through to the existence check.
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

if(NOT EXISTS "${REFERENCE_JSON}")
    message(FATAL_ERROR "reference JSON not found at ${REFERENCE_JSON}")
endif()

# Pull the C11 fixture catalog from sce-codegen itself.
# The Rust binary owns the manifest schema and the per-language `kind`
# gate (`c11_supported_kind`), so cmake never has to touch JSON or
# duplicate the kind list — `--language c11` filters to kinds
# whose `kinds/<kind>.c.jinja2` fragment exists,
# matching the filter `generate-conformance --language c11` applies at
# harness render time. Every forge kind has its
# fragment, so today the filter is a no-op identity over the manifest;
# it stays in the toolchain so a future kind addition forces an
# explicit decision in `c11_supported_kind` instead of silently being
# pulled into the harness.
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
            --language c11
            --resource-dir "${RESOURCE_DIR}"
    OUTPUT_VARIABLE C11_FIXTURES
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _manifest_rc
)
if(NOT _manifest_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures --language c11 failed for "
        "${FIXTURE_MANIFEST}")
endif()

# Each fixture's document, in the same order: a path under RESOURCE_DIR, or
# the `sce:std/...` name of a standard document, which `generate` reads from
# the library it embeds. Asked rather than spelled here, so this build does
# not know where a fixture's document lives.
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
            --language c11
            --resource-dir "${RESOURCE_DIR}"
            --documents
    OUTPUT_VARIABLE C11_FIXTURE_DOCUMENTS
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _documents_rc
)
if(NOT _documents_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures --documents --language c11 failed for "
        "${FIXTURE_MANIFEST}")
endif()

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

# What this run's storing forms hold (docs/adr/0077 §7). A document states the
# protocol's bound, and C11 stores every field, so a field with no bound, or one
# no array of this run could hold, takes its capacity from this deploy input.
set(CONFORMANCE_DEPLOY "${REPO_ROOT}/tests/forge/conformance/deploy.yaml")

# Step 1: per-fixture product code generation, each fixture's document
# generated for C11 with the deploy. What a generation writes is its plan's
# answer: a fixture whose document carries `<sce:test-vector>` rows (RFC
# §synth-5-B B2-test-vector) writes a `<fixture>_test.h` sidecar beside its
# header, which the harness includes, and its plan names it. Every fixture is
# generated into one directory, so they claim in one list: a file two
# generations would write is the first's.
set(GENERATED_HEADERS "")
list(LENGTH C11_FIXTURES _fixture_count)
foreach(_fixture_index RANGE ${_fixture_count})
    if(_fixture_index EQUAL _fixture_count)
        break()
    endif()
    list(GET C11_FIXTURES ${_fixture_index} fixture)
    list(GET C11_FIXTURE_DOCUMENTS ${_fixture_index} scxml)
    sce_codegen_generation(_FIXTURE
        STEM "${fixture}"
        DOCUMENT "${scxml}"
        OUTPUT_DIR "${GENERATED_DIR}"
        LANGUAGE c11
        DEPLOY "${CONFORMANCE_DEPLOY}"
        CLAIM SCE_FORGE_CONFORMANCE_C_DECLARED_OUTPUTS
        COMMENT "Generating C11 fixture ${fixture}")
    list(APPEND GENERATED_HEADERS ${_FIXTURE_HEADERS})
endforeach()

# Step 2: render the test harness from the shared template, a generation of
# the fixture catalog (`generate-conformance`). The scaffold pulls in
# per-kind fragments through a dynamic Jinja2 include, folds every fixture
# document under tests/forge/resources into the harness `source-hash`, and
# bakes the oracle of the reference JSON into the source: all of it is what
# the generator's depfile and its plan name.
sce_codegen_generation(_HARNESS
    MANIFEST "${FIXTURE_MANIFEST}"
    OUTPUT_DIR "${GENERATED_DIR}"
    LANGUAGE c11
    CLAIM SCE_FORGE_CONFORMANCE_C_DECLARED_OUTPUTS
    COMMENT "Generating C11 conformance harness from template")

# Every fixture's header and the harness, for the programs that include
# them to wait for.
add_custom_target(generate_forge_fixtures_c)
target_sources(generate_forge_fixtures_c PRIVATE ${GENERATED_HEADERS} ${_HARNESS_FILES})

add_executable(numerical_conformance_test_c ${_HARNESS_SOURCES})
add_dependencies(numerical_conformance_test_c generate_forge_fixtures_c)

target_include_directories(numerical_conformance_test_c PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${CORE_RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)

# Acceptance gate: -Werror with -Wall/-Wextra/-Wpedantic
# is the bar — emitted code must survive without warnings.
target_compile_options(numerical_conformance_test_c PRIVATE
    -Wall -Wextra -Wpedantic -Werror
)

# math.h is included by the harness for fabs-style helpers; on most
# Unix toolchains math is in libc, but glibc requires explicit -lm.
target_link_libraries(numerical_conformance_test_c PRIVATE m)

enable_testing()
add_test(NAME numerical_conformance_c COMMAND numerical_conformance_test_c)

# A value a generated codec holds that may be absent is the struct
# sce/forge/optional.h declares for its element, the one a statechart's variable
# of the same type is held in (docs/adr/0055, section 8): decoded from the
# codec_cbor_map and codec_content_line_event codecs this directory generates, it is
# taken whole by a variable of the runtime's own type, which compiles only where the
# two are one type. The bytes are the harness above's.
add_executable(codec_optional_test_c "${CMAKE_CURRENT_SOURCE_DIR}/codec_optional_test.c")
add_dependencies(codec_optional_test_c generate_forge_fixtures_c)
target_include_directories(codec_optional_test_c PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${CORE_RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)
target_compile_options(codec_optional_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME codec_optional_c COMMAND codec_optional_test_c)

# The encode of a byte string, a text, a list or a TLV chain whose length is above
# the bound of the array that holds it: refused before the first byte is written,
# for each shape of the generated encode (codec_tail, codec_length_ref,
# codec_zenoh_locator, codec_present_if_tail, codec_repeat_basic,
# codec_tlv_chain_basic), and the same length at the bound written whole.
add_executable(codec_encode_bound_test_c "${CMAKE_CURRENT_SOURCE_DIR}/codec_encode_bound_test.c")
add_dependencies(codec_encode_bound_test_c generate_forge_fixtures_c)
target_include_directories(codec_encode_bound_test_c PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${CORE_RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)
target_compile_options(codec_encode_bound_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME codec_encode_bound_c COMMAND codec_encode_bound_test_c)

# The runtime's CBOR items on their own (sce/forge/cbor.h), under the same
# -Werror bar; what a generated CBOR codec does with them is judged by the
# harness above (codec_cbor_map).
add_executable(cbor_runtime_test_c "${CMAKE_CURRENT_SOURCE_DIR}/cbor_runtime_test.c")
target_include_directories(cbor_runtime_test_c PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(cbor_runtime_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME cbor_runtime_c COMMAND cbor_runtime_test_c)

# The cursor's VLE readers on their own (sce/forge/codec.h, RFC Appendix B),
# under the same -Werror bar: the wire is a ZInt64 at every width and the type
# bounds the value. What a generated codec does with them is judged by the
# harness above (codec_vle_zint_u16, _u32 and _u64).
add_executable(vle_runtime_test_c "${CMAKE_CURRENT_SOURCE_DIR}/vle_runtime_test.c")
target_include_directories(vle_runtime_test_c PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(vle_runtime_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME vle_runtime_c COMMAND vle_runtime_test_c)

# The runtime's content lines on their own (sce/forge/content_line.h), under the
# same -Werror bar; what a generated content-line codec does with them is judged
# by the harness above (codec_content_line_event).
add_executable(content_line_runtime_test_c "${CMAKE_CURRENT_SOURCE_DIR}/content_line_runtime_test.c")
target_include_directories(content_line_runtime_test_c PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(content_line_runtime_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME content_line_runtime_c COMMAND content_line_runtime_test_c)

# The writer a text-template unit renders into, and the removal of its fence
# markers (sce/forge/text_template.h, docs/adr/0087 section 3.1), under the
# same -Werror bar.
add_executable(text_template_runtime_test_c "${CMAKE_CURRENT_SOURCE_DIR}/text_template_runtime_test.c")
target_include_directories(text_template_runtime_test_c PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(text_template_runtime_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME text_template_runtime_c COMMAND text_template_runtime_test_c)

# The runtime's wrapping and saturating integer helpers on their own
# (sce/forge/algorithm.h, docs/adr/0063) against their definition, under the
# same -Werror bar and the conversion warnings a bare-metal consumer builds
# with: the helpers convert only where they say so. What a generated algorithm
# does with them is the conformance fixtures' (docs/adr/0063, section 9).
add_executable(total_arith_runtime_test_c "${CMAKE_CURRENT_SOURCE_DIR}/total_arith_runtime_test.c")
target_include_directories(total_arith_runtime_test_c PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(total_arith_runtime_test_c PRIVATE
    -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion)
add_test(NAME total_arith_runtime_c COMMAND total_arith_runtime_test_c)

# ---------------------------------------------------------------------------
# The queue kind (SCE Protocol-Synthesis RFC §synth-5-P)
# ---------------------------------------------------------------------------
#
# The runtime's queues on their own (sce/forge/queue.h, queue_irq.h), under the
# same -Werror bar, against a hosted implementation of the `sce_atomic_*` and
# `sce_irq_*` symbols they are built on (sce_atomic_host.c, sce_irq_host.c: a
# bare-metal target supplies its own). What a generated header does with them is
# judged by queue_generated_test_c further down.
find_package(Threads REQUIRED)

add_library(sce_queue_host_intrinsics STATIC
    "${CMAKE_CURRENT_SOURCE_DIR}/sce_atomic_host.c"
    "${CMAKE_CURRENT_SOURCE_DIR}/sce_irq_host.c")
target_include_directories(sce_queue_host_intrinsics PUBLIC
    "${RUNTIME_INCLUDE_DIR}"
    "${CORE_RUNTIME_INCLUDE_DIR}"
    "${CMAKE_CURRENT_SOURCE_DIR}")
target_compile_options(sce_queue_host_intrinsics PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(sce_queue_host_intrinsics PUBLIC Threads::Threads)

# The contract scenarios, written as a C source from the JSON. C has no JSON
# reader within the backend's zero-dependency rule, and one written for a test
# would be a second reading of the contract; CMake reads JSON, so it states the
# scenarios to the compiler. Re-run when the contract changes.
set(QUEUE_CONTRACT_JSON "${REPO_ROOT}/tests/forge/conformance/queue_contract.json")
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${QUEUE_CONTRACT_JSON}")
file(READ "${QUEUE_CONTRACT_JSON}" _queue_contract)
string(JSON _queue_contract_version GET "${_queue_contract}" version)
if(NOT _queue_contract_version EQUAL 1)
    message(FATAL_ERROR "this arm reads version 1 of the queue contract, got ${_queue_contract_version}")
endif()
string(JSON _scenario_count LENGTH "${_queue_contract}" scenarios)
if(_scenario_count EQUAL 0)
    message(FATAL_ERROR "a queue contract with no scenarios checks nothing")
endif()
math(EXPR _scenario_last "${_scenario_count} - 1")
set(_queue_scenarios_c "/* Generated by CMakeLists.txt from tests/forge/conformance/queue_contract.json. */\n\n")
set(_queue_scenario_table "")
# The scenarios this arm runs.
set(_queue_scenarios_run 0)
foreach(_s RANGE ${_scenario_last})
    string(JSON _id GET "${_queue_contract}" scenarios ${_s} id)
    string(JSON _storage GET "${_queue_contract}" scenarios ${_s} storage)
    string(JSON _producers GET "${_queue_contract}" scenarios ${_s} producers)
    string(JSON _consumers GET "${_queue_contract}" scenarios ${_s} consumers)
    set(_segmented 0)
    set(_segments 0)
    if(_storage STREQUAL "bounded")
        string(JSON _capacity GET "${_queue_contract}" scenarios ${_s} capacity)
        set(_intrusive 0)
    elseif(_storage STREQUAL "segmented")
        # A segmented scenario has a segment length where a bounded one has a
        # capacity, and `segments` when its allocator refuses: how many segments it
        # gives in all, the first included. Zero means it never refuses.
        string(JSON _capacity GET "${_queue_contract}" scenarios ${_s} segment)
        string(JSON _segments ERROR_VARIABLE _segments_error GET "${_queue_contract}" scenarios ${_s} segments)
        if(_segments_error)
            set(_segments 0)
        endif()
        set(_intrusive 0)
        set(_segmented 1)
    elseif(_storage STREQUAL "intrusive")
        # An intrusive scenario has no capacity: it names the nodes the caller
        # owns, and `value` and `expect` are indices among them.
        string(JSON _capacity GET "${_queue_contract}" scenarios ${_s} nodes)
        set(_intrusive 1)
    else()
        message(FATAL_ERROR "the C11 arm has no runtime for the row ${_storage} (scenario ${_id})")
    endif()
    string(JSON _step_count LENGTH "${_queue_contract}" scenarios ${_s} steps)
    math(EXPR _step_last "${_step_count} - 1")
    string(APPEND _queue_scenarios_c "static const step_t g_steps_${_s}[] = {\n")
    foreach(_t RANGE ${_step_last})
        string(JSON _op GET "${_queue_contract}" scenarios ${_s} steps ${_t} op)
        if(_op STREQUAL "capacity")
            string(JSON _expect GET "${_queue_contract}" scenarios ${_s} steps ${_t} expect)
            string(APPEND _queue_scenarios_c "    {OP_CAPACITY, 0, ${_expect}},\n")
        elseif(_op STREQUAL "push")
            string(JSON _value GET "${_queue_contract}" scenarios ${_s} steps ${_t} value)
            string(JSON _expect GET "${_queue_contract}" scenarios ${_s} steps ${_t} expect)
            if(_expect STREQUAL "ok")
                set(_want 1)
            elseif(_expect STREQUAL "full")
                set(_want 0)
            elseif(_expect STREQUAL "out_of_memory")
                set(_want 2)
            else()
                message(FATAL_ERROR "scenario ${_id} step ${_t}: a push expects ok, full or out_of_memory, not ${_expect}")
            endif()
            string(APPEND _queue_scenarios_c "    {OP_PUSH, ${_value}, ${_want}},\n")
        elseif(_op STREQUAL "pop")
            string(JSON _expect_type TYPE "${_queue_contract}" scenarios ${_s} steps ${_t} expect)
            string(JSON _expect GET "${_queue_contract}" scenarios ${_s} steps ${_t} expect)
            if(_expect_type STREQUAL "STRING")
                if(NOT _expect STREQUAL "empty")
                    message(FATAL_ERROR "scenario ${_id} step ${_t}: a pop expects a number or empty, not ${_expect}")
                endif()
                set(_want -1)
            else()
                set(_want ${_expect})
            endif()
            string(APPEND _queue_scenarios_c "    {OP_POP, 0, ${_want}},\n")
        elseif(_op STREQUAL "destroy")
            string(APPEND _queue_scenarios_c "    {OP_DESTROY, 0, 0},\n")
        else()
            message(FATAL_ERROR "scenario ${_id} step ${_t}: unknown op ${_op}")
        endif()
    endforeach()
    string(APPEND _queue_scenarios_c "};\n\n")
    if(_producers STREQUAL "one")
        set(_p1 1)
    else()
        set(_p1 0)
    endif()
    if(_consumers STREQUAL "one")
        set(_c1 1)
    else()
        set(_c1 0)
    endif()
    string(APPEND _queue_scenario_table
        "    {\"${_id}\", ${_p1}, ${_c1}, ${_intrusive}, ${_segmented}, ${_capacity}u, ${_segments}u, g_steps_${_s}, ${_step_count}u},\n")
    math(EXPR _queue_scenarios_run "${_queue_scenarios_run} + 1")
endforeach()
string(APPEND _queue_scenarios_c
    "static const scenario_t g_scenarios[] = {\n${_queue_scenario_table}};\n\n"
    "static const size_t g_scenario_count = ${_queue_scenarios_run};\n")
set(QUEUE_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/queue_generated")
file(MAKE_DIRECTORY "${QUEUE_GENERATED_DIR}")
sce_codegen_write_input("${QUEUE_GENERATED_DIR}/queue_contract_scenarios.inc" "${_queue_scenarios_c}")

add_executable(queue_runtime_test_c "${CMAKE_CURRENT_SOURCE_DIR}/queue_runtime_test.c")
target_include_directories(queue_runtime_test_c PRIVATE "${QUEUE_GENERATED_DIR}")
target_compile_options(queue_runtime_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(queue_runtime_test_c PRIVATE sce_queue_host_intrinsics)
add_test(NAME queue_runtime_c COMMAND queue_runtime_test_c)

# Layer 2: the stress runs written as histories, judged by the command every
# backend's histories are judged by. 4 Lamport capacities recorded 3 times, and 6
# shapes recorded 25 times on each of the three queues that serve a `many` side.
add_executable(queue_history_recorder_c "${CMAKE_CURRENT_SOURCE_DIR}/queue_history_recorder.c")
target_compile_options(queue_history_recorder_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(queue_history_recorder_c PRIVATE sce_queue_host_intrinsics)
set(QUEUE_HISTORIES_DIR "${CMAKE_CURRENT_BINARY_DIR}/queue_histories")
add_test(NAME queue_history_record_c COMMAND queue_history_recorder_c "${QUEUE_HISTORIES_DIR}")
set_tests_properties(queue_history_record_c PROPERTIES FIXTURES_SETUP queue_histories_c)
add_test(NAME queue_history_check_c
    COMMAND "${CMAKE_COMMAND}"
            "-DCODEGEN=${SCE_CODEGEN}"
            "-DDIR=${QUEUE_HISTORIES_DIR}"
            -DMINIMUM=837
            -P "${REPO_ROOT}/tests/forge/conformance/check_queue_histories.cmake")
set_tests_properties(queue_history_check_c PROPERTIES FIXTURES_REQUIRED queue_histories_c)

# The same script over the corpus of histories sce-build keeps, which holds
# histories that are not linearizable: it must refuse them, saying so. A check that
# could only pass would prove nothing about the histories the arm wrote.
add_test(NAME queue_history_check_c_refuses
    COMMAND "${CMAKE_COMMAND}"
            "-DCODEGEN=${SCE_CODEGEN}"
            "-DDIR=${REPO_ROOT}/sce-build/tests/fixtures/queue_histories"
            -DMINIMUM=1
            -P "${REPO_ROOT}/tests/forge/conformance/check_queue_histories.cmake")
set_tests_properties(queue_history_check_c_refuses PROPERTIES
    PASS_REGULAR_EXPRESSION "queue histories refused")

# ---------------------------------------------------------------------------
# The broadcast-log kind (ADR 0062)
# ---------------------------------------------------------------------------
#
# The runtime's log on its own (sce/forge/broadcast_log.h), under the same -Werror
# bar and against the same hosted `sce_atomic_*` symbols as the queues: the unit
# tests of the Rust row, the refusals of the constructor, and real threads.
add_executable(broadcast_log_runtime_test_c "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_runtime_test.c")
target_compile_options(broadcast_log_runtime_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_runtime_test_c PRIVATE sce_queue_host_intrinsics)
add_test(NAME broadcast_log_runtime_c COMMAND broadcast_log_runtime_test_c)

# The narrow row's runtime (sce/forge/broadcast_log_narrow.h), for a target whose atomics are
# 32 bits wide or only loads and stores, under the same bar and the same hosted atomics. Two
# programs because the stamp's width is a compile-time parameter of one translation unit: the
# first sets a stamp of 3 bits, so that the wrap is a few publishes away, and the second keeps
# the 32 bits a row has, with the constructor's refusals and real threads.
add_executable(broadcast_log_narrow_runtime_test_c
    "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_narrow_runtime_test.c")
target_compile_options(broadcast_log_narrow_runtime_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_narrow_runtime_test_c PRIVATE sce_queue_host_intrinsics)
add_test(NAME broadcast_log_narrow_runtime_c COMMAND broadcast_log_narrow_runtime_test_c)

add_executable(broadcast_log_narrow_runtime_32_test_c
    "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_narrow_runtime_32_test.c")
target_compile_options(broadcast_log_narrow_runtime_32_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_narrow_runtime_32_test_c PRIVATE sce_queue_host_intrinsics)
add_test(NAME broadcast_log_narrow_runtime_32_c COMMAND broadcast_log_narrow_runtime_32_test_c)

# Layer 3: the stress runs written as histories, judged by the command every
# backend's broadcast-log histories are judged by. 5 shapes recorded 20 times.
add_executable(broadcast_log_history_recorder_c "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_history_recorder.c")
target_compile_options(broadcast_log_history_recorder_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_history_recorder_c PRIVATE sce_queue_host_intrinsics)
set(BROADCAST_HISTORIES_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_histories")
add_test(NAME broadcast_history_record_c COMMAND broadcast_log_history_recorder_c "${BROADCAST_HISTORIES_DIR}")
set_tests_properties(broadcast_history_record_c PROPERTIES FIXTURES_SETUP broadcast_histories_c)
add_test(NAME broadcast_history_check_c
    COMMAND "${CMAKE_COMMAND}"
            "-DCODEGEN=${SCE_CODEGEN}"
            "-DDIR=${BROADCAST_HISTORIES_DIR}"
            -DMINIMUM=100
            -P "${REPO_ROOT}/tests/forge/conformance/check_broadcast_histories.cmake")
set_tests_properties(broadcast_history_check_c PROPERTIES FIXTURES_REQUIRED broadcast_histories_c)

# The same script over the corpus of histories sce-build keeps, which holds
# histories that are not linearizable: it must refuse them, saying so.
add_test(NAME broadcast_history_check_c_refuses
    COMMAND "${CMAKE_COMMAND}"
            "-DCODEGEN=${SCE_CODEGEN}"
            "-DDIR=${REPO_ROOT}/sce-build/tests/fixtures/broadcast_histories"
            -DMINIMUM=1
            -P "${REPO_ROOT}/tests/forge/conformance/check_broadcast_histories.cmake")
set_tests_properties(broadcast_history_check_c_refuses PROPERTIES
    PASS_REGULAR_EXPRESSION "broadcast-log histories refused")

# The headers the generator writes for a queue, compiled and used.
#
# A C11 queue depends on what the target's atomics are, which the generator cannot
# see (the `sce_atomic_*` symbols are the platform's), so the deploy states it
# (`platform.atomic_rmw_width`) and `generate --deploy --target-machine` reads it.
# One deploy, three machines: a 64-bit target, a 32-bit one that states the delay
# its design relies on, and one with no read-modify-write atomic at all.
#
# The documents are derived from the conformance fixtures every other backend
# compiles, renamed (the header is named for the document) and, for the target
# with no atomics, declaring the `blocking` it can give: one set of documents in
# the tree, and the unchanged fixtures are what the refusal tests hand the
# generator.
set(QUEUE_C_DOCS_DIR "${CMAKE_CURRENT_BINARY_DIR}/queue_documents")
set(QUEUE_C_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/queue_headers")
file(MAKE_DIRECTORY "${QUEUE_C_DOCS_DIR}" "${QUEUE_C_GENERATED_DIR}")
set(QUEUE_C_DEPLOY "${QUEUE_C_DOCS_DIR}/deploy.yaml")
sce_codegen_write_input("${QUEUE_C_DEPLOY}" [=[
version: "1.0"
topology:
  ecu1:
    machines:
      mcu64:
        source: mcu64.scxml
        platform:
          class: mcu
          os: bare_metal
          core_count: 4
          atomic_rmw_width: 64
      mcu32:
        source: mcu32.scxml
        platform:
          class: mcu
          os: bare_metal
          core_count: 2
          atomic_rmw_width: 32
        queues:
          queue_c_scq32:
            min_wrap_ops: 100000000
      mcu0:
        source: mcu0.scxml
        platform:
          class: mcu
          os: bare_metal
          core_count: 1
          atomic_rmw_width: 0
      # The segmented queues take their segments from an allocator, so they need
      # a machine that has one (class ap, not mcu), and an LSCQ needs 64-bit atomics.
      ap64:
        source: ap64.scxml
        platform:
          class: ap
          os: linux
          core_count: 4
          atomic_rmw_width: 64
      ap32:
        source: ap32.scxml
        platform:
          class: ap
          os: linux
          core_count: 2
          atomic_rmw_width: 32
]=])

# A copy of the fixture under `name`, declaring `progress` when one is given.
function(sce_queue_variant fixture name progress)
    file(READ "${RESOURCE_DIR}/${fixture}.scxml" _text)
    string(REPLACE "name=\"${fixture}\"" "name=\"${name}\"" _text "${_text}")
    if(NOT progress STREQUAL "")
        string(REGEX REPLACE "<sce:progress>[a-z-]+</sce:progress>"
               "<sce:progress>${progress}</sce:progress>" _text "${_text}")
    endif()
    sce_codegen_write_input("${QUEUE_C_DOCS_DIR}/${name}.scxml" "${_text}")
endfunction()

sce_queue_variant(queue_conformance_spsc queue_c_spsc "")
sce_queue_variant(queue_conformance_scq queue_c_scq64 "")
sce_queue_variant(queue_conformance_scq queue_c_scq32 "")
sce_queue_variant(queue_conformance_spsc queue_c_irq_one "blocking")
sce_queue_variant(queue_conformance_scq queue_c_irq_many "blocking")
# The intrusive lists: over atomics (many producers, one consumer) and under the
# critical section of the target with none (many, many).
sce_queue_variant(queue_conformance_intrusive queue_c_intrusive "")
sce_queue_variant(queue_conformance_intrusive_many queue_c_intrusive_irq "")
# The segmented queues: linked Lamport rings (one producer, one consumer) and an
# LSCQ, each over an allocator the caller injects.
sce_queue_variant(queue_conformance_segmented queue_c_segmented "")
sce_queue_variant(queue_conformance_segmented_many queue_c_segmented_many "")

# The element every queue holds, then each queue for the machine whose atomics it
# is built for. The intrusive queues hold the same element, in the node their
# headers generate around it (ADR 0088). All of them are generated into one
# directory, and claim in one list.
set(QUEUE_C_HEADERS "")
sce_codegen_generation(_QUEUE_ELEMENT
    DOCUMENT "${RESOURCE_DIR}/queue_conformance_event.scxml"
    OUTPUT_DIR "${QUEUE_C_GENERATED_DIR}"
    LANGUAGE c11
    CLAIM SCE_QUEUE_C_DECLARED_OUTPUTS
    COMMENT "Generating C11 queue element queue_conformance_event")
list(APPEND QUEUE_C_HEADERS ${_QUEUE_ELEMENT_HEADERS})
foreach(_pair
        "queue_c_spsc:mcu64" "queue_c_scq64:mcu64" "queue_c_scq32:mcu32"
        "queue_c_irq_one:mcu0" "queue_c_irq_many:mcu0"
        "queue_c_intrusive:mcu64" "queue_c_intrusive_irq:mcu0"
        "queue_c_segmented:ap64" "queue_c_segmented_many:ap64")
    string(REPLACE ":" ";" _parts "${_pair}")
    list(GET _parts 0 _queue)
    list(GET _parts 1 _machine)
    sce_codegen_generation(_QUEUE
        DOCUMENT "${QUEUE_C_DOCS_DIR}/${_queue}.scxml"
        OUTPUT_DIR "${QUEUE_C_GENERATED_DIR}"
        LANGUAGE c11
        DEPLOY "${QUEUE_C_DEPLOY}"
        CLAIM SCE_QUEUE_C_DECLARED_OUTPUTS
        ARGS --target-machine ${_machine}
        COMMENT "Generating C11 queue ${_queue} for ${_machine}")
    list(APPEND QUEUE_C_HEADERS ${_QUEUE_HEADERS})
endforeach()
add_custom_target(generate_queue_headers_c)
target_sources(generate_queue_headers_c PRIVATE ${QUEUE_C_HEADERS})

add_executable(queue_generated_test_c "${CMAKE_CURRENT_SOURCE_DIR}/queue_generated_test.c")
add_dependencies(queue_generated_test_c generate_queue_headers_c)
target_include_directories(queue_generated_test_c PRIVATE "${QUEUE_C_GENERATED_DIR}")
target_compile_options(queue_generated_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(queue_generated_test_c PRIVATE sce_queue_host_intrinsics)
add_test(NAME queue_generated_c COMMAND queue_generated_test_c)

# The same headers consumed as C++: every generated C header opens `extern "C"`
# for a C++ consumer, and a construct valid in C only would turn one into a parse
# error that no C compiler reports. This project builds C, so the C++ language is
# enabled for this one translation unit, and only where a C++ compiler is.
include(CheckLanguage)
check_language(CXX)
if(CMAKE_CXX_COMPILER)
    enable_language(CXX)
    set(CMAKE_CXX_STANDARD 17)
    set(CMAKE_CXX_STANDARD_REQUIRED ON)
    set(CMAKE_CXX_EXTENSIONS OFF)
    add_executable(queue_generated_cxx_test_c "${CMAKE_CURRENT_SOURCE_DIR}/queue_generated_cxx_test.cpp")
    add_dependencies(queue_generated_cxx_test_c generate_queue_headers_c)
    target_include_directories(queue_generated_cxx_test_c PRIVATE "${QUEUE_C_GENERATED_DIR}")
    target_compile_options(queue_generated_cxx_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
    target_link_libraries(queue_generated_cxx_test_c PRIVATE sce_queue_host_intrinsics)
    add_test(NAME queue_generated_cxx_c COMMAND queue_generated_cxx_test_c)
endif()

# What the generator refuses is as much its contract as what it writes. Each of
# these hands it a document it must stop at, and the test passes only when the
# refusal says why.
function(sce_queue_refusal test_name expected_text)
    add_test(NAME ${test_name}
        COMMAND "${SCE_CODEGEN}" generate ${ARGN}
                --language c11 --output-dir "${CMAKE_CURRENT_BINARY_DIR}/queue_refused")
    set_tests_properties(${test_name} PROPERTIES PASS_REGULAR_EXPRESSION "${expected_text}")
endfunction()
# An SCQ row with no machine to read the atomics from.
sce_queue_refusal(queue_generate_c_scq_without_a_target "atomic_rmw_width"
    "${RESOURCE_DIR}/queue_conformance_scq.scxml")
# The same row for a machine whose deploy is the 32-bit target but a queue that
# states no delay (the fixture is not the one named in the deploy).
sce_queue_refusal(queue_generate_c_scq32_without_a_stated_delay "min_wrap_ops"
    "${RESOURCE_DIR}/queue_conformance_scq.scxml"
    --deploy "${QUEUE_C_DEPLOY}" --target-machine mcu32)
# `check` is held to the verdict `generate` reaches, and the document-set route
# (which `--deploy` puts it on) resolves the queue the same way.
add_test(NAME queue_check_c_scq32_without_a_stated_delay
    COMMAND "${SCE_CODEGEN}" check
            --forge "${RESOURCE_DIR}/queue_conformance_scq.scxml"
            --forge "${RESOURCE_DIR}/queue_conformance_event.scxml"
            --deploy "${QUEUE_C_DEPLOY}" --target-machine mcu32 --language c11)
set_tests_properties(queue_check_c_scq32_without_a_stated_delay PROPERTIES
    PASS_REGULAR_EXPRESSION "min_wrap_ops")
# A wait-free document for a target that can give blocking at most.
sce_queue_refusal(queue_generate_c_wait_free_without_atomics "cannot be met on the c11 backend"
    "${RESOURCE_DIR}/queue_conformance_spsc.scxml"
    --deploy "${QUEUE_C_DEPLOY}" --target-machine mcu0)
# A machine with a heap and no read-modify-write atomic.
sce_codegen_write_input("${QUEUE_C_DOCS_DIR}/deploy_no_atomics_heap.yaml" [=[
version: "1.0"
topology:
  ecu1:
    machines:
      ap0:
        source: ap0.scxml
        platform:
          class: ap
          os: linux
          core_count: 1
          atomic_rmw_width: 0
]=])
# Interrupt masking excludes nothing on another core.
sce_codegen_write_input("${QUEUE_C_DOCS_DIR}/deploy_two_cores.yaml" [=[
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_smp:
        source: mcu_smp.scxml
        platform:
          class: mcu
          os: bare_metal
          core_count: 2
          atomic_rmw_width: 0
]=])
sce_queue_refusal(queue_generate_c_no_atomics_across_cores "interrupt-masked critical section"
    "${QUEUE_C_DOCS_DIR}/queue_c_irq_one.scxml"
    --deploy "${QUEUE_C_DOCS_DIR}/deploy_two_cores.yaml" --target-machine mcu_smp)
# A segmented queue is bounded by an allocator, and a machine of class mcu has none.
sce_queue_refusal(queue_generate_c_segmented_where_nothing_allocates "storage is bounded by an injected allocator"
    "${QUEUE_C_DOCS_DIR}/queue_c_segmented.scxml"
    --deploy "${QUEUE_C_DEPLOY}" --target-machine mcu64)
# An LSCQ ring is closed by the top bit of a 64-bit tail, so a target whose widest
# atomic is 32 bits is refused by name.
sce_queue_refusal(queue_generate_c_lscq_on_32_bit_atomics "below 64 bits"
    "${QUEUE_C_DOCS_DIR}/queue_c_segmented_many.scxml"
    --deploy "${QUEUE_C_DEPLOY}" --target-machine ap32)
# Neither segmented row has a critical-section form.
sce_queue_refusal(queue_generate_c_segmented_without_atomics "no read-modify-write atomic"
    "${QUEUE_C_DOCS_DIR}/queue_c_segmented.scxml"
    --deploy "${QUEUE_C_DOCS_DIR}/deploy_no_atomics_heap.yaml" --target-machine ap0)
# The segmented queues need the target's atomic width stated, as an SCQ row does.
sce_queue_refusal(queue_generate_c_segmented_without_a_target "atomic_rmw_width"
    "${QUEUE_C_DOCS_DIR}/queue_c_segmented.scxml")

# ---------------------------------------------------------------------------
# The headers the generator writes for a broadcast log, compiled and used
# (ADR 0062)
# ---------------------------------------------------------------------------
#
# A log is generated from its document SET and not one document at a time, because
# a slot is sized from the element document's largest encoded size, which only the
# set compile reads (and a retention written as a deploy key resolves only against a
# deploy and the machine it is generated for). So `orchestrate` is the command, over
# the element documents and the logs together.
#
# One set is the documents every other backend compiles, untouched: two elements
# and two logs, the second log's retention the deploy limit `event_retention` of
# machine `mcu_node`. The other is derived from them in the build tree, as the
# queue's variants are: a log over `codec_cbor_map`, a CBOR map with a text and byte
# strings, to hold what a log of the literal document does not reach (an element a
# codec refuses, and values that own their bytes). The derived set is copied beside
# its log because `orchestrate` reads the documents of one set from one directory.
set(BLOG_C_DOCS_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_documents")
set(BLOG_C_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_headers")
file(MAKE_DIRECTORY "${BLOG_C_DOCS_DIR}" "${BLOG_C_GENERATED_DIR}")

set(BLOG_C_DEPLOY "${BLOG_C_DOCS_DIR}/deploy.yaml")
sce_codegen_write_input("${BLOG_C_DEPLOY}" [=[
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        limits:
          event_retention: 5
]=])

set(BLOG_C_RESOURCE_SET
    broadcast_log_conformance_event
    broadcast_log_conformance_record
    broadcast_log_conformance_literal
    broadcast_log_conformance_deployed)
set(BLOG_C_RESOURCE_DOCUMENTS "")
foreach(_document IN LISTS BLOG_C_RESOURCE_SET)
    list(APPEND BLOG_C_RESOURCE_DOCUMENTS "${RESOURCE_DIR}/${_document}.scxml")
endforeach()
# Both sets are generated into one directory, and claim in one list.
sce_codegen_generation(_BLOG_C_RESOURCE
    DOCUMENTS ${BLOG_C_RESOURCE_DOCUMENTS}
    OUTPUT_DIR "${BLOG_C_GENERATED_DIR}"
    LANGUAGE c11
    DEPLOY "${BLOG_C_DEPLOY}"
    CLAIM SCE_BLOG_C_DECLARED_OUTPUTS
    ARGS --target-machine mcu_node
    COMMENT "Generating C11 broadcast logs and their elements")

# The derived set: the literal document with its name and its element replaced.
configure_file("${RESOURCE_DIR}/codec_cbor_map.scxml" "${BLOG_C_DOCS_DIR}/codec_cbor_map.scxml" COPYONLY)
file(READ "${RESOURCE_DIR}/broadcast_log_conformance_literal.scxml" _blog_text)
string(REPLACE "name=\"broadcast_log_conformance_literal\"" "name=\"broadcast_log_c_cbor\"" _blog_text "${_blog_text}")
string(REPLACE "<sce:element-type>broadcast_log_conformance_event</sce:element-type>"
       "<sce:element-type>codec_cbor_map</sce:element-type>" _blog_text "${_blog_text}")
sce_codegen_write_input("${BLOG_C_DOCS_DIR}/broadcast_log_c_cbor.scxml" "${_blog_text}")
sce_codegen_generation(_BLOG_C_CBOR
    DOCUMENTS "${BLOG_C_DOCS_DIR}/codec_cbor_map.scxml"
              "${BLOG_C_DOCS_DIR}/broadcast_log_c_cbor.scxml"
    OUTPUT_DIR "${BLOG_C_GENERATED_DIR}"
    LANGUAGE c11
    CLAIM SCE_BLOG_C_DECLARED_OUTPUTS
    COMMENT "Generating C11 broadcast log over a CBOR element")
add_custom_target(generate_broadcast_log_headers_c)
target_sources(generate_broadcast_log_headers_c PRIVATE
    ${_BLOG_C_RESOURCE_HEADERS} ${_BLOG_C_CBOR_HEADERS})

add_executable(broadcast_log_generated_test_c "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_generated_test.c")
add_dependencies(broadcast_log_generated_test_c generate_broadcast_log_headers_c)
target_include_directories(broadcast_log_generated_test_c PRIVATE "${BLOG_C_GENERATED_DIR}")
target_compile_options(broadcast_log_generated_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_generated_test_c PRIVATE sce_queue_host_intrinsics)
add_test(NAME broadcast_log_generated_c COMMAND broadcast_log_generated_test_c)

# The narrow row (ADR 0062, "Stamp width"): the headers the generator writes for a log whose
# deploy names a target without 64-bit atomics. The generator chooses the row from the
# target of the machine's device, and judges the log against the stamp the row keeps, so the
# deploy also states the longest pause in publishes the design relies on. Two sets, one for
# each way the row hands out its one publisher: a target with a 32-bit compare-and-swap
# (thumbv7em), whose header has the publisher taken by it beside the ownership forms, and a
# target with loads and stores only (thumbv6m), whose header has the ownership forms alone.
# Each log is derived from the literal document, as the CBOR one above is, and generated
# beside a copy of its element document, because `orchestrate` reads a set from one directory.
set(BLOGN_C_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_narrow_headers")
file(MAKE_DIRECTORY "${BLOGN_C_GENERATED_DIR}")
file(READ "${RESOURCE_DIR}/broadcast_log_conformance_literal.scxml" _blogn_literal)

set(BLOGN_C_NARROW_DOCS_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_narrow_documents")
file(MAKE_DIRECTORY "${BLOGN_C_NARROW_DOCS_DIR}")
configure_file("${RESOURCE_DIR}/broadcast_log_conformance_record.scxml"
               "${BLOGN_C_NARROW_DOCS_DIR}/broadcast_log_conformance_record.scxml" COPYONLY)
string(REPLACE "name=\"broadcast_log_conformance_literal\"" "name=\"broadcast_log_c_narrow\"" _blogn_text "${_blogn_literal}")
string(REPLACE "<sce:element-type>broadcast_log_conformance_event</sce:element-type>"
       "<sce:element-type>broadcast_log_conformance_record</sce:element-type>" _blogn_text "${_blogn_text}")
sce_codegen_write_input("${BLOGN_C_NARROW_DOCS_DIR}/broadcast_log_c_narrow.scxml" "${_blogn_text}")
sce_codegen_write_input("${BLOGN_C_NARROW_DOCS_DIR}/deploy.yaml" [=[
version: "1.0"
topology:
  ecu1:
    target: thumbv7em-none-eabihf
    machines:
      mcu_node:
        source: mcu_node.scxml
        queues:
          broadcast_log_c_narrow:
            min_wrap_ops: 1000000
]=])
# The same deploy without the pause: the target says the stamp wraps, and the generator
# refuses the log rather than guess a bound.
sce_codegen_write_input("${BLOGN_C_NARROW_DOCS_DIR}/deploy_unstated.yaml" [=[
version: "1.0"
topology:
  ecu1:
    target: thumbv7em-none-eabihf
    machines:
      mcu_node:
        source: mcu_node.scxml
]=])

set(BLOGN_C_OWNERSHIP_DOCS_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_ownership_documents")
file(MAKE_DIRECTORY "${BLOGN_C_OWNERSHIP_DOCS_DIR}")
configure_file("${RESOURCE_DIR}/broadcast_log_conformance_event.scxml"
               "${BLOGN_C_OWNERSHIP_DOCS_DIR}/broadcast_log_conformance_event.scxml" COPYONLY)
string(REPLACE "name=\"broadcast_log_conformance_literal\"" "name=\"broadcast_log_c_ownership\"" _blogn_text "${_blogn_literal}")
sce_codegen_write_input("${BLOGN_C_OWNERSHIP_DOCS_DIR}/broadcast_log_c_ownership.scxml" "${_blogn_text}")
sce_codegen_write_input("${BLOGN_C_OWNERSHIP_DOCS_DIR}/deploy.yaml" [=[
version: "1.0"
topology:
  ecu1:
    target: thumbv6m-none-eabi
    machines:
      mcu_node:
        source: mcu_node.scxml
        queues:
          broadcast_log_c_ownership:
            min_wrap_ops: 1000000
]=])

# Both sets are generated into one directory, and claim in one list.
sce_codegen_generation(_BLOGN_C_NARROW
    DOCUMENTS "${BLOGN_C_NARROW_DOCS_DIR}/broadcast_log_conformance_record.scxml"
              "${BLOGN_C_NARROW_DOCS_DIR}/broadcast_log_c_narrow.scxml"
    OUTPUT_DIR "${BLOGN_C_GENERATED_DIR}"
    LANGUAGE c11
    DEPLOY "${BLOGN_C_NARROW_DOCS_DIR}/deploy.yaml"
    CLAIM SCE_BLOGN_C_DECLARED_OUTPUTS
    ARGS --target-machine mcu_node
    COMMENT "Generating the C11 broadcast log of a target with a 32-bit compare-and-swap")
sce_codegen_generation(_BLOGN_C_OWNERSHIP
    DOCUMENTS "${BLOGN_C_OWNERSHIP_DOCS_DIR}/broadcast_log_conformance_event.scxml"
              "${BLOGN_C_OWNERSHIP_DOCS_DIR}/broadcast_log_c_ownership.scxml"
    OUTPUT_DIR "${BLOGN_C_GENERATED_DIR}"
    LANGUAGE c11
    DEPLOY "${BLOGN_C_OWNERSHIP_DOCS_DIR}/deploy.yaml"
    CLAIM SCE_BLOGN_C_DECLARED_OUTPUTS
    ARGS --target-machine mcu_node
    COMMENT "Generating the C11 broadcast log of a target with loads and stores only")
add_custom_target(generate_broadcast_log_narrow_headers_c)
target_sources(generate_broadcast_log_narrow_headers_c PRIVATE
    ${_BLOGN_C_NARROW_HEADERS} ${_BLOGN_C_OWNERSHIP_HEADERS})

add_executable(broadcast_log_narrow_generated_test_c
    "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_narrow_generated_test.c")
add_dependencies(broadcast_log_narrow_generated_test_c generate_broadcast_log_narrow_headers_c)
target_include_directories(broadcast_log_narrow_generated_test_c PRIVATE "${BLOGN_C_GENERATED_DIR}")
target_compile_options(broadcast_log_narrow_generated_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_narrow_generated_test_c PRIVATE sce_queue_host_intrinsics)
add_test(NAME broadcast_log_narrow_generated_c COMMAND broadcast_log_narrow_generated_test_c)

# What the generator refuses of a narrow log is as much its contract as what it writes: the
# same set with no pause stated is refused, naming the key the deploy would state it under.
add_test(NAME broadcast_log_narrow_unstated_pause_c
    COMMAND "${SCE_CODEGEN}" orchestrate
            --forge "${BLOGN_C_NARROW_DOCS_DIR}/broadcast_log_conformance_record.scxml"
            --forge "${BLOGN_C_NARROW_DOCS_DIR}/broadcast_log_c_narrow.scxml"
            --language c11 --output-dir "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_narrow_refused"
            --deploy "${BLOGN_C_NARROW_DOCS_DIR}/deploy_unstated.yaml" --target-machine mcu_node)
set_tests_properties(broadcast_log_narrow_unstated_pause_c PROPERTIES
    PASS_REGULAR_EXPRESSION "queues.broadcast_log_c_narrow.min_wrap_ops")

# The logs over an element whose codec asks its caller for what its wire form does
# not carry (ADR 0062 section 4, "Caller inputs are part of the element"): one over a
# codec with a flag input, one over a variant that has no tag of its own. The six
# documents are the ones every backend's conformance lane reads, in one set, in the
# order that puts what a document imports before it; `orchestrate` reads a set from
# the one directory they are in.
set(BLOG_CONTEXT_DIR "${REPO_ROOT}/tests/forge/conformance/broadcast_log_context")
set(BLOG_CONTEXT_C_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_context_headers")
file(MAKE_DIRECTORY "${BLOG_CONTEXT_C_GENERATED_DIR}")
set(BLOG_CONTEXT_C_SET
    broadcast_log_conformance_flagged_event
    broadcast_log_conformance_flagged
    broadcast_log_conformance_choice_small
    broadcast_log_conformance_choice_large
    broadcast_log_conformance_choice
    broadcast_log_conformance_chosen)
set(BLOG_CONTEXT_C_DOCUMENTS "")
foreach(_document IN LISTS BLOG_CONTEXT_C_SET)
    list(APPEND BLOG_CONTEXT_C_DOCUMENTS "${BLOG_CONTEXT_DIR}/${_document}.scxml")
endforeach()
sce_codegen_generation(_BLOG_CONTEXT_C
    DOCUMENTS ${BLOG_CONTEXT_C_DOCUMENTS}
    OUTPUT_DIR "${BLOG_CONTEXT_C_GENERATED_DIR}"
    LANGUAGE c11
    COMMENT "Generating C11 broadcast logs over elements that ask their caller for inputs")
add_custom_target(generate_broadcast_log_context_headers_c)
target_sources(generate_broadcast_log_context_headers_c PRIVATE ${_BLOG_CONTEXT_C_HEADERS})

add_executable(broadcast_log_context_generated_test_c
    "${CMAKE_CURRENT_SOURCE_DIR}/broadcast_log_context_generated_test.c")
add_dependencies(broadcast_log_context_generated_test_c generate_broadcast_log_context_headers_c)
target_include_directories(broadcast_log_context_generated_test_c PRIVATE "${BLOG_CONTEXT_C_GENERATED_DIR}")
target_compile_options(broadcast_log_context_generated_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(broadcast_log_context_generated_test_c PRIVATE sce_queue_host_intrinsics)
add_test(NAME broadcast_log_context_generated_c COMMAND broadcast_log_context_generated_test_c)

# What the generator refuses is as much its contract as what it writes. One
# document handed to `generate` has no element document to read the size from and
# no deploy to read a retention from, so it is refused naming what is missing and
# is never given a guessed number.
add_test(NAME broadcast_log_generate_c_single_document
    COMMAND "${SCE_CODEGEN}" generate "${RESOURCE_DIR}/broadcast_log_conformance_literal.scxml"
            --language c11 --output-dir "${CMAKE_CURRENT_BINARY_DIR}/broadcast_log_refused")
set_tests_properties(broadcast_log_generate_c_single_document PROPERTIES
    PASS_REGULAR_EXPRESSION "ForgeCompileOptions::broadcast_log_resolutions")
# The same log with its element document beside it is a set, which `check` judges as
# `orchestrate` does: no refusal.
add_test(NAME broadcast_log_check_c_set
    COMMAND "${SCE_CODEGEN}" check
            --forge "${RESOURCE_DIR}/broadcast_log_conformance_event.scxml"
            --forge "${RESOURCE_DIR}/broadcast_log_conformance_literal.scxml"
            --language c11)
