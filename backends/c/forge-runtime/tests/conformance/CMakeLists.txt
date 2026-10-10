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
# Invokes sce-codegen at configure time on each fixture SCXML
# and writes the generated C headers into ${CMAKE_CURRENT_BINARY_DIR}/
# generated/, which is added to the test target's include path. The
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

# Resolve the generator. The search lives in cmake/SCEFindCodegen.cmake
# so every harness finds it the same way and none of them names a
# build profile; it aborts with the build command when no profile
# holds a binary, which is what the hand-written check here used to do.
include("${REPO_ROOT}/cmake/SCEFindCodegen.cmake")

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

# RFC §synth-5-B B2-test-vector: ask sce-codegen for the subset of C11
# fixtures whose SCXML carries `<sce:test-vector>` rows. Each such
# fixture emits `<fixture>_test.h` alongside `<fixture>.h`, so the
# per-fixture custom_command must declare both as OUTPUT for cmake's
# dependency tracker to pick them up. The harness template includes
# the sidecar conditionally on `f.has_test_vectors`.
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
            --language c11
            --has-test-vectors
            --resource-dir "${RESOURCE_DIR}"
    OUTPUT_VARIABLE C11_FIXTURES_WITH_TV
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _tv_rc
)
if(NOT _tv_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures --has-test-vectors --language c11 failed for "
        "${FIXTURE_MANIFEST}")
endif()

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

# Step 1: per-fixture product code generation. Each fixture's SCXML is
# fed to sce-codegen --language c11 to produce a single header. RFC §synth-5-B
# B2-test-vector: fixtures whose SCXML carries `<sce:test-vector>` rows
# emit a sibling `<fixture>_test.h` sidecar; the OUTPUT list is widened
# so cmake's dependency tracker rebuilds the harness when either file
# moves.
set(GENERATED_HEADERS "")
list(LENGTH C11_FIXTURES _fixture_count)
foreach(_fixture_index RANGE ${_fixture_count})
    if(_fixture_index EQUAL _fixture_count)
        break()
    endif()
    list(GET C11_FIXTURES ${_fixture_index} fixture)
    list(GET C11_FIXTURE_DOCUMENTS ${_fixture_index} scxml)
    set(out_header "${GENERATED_DIR}/${fixture}.h")
    # A standard document is in the generator, which is already a
    # prerequisite; naming it here would name a file that never exists.
    set(_document_deps "${scxml}")
    if(scxml MATCHES "^sce:std/")
        set(_document_deps "")
    endif()
    set(_outputs "${out_header}")
    list(FIND C11_FIXTURES_WITH_TV "${fixture}" _tv_idx)
    if(NOT _tv_idx EQUAL -1)
        set(_sidecar "${GENERATED_DIR}/${fixture}_test.h")
        list(APPEND _outputs "${_sidecar}")
    endif()
    # DEPFILE carries the ~120 jinja2 templates that rendered the output;
    # `DEPENDS` alone names only the SCXML, so a template edit would
    # leave the fixture stale and the build would reuse it silently.
    add_custom_command(
        OUTPUT ${_outputs}
        COMMAND "${SCE_CODEGEN}" generate "${scxml}"
                --language c11
                --output-dir "${GENERATED_DIR}"
                --write-deps "${GENERATED_DIR}/${fixture}.h.d"
        DEPENDS ${_document_deps} "${SCE_CODEGEN}"
        DEPFILE "${GENERATED_DIR}/${fixture}.h.d"
        COMMENT "Generating C11 fixture ${fixture}.h"
        VERBATIM
    )
    list(APPEND GENERATED_HEADERS ${_outputs})
endforeach()

# Step 2: render the test harness from the shared template. The
# scaffold pulls in per-kind fragments via dynamic Jinja2 include, and
# folds every fixture document under tests/forge/resources into the
# harness `source-hash` — so the generator states its own prerequisites
# through a DEPFILE, as every other codegen step in the tree does. The
# `file(GLOB ... CONFIGURE_DEPENDS)` this replaced named the fragments
# the scaffold includes directly and nothing they pull in, and named no
# fixture document at all.
set(HARNESS_SRC "${GENERATED_DIR}/numerical_conformance_test.c")
add_custom_command(
    OUTPUT "${HARNESS_SRC}"
    COMMAND "${SCE_CODEGEN}" generate-conformance
            --language c11
            --manifest "${FIXTURE_MANIFEST}"
            --output-dir "${GENERATED_DIR}"
            --write-deps "${HARNESS_SRC}.d"
    DEPENDS "${FIXTURE_MANIFEST}"
            "${REFERENCE_JSON}"
            "${SCE_CODEGEN}"
    DEPFILE "${HARNESS_SRC}.d"
    COMMENT "Generating C11 conformance harness from template"
    VERBATIM
)

add_custom_target(generate_forge_fixtures_c
    DEPENDS ${GENERATED_HEADERS} "${HARNESS_SRC}"
)

add_executable(numerical_conformance_test_c "${HARNESS_SRC}")
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

