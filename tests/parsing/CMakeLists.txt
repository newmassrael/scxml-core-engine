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
