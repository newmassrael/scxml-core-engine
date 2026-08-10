# PositionMap unit-test family.
#
# Exercises every public method of the PositionMap primitive; the
# production consumers are `PugiXMLDocument::processXInclude` /
# `processSceTemplate`.

add_executable(position_map_test
    PositionMap_test.cpp
)

target_link_libraries(position_map_test
    PRIVATE
        sce_runtime
        sce_core
        GTest::GTest
        GTest::Main
        pthread
)

target_compile_features(position_map_test PRIVATE cxx_std_17)

apply_sce_test_policies(position_map_test)

add_test(NAME PositionMapTests COMMAND position_map_test)

# TemplateExpander scanner-primitive unit tests.
# Exercises `parsing/TemplateExpander.h`; the production consumer is
# `PugiXMLDocument::processSceTemplate` via `expandString`.
add_executable(template_expander_test
    TemplateExpander_test.cpp
)

target_link_libraries(template_expander_test
    PRIVATE
        sce_runtime
        sce_core
        GTest::GTest
        GTest::Main
        pthread
)

target_compile_features(template_expander_test PRIVATE cxx_std_17)

apply_sce_test_policies(template_expander_test)

add_test(NAME TemplateExpanderTests COMMAND template_expander_test)

# XIncludeExpander unit tests.
# Exercises `parsing/XIncludeExpander.h`; the production consumer is
# `PugiXMLDocument::processXInclude` via `expandStringX`.
add_executable(xinclude_expander_test
    XIncludeExpander_test.cpp
)

target_link_libraries(xinclude_expander_test
    PRIVATE
        sce_runtime
        sce_core
        GTest::GTest
        GTest::Main
        pthread
)

target_compile_features(xinclude_expander_test PRIVATE cxx_std_17)

apply_sce_test_policies(xinclude_expander_test)

add_test(NAME XIncludeExpanderTests COMMAND xinclude_expander_test)

# RFC §W1 — Diagnostic abstract base interface.
# Standing consumer for `parsing/Diagnostic.h` while the concrete
# `TemplateError` refit lands in the next W1 commit (per
# `feedback_built_but_unconsumed.md`).
add_executable(diagnostic_test
    Diagnostic_test.cpp
)

target_link_libraries(diagnostic_test
    PRIVATE
        sce_runtime
        sce_core
        GTest::GTest
        GTest::Main
        pthread
)

target_compile_features(diagnostic_test PRIVATE cxx_std_17)

# Path to the shared wire schema, so this suite can DERIVE the legal
# key set instead of restating it.
#
# The C++ producer is a second independent conformer to
# `schemas/sce-diagnostic.v1.schema.json` (see `parsing/Diagnostic.h`),
# and this suite used to encode that schema's envelope as hand-written
# `std::set` literals. A hand-written mirror only holds while someone
# remembers to update it: when `generator` became a required field, this
# suite noticed nothing, and a probe that added a required field the C++
# side does not emit still passed 66/66. Reading the schema makes the
# claim "this producer conforms" answerable by the file it conforms to.
#
# The header directory and this suite's own source, for the same reason
# one level up: which wire codes exist is a fact about the headers, and
# which ones this suite exercises is a fact about its source. Both were
# restated here as curated `std::array` lists whose only assertions were
# their own length and their own uniqueness — so a leaf that gained a
# code without gaining a test looked identical to one that had both, and
# `validation/missing-element` sat untested behind a list named
# "EveryCuratedCodeIsExercised".
target_compile_definitions(diagnostic_test
    PRIVATE
        SCE_DIAGNOSTIC_SCHEMA_PATH="${CMAKE_SOURCE_DIR}/schemas/sce-diagnostic.v1.schema.json"
        SCE_PARSING_HEADER_DIR="${CMAKE_SOURCE_DIR}/sce/include/parsing"
        SCE_DIAGNOSTIC_TEST_SOURCE="${CMAKE_CURRENT_SOURCE_DIR}/Diagnostic_test.cpp"
)

apply_sce_test_policies(diagnostic_test)

add_test(NAME DiagnosticTests COMMAND diagnostic_test)

# Cross-producer diagnostic-`id` parity.
#
# `id` is the dedup key (SCE_ERROR_CONTRACT.md §2.1) and this tree has
# two producers for it — the Rust `sce-codegen` CLI and the C++ runtime
# parser. The suite runs both over one fixture document and compares
# what they emit, so the "second independent conformer" claim in
# `parsing/Diagnostic.h` is answerable by a run rather than by prose.
#
# Spawns `sce-codegen` via popen, so it is excluded on WASM for the
# same reason `w3c_template_parity` is.
if(NOT EMSCRIPTEN)
    add_executable(cross_producer_diagnostic_id_test
        CrossProducerDiagnosticId_test.cpp
    )

    target_link_libraries(cross_producer_diagnostic_id_test
        PRIVATE
            sce_runtime
            sce_core
            GTest::GTest
            GTest::Main
            pthread
    )

    target_compile_features(cross_producer_diagnostic_id_test PRIVATE cxx_std_17)

    # The header directory, so the suite can DERIVE which diagnostic
    # leaves exist instead of restating them: a leaf added without a
    # fixture is then red, which is the order the contract wants.
    target_compile_definitions(cross_producer_diagnostic_id_test
        PRIVATE
            SCE_PARSING_HEADER_DIR="${CMAKE_SOURCE_DIR}/sce/include/parsing"
    )

    apply_sce_test_policies(cross_producer_diagnostic_id_test)

    add_test(NAME CrossProducerDiagnosticIdTests COMMAND cross_producer_diagnostic_id_test)
    set_tests_properties(CrossProducerDiagnosticIdTests PROPERTIES
        ENVIRONMENT
            "SCE_CODEGEN_BIN=${SCE_CODEGEN};SCE_CROSS_PRODUCER_FIXTURES_ROOT=${CMAKE_CURRENT_SOURCE_DIR}/fixtures/cross_producer"
    )
endif()

# Regression test for `PugiXMLElement::serializeChildContent` xmlns
# propagation. Pins the W3C Test_338 / Test_347 / Test_530 interpreter
# failure surfaced when `a46d2c27` flipped `isScxmlNamespace` strict.
# Without this serializer fix, `<invoke><content><scxml>` payloads
# lose their inherited default xmlns at the serialization boundary
# and the child-session re-parse rejects them as `ParseWrongRootElement`.
add_executable(serialize_child_content_xmlns_test
    SerializeChildContentXmlns_test.cpp
)

target_link_libraries(serialize_child_content_xmlns_test
    PRIVATE
        sce_runtime
        sce_core
        GTest::GTest
        GTest::Main
        pthread
)

target_compile_features(serialize_child_content_xmlns_test PRIVATE cxx_std_17)

apply_sce_test_policies(serialize_child_content_xmlns_test)

add_test(NAME SerializeChildContentXmlnsTests COMMAND serialize_child_content_xmlns_test)

# Child-state collection order. §scxml-3.2 / §scxml-3.3 read the default
# initial state off "the first child state in document order", so a
# per-element-name collection (every <state>, then <parallel>, then
# <final>, then <history>) picks the wrong one whenever a document does
# not write its <state> children first.
add_executable(child_state_document_order_test
    ChildStateDocumentOrder_test.cpp
)

target_link_libraries(child_state_document_order_test
    PRIVATE
        sce_runtime
        sce_core
        GTest::GTest
        GTest::Main
        pthread
)

target_compile_features(child_state_document_order_test PRIVATE cxx_std_17)

apply_sce_test_policies(child_state_document_order_test)

add_test(NAME ChildStateDocumentOrderTests COMMAND child_state_document_order_test)
