# Phase C P1 — PositionMap unit-test family.
# claudedocs/rfc-sce-template-phase-c.md §3 P1 deliverable #4.
#
# Standing consumer for the PositionMap primitive (see
# `feedback_built_but_unconsumed.md`) until P2 wires
# `PugiXMLDocument::processSceTemplate` into the remap pipeline.

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

# Phase C P2 — TemplateExpander scanner-primitive unit tests.
# Standing consumer for `parsing/TemplateExpander.h` while the
# full recursive expansion lands in follow-up P2 commits.
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

# Phase X B1 — XIncludeExpander unit tests.
# Standing consumer for `parsing/XIncludeExpander.h` while
# Phase X B2 swaps `PugiXMLDocument::processXInclude` to use it
# (per `feedback_built_but_unconsumed.md`).
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

apply_sce_test_policies(diagnostic_test)

add_test(NAME DiagnosticTests COMMAND diagnostic_test)

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