# The runtime's CBOR items on their own (sce/forge/cbor.h), under the same
# -Werror bar; what a generated CBOR codec does with them is judged by the
# harness above (codec_cbor_map).
add_executable(cbor_runtime_test_c "${CMAKE_CURRENT_SOURCE_DIR}/cbor_runtime_test.c")
target_include_directories(cbor_runtime_test_c PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(cbor_runtime_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME cbor_runtime_c COMMAND cbor_runtime_test_c)

# The runtime's content lines on their own (sce/forge/content_line.h), under the
# same -Werror bar; what a generated content-line codec does with them is judged
# by the harness above (codec_content_line_event).
add_executable(content_line_runtime_test_c "${CMAKE_CURRENT_SOURCE_DIR}/content_line_runtime_test.c")
target_include_directories(content_line_runtime_test_c PRIVATE "${RUNTIME_INCLUDE_DIR}")
target_compile_options(content_line_runtime_test_c PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME content_line_runtime_c COMMAND content_line_runtime_test_c)

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
# The scenarios this arm runs: the contract holds rows it does not lower yet.
set(_queue_scenarios_run 0)
foreach(_s RANGE ${_scenario_last})
    string(JSON _id GET "${_queue_contract}" scenarios ${_s} id)
    string(JSON _storage GET "${_queue_contract}" scenarios ${_s} storage)
    string(JSON _producers GET "${_queue_contract}" scenarios ${_s} producers)
    string(JSON _consumers GET "${_queue_contract}" scenarios ${_s} consumers)
    if(_storage STREQUAL "bounded")
        string(JSON _capacity GET "${_queue_contract}" scenarios ${_s} capacity)
        set(_intrusive 0)
    elseif(_storage STREQUAL "segmented")
        # Not lowered to C11 yet; the generator refuses the row by name
        # (queue/storage-runtime-missing), so there is no runtime to run.
        continue()
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
            else()
                message(FATAL_ERROR "scenario ${_id} step ${_t}: a push expects ok or full, not ${_expect}")
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
        "    {\"${_id}\", ${_p1}, ${_c1}, ${_intrusive}, ${_capacity}u, g_steps_${_s}, ${_step_count}u},\n")
    math(EXPR _queue_scenarios_run "${_queue_scenarios_run} + 1")
endforeach()
string(APPEND _queue_scenarios_c
    "static const scenario_t g_scenarios[] = {\n${_queue_scenario_table}};\n\n"
    "static const size_t g_scenario_count = ${_queue_scenarios_run};\n")
set(QUEUE_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/queue_generated")
file(MAKE_DIRECTORY "${QUEUE_GENERATED_DIR}")
file(WRITE "${QUEUE_GENERATED_DIR}/queue_contract_scenarios.inc" "${_queue_scenarios_c}")

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
            -DMINIMUM=712
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
file(WRITE "${QUEUE_C_DEPLOY}" [=[
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
]=])

# A copy of the fixture under `name`, declaring `progress` when one is given.
function(sce_queue_variant fixture name progress)
    file(READ "${RESOURCE_DIR}/${fixture}.scxml" _text)
    string(REPLACE "name=\"${fixture}\"" "name=\"${name}\"" _text "${_text}")
    if(NOT progress STREQUAL "")
        string(REGEX REPLACE "<sce:progress>[a-z-]+</sce:progress>"
               "<sce:progress>${progress}</sce:progress>" _text "${_text}")
    endif()
    file(WRITE "${QUEUE_C_DOCS_DIR}/${name}.scxml" "${_text}")
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

# The elements every queue holds, then each queue for the machine whose atomics it
# is built for. The intrusive queues hold a node that carries its own link.
set(QUEUE_C_HEADERS "")
foreach(_element queue_conformance_event queue_conformance_node)
    add_custom_command(
        OUTPUT "${QUEUE_C_GENERATED_DIR}/${_element}.h"
        COMMAND "${SCE_CODEGEN}" generate "${RESOURCE_DIR}/${_element}.scxml"
                --language c11 --output-dir "${QUEUE_C_GENERATED_DIR}"
                --write-deps "${QUEUE_C_GENERATED_DIR}/${_element}.h.d"
        DEPENDS "${RESOURCE_DIR}/${_element}.scxml" "${SCE_CODEGEN}"
        DEPFILE "${QUEUE_C_GENERATED_DIR}/${_element}.h.d"
        COMMENT "Generating C11 queue element ${_element}.h"
        VERBATIM)
    list(APPEND QUEUE_C_HEADERS "${QUEUE_C_GENERATED_DIR}/${_element}.h")
endforeach()
foreach(_pair
        "queue_c_spsc:mcu64" "queue_c_scq64:mcu64" "queue_c_scq32:mcu32"
        "queue_c_irq_one:mcu0" "queue_c_irq_many:mcu0"
        "queue_c_intrusive:mcu64" "queue_c_intrusive_irq:mcu0")
    string(REPLACE ":" ";" _parts "${_pair}")
    list(GET _parts 0 _queue)
    list(GET _parts 1 _machine)
    add_custom_command(
        OUTPUT "${QUEUE_C_GENERATED_DIR}/${_queue}.h"
        COMMAND "${SCE_CODEGEN}" generate "${QUEUE_C_DOCS_DIR}/${_queue}.scxml"
                --language c11 --output-dir "${QUEUE_C_GENERATED_DIR}"
                --deploy "${QUEUE_C_DEPLOY}" --target-machine ${_machine}
                --write-deps "${QUEUE_C_GENERATED_DIR}/${_queue}.h.d"
        DEPENDS "${QUEUE_C_DOCS_DIR}/${_queue}.scxml" "${QUEUE_C_DEPLOY}" "${SCE_CODEGEN}"
        DEPFILE "${QUEUE_C_GENERATED_DIR}/${_queue}.h.d"
        COMMENT "Generating C11 queue ${_queue}.h for ${_machine}"
        VERBATIM)
    list(APPEND QUEUE_C_HEADERS "${QUEUE_C_GENERATED_DIR}/${_queue}.h")
endforeach()
add_custom_target(generate_queue_headers_c DEPENDS ${QUEUE_C_HEADERS})

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
# Interrupt masking excludes nothing on another core.
file(WRITE "${QUEUE_C_DOCS_DIR}/deploy_two_cores.yaml" [=[
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
